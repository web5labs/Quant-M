use crate::config::Config;
use crate::fsm_core::{
    PolicyApprovalEvent, PolicyApprovalFsm, PolicyApprovalState, SkillExecutionEvent,
    SkillExecutionFsm, SkillExecutionState, StateMachine,
};
use crate::sessions::{self, SessionEvent};
use crate::side_effect_gate::{SideEffectKind, SideEffectRequest, evaluate_side_effect};
use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use tokio::process::Command;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub description: String,
    pub path: PathBuf,
    pub runnable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillDetail {
    pub info: SkillInfo,
    pub markdown: String,
}

#[derive(Debug, Deserialize)]
struct SkillToml {
    #[serde(default)]
    skill: SkillMeta,
    run: Option<SkillRun>,
}

#[derive(Debug, Default, Deserialize)]
struct SkillMeta {
    #[serde(default)]
    name: String,
    #[serde(default)]
    description: String,
}

#[derive(Debug, Deserialize)]
struct SkillRun {
    command: String,
}

pub fn list_skills(cfg: &Config) -> Result<Vec<SkillInfo>> {
    if !cfg.skills.dir.exists() {
        return Ok(vec![]);
    }

    let mut skills = Vec::new();
    for entry in fs::read_dir(&cfg.skills.dir)
        .with_context(|| format!("failed to read {}", cfg.skills.dir.display()))?
    {
        let entry = entry.context("failed to read skill directory entry")?;
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let md_path = path.join("SKILL.md");
        if !md_path.exists() {
            continue;
        }
        let markdown = fs::read_to_string(&md_path)
            .with_context(|| format!("failed to read {}", md_path.display()))?;
        let toml_path = path.join("SKILL.toml");
        let parsed = parse_skill_toml(&toml_path).ok();

        let fallback_name = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("unknown")
            .to_string();

        let name = parsed
            .as_ref()
            .and_then(|value| non_empty(&value.skill.name))
            .unwrap_or(fallback_name);
        let description = parsed
            .as_ref()
            .and_then(|value| non_empty(&value.skill.description))
            .unwrap_or_else(|| description_from_markdown(&markdown));
        let runnable = parsed.and_then(|value| value.run).is_some();

        skills.push(SkillInfo {
            name,
            description,
            path: path.clone(),
            runnable,
        });
    }

    skills.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(skills)
}

pub fn show_skill(cfg: &Config, name: &str) -> Result<SkillDetail> {
    let info = list_skills(cfg)?
        .into_iter()
        .find(|skill| skill.name == name)
        .ok_or_else(|| anyhow!("skill '{}' not found", name))?;
    let markdown_path = info.path.join("SKILL.md");
    let markdown = fs::read_to_string(&markdown_path)
        .with_context(|| format!("failed to read {}", markdown_path.display()))?;
    Ok(SkillDetail { info, markdown })
}

