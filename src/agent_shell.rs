use crate::bootstrap;
use crate::config::{Config, ModelPreference};
use crate::domain;
use crate::execution_runtime::{self, WorkflowRunResult};
use crate::sessions::{self, SessionId, SessionReplay, SessionSummary};
use crate::shared_state::{self, SharedStateKey, SharedStateRecord};
use crate::state_sql;
use crate::workflow_registry::{self, WorkflowId};
use anyhow::{Context, Result, anyhow};
use serde::Serialize;
use std::io::{self, Write};
use std::path::Path;

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MOCK_RESEARCH_WORKFLOW: &str = "workflow:mock-research-brief";
const RECENT_SESSION_LIMIT: usize = 5;
const RECENT_STATE_LIMIT: usize = 5;

#[derive(Debug, Clone, PartialEq, Eq)]
enum AgentShellCommand {
    Help,
    Doctor,
    RunDemo,
    RunWorkflow(String),
    StateSummary,
    StateList { json: bool },
    StateShow(String),
    SessionRecent,
    SessionList { json: bool },
    SessionShow(String),
    SessionReplay(String),
    ConfigShow,
    Exit,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct AgentShellResponse {
    output: String,
    should_exit: bool,
}

#[derive(Debug, Clone, Serialize)]
struct AgentDoctorReport {
    config_exists: bool,
    workspace_exists: bool,
    state_path_exists: bool,
    session_path_exists: bool,
    workflow_run_ok: bool,
    shared_state_list_ok: bool,
    session_list_ok: bool,
    generated_session_id: Option<String>,
}

pub fn run(cfg: &Config, config_path: &Path) -> Result<()> {
    println!("{}", startup_summary(cfg)?);

    let stdin = io::stdin();
    loop {
        print!("quant-m> ");
        io::stdout().flush().context("failed to flush prompt")?;

        let mut line = String::new();
        let read = stdin
            .read_line(&mut line)
            .context("failed to read shell input")?;
        if read == 0 {
            println!();
            break;
        }

        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let command = match parse_command(trimmed) {
            Ok(command) => command,
            Err(err) => {
                println!("error: {err}");
                continue;
            }
        };

        match execute_command(cfg, config_path, command) {
            Ok(response) => {
                if !response.output.is_empty() {
                    println!("{}", response.output);
                }
                if response.should_exit {
                    break;
                }
            }
            Err(err) => {
                println!("error: {err}");
            }
        }
    }

    Ok(())
}

fn startup_summary(cfg: &Config) -> Result<String> {
    let domain_count = domain::builtin_registry()?.list().len();
    let workflow_count = workflow_registry::builtin_registry()?.list(None).len();
    let session_count = sessions::list_sessions(cfg)?.len();
    let shared_state_count = shared_state::list_state(cfg, None)?.len();

    Ok(format!(
        "Quant-M Agent Shell v{APP_VERSION}\nmode: operator_shell\nworkspace: {}\nruntime_profile: {}\nnetwork: {}\npreferred_local_model: {}\npreferred_openrouter_model: {}\ndomains: {} | workflows: {} | sessions: {} | shared_state: {}\nhint: type help",
        cfg.workspace_dir.display(),
        format!("{:?}", cfg.runtime.profile).to_lowercase(),
        if cfg.runtime.external_network_enabled {
            "enabled"
        } else {
            "disabled"
        },
        format_model_preference(cfg.preferences.preferred_local_model.as_ref()),
        cfg.preferences
            .preferred_openrouter_model
            .as_deref()
            .unwrap_or("unset"),
        domain_count,
        workflow_count,
        session_count,
        shared_state_count,
    ))
}

fn parse_command(input: &str) -> Result<AgentShellCommand> {
    let trimmed = input.trim();
    let parts = trimmed.split_whitespace().collect::<Vec<_>>();
    match parts.as_slice() {
        ["help"] => Ok(AgentShellCommand::Help),
        ["doctor"] => Ok(AgentShellCommand::Doctor),
        ["run", "demo"] | ["run", "mock-research"] => Ok(AgentShellCommand::RunDemo),
        ["run", "workflow", workflow_id] => {
            Ok(AgentShellCommand::RunWorkflow((*workflow_id).to_string()))
        }
        ["state", "summary"] => Ok(AgentShellCommand::StateSummary),
        ["state", "list"] => Ok(AgentShellCommand::StateList { json: false }),
        ["state", "list", "--json"] => Ok(AgentShellCommand::StateList { json: true }),
        ["state", "show", key] => Ok(AgentShellCommand::StateShow((*key).to_string())),
        ["session", "recent"] => Ok(AgentShellCommand::SessionRecent),
        ["session", "list"] => Ok(AgentShellCommand::SessionList { json: false }),
        ["session", "list", "--json"] => Ok(AgentShellCommand::SessionList { json: true }),
        ["session", "show", session_id] => {
            Ok(AgentShellCommand::SessionShow((*session_id).to_string()))
        }
        ["session", "replay", session_id] => {
            Ok(AgentShellCommand::SessionReplay((*session_id).to_string()))
        }
        ["config", "show"] => Ok(AgentShellCommand::ConfigShow),
        ["exit"] | ["quit"] => Ok(AgentShellCommand::Exit),
        _ => Err(anyhow!(
            "unknown command '{}'. type help for supported commands",
            trimmed
        )),
    }
}

#[cfg(feature = "fuzzing_hooks")]
pub fn parse_command_for_fuzz(input: &str) -> Result<&'static str> {
    let command = parse_command(input)?;
    let label = match command {
        AgentShellCommand::Help => "help",
        AgentShellCommand::Doctor => "doctor",
        AgentShellCommand::RunDemo => "run_demo",
        AgentShellCommand::RunWorkflow(_) => "run_workflow",
        AgentShellCommand::StateSummary => "state_summary",
        AgentShellCommand::StateList { json: false } => "state_list",
        AgentShellCommand::StateList { json: true } => "state_list_json",
        AgentShellCommand::StateShow(_) => "state_show",
        AgentShellCommand::SessionRecent => "session_recent",
        AgentShellCommand::SessionList { json: false } => "session_list",
        AgentShellCommand::SessionList { json: true } => "session_list_json",
        AgentShellCommand::SessionShow(_) => "session_show",
        AgentShellCommand::SessionReplay(_) => "session_replay",
        AgentShellCommand::ConfigShow => "config_show",
        AgentShellCommand::Exit => "exit",
    };
    Ok(label)
}

