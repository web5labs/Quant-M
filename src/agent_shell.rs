use crate::bootstrap;
use crate::config::{Config, ModelPreference};
use crate::domain;
use crate::execution_runtime::{self, WorkflowRunResult};
use crate::sessions::{self, SessionId, SessionReplay, SessionSummary};
use crate::shared_state::{self, SharedStateKey, SharedStateRecord};
use crate::state_sql;
use crate::workflow_registry::{self, WorkflowId};
use anyhow::{Context, Result};
use serde::Serialize;
use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Stdio};

const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
const MOCK_RESEARCH_WORKFLOW: &str = "workflow:mock-research-brief";
const RECENT_SESSION_LIMIT: usize = 5;
const RECENT_STATE_LIMIT: usize = 5;
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_BLUE: &str = "\x1b[38;2;80;160;255m";
const ANSI_CYAN: &str = "\x1b[38;2;70;220;230m";
const ANSI_GREEN: &str = "\x1b[38;2;80;220;140m";
const ANSI_RED: &str = "\x1b[38;2;255;95;95m";

#[derive(Debug, Clone, PartialEq, Eq)]
enum AgentShellCommand {
    Help,
    Ask(String),
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
    Settings,
    Hint(String),
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
        print!("{}{}quant-m>{} ", ANSI_BOLD, ANSI_CYAN, ANSI_RESET);
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
                println!("{}error:{} {err}", ANSI_RED, ANSI_RESET);
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
                println!("{}error:{} {err}", ANSI_RED, ANSI_RESET);
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
        "{ANSI_BOLD}{ANSI_BLUE}Quant-M Agent Shell{ANSI_RESET} v{APP_VERSION}
{ANSI_DIM}mode:{ANSI_RESET} codex_harness
{ANSI_DIM}workspace:{ANSI_RESET} {}
{ANSI_DIM}runtime_profile:{ANSI_RESET} {}
{ANSI_DIM}network:{ANSI_RESET} {}
{ANSI_DIM}preferred_local_model:{ANSI_RESET} {}
{ANSI_DIM}preferred_openrouter_model:{ANSI_RESET} {}
{ANSI_DIM}domains:{ANSI_RESET} {} | {ANSI_DIM}workflows:{ANSI_RESET} {} | {ANSI_DIM}sessions:{ANSI_RESET} {} | {ANSI_DIM}shared_state:{ANSI_RESET} {}

{ANSI_GREEN}Type a question to chat through Codex, or type help for commands.{ANSI_RESET}",
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
    if let Some(inner) = strip_launcher_prefix(&parts) {
        let inner = inner.trim();
        if inner.is_empty() || matches!(inner, "start" | "agent" | "shell") {
            return Ok(AgentShellCommand::Hint(
                "You are already inside the Quant-M shell. Type help, demo, doctor, or exit."
                    .to_string(),
            ));
        }
        let command = parse_command(inner)?;
        if matches!(command, AgentShellCommand::Ask(_)) {
            return Ok(AgentShellCommand::Hint(format!(
                "You are already inside the Quant-M shell. Type exit first to run `{trimmed}` from your terminal, or type help for shell commands."
            )));
        }
        return Ok(command);
    }

    match parts.as_slice() {
        ["help"] => Ok(AgentShellCommand::Help),
        ["ask", rest @ ..] if !rest.is_empty() => Ok(AgentShellCommand::Ask(rest.join(" "))),
        ["doctor"] => Ok(AgentShellCommand::Doctor),
        ["demo"] | ["run", "mock-research"] => Ok(AgentShellCommand::RunDemo),
        ["run", "demo"] => Ok(AgentShellCommand::Hint(
            "Did you mean demo? Try: demo".to_string(),
        )),
        ["run", "workflow", workflow_id] => {
            Ok(AgentShellCommand::RunWorkflow((*workflow_id).to_string()))
        }
        ["state", "summary"] => Ok(AgentShellCommand::StateSummary),
        ["state", "list"] => Ok(AgentShellCommand::StateList { json: false }),
        ["state", "list", "--json"] => Ok(AgentShellCommand::StateList { json: true }),
        ["state", "show", key] => Ok(AgentShellCommand::StateShow((*key).to_string())),
        ["sessions"] | ["session", "recent"] => Ok(AgentShellCommand::SessionRecent),
        ["session", "list"] => Ok(AgentShellCommand::SessionList { json: false }),
        ["session", "list", "--json"] => Ok(AgentShellCommand::SessionList { json: true }),
        ["session", "show", session_id] => {
            Ok(AgentShellCommand::SessionShow((*session_id).to_string()))
        }
        ["session", "replay", session_id] => {
            Ok(AgentShellCommand::SessionReplay((*session_id).to_string()))
        }
        ["config", "show"] => Ok(AgentShellCommand::ConfigShow),
        ["settings"] | ["/settings"] => Ok(AgentShellCommand::Settings),
        ["cli"] => Ok(AgentShellCommand::Hint(
            "Did you mean shell? Try: quant-m shell".to_string(),
        )),
        ["exit"] | ["quit"] | ["bye"] => Ok(AgentShellCommand::Exit),
        _ if looks_like_local_command(trimmed) => Ok(AgentShellCommand::Hint(format!(
            "That looks like a Quant-M command, but this shell does not run it directly yet. Type exit first, then run `{trimmed}` from your terminal."
        ))),
        _ if parts.len() <= 2 => {
            if let Some(suggestion) = suggest_command(trimmed) {
                Ok(AgentShellCommand::Hint(format!(
                    "Did you mean {suggestion}? Try: {suggestion}"
                )))
            } else {
                Ok(AgentShellCommand::Ask(trimmed.to_string()))
            }
        }
        _ => Ok(AgentShellCommand::Ask(trimmed.to_string())),
    }
}

