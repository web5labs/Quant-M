use crate::compaction::CompactPacket;
use crate::config::Config;
use crate::fsm_core::{
    ContextGuardianEvent, ContextGuardianFsm, ContextGuardianState, ContextRecommendedAction,
    TransitionRecord, transition_record,
};
use crate::sessions::{self, SessionEvent, SessionId};
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MISSING_VALIDATION: &str = "No validation evidence found.";
const MISSING_CHANGED_FILES: &str = "No changed-file evidence found.";
const MISSING_SHIPPABLE: &str = "No shippable definition found.";
const NO_POLICY_BLOCKS: &str = "No policy blocks found.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextState {
    Green,
    Yellow,
    Red,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextStatusReport {
    pub context_state: ContextState,
    pub guardian_state: ContextGuardianState,
    pub context_event: Option<ContextGuardianEvent>,
    pub recommended_action: ContextRecommendedAction,
    pub fsm_transition: Option<TransitionRecord>,
    pub blocked: bool,
    pub operator_review_required: bool,
    pub latest_session_id: Option<String>,
    pub latest_event_count: usize,
    pub latest_compact_packet_path: Option<PathBuf>,
    pub compact_packet_present: bool,
    pub compact_packet_stale: bool,
    pub policy_block_present: bool,
    pub validation_evidence_present: bool,
    pub changed_file_evidence_present: bool,
    pub shippable_definition_present: bool,
    pub missing_required_context: Vec<String>,
    pub recommended_next_action: String,
}