fn execute_command(
    cfg: &Config,
    config_path: &Path,
    command: AgentShellCommand,
) -> Result<AgentShellResponse> {
    match command {
        AgentShellCommand::Help => Ok(AgentShellResponse {
            output: help_text().to_string(),
            should_exit: false,
        }),
        AgentShellCommand::Doctor => {
            let report = run_doctor(cfg, config_path)?;
            Ok(AgentShellResponse {
                output: format_doctor_report(&report),
                should_exit: false,
            })
        }
        AgentShellCommand::RunDemo => {
            let workflow_id = WorkflowId::new(MOCK_RESEARCH_WORKFLOW);
            let result = execution_runtime::run_workflow(cfg, &workflow_id)?;
            Ok(AgentShellResponse {
                output: format_run_result(&result, true),
                should_exit: false,
            })
        }
        AgentShellCommand::RunWorkflow(workflow_id) => {
            let workflow_id = workflow_id.parse::<WorkflowId>()?;
            let result = execution_runtime::run_workflow(cfg, &workflow_id)?;
            Ok(AgentShellResponse {
                output: format_run_result(&result, false),
                should_exit: false,
            })
        }
        AgentShellCommand::StateSummary => {
            let summary = state_sql::summary(cfg)?;
            let current_records = shared_state::list_state(cfg, None)?.len();
            Ok(AgentShellResponse {
                output: format_state_summary(current_records, &summary),
                should_exit: false,
            })
        }
        AgentShellCommand::StateList { json } => {
            let records = shared_state::list_state(cfg, None)?;
            Ok(AgentShellResponse {
                output: if json {
                    serde_json::to_string_pretty(&records)?
                } else {
                    format_state_list_compact(&records)
                },
                should_exit: false,
            })
        }
        AgentShellCommand::StateShow(key) => {
            let key = key.parse::<SharedStateKey>()?;
            let record = shared_state::show_state(cfg, &key)?;
            Ok(AgentShellResponse {
                output: format_state_show(&key, record.as_ref())?,
                should_exit: false,
            })
        }
        AgentShellCommand::SessionRecent => {
            let summaries = sessions::list_sessions(cfg)?;
            Ok(AgentShellResponse {
                output: format_session_recent(cfg, &summaries)?,
                should_exit: false,
            })
        }
        AgentShellCommand::SessionList { json } => {
            let summaries = sessions::list_sessions(cfg)?;
            Ok(AgentShellResponse {
                output: if json {
                    serde_json::to_string_pretty(&summaries)?
                } else {
                    format_session_recent(cfg, &summaries)?
                },
                should_exit: false,
            })
        }
        AgentShellCommand::SessionShow(session_id) => {
            let session_id = session_id.parse::<SessionId>()?;
            let detail = sessions::show_session(cfg, &session_id)?;
            let replay = sessions::replay_session(cfg, &session_id)?;
            Ok(AgentShellResponse {
                output: format_session_show(&detail.summary, &replay),
                should_exit: false,
            })
        }
        AgentShellCommand::SessionReplay(session_id) => {
            let session_id = session_id.parse::<SessionId>()?;
            let replay = sessions::replay_session(cfg, &session_id)?;
            Ok(AgentShellResponse {
                output: serde_json::to_string_pretty(&replay)?,
                should_exit: false,
            })
        }
        AgentShellCommand::ConfigShow => {
            let cfg = Config::load_or_create(config_path)?;
            Ok(AgentShellResponse {
                output: cfg.render_toml(config_path)?,
                should_exit: false,
            })
        }
        AgentShellCommand::Exit => Ok(AgentShellResponse {
            output: "bye".to_string(),
            should_exit: true,
        }),
    }
}