fn strip_launcher_prefix(parts: &[&str]) -> Option<String> {
    match parts {
        ["./quantm", rest @ ..]
        | ["quantm", rest @ ..]
        | ["quant-m", rest @ ..]
        | ["./target/release/quant-m", rest @ ..] => Some(rest.join(" ")),
        ["cargo", "run", "--release", "--", rest @ ..] | ["cargo", "run", "--", rest @ ..] => {
            Some(rest.join(" "))
        }
        _ => None,
    }
}

fn looks_like_local_command(input: &str) -> bool {
    let first = input.split_whitespace().next().unwrap_or_default();
    matches!(
        first,
        "context"
            | "context-status"
            | "cost"
            | "replay"
            | "compact"
            | "consensus"
            | "daemon"
            | "worker"
            | "state"
            | "session"
            | "tool"
            | "provider"
            | "setup"
            | "init"
            | "init-truth"
            | "onboard"
            | "tui"
    )
}

fn suggest_command(input: &str) -> Option<&'static str> {
    let normalized = input
        .trim()
        .trim_start_matches('/')
        .to_ascii_lowercase()
        .replace(['_', ' '], "-");
    let first = normalized.split('-').next().unwrap_or_default();
    let candidates = [
        "help",
        "doctor",
        "demo",
        "settings",
        "state summary",
        "state list",
        "session recent",
        "session list",
        "context-status",
        "config show",
        "exit",
    ];

    let mut best = None;
    let mut best_distance = usize::MAX;
    for candidate in candidates {
        let candidate_key = candidate.replace(' ', "-");
        let distance =
            edit_distance(&normalized, &candidate_key).min(edit_distance(first, candidate));
        if distance < best_distance {
            best_distance = distance;
            best = Some(candidate);
        }
    }

    if best_distance <= 2 { best } else { None }
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous = (0..=right.chars().count()).collect::<Vec<_>>();
    let mut current = vec![0; previous.len()];

    for (i, left_char) in left.chars().enumerate() {
        current[0] = i + 1;
        for (j, right_char) in right.chars().enumerate() {
            let cost = usize::from(left_char != right_char);
            current[j + 1] = (previous[j + 1] + 1)
                .min(current[j] + 1)
                .min(previous[j] + cost);
        }
        std::mem::swap(&mut previous, &mut current);
    }

    previous[right.chars().count()]
}

#[cfg(feature = "fuzzing_hooks")]
pub fn parse_command_for_fuzz(input: &str) -> Result<&'static str> {
    let command = parse_command(input)?;
    let label = match command {
        AgentShellCommand::Help => "help",
        AgentShellCommand::Ask(_) => "ask",
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
        AgentShellCommand::Settings => "settings",
        AgentShellCommand::Hint(_) => "hint",
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
        AgentShellCommand::Ask(prompt) => Ok(AgentShellResponse {
            output: run_codex_chat(cfg, &prompt)?,
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
        AgentShellCommand::Settings => Ok(AgentShellResponse {
            output: format_shell_settings(cfg),
            should_exit: false,
        }),
        AgentShellCommand::Hint(message) => Ok(AgentShellResponse {
            output: message,
            should_exit: false,
        }),
        AgentShellCommand::Exit => Ok(AgentShellResponse {
            output: "bye\n\nOutside the shell, use:\n  ./quantm demo\n  ./quantm agent\n  ./quantm run workflow workflow:mock-research-brief".to_string(),
            should_exit: true,
        }),
    }
}

fn help_text() -> &'static str {
    "Quant-M Agent Shell Commands

Chat:
  <any question>        send a prompt through local Codex CLI
  ask <question>        explicit Codex prompt

Overview:
  help
  doctor
  settings
  /settings
  config show

Run:
  demo
  run mock-research
  run workflow workflow:mock-research-brief

Outside this shell:
  ./quantm demo
  ./quantm agent

State:
  state summary
  state list
  state list --json
  state show shared.research.summary

Sessions:
  sessions
  session recent
  session list
  session list --json
  session show <session_id>
  session replay <session_id>

Exit:
  quit
  exit"
}