pub async fn run_skill(cfg: &Config, name: &str, input: &str) -> Result<String> {
    let session = sessions::runtime_context(&cfg.node_id, "skills");
    record_session_event(
        cfg,
        &session,
        SessionEvent::Observation {
            message: "skill_requested".to_string(),
            job_id: None,
            detail: Some(name.to_string()),
        },
    );

    let info = list_skills(cfg)?
        .into_iter()
        .find(|skill| skill.name == name)
        .ok_or_else(|| anyhow!("skill '{}' not found", name))?;

    let skill_toml = parse_skill_toml(&info.path.join("SKILL.toml"))
        .with_context(|| format!("skill '{}' is missing run.command in SKILL.toml", name))?;
    let run = skill_toml
        .run
        .ok_or_else(|| anyhow!("skill '{}' has no [run] command", name))?;
    let command = run.command.replace("{{input}}", input);
    let side_effect_level = "external_action";
    let shell_required = true;
    let shell_allowed = cfg.skills.allow_shell_commands;
    let side_effect_gate = evaluate_side_effect(
        SideEffectRequest::new(SideEffectKind::ShellCommand, format!("skills.run.{name}"))
            .config_allowed(shell_allowed)
            .policy_allowed(shell_allowed)
            .session_id(session.session_id.to_string())
            .evidence_ref(name.to_string()),
    );

    record_skill_transition(
        cfg,
        &session,
        SkillExecutionState::Declared,
        SkillExecutionEvent::Load,
        &format!(
            "skill_id={name} runnable={} shell_required=true",
            info.runnable
        ),
    );
    record_skill_transition(
        cfg,
        &session,
        SkillExecutionState::Loaded,
        SkillExecutionEvent::CheckPolicy,
        &format!(
            "skill_id={name} side_effect_level={side_effect_level} shell_required={shell_required}"
        ),
    );
    record_policy_transition(
        cfg,
        &session,
        PolicyApprovalState::Requested,
        PolicyApprovalEvent::Request,
        &format!("skill_id={name} requested shell-backed execution"),
    );

    if !shell_allowed {
        record_session_event(
            cfg,
            &session,
            SessionEvent::PolicyDecision {
                policy: "skills.allow_shell_commands".to_string(),
                allowed: false,
                reason: "skill shell execution is disabled".to_string(),
            },
        );
        record_session_event(
            cfg,
            &session,
            SessionEvent::AuditNote {
                note: side_effect_gate.audit_note(),
            },
        );
        record_policy_transition(
            cfg,
            &session,
            PolicyApprovalState::EvaluatingPolicy,
            PolicyApprovalEvent::PolicyBlocks,
            &format!(
                "skill_id={name} shell_required={shell_required} shell_allowed={shell_allowed}"
            ),
        );
        record_skill_transition(
            cfg,
            &session,
            SkillExecutionState::PolicyChecked,
            SkillExecutionEvent::PolicyBlocks,
            &format!(
                "skill_id={name} blocked shell_required={shell_required} shell_allowed={shell_allowed}"
            ),
        );
        record_session_event(
            cfg,
            &session,
            SessionEvent::SkillCall {
                skill_name: name.to_string(),
                input_preview: truncate_for_session(input),
                command_preview: Some(truncate_for_session(&command)),
                status: "blocked".to_string(),
            },
        );
        return Err(anyhow!(
            "skill shell execution is disabled (skills.allow_shell_commands=false)"
        ));
    }

    record_session_event(
        cfg,
        &session,
        SessionEvent::AuditNote {
            note: side_effect_gate.audit_note(),
        },
    );
    record_policy_transition(
        cfg,
        &session,
        PolicyApprovalState::EvaluatingPolicy,
        PolicyApprovalEvent::PolicyAllows,
        &format!("skill_id={name} shell_required={shell_required} shell_allowed={shell_allowed}"),
    );
    record_skill_transition(
        cfg,
        &session,
        SkillExecutionState::PolicyChecked,
        SkillExecutionEvent::PolicyAllows,
        &format!(
            "skill_id={name} ready side_effect_level={side_effect_level} shell_allowed={shell_allowed}"
        ),
    );
    record_skill_transition(
        cfg,
        &session,
        SkillExecutionState::Ready,
        SkillExecutionEvent::Start,
        &format!("skill_id={name} starting shell-backed command"),
    );
    record_policy_transition(
        cfg,
        &session,
        PolicyApprovalState::ExecutionAllowed,
        PolicyApprovalEvent::Execute,
        &format!("skill_id={name} entering command execution"),
    );
    record_session_event(
        cfg,
        &session,
        SessionEvent::SkillCall {
            skill_name: name.to_string(),
            input_preview: truncate_for_session(input),
            command_preview: Some(truncate_for_session(&command)),
            status: "running".to_string(),
        },
    );

    let output = timeout(
        Duration::from_secs(cfg.worker.command_timeout_seconds.max(1)),
        Command::new("sh")
            .arg("-lc")
            .arg(&command)
            .kill_on_drop(true)
            .output(),
    )
    .await
    .context("skill run timed out");
    let output = match output {
        Ok(result) => match result.context("failed to execute skill command") {
            Ok(output) => output,
            Err(err) => {
                record_session_event(
                    cfg,
                    &session,
                    SessionEvent::Error {
                        code: Some("skills_exec_failed".to_string()),
                        message: err.to_string(),
                    },
                );
                record_skill_transition(
                    cfg,
                    &session,
                    SkillExecutionState::Running,
                    SkillExecutionEvent::Fail,
                    &format!("skill_id={name} command spawn failed"),
                );
                return Err(err);
            }
        },
        Err(err) => {
            record_session_event(
                cfg,
                &session,
                SessionEvent::Error {
                    code: Some("skills_timeout".to_string()),
                    message: err.to_string(),
                },
            );
            record_skill_transition(
                cfg,
                &session,
                SkillExecutionState::Running,
                SkillExecutionEvent::Fail,
                &format!("skill_id={name} command timed out"),
            );
            return Err(err);
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    if !output.status.success() {
        let message = format!(
            "skill '{}' failed with status {}: {}",
            name,
            output.status,
            if stderr.trim().is_empty() {
                stdout.clone()
            } else {
                stderr.clone()
            }
        );
        record_session_event(
            cfg,
            &session,
            SessionEvent::Error {
                code: Some("skills_command_failed".to_string()),
                message: message.clone(),
            },
        );
        record_skill_transition(
            cfg,
            &session,
            SkillExecutionState::Running,
            SkillExecutionEvent::Fail,
            &format!("skill_id={name} command exited unsuccessfully"),
        );
        return Err(anyhow!("{}", message));
    }

    let final_output = if stdout.trim().is_empty() {
        stderr
    } else {
        stdout
    };
    record_session_event(
        cfg,
        &session,
        SessionEvent::Output {
            channel: "skills".to_string(),
            summary: truncate_for_session(&final_output),
            job_id: None,
        },
    );
    record_skill_transition(
        cfg,
        &session,
        SkillExecutionState::Running,
        SkillExecutionEvent::Complete,
        &format!("skill_id={name} command completed successfully"),
    );
    Ok(final_output)
}

fn parse_skill_toml(path: &Path) -> Result<SkillToml> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    toml::from_str::<SkillToml>(&raw).context("invalid SKILL.toml")
}

fn description_from_markdown(markdown: &str) -> String {
    for line in markdown.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        return trimmed.to_string();
    }
    "No description".to_string()
}