fn help_text() -> &'static str {
    "Quant-M Agent Shell Commands

Overview:
  help
  doctor
  config show

Run:
  run demo
  run mock-research
  run workflow workflow:mock-research-brief

State:
  state summary
  state list
  state list --json
  state show shared.research.summary

Sessions:
  session recent
  session list
  session list --json
  session show <session_id>
  session replay <session_id>

Exit:
  quit
  exit"
}

fn run_doctor(cfg: &Config, config_path: &Path) -> Result<AgentDoctorReport> {
    if !config_path.exists() {
        return Ok(AgentDoctorReport {
            config_exists: false,
            workspace_exists: cfg.workspace_dir.exists(),
            state_path_exists: false,
            session_path_exists: cfg.runtime.session_dir.exists(),
            workflow_run_ok: false,
            shared_state_list_ok: shared_state::list_state(cfg, None).is_ok(),
            session_list_ok: sessions::list_sessions(cfg).is_ok(),
            generated_session_id: None,
        });
    }

    bootstrap::ensure_workspace(cfg)?;
    let workflow_run =
        execution_runtime::run_workflow(cfg, &WorkflowId::new(MOCK_RESEARCH_WORKFLOW));

    Ok(AgentDoctorReport {
        config_exists: true,
        workspace_exists: cfg.workspace_dir.exists(),
        state_path_exists: cfg
            .state_sql
            .sqlite_path
            .parent()
            .is_some_and(|parent| parent.exists()),
        session_path_exists: cfg.runtime.session_dir.exists(),
        workflow_run_ok: workflow_run.is_ok(),
        shared_state_list_ok: shared_state::list_state(cfg, None).is_ok(),
        session_list_ok: sessions::list_sessions(cfg).is_ok(),
        generated_session_id: workflow_run
            .ok()
            .map(|result: WorkflowRunResult| result.session_id.to_string()),
    })
}

fn format_doctor_report(report: &AgentDoctorReport) -> String {
    format!(
        "Doctor

Paths:
  [{}] config
  [{}] workspace
  [{}] state path
  [{}] session path

Checks:
  [{}] workflow run
  [{}] shared-state list
  [{}] session list

Artifacts:
  generated_session_id: {}",
        pass_fail(report.config_exists),
        pass_fail(report.workspace_exists),
        pass_fail(report.state_path_exists),
        pass_fail(report.session_path_exists),
        pass_fail(report.workflow_run_ok),
        pass_fail(report.shared_state_list_ok),
        pass_fail(report.session_list_ok),
        report.generated_session_id.as_deref().unwrap_or("none")
    )
}

fn format_run_result(result: &WorkflowRunResult, demo_alias: bool) -> String {
    let alias_note = if demo_alias {
        "alias: run demo -> workflow:mock-research-brief\n".to_string()
    } else {
        String::new()
    };
    let writes = if result.shared_state_writes.is_empty() {
        "none".to_string()
    } else {
        result.shared_state_writes.join(", ")
    };
    format!(
        "Workflow run complete

{alias_note}workflow_id: {}
status: {}
steps_completed: {}
shared_state_writes: {}
session_id: {}
next: state summary | session replay {}",
        result.workflow_id,
        result.status,
        result.steps_completed,
        writes,
        result.session_id,
        result.session_id
    )
}

fn format_state_summary(current_records: usize, summary: &state_sql::StateSummary) -> String {
    format!(
        "State Summary

current_shared_state_records: {}
shared_signals: {}
desk_handoffs: {}
risk_reviews: {}
paper_orders: {}
sqlite_db: {}",
        current_records,
        summary.shared_signals,
        summary.desk_handoffs,
        summary.risk_reviews,
        summary.paper_orders,
        summary.db_path
    )
}

