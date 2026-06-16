use crate::config::Config;
use crate::consensus;
use crate::sessions::{DomainId, SessionId};
use crate::shared_state::{self, SharedStateRecord, SharedStateValue};
use anyhow::Result;
use chrono::DateTime;
use serde::{Deserialize, Serialize};
use serde_json::Value;

const DEFAULT_REVIEW_DOMAIN: &str = "domain:consensus";
const CONFIDENCE_THRESHOLD: f64 = 0.70;
const FRESHNESS_THRESHOLD: f64 = 0.50;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StateRecordReviewStatus {
    Usable,
    NeedsReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StateReviewStatus {
    Ok,
    NeedsReview,
    Empty,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateReviewRecord {
    pub key: String,
    pub session_id: Option<String>,
    pub workflow_id: Option<String>,
    pub decision_question: Option<String>,
    pub memory_class: Option<String>,
    pub confidence: Option<f64>,
    pub freshness: Option<f64>,
    pub source_count: Option<usize>,
    pub contradiction_count: Option<usize>,
    pub last_verified_at: Option<String>,
    pub policy_result: Option<String>,
    pub replay_status: Option<String>,
    pub artifact_status: Option<String>,
    pub shared_state_status: Option<String>,
    pub recommended_next_command: Option<String>,
    pub freshness_status: String,
    pub status: StateRecordReviewStatus,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StateReviewReport {
    pub domain: String,
    pub records: Vec<StateReviewRecord>,
    pub total_count: usize,
    pub stale_count: usize,
    pub contradicted_count: usize,
    pub canonical_count: usize,
    pub tactical_count: usize,
    pub strategic_count: usize,
    pub ephemeral_count: usize,
    pub review_status: StateReviewStatus,
    pub next_recommended_command: String,
}

pub fn review_state(cfg: &Config, domain: Option<&str>) -> Result<StateReviewReport> {
    let domain = normalize_domain(domain);
    let domain_id = domain.parse::<DomainId>()?;
    let records = shared_state::snapshot_state(cfg, Some(&domain_id))?;
    let mut reviewed = Vec::new();
    for record in records {
        reviewed.push(review_record(cfg, record));
    }

    let total_count = reviewed.len();
    let stale_count = reviewed
        .iter()
        .filter(|record| record.freshness_status == "stale")
        .count();
    let contradicted_count = reviewed
        .iter()
        .filter(|record| record.contradiction_count.unwrap_or(0) > 0)
        .count();
    let canonical_count = count_memory_class(&reviewed, "canonical");
    let tactical_count = count_memory_class(&reviewed, "tactical");
    let strategic_count = count_memory_class(&reviewed, "strategic");
    let ephemeral_count = count_memory_class(&reviewed, "ephemeral");
    let review_status = if total_count == 0 {
        StateReviewStatus::Empty
    } else if reviewed
        .iter()
        .any(|record| record.status == StateRecordReviewStatus::NeedsReview)
    {
        StateReviewStatus::NeedsReview
    } else {
        StateReviewStatus::Ok
    };
    let next_recommended_command = match review_status {
        StateReviewStatus::Empty => {
            "quant-m consensus --dry-run \"Should we adopt this API design?\"".to_string()
        }
        StateReviewStatus::Ok => "quant-m replay <session_id>".to_string(),
        StateReviewStatus::NeedsReview => "quant-m replay <session_id> --json".to_string(),
    };

    Ok(StateReviewReport {
        domain,
        records: reviewed,
        total_count,
        stale_count,
        contradicted_count,
        canonical_count,
        tactical_count,
        strategic_count,
        ephemeral_count,
        review_status,
        next_recommended_command,
    })
}

pub fn render_state_review(report: &StateReviewReport) -> String {
    if report.records.is_empty() {
        return format!(
            "State review\n\
             domain: {}\n\
             status: empty\n\
             records: 0\n\
             next: {}\n",
            report.domain, report.next_recommended_command
        );
    }

    let mut out = format!(
        "State review\n\
         domain: {}\n\
         status: {:?}\n\
         total: {} | stale: {} | contradicted: {} | tactical: {} | strategic: {} | canonical: {} | ephemeral: {}\n",
        report.domain,
        report.review_status,
        report.total_count,
        report.stale_count,
        report.contradicted_count,
        report.tactical_count,
        report.strategic_count,
        report.canonical_count,
        report.ephemeral_count,
    );

    for record in &report.records {
        out.push_str(&format!(
            "\n- decision: {}\n  session_id: {}\n  workflow_id: {}\n  memory_class: {}\n  confidence: {}\n  freshness: {}\n  source_count: {}\n  contradiction_count: {}\n  last_verified_at: {}\n  policy_result: {}\n  replay_status: {}\n  artifact_status: {}\n  shared_state_status: {}\n  status: {:?}\n  next: {}\n",
            record.decision_question.as_deref().unwrap_or("unknown"),
            record.session_id.as_deref().unwrap_or("unknown"),
            record.workflow_id.as_deref().unwrap_or("unknown"),
            record.memory_class.as_deref().unwrap_or("unknown"),
            format_optional_f64(record.confidence),
            format_optional_f64(record.freshness),
            record
                .source_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            record
                .contradiction_count
                .map(|value| value.to_string())
                .unwrap_or_else(|| "unknown".to_string()),
            record.last_verified_at.as_deref().unwrap_or("unknown"),
            record.policy_result.as_deref().unwrap_or("unknown"),
            record.replay_status.as_deref().unwrap_or("unknown"),
            record.artifact_status.as_deref().unwrap_or("unknown"),
            record.shared_state_status.as_deref().unwrap_or("unknown"),
            record.status,
            record
                .recommended_next_command
                .as_deref()
                .unwrap_or("quant-m replay <session_id> --json"),
        ));
        if !record.notes.is_empty() {
            out.push_str(&format!("  notes: {}\n", record.notes.join("; ")));
        }
    }
    out.push_str(&format!("\nnext: {}\n", report.next_recommended_command));
    out
}

fn review_record(cfg: &Config, record: SharedStateRecord) -> StateReviewRecord {
    let mut notes = Vec::new();
    let value = match record.value {
        SharedStateValue::Json(value) => value,
        _ => {
            notes.push("record value is not JSON".to_string());
            Value::Null
        }
    };

    let metadata = value.get("metadata");
    if metadata.is_none() {
        notes.push("missing metadata".to_string());
    }

    let session_id = string_field(&value, "session_id")
        .or_else(|| metadata.and_then(|value| string_field(value, "session_id")))
        .or_else(|| record.session_id.map(|value| value.to_string()));
    let workflow_id = string_field(&value, "workflow_id")
        .or_else(|| metadata.and_then(|value| string_field(value, "workflow_id")));
    let decision_question = string_field(&value, "decision_question");
    let memory_class = metadata
        .and_then(|value| string_field(value, "memory_class"))
        .map(|value| value.to_ascii_lowercase());
    let confidence = metadata
        .and_then(|value| number_field(value, "confidence"))
        .or(Some(record.confidence));
    let freshness = metadata.and_then(|value| number_field(value, "freshness"));
    let source_count = metadata.and_then(|value| usize_field(value, "source_count"));
    let contradiction_count = metadata.and_then(|value| usize_field(value, "contradiction_count"));
    let last_verified_at = metadata.and_then(|value| string_field(value, "last_verified_at"));
    let policy_result = string_field(&value, "policy_result");
    let recommended_next_command = session_id
        .as_ref()
        .map(|session_id| format!("quant-m replay {session_id}"));

    if memory_class
        .as_deref()
        .is_none_or(|class| !matches!(class, "ephemeral" | "tactical" | "strategic" | "canonical"))
    {
        notes.push("invalid or missing memory_class".to_string());
    }
    if last_verified_at
        .as_deref()
        .is_none_or(|value| DateTime::parse_from_rfc3339(value).is_err())
    {
        notes.push("missing or invalid last_verified_at".to_string());
    }
    if policy_result.is_none() {
        notes.push("missing policy_result".to_string());
    }
    if confidence.is_none_or(|value| value < CONFIDENCE_THRESHOLD) {
        notes.push("low confidence".to_string());
    }
    let freshness_status = if freshness.is_some_and(|value| value >= FRESHNESS_THRESHOLD)
        && last_verified_at
            .as_deref()
            .is_some_and(|value| DateTime::parse_from_rfc3339(value).is_ok())
    {
        "fresh".to_string()
    } else {
        notes.push("stale or missing freshness".to_string());
        "stale".to_string()
    };
    if contradiction_count.unwrap_or(0) > 0 {
        notes.push("contradictions present".to_string());
    }

    let mut replay_status = None;
    let mut artifact_status = None;
    let mut shared_state_status = None;
    if let Some(session_id_raw) = &session_id {
        match consensus::replay_consensus_session(cfg, &SessionId::new(session_id_raw.clone())) {
            Ok(summary) => {
                replay_status = Some(format!("{:?}", summary.replay_status));
                artifact_status = Some(format!("{:?}", summary.artifact_status));
                shared_state_status = Some(format!("{:?}", summary.shared_state_status));
            }
            Err(err) => {
                notes.push(format!("replay check failed: {err}"));
                replay_status = Some("NeedsReview".to_string());
                artifact_status = Some("NeedsReview".to_string());
                shared_state_status = Some("NeedsReview".to_string());
            }
        }
    } else {
        notes.push("missing session_id".to_string());
    }

    let status = if notes.is_empty() {
        StateRecordReviewStatus::Usable
    } else {
        StateRecordReviewStatus::NeedsReview
    };

    StateReviewRecord {
        key: record.key.to_string(),
        session_id,
        workflow_id,
        decision_question,
        memory_class,
        confidence,
        freshness,
        source_count,
        contradiction_count,
        last_verified_at,
        policy_result,
        replay_status,
        artifact_status,
        shared_state_status,
        recommended_next_command,
        freshness_status,
        status,
        notes,
    }
}

fn normalize_domain(domain: Option<&str>) -> String {
    match domain.map(str::trim).filter(|value| !value.is_empty()) {
        Some("consensus") => DEFAULT_REVIEW_DOMAIN.to_string(),
        Some(value) if value.starts_with("domain:") => value.to_string(),
        Some(value) => format!("domain:{value}"),
        None => DEFAULT_REVIEW_DOMAIN.to_string(),
    }
}

fn string_field(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
}

fn number_field(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn usize_field(value: &Value, key: &str) -> Option<usize> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
}

fn count_memory_class(records: &[StateReviewRecord], class: &str) -> usize {
    records
        .iter()
        .filter(|record| record.memory_class.as_deref() == Some(class))
        .count()
}

fn format_optional_f64(value: Option<f64>) -> String {
    value
        .map(|value| format!("{value:.2}"))
        .unwrap_or_else(|| "unknown".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use crate::shared_state::{HybridSharedStateStore, SharedStateKey, SharedStateStore};
    use serde_json::json;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().join("workspace");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        (tmp, cfg)
    }

    #[test]
    fn review_works_with_valid_consensus_record() {
        let (_tmp, cfg) = temp_cfg();
        crate::consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.total_count, 1);
        assert_eq!(review.review_status, StateReviewStatus::Ok);
        assert_eq!(review.tactical_count, 1);
        assert_eq!(review.records[0].status, StateRecordReviewStatus::Usable);
        assert_eq!(review.records[0].freshness_status, "fresh");
    }

    #[test]
    fn review_domain_filter_accepts_short_name() {
        let (_tmp, cfg) = temp_cfg();
        crate::consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.domain, "domain:consensus");
        assert_eq!(review.total_count, 1);
    }

    #[test]
    fn review_json_is_machine_readable() {
        let (_tmp, cfg) = temp_cfg();
        crate::consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let review = review_state(&cfg, None).expect("review");
        let value = serde_json::to_value(&review).expect("json");

        assert_eq!(value["domain"], "domain:consensus");
        assert_eq!(value["total_count"], 1);
        assert!(value["records"].is_array());
    }

    #[test]
    fn missing_metadata_is_reported_safely() {
        let (_tmp, cfg) = temp_cfg();
        put_raw_consensus_record(&cfg, json!({"session_id":"session:missing-meta"}), 0.9);
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.review_status, StateReviewStatus::NeedsReview);
        assert!(
            review.records[0]
                .notes
                .iter()
                .any(|note| note.contains("missing metadata"))
        );
    }

    #[test]
    fn low_confidence_needs_review() {
        let (_tmp, cfg) = temp_cfg();
        put_raw_consensus_record(&cfg, base_value(0.4, 1.0, 0), 0.4);
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.review_status, StateReviewStatus::NeedsReview);
        assert!(
            review.records[0]
                .notes
                .iter()
                .any(|note| note.contains("low confidence"))
        );
    }

    #[test]
    fn contradictions_are_surfaced() {
        let (_tmp, cfg) = temp_cfg();
        put_raw_consensus_record(&cfg, base_value(0.9, 1.0, 2), 0.9);
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.contradicted_count, 1);
        assert!(
            review.records[0]
                .notes
                .iter()
                .any(|note| note.contains("contradictions present"))
        );
    }

    #[test]
    fn stale_freshness_is_surfaced() {
        let (_tmp, cfg) = temp_cfg();
        put_raw_consensus_record(&cfg, base_value(0.9, 0.1, 0), 0.9);
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.stale_count, 1);
        assert_eq!(review.records[0].freshness_status, "stale");
    }

    #[test]
    fn empty_state_has_helpful_message() {
        let (_tmp, cfg) = temp_cfg();
        let review = review_state(&cfg, Some("consensus")).expect("review");
        let rendered = render_state_review(&review);

        assert_eq!(review.review_status, StateReviewStatus::Empty);
        assert!(rendered.contains("records: 0"));
        assert!(rendered.contains("quant-m consensus --dry-run"));
    }

    #[test]
    fn review_does_not_mutate_records() {
        let (_tmp, cfg) = temp_cfg();
        crate::consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let before =
            shared_state::snapshot_state(&cfg, Some(&DomainId::new(DEFAULT_REVIEW_DOMAIN)))
                .expect("before");
        review_state(&cfg, Some("consensus")).expect("review");
        let after = shared_state::snapshot_state(&cfg, Some(&DomainId::new(DEFAULT_REVIEW_DOMAIN)))
            .expect("after");

        assert_eq!(before, after);
    }

    #[test]
    fn review_does_not_require_provider_keys_or_network() {
        let (_tmp, mut cfg) = temp_cfg();
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        crate::consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let review = review_state(&cfg, Some("consensus")).expect("review");

        assert_eq!(review.total_count, 1);
    }

    fn put_raw_consensus_record(cfg: &Config, value: Value, confidence: f64) {
        let store = HybridSharedStateStore::from_config(cfg);
        store
            .put(SharedStateRecord {
                key: SharedStateKey::new(format!("shared.consensus.test-{confidence}")),
                value: SharedStateValue::Json(value),
                domain_id: DomainId::new(DEFAULT_REVIEW_DOMAIN),
                source: "test".to_string(),
                confidence,
                updated_at: "2026-06-15T00:00:00Z".to_string(),
                expires_at: None,
                session_id: None,
            })
            .expect("put record");
    }

    fn base_value(confidence: f64, freshness: f64, contradiction_count: usize) -> Value {
        json!({
            "session_id": "session:test",
            "workflow_id": "workflow:consensus-dry-run:test",
            "decision_question": "Should we adopt this API design?",
            "policy_result": "requires_operator_approval",
            "metadata": {
                "confidence": confidence,
                "freshness": freshness,
                "source_count": 1,
                "contradiction_count": contradiction_count,
                "memory_class": "tactical",
                "last_verified_at": "2026-06-15T00:00:00Z",
                "decision_scope": "technical_decision",
                "session_id": "session:test",
                "workflow_id": "workflow:consensus-dry-run:test"
            }
        })
    }
}