pub fn context_status(cfg: &Config) -> Result<ContextStatusReport> {
    let latest_session = sessions::list_sessions(cfg)?.into_iter().next();
    let latest_session_id = latest_session
        .as_ref()
        .map(|summary| summary.session_id.to_string());

    let Some(session_summary) = latest_session else {
        return Ok(ContextStatusReport {
            context_state: ContextState::Red,
            guardian_state: ContextGuardianState::NoSession,
            context_event: None,
            recommended_action: ContextRecommendedAction::Observe,
            fsm_transition: None,
            blocked: false,
            operator_review_required: false,
            latest_session_id: None,
            latest_event_count: 0,
            latest_compact_packet_path: None,
            compact_packet_present: false,
            compact_packet_stale: false,
            policy_block_present: false,
            validation_evidence_present: false,
            changed_file_evidence_present: false,
            shippable_definition_present: false,
            missing_required_context: vec![
                "No session history found.".to_string(),
                "No compact packet found.".to_string(),
            ],
            recommended_next_action:
                "Run a Quant-M workflow first, then compact the resulting session.".to_string(),
        });
    };

    let session_id = session_summary.session_id.clone();
    let compact_path = compact_json_path(cfg, &session_id);
    if !compact_path.exists() {
        let session_detail = sessions::show_session(cfg, &session_id)?;
        let execution_session = is_execution_session(&session_detail.events);
        let policy_block_present = has_policy_block_event(&session_detail.events);
        let validation_evidence_present = has_validation_event(&session_detail.events);
        let changed_file_evidence_present = has_changed_file_event(&session_detail.events);
        let shippable_definition_present = has_shippable_event(&session_detail.events);
        let mut missing = vec!["No compact packet found.".to_string()];
        if execution_session && !policy_block_present {
            missing.push("No policy block evidence found for execution session.".to_string());
        }
        let context_eval = evaluate_context_guardian(ContextGuardianEvaluationInput {
            display_state: &ContextState::Red,
            event_count: session_summary.event_count,
            compact_present: false,
            compact_stale: false,
            policy_block_present,
            validation_evidence_present,
            changed_file_evidence_present,
            shippable_definition_present,
            evidence_ref: compact_path.to_string_lossy().to_string(),
        });
        return Ok(ContextStatusReport {
            context_state: ContextState::Red,
            guardian_state: context_eval.state,
            context_event: Some(context_eval.event),
            recommended_action: context_eval.action,
            fsm_transition: Some(context_eval.transition),
            blocked: context_eval.blocked,
            operator_review_required: context_eval.operator_review_required,
            latest_session_id,
            latest_event_count: session_summary.event_count,
            latest_compact_packet_path: None,
            compact_packet_present: false,
            compact_packet_stale: false,
            policy_block_present,
            validation_evidence_present,
            changed_file_evidence_present,
            shippable_definition_present,
            missing_required_context: with_project_file_missing(cfg, missing),
            recommended_next_action: format!("Run quant-m compact {}", session_id),
        });
    }

    let packet = read_compact_packet(&compact_path)?;
    let stale = packet.session_id != session_id.to_string()
        || packet.source_event_count != session_summary.event_count
        || compact_is_older_than_session(&packet.created_at, &session_summary.last_event_at);
    let policy_block_present = packet
        .policy_blocks
        .iter()
        .any(|value| value != NO_POLICY_BLOCKS);
    let validation_evidence_present = packet
        .commands_observed
        .iter()
        .any(|value| value != MISSING_VALIDATION);
    let changed_file_evidence_present = packet
        .files_changed
        .iter()
        .any(|value| value != MISSING_CHANGED_FILES);
    let shippable_definition_present = packet.definition_of_shippable != MISSING_SHIPPABLE;

    let mut missing = Vec::new();
    if stale {
        missing.push("Compact packet is stale for the latest session state.".to_string());
    }
    if !policy_block_present {
        missing.push("No policy block evidence found for execution session.".to_string());
    }
    if !validation_evidence_present {
        missing.push(MISSING_VALIDATION.to_string());
    }
    if !changed_file_evidence_present {
        missing.push(MISSING_CHANGED_FILES.to_string());
    }
    if !shippable_definition_present {
        missing.push(MISSING_SHIPPABLE.to_string());
    }
    let missing = with_project_file_missing(cfg, missing);

    let context_state = classify_context(
        stale,
        policy_block_present,
        validation_evidence_present,
        changed_file_evidence_present,
        shippable_definition_present,
    );
    let recommended_next_action = recommended_next_action(
        &context_state,
        &session_id,
        stale,
        validation_evidence_present,
        shippable_definition_present,
    );
    let context_eval = evaluate_context_guardian(ContextGuardianEvaluationInput {
        display_state: &context_state,
        event_count: session_summary.event_count,
        compact_present: true,
        compact_stale: stale,
        policy_block_present,
        validation_evidence_present,
        changed_file_evidence_present,
        shippable_definition_present,
        evidence_ref: compact_path.to_string_lossy().to_string(),
    });

    Ok(ContextStatusReport {
        context_state,
        guardian_state: context_eval.state,
        context_event: Some(context_eval.event),
        recommended_action: context_eval.action,
        fsm_transition: Some(context_eval.transition),
        blocked: context_eval.blocked,
        operator_review_required: context_eval.operator_review_required,
        latest_session_id,
        latest_event_count: session_summary.event_count,
        latest_compact_packet_path: Some(compact_path),
        compact_packet_present: true,
        compact_packet_stale: stale,
        policy_block_present,
        validation_evidence_present,
        changed_file_evidence_present,
        shippable_definition_present,
        missing_required_context: missing,
        recommended_next_action,
    })
}

struct ContextGuardianEvaluation {
    state: ContextGuardianState,
    event: ContextGuardianEvent,
    action: ContextRecommendedAction,
    transition: TransitionRecord,
    blocked: bool,
    operator_review_required: bool,
}

struct ContextGuardianEvaluationInput<'a> {
    display_state: &'a ContextState,
    event_count: usize,
    compact_present: bool,
    compact_stale: bool,
    policy_block_present: bool,
    validation_evidence_present: bool,
    changed_file_evidence_present: bool,
    shippable_definition_present: bool,
    evidence_ref: String,
}