fn format_state_list_compact(records: &[SharedStateRecord]) -> String {
    let mut lines = vec![format!(
        "State records (showing {} of {})",
        records.len().min(RECENT_STATE_LIMIT),
        records.len()
    )];
    if records.is_empty() {
        lines.push("- no shared-state records".to_string());
    } else {
        for record in records.iter().take(RECENT_STATE_LIMIT) {
            lines.push(format!(
                "- {} | domain={} | updated={} | session={}",
                record.key,
                record.domain_id,
                record.updated_at,
                record
                    .session_id
                    .as_ref()
                    .map(|id| id.as_str())
                    .unwrap_or("none")
            ));
        }
    }
    lines.push("next: state show <key> | state list --json".to_string());
    lines.join("\n")
}

fn format_state_show(key: &SharedStateKey, record: Option<&SharedStateRecord>) -> Result<String> {
    match record {
        Some(record) => Ok(serde_json::to_string_pretty(record)?),
        None => Ok(format!("state key '{}' not found", key)),
    }
}

fn format_session_recent(cfg: &Config, summaries: &[SessionSummary]) -> Result<String> {
    let mut lines = vec![format!(
        "Recent sessions (showing {} of {})",
        summaries.len().min(RECENT_SESSION_LIMIT),
        summaries.len()
    )];
    if summaries.is_empty() {
        lines.push("- no sessions recorded".to_string());
    } else {
        for summary in summaries.iter().take(RECENT_SESSION_LIMIT) {
            let replay = sessions::replay_session(cfg, &summary.session_id)?;
            let fsm_state = replay.state.current_fsm_state.as_deref().unwrap_or("-");
            lines.push(format!(
                "- {} | status={} | fsm={} | updated={}",
                summary.session_id, summary.final_status, fsm_state, summary.last_event_at
            ));
        }
    }
    lines.push("next: session show <session_id> | session list --json".to_string());
    Ok(lines.join("\n"))
}

fn format_session_show(summary: &SessionSummary, replay: &SessionReplay) -> String {
    format!(
        "Session

session_id: {}
run_id: {}
domain_id: {}
status: {}
started_at: {}
updated_at: {}
final_fsm_state: {}
event_count: {}
output_count: {}
error_count: {}
next: session replay {}",
        summary.session_id,
        summary.run_id,
        summary.domain_id,
        summary.final_status,
        summary.started_at,
        summary.last_event_at,
        replay.state.current_fsm_state.as_deref().unwrap_or("-"),
        summary.event_count,
        summary.output_count,
        summary.error_count,
        summary.session_id
    )
}

fn pass_fail(value: bool) -> &'static str {
    if value { "ok" } else { "fail" }
}

