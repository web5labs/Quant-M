use crate::config::Config;
use crate::sessions::{self, SessionEvent};
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
    if !cfg.skills.allow_shell_commands {
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
            SessionEvent::Error {
                code: Some("skills_shell_disabled".to_string()),
                message: "skill shell execution is disabled (skills.allow_shell_commands=false)"
                    .to_string(),
            },
        );
        return Err(anyhow!(
            "skill shell execution is disabled (skills.allow_shell_commands=false)"
        ));
    }

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

fn truncate_for_session(value: &str) -> String {
    const MAX: usize = 512;
    if value.len() <= MAX {
        value.to_string()
    } else {
        format!("{}...[truncated]", &value[..MAX])
    }
}