fn evaluate_context_guardian(
    input: ContextGuardianEvaluationInput<'_>,
) -> ContextGuardianEvaluation {
    let (event, action) = if !input.compact_present {
        (
            ContextGuardianEvent::ContextMeasured,
            ContextRecommendedAction::Compact,
        )
    } else if input.compact_stale {
        (
            ContextGuardianEvent::CompactExpired,
            ContextRecommendedAction::RefreshCompact,
        )
    } else if matches!(input.display_state, ContextState::Red) {
        (
            ContextGuardianEvent::Block,
            ContextRecommendedAction::BlockContinuation,
        )
    } else if !input.policy_block_present
        || !input.validation_evidence_present
        || !input.changed_file_evidence_present
        || !input.shippable_definition_present
    {
        (
            ContextGuardianEvent::ReviewRequested,
            ContextRecommendedAction::RequestOperatorReview,
        )
    } else if input.event_count >= 40 {
        (
            ContextGuardianEvent::NearBudgetLimit,
            ContextRecommendedAction::Compact,
        )
    } else {
        (
            ContextGuardianEvent::CompactCreated,
            ContextRecommendedAction::Continue,
        )
    };

    let from_state = match event {
        ContextGuardianEvent::CompactExpired => ContextGuardianState::CompactFresh,
        _ => ContextGuardianState::Observing,
    };
    let transition = transition_record(
        &ContextGuardianFsm,
        &from_state,
        &event,
        Utc::now().to_rfc3339(),
        Some(input.evidence_ref),
    );
    let state = transition
        .next_state
        .as_deref()
        .and_then(|value| match value {
            "healthy" => Some(ContextGuardianState::Healthy),
            "near_limit" => Some(ContextGuardianState::NearLimit),
            "needs_compact" => Some(ContextGuardianState::NeedsCompact),
            "compact_fresh" => Some(ContextGuardianState::CompactFresh),
            "compact_stale" => Some(ContextGuardianState::CompactStale),
            "handoff_ready" => Some(ContextGuardianState::HandoffReady),
            "operator_review_required" => Some(ContextGuardianState::OperatorReviewRequired),
            "blocked" => Some(ContextGuardianState::Blocked),
            _ => None,
        })
        .unwrap_or(ContextGuardianState::Blocked);

    ContextGuardianEvaluation {
        state,
        event,
        action,
        transition,
        blocked: state == ContextGuardianState::Blocked
            || action == ContextRecommendedAction::BlockContinuation,
        operator_review_required: state == ContextGuardianState::OperatorReviewRequired
            || action == ContextRecommendedAction::RequestOperatorReview,
    }
}

pub fn render_context_status(report: &ContextStatusReport) -> String {
    format!(
        "context_state: {}\nguardian_state: {}\nrecommended_action: {}\nblocked: {}\noperator_review_required: {}\nlatest_session_id: {}\nlatest_event_count: {}\ncompact_packet_present: {}\ncompact_packet_stale: {}\npolicy_block_present: {}\nvalidation_evidence_present: {}\nchanged_file_evidence_present: {}\nshippable_definition_present: {}\nrecommended_next_action: {}\nmissing_required_context:\n{}\n",
        context_state_label(&report.context_state),
        report.guardian_state,
        report.recommended_action,
        report.blocked,
        report.operator_review_required,
        report.latest_session_id.as_deref().unwrap_or("none"),
        report.latest_event_count,
        report.compact_packet_present,
        report.compact_packet_stale,
        report.policy_block_present,
        report.validation_evidence_present,
        report.changed_file_evidence_present,
        report.shippable_definition_present,
        report.recommended_next_action,
        render_missing(&report.missing_required_context)
    )
}

fn context_state_label(state: &ContextState) -> &'static str {
    match state {
        ContextState::Green => "green",
        ContextState::Yellow => "yellow",
        ContextState::Red => "red",
    }
}

fn render_missing(items: &[String]) -> String {
    if items.is_empty() {
        return "- none".to_string();
    }
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn read_compact_packet(path: &PathBuf) -> Result<CompactPacket> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to parse {}", path.display()))
}

fn compact_json_path(cfg: &Config, session_id: &SessionId) -> PathBuf {
    cfg.workspace_dir
        .join("state")
        .join("compacted")
        .join(session_id.as_str())
        .join("compact.json")
}

