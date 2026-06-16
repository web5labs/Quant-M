use crate::compaction;
use crate::config::Config;
use crate::context_status::{self, ContextState};
use crate::{logutil, sessions};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use tokio::sync::watch;
use tokio::time::Duration;

const GUARDIAN_METADATA_SCHEMA_VERSION: u32 = 1;
const COMPACT_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GuardianAction {
    Disabled,
    NoSession,
    Observe,
    Compact,
    HandoffReady,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGuardianReport {
    pub enabled: bool,
    pub checked_at: String,
    pub context_state: Option<ContextState>,
    pub latest_session_id: Option<String>,
    pub latest_event_count: usize,
    pub risk_score: u8,
    pub action: GuardianAction,
    pub compacted: bool,
    pub compact_packet_path: Option<PathBuf>,
    pub metadata_path: Option<PathBuf>,
    pub guardian_action_id: Option<String>,
    pub handoff_path: Option<PathBuf>,
    pub recommended_next_action: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuardianTickOptions {
    pub force: bool,
}

impl GuardianTickOptions {
    pub fn force() -> Self {
        Self { force: true }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct GuardianMetadata {
    pub metadata_schema_version: u32,
    pub compact_schema_version: u32,
    pub latest_session_id: String,
    pub latest_event_count: usize,
    pub compact_packet_path: PathBuf,
    pub compact_artifact_hash: String,
    pub compact_created_at: String,
    pub guardian_action_id: String,
    pub action: GuardianAction,
    pub updated_at: String,
}

pub async fn run_loop_with_shutdown(
    cfg: Config,
    mut shutdown_rx: Option<watch::Receiver<bool>>,
) -> Result<()> {
    loop {
        let report = tick(&cfg)?;
        logutil::append_log(
            &cfg.logging,
            &format!(
                "context guardian action={:?} risk={} session={}",
                report.action,
                report.risk_score,
                report.latest_session_id.as_deref().unwrap_or("none")
            ),
        )?;

        let sleep = tokio::time::sleep(Duration::from_secs(cfg.context_guardian.interval_seconds));
        tokio::pin!(sleep);
        if let Some(rx) = shutdown_rx.as_mut() {
            tokio::select! {
                _ = &mut sleep => {}
                changed = rx.changed() => {
                    if changed.is_ok() && *rx.borrow() {
                        return Ok(());
                    }
                }
            }
        } else {
            sleep.await;
        }
    }
}

pub fn tick(cfg: &Config) -> Result<ContextGuardianReport> {
    tick_with_options(cfg, GuardianTickOptions::default())
}

pub fn tick_with_options(
    cfg: &Config,
    options: GuardianTickOptions,
) -> Result<ContextGuardianReport> {
    if !cfg.context_guardian.enabled {
        return Ok(ContextGuardianReport {
            enabled: false,
            checked_at: Utc::now().to_rfc3339(),
            context_state: None,
            latest_session_id: None,
            latest_event_count: 0,
            risk_score: 0,
            action: GuardianAction::Disabled,
            compacted: false,
            compact_packet_path: None,
            metadata_path: None,
            guardian_action_id: None,
            handoff_path: None,
            recommended_next_action: "Context guardian is disabled.".to_string(),
        });
    }

    let mut status = context_status::context_status(cfg)?;
    let latest = sessions::list_sessions(cfg)?.into_iter().next();
    let Some(summary) = latest else {
        return Ok(ContextGuardianReport {
            enabled: true,
            checked_at: Utc::now().to_rfc3339(),
            context_state: Some(status.context_state),
            latest_session_id: None,
            latest_event_count: 0,
            risk_score: 10,
            action: GuardianAction::NoSession,
            compacted: false,
            compact_packet_path: None,
            metadata_path: Some(metadata_path(cfg)),
            guardian_action_id: None,
            handoff_path: None,
            recommended_next_action: "Run a Quant-M workflow before creating a continuity handoff."
                .to_string(),
        });
    };

    let risk_score = score_risk(&status.context_state, summary.event_count);
    let metadata_before = read_metadata(cfg).ok();
    let existing_compact_md = status
        .latest_compact_packet_path
        .as_ref()
        .map(|path| path.with_file_name("compact.md"));
    let compact_reason = compact_reason(
        &status,
        &summary,
        existing_compact_md.as_ref(),
        metadata_before.as_ref(),
        options.force,
    );
    let should_compact =
        summary.event_count >= cfg.context_guardian.min_event_count && compact_reason.is_some();

    let (compacted, compact_packet_path) = if should_compact {
        let result = compaction::compact_session(cfg, &summary.session_id)?;
        status = context_status::context_status(cfg)?;
        (true, Some(result.artifacts.compact_md))
    } else {
        (
            false,
            status
                .latest_compact_packet_path
                .as_ref()
                .map(|path| path.with_file_name("compact.md")),
        )
    };
    let action_id = build_action_id(
        &summary.session_id.to_string(),
        &status,
        &compact_packet_path,
    );
    let metadata_path = if let Some(path) = &compact_packet_path {
        Some(write_metadata(
            cfg,
            GuardianMetadataInput {
                latest_session_id: summary.session_id.to_string(),
                latest_event_count: summary.event_count,
                compact_packet_path: path.clone(),
                compact_created_at: Utc::now().to_rfc3339(),
                guardian_action_id: action_id.clone(),
                action: if compacted {
                    GuardianAction::Compact
                } else {
                    GuardianAction::Observe
                },
            },
        )?)
    } else {
        None
    };

    let handoff_path = if risk_score >= 6 || compacted {
        Some(write_handoff(
            cfg,
            &status,
            summary.event_count,
            risk_score,
            compact_packet_path.as_ref(),
        )?)
    } else {
        None
    };

    let action = if handoff_path.is_some() {
        GuardianAction::HandoffReady
    } else if compacted {
        GuardianAction::Compact
    } else {
        GuardianAction::Observe
    };

    let recommended_next_action = if let Some(path) = &handoff_path {
        format!(
            "Use {} to continue in a fresh context if drift appears.",
            path.display()
        )
    } else {
        status.recommended_next_action.clone()
    };

    Ok(ContextGuardianReport {
        enabled: true,
        checked_at: Utc::now().to_rfc3339(),
        context_state: Some(status.context_state),
        latest_session_id: Some(summary.session_id.to_string()),
        latest_event_count: summary.event_count,
        risk_score,
        action,
        compacted,
        compact_packet_path,
        metadata_path,
        guardian_action_id: Some(action_id),
        handoff_path,
        recommended_next_action,
    })
}

pub fn render_guardian_report(report: &ContextGuardianReport) -> String {
    format!(
        "context_guardian: {}\nchecked_at: {}\ncontext_state: {}\nlatest_session_id: {}\nlatest_event_count: {}\nrisk_score: {}\naction: {:?}\ncompacted: {}\ncompact_packet_path: {}\nmetadata_path: {}\nguardian_action_id: {}\nhandoff_path: {}\nrecommended_next_action: {}\n",
        if report.enabled {
            "enabled"
        } else {
            "disabled"
        },
        report.checked_at,
        report
            .context_state
            .as_ref()
            .map(context_state_label)
            .unwrap_or("unknown"),
        report.latest_session_id.as_deref().unwrap_or("none"),
        report.latest_event_count,
        report.risk_score,
        report.action,
        report.compacted,
        report
            .compact_packet_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        report
            .metadata_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        report.guardian_action_id.as_deref().unwrap_or("none"),
        report
            .handoff_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "none".to_string()),
        report.recommended_next_action
    )
}

fn score_risk(state: &ContextState, event_count: usize) -> u8 {
    let base = match state {
        ContextState::Green => 1,
        ContextState::Yellow => 5,
        ContextState::Red => 8,
    };
    let event_pressure = if event_count >= 80 {
        2
    } else if event_count >= 40 {
        1
    } else {
        0
    };
    (base + event_pressure).min(10)
}

fn compact_reason(
    status: &context_status::ContextStatusReport,
    summary: &sessions::SessionSummary,
    compact_packet_path: Option<&PathBuf>,
    metadata: Option<&GuardianMetadata>,
    force: bool,
) -> Option<&'static str> {
    if force {
        return Some("force");
    }
    if !status.compact_packet_present || compact_packet_path.is_none() {
        return Some("compact_missing");
    }
    if status.compact_packet_stale {
        return Some("compact_stale");
    }
    let metadata = metadata?;
    if metadata.metadata_schema_version != GUARDIAN_METADATA_SCHEMA_VERSION
        || metadata.compact_schema_version != COMPACT_SCHEMA_VERSION
    {
        return Some("schema_version_changed");
    }
    if metadata.latest_session_id != summary.session_id.to_string()
        || metadata.latest_event_count != summary.event_count
    {
        return Some("latest_session_changed");
    }
    if let Some(compact_path) = compact_packet_path
        && (!metadata.compact_packet_path.exists()
            || metadata.compact_packet_path != *compact_path
            || metadata.compact_artifact_hash != artifact_hash(compact_path).unwrap_or_default())
    {
        return Some("compact_marker_outdated");
    }
    None
}

struct GuardianMetadataInput {
    latest_session_id: String,
    latest_event_count: usize,
    compact_packet_path: PathBuf,
    compact_created_at: String,
    guardian_action_id: String,
    action: GuardianAction,
}

fn metadata_path(cfg: &Config) -> PathBuf {
    cfg.context_guardian
        .branch_packet_path
        .parent()
        .map(|path| path.join("metadata.json"))
        .unwrap_or_else(|| {
            cfg.workspace_dir
                .join("state/context-guardian/metadata.json")
        })
}

fn read_metadata(cfg: &Config) -> Result<GuardianMetadata> {
    let path = metadata_path(cfg);
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn write_metadata(cfg: &Config, input: GuardianMetadataInput) -> Result<PathBuf> {
    let path = metadata_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let metadata = GuardianMetadata {
        metadata_schema_version: GUARDIAN_METADATA_SCHEMA_VERSION,
        compact_schema_version: COMPACT_SCHEMA_VERSION,
        latest_session_id: input.latest_session_id,
        latest_event_count: input.latest_event_count,
        compact_artifact_hash: artifact_hash(&input.compact_packet_path)?,
        compact_packet_path: input.compact_packet_path,
        compact_created_at: input.compact_created_at,
        guardian_action_id: input.guardian_action_id,
        action: input.action,
        updated_at: Utc::now().to_rfc3339(),
    };
    fs::write(
        &path,
        format!("{}\n", serde_json::to_string_pretty(&metadata)?),
    )
    .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn artifact_hash(path: &PathBuf) -> Result<String> {
    let bytes = fs::read(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    Ok(format!("{:016x}", hasher.finish()))
}

fn build_action_id(
    session_id: &str,
    status: &context_status::ContextStatusReport,
    compact_path: &Option<PathBuf>,
) -> String {
    let mut hasher = DefaultHasher::new();
    session_id.hash(&mut hasher);
    status.compact_packet_stale.hash(&mut hasher);
    compact_path.hash(&mut hasher);
    Utc::now()
        .timestamp_nanos_opt()
        .unwrap_or_default()
        .hash(&mut hasher);
    format!("guardian-action-{:016x}", hasher.finish())
}

fn write_handoff(
    cfg: &Config,
    status: &context_status::ContextStatusReport,
    event_count: usize,
    risk_score: u8,
    compact_packet_path: Option<&PathBuf>,
) -> Result<PathBuf> {
    let path = cfg.context_guardian.branch_packet_path.clone();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let compact_path = compact_packet_path
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "No compact packet available.".to_string());
    let missing = if status.missing_required_context.is_empty() {
        "- none".to_string()
    } else {
        status
            .missing_required_context
            .iter()
            .map(|item| format!("- {item}"))
            .collect::<Vec<_>>()
            .join("\n")
    };
    let session_id = status.latest_session_id.as_deref().unwrap_or("none");
    let body = format!(
        "# Quant-M Continuity Handoff\n\n\
## Fresh Thread Starter\n\n\
Continue from this Quant-M continuity handoff. Trust the session evidence and compact packet paths over broad memory. Start by opening the compact packet, then follow the next action.\n\n\
## Session\n\n\
- session_id: {session_id}\n\
- event_count: {event_count}\n\
- context_state: {}\n\
- risk_score: {risk_score}\n\
- compact_packet: {compact_path}\n\n\
## Missing Or Risky Context\n\n{missing}\n\n\
## Next Action\n\n{}\n\n\
## Do Not Redo\n\n\
- Do not branch again from this handoff until new session evidence is added.\n\
- Do not claim validation success unless the compact packet lists validation evidence.\n",
        context_state_label(&status.context_state),
        status.recommended_next_action,
    );
    fs::write(&path, body).with_context(|| format!("failed to write {}", path.display()))?;
    Ok(path)
}

fn context_state_label(state: &ContextState) -> &'static str {
    match state {
        ContextState::Green => "green",
        ContextState::Yellow => "yellow",
        ContextState::Red => "red",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::sessions::{SessionEvent, append_event, runtime_context};
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.context_guardian.branch_packet_path = cfg
            .workspace_dir
            .join("state/context-guardian/continuity-handoff.md");
        cfg.logging.file = cfg.workspace_dir.join("logs/quant-m.log");
        (tmp, cfg)
    }

    fn append_guardian_ready_session(cfg: &Config, node: &str) -> sessions::SessionContext {
        let context = runtime_context(node, "worker");
        append_event(
            cfg,
            &context,
            SessionEvent::Observation {
                message: "goal".to_string(),
                job_id: None,
                detail: Some(
                    "validate guardian for quantm/src/context_guardian.rs with shippable definition"
                        .to_string(),
                ),
            },
        )
        .expect("append goal");
        append_event(
            cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "validation".to_string(),
                input_preview: "run guardian tests".to_string(),
                command_preview: Some("cargo test context_guardian".to_string()),
                status: "ok".to_string(),
            },
        )
        .expect("append validation");
        append_event(
            cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.http_get_sandbox_hosts".to_string(),
                allowed: false,
                reason: "host not allowed".to_string(),
            },
        )
        .expect("append policy");
        append_event(
            cfg,
            &context,
            SessionEvent::Output {
                channel: "guardian".to_string(),
                summary: "context guardian feature path validated".to_string(),
                job_id: None,
            },
        )
        .expect("append output");
        context
    }

    #[test]
    fn red_context_scores_as_handoff_risk() {
        assert!(score_risk(&ContextState::Red, 1) >= 6);
    }

    #[test]
    fn event_pressure_increases_green_risk_slightly() {
        assert!(score_risk(&ContextState::Green, 80) > score_risk(&ContextState::Green, 1));
    }

    #[test]
    fn tick_writes_handoff_and_metadata() {
        let (_tmp, cfg) = temp_cfg();
        append_guardian_ready_session(&cfg, "node-guardian-first");

        let report = tick(&cfg).expect("guardian tick");

        assert!(report.compacted);
        assert_eq!(report.action, GuardianAction::HandoffReady);
        assert!(report.handoff_path.as_ref().expect("handoff").exists());
        assert!(report.metadata_path.as_ref().expect("metadata").exists());
        assert!(report.guardian_action_id.is_some());
    }

    #[test]
    fn tick_avoids_repeated_compaction_for_current_session() {
        let (_tmp, cfg) = temp_cfg();
        append_guardian_ready_session(&cfg, "node-guardian-repeat");

        let first = tick(&cfg).expect("first tick");
        let first_compact = first.compact_packet_path.clone().expect("compact");
        let first_modified = fs::metadata(&first_compact)
            .expect("first compact metadata")
            .modified()
            .expect("first modified");
        let second = tick(&cfg).expect("second tick");
        let second_modified = fs::metadata(&first_compact)
            .expect("second compact metadata")
            .modified()
            .expect("second modified");

        assert!(first.compacted);
        assert!(!second.compacted);
        assert_eq!(first_modified, second_modified);
    }

    #[test]
    fn force_recompacts_current_session() {
        let (_tmp, cfg) = temp_cfg();
        append_guardian_ready_session(&cfg, "node-guardian-force");

        let first = tick(&cfg).expect("first tick");
        let second =
            tick_with_options(&cfg, GuardianTickOptions::force()).expect("forced guardian tick");

        assert!(first.compacted);
        assert!(second.compacted);
    }

    #[test]
    fn new_session_recompacts() {
        let (_tmp, cfg) = temp_cfg();
        append_guardian_ready_session(&cfg, "node-guardian-old");
        let first = tick(&cfg).expect("first tick");
        append_guardian_ready_session(&cfg, "node-guardian-new");
        let second = tick(&cfg).expect("new session tick");

        assert!(first.compacted);
        assert!(second.compacted);
        assert_ne!(first.latest_session_id, second.latest_session_id);
    }

    #[test]
    fn stale_compact_recompacts() {
        let (_tmp, cfg) = temp_cfg();
        let context = append_guardian_ready_session(&cfg, "node-guardian-stale");
        let first = tick(&cfg).expect("first tick");
        append_event(
            &cfg,
            &context,
            SessionEvent::AuditNote {
                note: "new evidence after compact".to_string(),
            },
        )
        .expect("append stale-making event");
        let second = tick(&cfg).expect("stale tick");

        assert!(first.compacted);
        assert!(second.compacted);
        assert_eq!(first.latest_session_id, second.latest_session_id);
    }

    #[test]
    fn disabled_config_skips_guardian() {
        let (_tmp, mut cfg) = temp_cfg();
        cfg.context_guardian.enabled = false;
        append_guardian_ready_session(&cfg, "node-guardian-disabled");

        let report = tick(&cfg).expect("disabled tick");

        assert_eq!(report.action, GuardianAction::Disabled);
        assert!(!report.compacted);
        assert!(report.handoff_path.is_none());
    }

    #[test]
    fn malformed_metadata_is_repaired_without_recompaction() {
        let (_tmp, cfg) = temp_cfg();
        append_guardian_ready_session(&cfg, "node-guardian-malformed");
        let first = tick(&cfg).expect("first tick");
        let metadata = metadata_path(&cfg);
        fs::write(&metadata, "{not json").expect("write malformed metadata");

        let second = tick(&cfg).expect("second tick repairs metadata");
        let repaired = fs::read_to_string(&metadata).expect("read repaired metadata");

        assert!(first.compacted);
        assert!(!second.compacted);
        assert!(serde_json::from_str::<GuardianMetadata>(&repaired).is_ok());
    }

    #[tokio::test(flavor = "current_thread")]
    async fn daemon_guardian_loop_starts_and_stops_cleanly() {
        let (_tmp, cfg) = temp_cfg();
        append_guardian_ready_session(&cfg, "node-guardian-loop");
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task = tokio::spawn(run_loop_with_shutdown(cfg.clone(), Some(shutdown_rx)));
        tokio::task::yield_now().await;
        shutdown_tx.send(true).expect("send shutdown");
        let result = tokio::time::timeout(Duration::from_secs(2), task)
            .await
            .expect("loop stopped")
            .expect("task joined");

        assert!(result.is_ok());
        assert!(cfg.logging.file.exists());
    }
}