fn format_model_preference(preference: Option<&ModelPreference>) -> String {
    preference
        .map(|model| format!("{} {}", model.provider, model.model))
        .unwrap_or_else(|| "unset".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use tempfile::tempdir;

    fn temp_cfg() -> (tempfile::TempDir, std::path::PathBuf, Config) {
        let temp = tempdir().expect("tempdir");
        let root = temp.path().join("project");
        std::fs::create_dir_all(&root).expect("root");
        let config_path = root.join("quant-m.toml");

        let workspace_dir = root.join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.memory.sqlite_path = cfg.workspace_dir.join("memory/brain.db");
        cfg.memory.core_markdown = cfg.workspace_dir.join("MEMORY.md");
        cfg.memory.daily_dir = cfg.workspace_dir.join("daily");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.heartbeat.tasks_file = cfg.workspace_dir.join("HEARTBEAT.md");
        cfg.worker.inbox_path = cfg.workspace_dir.join("queue/inbox.ndjson");
        cfg.worker.outbox_path = cfg.workspace_dir.join("queue/outbox.ndjson");
        cfg.worker.inflight_path = cfg.workspace_dir.join("queue/inflight.json");
        cfg.worker.state_path = cfg.workspace_dir.join("state/worker_state.json");
        cfg.worker.dead_letter_path = cfg.workspace_dir.join("queue/dead-letter.ndjson");
        cfg.logging.file = cfg.workspace_dir.join("logs/quant-m.log");
        cfg.skills.dir = cfg.workspace_dir.join("skills");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.save(&config_path).expect("save");
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        (temp, config_path, cfg)
    }

    #[test]
    fn shell_command_parser_recognizes_supported_commands() {
        assert_eq!(
            parse_command("help").expect("help"),
            AgentShellCommand::Help
        );
        assert_eq!(
            parse_command("run mock-research").expect("mock"),
            AgentShellCommand::RunDemo
        );
        assert_eq!(
            parse_command("run demo").expect("demo"),
            AgentShellCommand::RunDemo
        );
        assert_eq!(
            parse_command("run workflow workflow:mock-research-brief").expect("workflow"),
            AgentShellCommand::RunWorkflow("workflow:mock-research-brief".to_string())
        );
        assert_eq!(
            parse_command("state summary").expect("summary"),
            AgentShellCommand::StateSummary
        );
        assert_eq!(
            parse_command("state list --json").expect("state json"),
            AgentShellCommand::StateList { json: true }
        );
        assert_eq!(
            parse_command("session recent").expect("recent"),
            AgentShellCommand::SessionRecent
        );
        assert_eq!(
            parse_command("session show session-1").expect("show"),
            AgentShellCommand::SessionShow("session-1".to_string())
        );
        assert_eq!(
            parse_command("session replay session-1").expect("replay"),
            AgentShellCommand::SessionReplay("session-1".to_string())
        );
        assert_eq!(
            parse_command("quit").expect("quit"),
            AgentShellCommand::Exit
        );
    }

    #[test]
    fn help_output_contains_grouped_commands() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::Help).expect("help response");
        assert!(response.output.contains("Overview:"));
        assert!(response.output.contains("Run:"));
        assert!(response.output.contains("Sessions:"));
        assert!(!response.should_exit);
    }

    #[test]
    fn run_demo_triggers_mock_research_workflow() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::RunDemo).expect("run demo");
        assert!(response.output.contains("Workflow run complete"));
        assert!(response.output.contains("session_id:"));
        assert_eq!(
            shared_state::list_state(&cfg, None).expect("state").len(),
            1
        );
    }

    #[test]
    fn state_summary_works() {
        let (_temp, config_path, cfg) = temp_cfg();
        execute_command(&cfg, &config_path, AgentShellCommand::RunDemo).expect("run demo");
        let response = execute_command(&cfg, &config_path, AgentShellCommand::StateSummary)
            .expect("state summary");
        assert!(response.output.contains("State Summary"));
        assert!(response.output.contains("current_shared_state_records: 1"));
    }

    #[test]
    fn state_show_works() {
        let (_temp, config_path, cfg) = temp_cfg();
        execute_command(&cfg, &config_path, AgentShellCommand::RunDemo).expect("run demo");
        let response = execute_command(
            &cfg,
            &config_path,
            AgentShellCommand::StateShow("shared.research.summary".to_string()),
        )
        .expect("state show");
        assert!(response.output.contains("shared.research.summary"));
    }

    #[test]
    fn session_recent_returns_compact_output() {
        let (_temp, config_path, cfg) = temp_cfg();
        execute_command(&cfg, &config_path, AgentShellCommand::RunDemo).expect("run demo");
        let response = execute_command(&cfg, &config_path, AgentShellCommand::SessionRecent)
            .expect("session recent");
        assert!(response.output.contains("Recent sessions"));
        assert!(response.output.contains("status=ok"));
        assert!(response.output.contains("fsm=state:summary_drafted"));
    }

    #[test]
    fn session_show_works() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::RunDemo).expect("run demo");
        let session_id = extract_session_id(&response.output);
        let response = execute_command(
            &cfg,
            &config_path,
            AgentShellCommand::SessionShow(session_id),
        )
        .expect("session show");
        assert!(response.output.contains("Session"));
        assert!(
            response
                .output
                .contains("final_fsm_state: state:summary_drafted")
        );
    }

    #[test]
    fn session_replay_works() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::RunDemo).expect("run demo");
        let session_id = extract_session_id(&response.output);
        let replay = execute_command(
            &cfg,
            &config_path,
            AgentShellCommand::SessionReplay(session_id),
        )
        .expect("replay");
        assert!(replay.output.contains("\"side_effects_replayed\": false"));
    }

    #[test]
    fn config_show_works() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::ConfigShow).expect("config");
        assert!(response.output.contains("workspace_dir = \"workspace\""));
    }

    #[test]
    fn unknown_commands_remain_readable() {
        let err = parse_command("launch everything").expect_err("unknown command");
        assert!(err.to_string().contains("unknown command"));
        assert!(err.to_string().contains("type help"));
    }

    #[test]
    fn quit_exits_cleanly() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::Exit).expect("exit response");
        assert_eq!(response.output, "bye");
        assert!(response.should_exit);
    }

    fn extract_session_id(output: &str) -> String {
        output
            .lines()
            .find_map(|line| line.strip_prefix("session_id: "))
            .expect("session id line")
            .trim()
            .to_string()
    }
}