fn format_shell_settings(cfg: &Config) -> String {
    let enabled_tools = cfg
        .tools
        .iter()
        .filter(|(_id, tool)| tool.enabled)
        .map(|(id, _tool)| id.as_str())
        .collect::<Vec<_>>();
    let enabled_tools = if enabled_tools.is_empty() {
        "none".to_string()
    } else {
        enabled_tools.join(", ")
    };
    format!(
        "{ANSI_BOLD}{ANSI_BLUE}Settings{ANSI_RESET}
multi_model_enabled: {}
search_enabled: {}
browser_harness_enabled: {}
external_network_enabled: {}
enabled_tools: {}

Tune more outside the shell:
  quant-m settings
  quant-m tool scan
  quant-m onboard --advanced",
        enabled_label(cfg.runtime.multi_model_enabled),
        enabled_label(cfg.runtime.search_enabled),
        enabled_label(cfg.runtime.browser_harness_enabled),
        enabled_label(cfg.runtime.external_network_enabled),
        enabled_tools,
    )
}

fn enabled_label(value: bool) -> &'static str {
    if value { "enabled" } else { "disabled" }
}

fn run_codex_chat(cfg: &Config, prompt: &str) -> Result<String> {
    if !command_present("codex") {
        return Ok(format!(
            "{ANSI_RED}Codex CLI is not on PATH.{ANSI_RESET}\nRun `codex login`, then retry from this shell."
        ));
    }
    let cwd = std::env::current_dir().unwrap_or_else(|_| cfg.workspace_dir.clone());
    let harness_prompt = format!(
        "You are Codex running through the Quant-M local agent harness.\n\
         Keep the answer concise and practical.\n\
         Quant-M workspace: {}\n\n\
         User prompt:\n{}",
        cfg.workspace_dir.display(),
        prompt
    );
    let mut child = Command::new("codex")
        .arg("exec")
        .arg("--color")
        .arg("always")
        .arg("--sandbox")
        .arg("read-only")
        .arg("--skip-git-repo-check")
        .arg("--cd")
        .arg(cwd)
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("failed to run codex exec")?;
    if let Some(stdin) = child.stdin.as_mut() {
        stdin
            .write_all(harness_prompt.as_bytes())
            .context("failed to send prompt to codex exec")?;
    }
    let output = child
        .wait_with_output()
        .context("failed to wait for codex exec")?;
    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
    if output.status.success() {
        let mut lines = vec![format!("{ANSI_BOLD}{ANSI_GREEN}Codex{ANSI_RESET}")];
        if !stderr.is_empty() {
            lines.push(format!("{ANSI_DIM}{stderr}{ANSI_RESET}"));
        }
        if stdout.is_empty() {
            lines.push("Codex completed without text output.".to_string());
        } else {
            lines.push(stdout);
        }
        Ok(lines.join("\n"))
    } else {
        let hint = if stderr.contains("readonly database")
            || stderr.contains("Operation not permitted")
        {
            "\n\nhint: Codex was blocked from writing its local state. Run this from your normal Terminal session, or check permissions under ~/.codex."
        } else {
            ""
        };
        Ok(format!(
            "{ANSI_RED}Codex exec failed.{ANSI_RESET}\nstatus: {}\n{}{}{}",
            output.status,
            if stderr.is_empty() { "" } else { "stderr: " },
            stderr,
            hint
        ))
    }
}