fn compact_is_older_than_session(compact_created_at: &str, session_last_event_at: &str) -> bool {
    let compact_time =
        DateTime::parse_from_rfc3339(compact_created_at).map(|value| value.with_timezone(&Utc));
    let session_time =
        DateTime::parse_from_rfc3339(session_last_event_at).map(|value| value.with_timezone(&Utc));
    match (compact_time, session_time) {
        (Ok(compact_time), Ok(session_time)) => compact_time < session_time,
        _ => false,
    }
}

fn classify_context(
    stale: bool,
    policy_block_present: bool,
    validation_evidence_present: bool,
    changed_file_evidence_present: bool,
    shippable_definition_present: bool,
) -> ContextState {
    if stale
        || !policy_block_present
        || (!validation_evidence_present && !shippable_definition_present)
    {
        return ContextState::Red;
    }
    if !validation_evidence_present
        || !changed_file_evidence_present
        || !shippable_definition_present
    {
        return ContextState::Yellow;
    }
    ContextState::Green
}

fn recommended_next_action(
    state: &ContextState,
    session_id: &SessionId,
    stale: bool,
    validation_evidence_present: bool,
    shippable_definition_present: bool,
) -> String {
    if stale {
        return format!("Run quant-m compact {}", session_id);
    }
    match state {
        ContextState::Green => "Context is ready for the next safe agent action.".to_string(),
        ContextState::Yellow => {
            if !validation_evidence_present {
                "Collect validation evidence before execution or shipping claims.".to_string()
            } else if !shippable_definition_present {
                "Add or capture a shippable definition before shipcheck.".to_string()
            } else {
                "Review missing context before continuing.".to_string()
            }
        }
        ContextState::Red => {
            if !validation_evidence_present && !shippable_definition_present {
                "Do not continue execution; compacted context lacks validation and shippable evidence.".to_string()
            } else {
                "Do not continue execution until required context is restored.".to_string()
            }
        }
    }
}

fn with_project_file_missing(cfg: &Config, mut missing: Vec<String>) -> Vec<String> {
    for file in ["QUANTM.md", "POLICY.md", "SHIPPABLE.md", "AGENTS.md"] {
        if !truth_file_exists(cfg, file) {
            missing.push(format!("{file} not found."));
        }
    }
    missing
}

fn truth_file_exists(cfg: &Config, file: &str) -> bool {
    cfg.workspace_dir.join(file).exists()
        || std::env::current_dir()
            .map(|cwd| cwd.join(file).exists())
            .unwrap_or(false)
}

fn is_execution_session(events: &[sessions::SessionLogEntry]) -> bool {
    events.iter().any(|entry| {
        matches!(
            entry.event,
            SessionEvent::SkillCall { .. }
                | SessionEvent::FsmTransition { .. }
                | SessionEvent::Output { .. }
                | SessionEvent::Error { .. }
                | SessionEvent::Retry { .. }
        )
    })
}

fn has_policy_block_event(events: &[sessions::SessionLogEntry]) -> bool {
    events.iter().any(|entry| {
        matches!(
            entry.event,
            SessionEvent::PolicyDecision { allowed: false, .. }
        )
    })
}

fn has_validation_event(events: &[sessions::SessionLogEntry]) -> bool {
    events.iter().any(|entry| {
        matches!(
            entry.event,
            SessionEvent::SkillCall {
                command_preview: Some(_),
                ..
            }
        )
    })
}

fn has_changed_file_event(events: &[sessions::SessionLogEntry]) -> bool {
    events.iter().any(|entry| {
        let text = match &entry.event {
            SessionEvent::Observation {
                message, detail, ..
            } => {
                format!("{} {}", message, detail.clone().unwrap_or_default())
            }
            SessionEvent::Output { summary, .. } => summary.clone(),
            SessionEvent::AuditNote { note } => note.clone(),
            _ => String::new(),
        };
        text.contains(".rs")
            || text.contains(".md")
            || text.contains(".json")
            || text.contains(".toml")
    })
}