fn non_empty(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn record_session_event(cfg: &Config, session: &sessions::SessionContext, event: SessionEvent) {
    if let Err(err) = sessions::append_event(cfg, session, event) {
        let _ = crate::logutil::append_log(
            &cfg.logging,
            &format!(
                "session_event_error session_id={} error={}",
                session.session_id, err
            ),
        );
    }
}

fn record_skill_transition(
    cfg: &Config,
    session: &sessions::SessionContext,
    from_state: SkillExecutionState,
    event: SkillExecutionEvent,
    reason: &str,
) {
    record_transition(
        cfg,
        session,
        &SkillExecutionFsm,
        from_state,
        event,
        reason,
        "skill_execution_invalid_fsm_transition",
    );
}

fn record_policy_transition(
    cfg: &Config,
    session: &sessions::SessionContext,
    from_state: PolicyApprovalState,
    event: PolicyApprovalEvent,
    reason: &str,
) {
    record_transition(
        cfg,
        session,
        &PolicyApprovalFsm,
        from_state,
        event,
        reason,
        "policy_approval_invalid_fsm_transition",
    );
}

fn record_transition<M>(
    cfg: &Config,
    session: &sessions::SessionContext,
    fsm: &M,
    from_state: M::State,
    event: M::Event,
    reason: &str,
    error_code: &str,
) where
    M: StateMachine,
{
    match fsm.transition(&from_state, &event) {
        Ok(to_state) => record_session_event(
            cfg,
            session,
            SessionEvent::FsmTransition {
                machine: fsm.machine_id().to_string(),
                from_state: Some(from_state.to_string()),
                to_state: to_state.to_string(),
                reason: reason.to_string(),
            },
        ),
        Err(err) => record_session_event(
            cfg,
            session,
            SessionEvent::Error {
                code: Some(error_code.to_string()),
                message: err.to_string(),
            },
        ),
    }
}

fn truncate_for_session(value: &str) -> String {
    const MAX: usize = 512;
    if value.len() <= MAX {
        value.to_string()
    } else {
        format!("{}...[truncated]", &value[..MAX])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, LoggingConfig, SkillsConfig};
    use std::fs;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir, allow_shell_commands: bool) -> Config {
        let workspace = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace.clone(),
            skills: SkillsConfig {
                dir: workspace.join("skills"),
                allow_shell_commands,
            },
            logging: LoggingConfig {
                file: workspace.join("logs/quant-m.log"),
                max_bytes: 1_048_576,
                keep_files: 3,
            },
            ..Config::default()
        };
        cfg.runtime.session_dir = workspace.join("state/sessions");
        cfg
    }

    fn write_shell_skill(cfg: &Config, name: &str, command: &str) {
        let dir = cfg.skills.dir.join(name);
        fs::create_dir_all(&dir).expect("skill dir");
        fs::write(dir.join("SKILL.md"), format!("# {name}\n\nTest skill.")).expect("skill md");
        fs::write(
            dir.join("SKILL.toml"),
            format!(
                "[skill]\nname = \"{name}\"\ndescription = \"Test skill\"\n\n[run]\ncommand = \"{}\"\n",
                command.replace('\\', "\\\\").replace('"', "\\\"")
            ),
        )
        .expect("skill toml");
    }

    fn only_session_detail(cfg: &Config) -> sessions::SessionDetail {
        let listed = sessions::list_sessions(cfg).expect("sessions");
        assert_eq!(listed.len(), 1);
        sessions::show_session(cfg, &listed[0].session_id).expect("detail")
    }

    fn has_transition(
        detail: &sessions::SessionDetail,
        machine_name: &str,
        from: &str,
        to: &str,
    ) -> bool {
        detail.events.iter().any(|entry| {
            matches!(
                &entry.event,
                SessionEvent::FsmTransition {
                    machine,
                    from_state: Some(from_state),
                    to_state,
                    ..
                } if machine == machine_name && from_state == from && to_state == to
            )
        })
    }

    #[tokio::test]
    async fn shell_skill_disabled_blocks_not_fails_and_does_not_execute() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp, false);
        let marker = tmp.path().join("skill-marker");
        write_shell_skill(&cfg, "blocked", &format!("touch {}", marker.display()));

        let err = run_skill(&cfg, "blocked", "input")
            .await
            .expect_err("blocked");
        assert!(
            err.to_string()
                .contains("skills.allow_shell_commands=false")
        );
        assert!(!marker.exists());

        let detail = only_session_detail(&cfg);
        let replay = sessions::replay_session(&cfg, &detail.summary.session_id).expect("replay");
        assert_eq!(replay.state.current_fsm_state.as_deref(), Some("blocked"));
        assert_eq!(replay.state.errors, 0);

        assert!(has_transition(
            &detail,
            "skill_execution",
            "policy_checked",
            "blocked"
        ));
        assert!(detail.events.iter().any(|entry| {
            matches!(
                &entry.event,
                SessionEvent::SkillCall { status, .. } if status == "blocked"
            )
        }));
    }

    #[tokio::test]
    async fn shell_skill_runs_only_after_policy_allows_and_records_success() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp, true);
        write_shell_skill(&cfg, "echoer", "printf '%s' '{{input}}'");

        let output = run_skill(&cfg, "echoer", "hello skill").await.expect("run");
        assert_eq!(output, "hello skill");

        let detail = only_session_detail(&cfg);
        assert!(has_transition(
            &detail,
            "policy_approval",
            "evaluating_policy",
            "execution_allowed"
        ));
        assert!(has_transition(
            &detail,
            "skill_execution",
            "running",
            "succeeded"
        ));
    }

    #[tokio::test]
    async fn failing_shell_skill_records_failed_lifecycle() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp, true);
        write_shell_skill(&cfg, "fails", "exit 7");

        run_skill(&cfg, "fails", "input")
            .await
            .expect_err("command fails");

        assert!(has_transition(
            &only_session_detail(&cfg),
            "skill_execution",
            "running",
            "failed"
        ));
    }

    #[test]
    fn skill_list_and_show_still_work_without_execution() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp, false);
        write_shell_skill(&cfg, "listed", "printf listed");

        let listed = list_skills(&cfg).expect("list");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].name, "listed");
        assert!(listed[0].runnable);

        let detail = show_skill(&cfg, "listed").expect("show");
        assert_eq!(detail.info.name, "listed");
        assert!(detail.markdown.contains("Test skill"));
    }

    #[tokio::test]
    async fn missing_skill_is_not_recorded_as_policy_failure() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp, false);

        let err = run_skill(&cfg, "missing", "input")
            .await
            .expect_err("missing");
        assert!(err.to_string().contains("skill 'missing' not found"));

        let detail = only_session_detail(&cfg);
        assert!(
            !detail
                .events
                .iter()
                .any(|entry| matches!(&entry.event, SessionEvent::PolicyDecision { .. }))
        );
    }
}