fn command_present(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    if command.contains(std::path::MAIN_SEPARATOR) {
        return std::path::Path::new(command).is_file();
    }
    let Some(path) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path).any(|dir| dir.join(command).is_file())
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
        "command: demo -> workflow:mock-research-brief\n".to_string()
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
            parse_command("ask what is Quant-M?").expect("ask"),
            AgentShellCommand::Ask("what is Quant-M?".to_string())
        );
        assert_eq!(
            parse_command("what is Quant-M?").expect("free text"),
            AgentShellCommand::Ask("what is Quant-M?".to_string())
        );
        assert_eq!(
            parse_command("run mock-research").expect("mock"),
            AgentShellCommand::RunDemo
        );
        assert_eq!(
            parse_command("demo").expect("demo alias"),
            AgentShellCommand::RunDemo
        );
        assert_eq!(
            parse_command("run demo").expect("demo"),
            AgentShellCommand::Hint("Did you mean demo? Try: demo".to_string())
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
            parse_command("sessions").expect("sessions alias"),
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
        assert_eq!(parse_command("bye").expect("bye"), AgentShellCommand::Exit);
        assert_eq!(
            parse_command("settings").expect("settings"),
            AgentShellCommand::Settings
        );
        assert_eq!(
            parse_command("/settings").expect("settings"),
            AgentShellCommand::Settings
        );
        assert_eq!(
            parse_command("cli").expect("cli hint"),
            AgentShellCommand::Hint("Did you mean shell? Try: quant-m shell".to_string())
        );
        assert_eq!(
            parse_command("./quantm doctor").expect("launcher doctor"),
            AgentShellCommand::Doctor
        );
        assert_eq!(
            parse_command("quantm doctor").expect("launcher doctor"),
            AgentShellCommand::Doctor
        );
        assert_eq!(
            parse_command("quant-m doctor").expect("launcher doctor"),
            AgentShellCommand::Doctor
        );
        assert_eq!(
            parse_command("./quantm demo").expect("launcher demo"),
            AgentShellCommand::RunDemo
        );
        assert_eq!(
            parse_command("cargo run --release -- demo").expect("cargo demo"),
            AgentShellCommand::RunDemo
        );
        assert_eq!(
            parse_command("cargo run -- doctor").expect("cargo doctor"),
            AgentShellCommand::Doctor
        );
        assert_eq!(
            parse_command("./quantm").expect("launcher shell"),
            AgentShellCommand::Hint(
                "You are already inside the Quant-M shell. Type help, demo, doctor, or exit."
                    .to_string()
            )
        );
        assert_eq!(
            parse_command("./quantm context guard --json").expect("outside command hint"),
            AgentShellCommand::Hint(
                "That looks like a Quant-M command, but this shell does not run it directly yet. Type exit first, then run `context guard --json` from your terminal."
                    .to_string()
            )
        );
        assert_eq!(
            parse_command("dotor").expect("doctor typo"),
            AgentShellCommand::Hint("Did you mean doctor? Try: doctor".to_string())
        );
        assert_eq!(
            parse_command("contex-status").expect("context typo"),
            AgentShellCommand::Hint("Did you mean context-status? Try: context-status".to_string())
        );
    }

    #[test]
    fn shell_command_suggestions_are_bounded() {
        assert_eq!(suggest_command("dotor"), Some("doctor"));
        assert_eq!(suggest_command("demoo"), Some("demo"));
        assert_eq!(suggest_command("hlep"), Some("help"));
        assert_eq!(suggest_command("contex-status"), Some("context-status"));
        assert_eq!(suggest_command("completely different prompt"), None);
        assert_eq!(edit_distance("doctor", "doctor"), 0);
        assert_eq!(edit_distance("dotor", "doctor"), 1);
    }

    #[test]
    fn help_output_contains_grouped_commands() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::Help).expect("help response");
        assert!(response.output.contains("Overview:"));
        assert!(response.output.contains("Run:"));
        assert!(response.output.contains("Sessions:"));
        assert!(response.output.contains("/settings"));
        assert!(!response.should_exit);
    }

    #[test]
    fn settings_output_lists_default_off_features() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::Settings).expect("settings");
        assert!(response.output.contains("multi_model_enabled: disabled"));
        assert!(
            response
                .output
                .contains("browser_harness_enabled: disabled")
        );
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
    fn free_text_routes_to_codex_prompt() {
        let command = parse_command("launch everything").expect("free text");
        assert_eq!(
            command,
            AgentShellCommand::Ask("launch everything".to_string())
        );
    }

    #[test]
    fn quit_exits_cleanly() {
        let (_temp, config_path, cfg) = temp_cfg();
        let response =
            execute_command(&cfg, &config_path, AgentShellCommand::Exit).expect("exit response");
        assert!(response.output.contains("bye"));
        assert!(response.output.contains("./quantm demo"));
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