fn has_shippable_event(events: &[sessions::SessionLogEntry]) -> bool {
    events.iter().any(|entry| {
        let text = match &entry.event {
            SessionEvent::Observation {
                message, detail, ..
            } => {
                format!("{} {}", message, detail.clone().unwrap_or_default())
            }
            SessionEvent::Output { summary, .. } => summary.clone(),
            SessionEvent::AuditNote { note } => note.clone(),
            _ => String::new(),
        };
        text.to_ascii_lowercase().contains("shippable")
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction;
    use crate::config::Config;
    use crate::sessions::{SessionContext, append_event, runtime_context};
    use crate::truth_files;
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        std::fs::create_dir_all(&cfg.workspace_dir).expect("workspace");
        (tmp, cfg)
    }

    fn write_truth_files(cfg: &Config) {
        for file in ["QUANTM.md", "POLICY.md", "SHIPPABLE.md", "AGENTS.md"] {
            std::fs::write(cfg.workspace_dir.join(file), format!("# {file}\n"))
                .expect("truth file");
        }
    }

    fn session_with_all_evidence(cfg: &Config) -> SessionContext {
        let context = runtime_context("context-node", "worker");
        append_event(
            cfg,
            &context,
            SessionEvent::Observation {
                message: "goal".to_string(),
                job_id: None,
                detail: Some(
                    "update quantm/src/context_status.rs shippable when tests pass".to_string(),
                ),
            },
        )
        .expect("goal");
        append_event(
            cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled until approval".to_string(),
            },
        )
        .expect("policy");
        append_event(
            cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "validation".to_string(),
                input_preview: "test".to_string(),
                command_preview: Some("cargo test context_status".to_string()),
                status: "ok".to_string(),
            },
        )
        .expect("skill");
        append_event(
            cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "updated quantm/src/context_status.rs and shippable definition satisfied"
                    .to_string(),
                job_id: None,
            },
        )
        .expect("output");
        context
    }

    fn session_missing_validation(cfg: &Config) -> SessionContext {
        let context = runtime_context("context-node-missing-validation", "worker");
        append_event(
            cfg,
            &context,
            SessionEvent::Observation {
                message: "goal".to_string(),
                job_id: None,
                detail: Some(
                    "update quantm/src/context_status.rs shippable when reviewed".to_string(),
                ),
            },
        )
        .expect("goal");
        append_event(
            cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled until approval".to_string(),
            },
        )
        .expect("policy");
        append_event(
            cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "updated quantm/src/context_status.rs shippable definition captured"
                    .to_string(),
                job_id: None,
            },
        )
        .expect("output");
        context
    }

    #[test]
    fn reports_green_when_all_context_is_present() {
        let (_tmp, cfg) = temp_cfg();
        write_truth_files(&cfg);
        let context = session_with_all_evidence(&cfg);
        compaction::compact_session(&cfg, &context.session_id).expect("compact");

        let report = context_status(&cfg).expect("status");

        assert_eq!(report.context_state, ContextState::Green);
        assert_eq!(report.guardian_state, ContextGuardianState::CompactFresh);
        assert_eq!(
            report.recommended_action,
            ContextRecommendedAction::Continue
        );
        assert!(!report.blocked);
        assert!(report.compact_packet_present);
        assert!(report.policy_block_present);
        assert!(report.validation_evidence_present);
        assert!(report.changed_file_evidence_present);
        assert!(report.shippable_definition_present);
    }

    #[test]
    fn reports_yellow_when_compact_exists_but_validation_is_missing() {
        let (_tmp, cfg) = temp_cfg();
        write_truth_files(&cfg);
        let context = session_missing_validation(&cfg);
        compaction::compact_session(&cfg, &context.session_id).expect("compact");

        let report = context_status(&cfg).expect("status");

        assert_eq!(report.context_state, ContextState::Yellow);
        assert_eq!(
            report.guardian_state,
            ContextGuardianState::OperatorReviewRequired
        );
        assert_eq!(
            report.recommended_action,
            ContextRecommendedAction::RequestOperatorReview
        );
        assert!(report.operator_review_required);
        assert!(report.compact_packet_present);
        assert!(!report.validation_evidence_present);
    }

    #[test]
    fn reports_red_when_no_compact_packet_exists() {
        let (_tmp, cfg) = temp_cfg();
        write_truth_files(&cfg);
        let context = session_with_all_evidence(&cfg);
        assert!(context.session_id.as_str().starts_with("session-"));

        let report = context_status(&cfg).expect("status");

        assert_eq!(report.context_state, ContextState::Red);
        assert_eq!(report.guardian_state, ContextGuardianState::NeedsCompact);
        assert_eq!(report.recommended_action, ContextRecommendedAction::Compact);
        assert!(!report.compact_packet_present);
        assert!(report.recommended_next_action.contains("quant-m compact"));
    }

    #[test]
    fn reports_red_when_policy_block_is_missing_for_execution_session() {
        let (_tmp, cfg) = temp_cfg();
        write_truth_files(&cfg);
        let context = runtime_context("context-node-no-policy", "worker");
        append_event(
            &cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "validation".to_string(),
                input_preview: "test".to_string(),
                command_preview: Some("cargo test context_status".to_string()),
                status: "ok".to_string(),
            },
        )
        .expect("skill");
        append_event(
            &cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "updated quantm/src/context_status.rs shippable definition captured"
                    .to_string(),
                job_id: None,
            },
        )
        .expect("output");
        compaction::compact_session(&cfg, &context.session_id).expect("compact");

        let report = context_status(&cfg).expect("status");

        assert_eq!(report.context_state, ContextState::Red);
        assert_eq!(report.guardian_state, ContextGuardianState::Blocked);
        assert_eq!(
            report.recommended_action,
            ContextRecommendedAction::BlockContinuation
        );
        assert!(report.blocked);
        assert!(!report.policy_block_present);
    }

    #[test]
    fn handles_empty_workspace_safely() {
        let (_tmp, cfg) = temp_cfg();

        let report = context_status(&cfg).expect("status");

        assert_eq!(report.context_state, ContextState::Red);
        assert_eq!(report.guardian_state, ContextGuardianState::NoSession);
        assert_eq!(report.recommended_action, ContextRecommendedAction::Observe);
        assert!(report.latest_session_id.is_none());
        assert!(!report.compact_packet_present);
    }

    #[test]
    fn detects_generated_truth_files() {
        let (_tmp, cfg) = temp_cfg();
        truth_files::init_truth_files(&cfg, false).expect("truth files");

        let report = context_status(&cfg).expect("status");

        assert!(
            !report
                .missing_required_context
                .iter()
                .any(|item| item.contains("QUANTM.md"))
        );
        assert!(
            !report
                .missing_required_context
                .iter()
                .any(|item| item.contains("POLICY.md"))
        );
        assert!(
            !report
                .missing_required_context
                .iter()
                .any(|item| item.contains("SHIPPABLE.md"))
        );
        assert!(
            !report
                .missing_required_context
                .iter()
                .any(|item| item.contains("AGENTS.md"))
        );
    }

    #[test]
    fn render_supports_plain_output() {
        let (_tmp, cfg) = temp_cfg();
        let report = context_status(&cfg).expect("status");

        let rendered = render_context_status(&report);

        assert!(rendered.contains("context_state"));
        assert!(rendered.contains("guardian_state"));
        assert!(rendered.contains("recommended_action"));
        assert!(rendered.contains("recommended_next_action"));
    }

    #[test]
    fn stale_compact_maps_to_refresh_compact_action() {
        let (_tmp, cfg) = temp_cfg();
        write_truth_files(&cfg);
        let context = session_with_all_evidence(&cfg);
        compaction::compact_session(&cfg, &context.session_id).expect("compact");
        append_event(
            &cfg,
            &context,
            SessionEvent::AuditNote {
                note: "new post-compact context".to_string(),
            },
        )
        .expect("append stale marker");

        let report = context_status(&cfg).expect("status");

        assert_eq!(report.guardian_state, ContextGuardianState::CompactStale);
        assert_eq!(
            report.recommended_action,
            ContextRecommendedAction::RefreshCompact
        );
    }
}
