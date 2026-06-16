use crate::cluster_boundary::{ClusterSurfaceKind, classify_worker_input};
use crate::config::Config;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::str::FromStr;

pub type WorkerSurfaceKind = ClusterSurfaceKind;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerProposalKind {
    Evidence,
    Review,
    StateSuggestion,
    CostSuggestion,
    CompletionReport,
    Escalation,
    NextActionSuggestion,
}

impl WorkerProposalKind {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkerProposalKind::Evidence => "evidence",
            WorkerProposalKind::Review => "review",
            WorkerProposalKind::StateSuggestion => "state_suggestion",
            WorkerProposalKind::CostSuggestion => "cost_suggestion",
            WorkerProposalKind::CompletionReport => "completion_report",
            WorkerProposalKind::Escalation => "escalation",
            WorkerProposalKind::NextActionSuggestion => "next_action_suggestion",
        }
    }
}

impl fmt::Display for WorkerProposalKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkerProposalKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "evidence" => Ok(Self::Evidence),
            "review" => Ok(Self::Review),
            "state_suggestion" | "state-suggestion" | "state" => Ok(Self::StateSuggestion),
            "cost_suggestion" | "cost-suggestion" | "cost" => Ok(Self::CostSuggestion),
            "completion_report" | "completion-report" | "completion" => Ok(Self::CompletionReport),
            "escalation" | "escalate" => Ok(Self::Escalation),
            "next_action_suggestion" | "next-action-suggestion" | "next_action" => {
                Ok(Self::NextActionSuggestion)
            }
            other => Err(anyhow!(
                "unsupported worker proposal kind '{other}'; expected evidence, review, state_suggestion, cost_suggestion, completion_report, escalation, or next_action_suggestion"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerProposalStatus {
    PendingReview,
    Reviewed,
    Rejected,
    AcceptedAsEvidence,
}

impl WorkerProposalStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            WorkerProposalStatus::PendingReview => "pending_review",
            WorkerProposalStatus::Reviewed => "reviewed",
            WorkerProposalStatus::Rejected => "rejected",
            WorkerProposalStatus::AcceptedAsEvidence => "accepted_as_evidence",
        }
    }
}

impl fmt::Display for WorkerProposalStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for WorkerProposalStatus {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "pending_review" | "pending-review" | "pending" => Ok(Self::PendingReview),
            "reviewed" => Ok(Self::Reviewed),
            "rejected" => Ok(Self::Rejected),
            "accepted_as_evidence" | "accepted-as-evidence" => Ok(Self::AcceptedAsEvidence),
            other => Err(anyhow!(
                "unsupported worker proposal status '{other}'; expected pending_review, reviewed, rejected, or accepted_as_evidence"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerProposalRiskLevel {
    Low,
    Medium,
    High,
}

impl fmt::Display for WorkerProposalRiskLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let label = match self {
            WorkerProposalRiskLevel::Low => "low",
            WorkerProposalRiskLevel::Medium => "medium",
            WorkerProposalRiskLevel::High => "high",
        };
        f.write_str(label)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerProposalReviewState {
    Unreviewed,
    OperatorReviewed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalEvidence {
    pub evidence_id: String,
    pub summary: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalReviewFinding {
    pub finding_id: String,
    pub summary: String,
    pub severity: WorkerProposalRiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalStateSuggestion {
    pub summary: String,
    pub canonical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalCostSuggestion {
    pub summary: String,
    pub accepted_ledger_truth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalCompletionReport {
    pub summary: String,
    pub replay_validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalRecord {
    pub proposal_id: String,
    pub source_surface: WorkerSurfaceKind,
    pub source_worker_id: String,
    pub proposal_kind: WorkerProposalKind,
    pub status: WorkerProposalStatus,
    pub review_state: WorkerProposalReviewState,
    pub created_at: String,
    pub session_id: Option<String>,
    pub workflow_id: Option<String>,
    pub decision_scope: Option<String>,
    pub summary: String,
    pub evidence_items: Vec<WorkerProposalEvidence>,
    pub review_findings: Vec<WorkerProposalReviewFinding>,
    pub suggested_state_metadata: Option<WorkerProposalStateSuggestion>,
    pub suggested_cost_metadata: Option<WorkerProposalCostSuggestion>,
    pub completion_report: Option<WorkerProposalCompletionReport>,
    pub risk_level: WorkerProposalRiskLevel,
    pub recommended_next_action: Option<String>,
    pub non_authoritative: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubmitWorkerProposalInput {
    pub source_surface: WorkerSurfaceKind,
    pub source_worker_id: String,
    pub proposal_kind: WorkerProposalKind,
    pub summary: String,
    pub session_id: Option<String>,
    pub workflow_id: Option<String>,
    pub decision_scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalSubmitSummary {
    pub proposal_id: String,
    pub source_surface: WorkerSurfaceKind,
    pub proposal_kind: WorkerProposalKind,
    pub status: WorkerProposalStatus,
    pub risk_level: WorkerProposalRiskLevel,
    pub non_authoritative: bool,
    pub artifact_path: PathBuf,
    pub next_recommended_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposalListSummary {
    pub proposals: Vec<WorkerProposalRecord>,
    pub total_count: usize,
    pub pending_review_count: usize,
    pub reviewed_count: usize,
    pub rejected_count: usize,
    pub accepted_as_evidence_count: usize,
    pub next_recommended_command: String,
}

pub fn proposal_dir(cfg: &Config) -> PathBuf {
    cfg.workspace_dir.join("state/worker-proposals")
}

pub fn proposal_index_path(cfg: &Config) -> PathBuf {
    proposal_dir(cfg).join("index.jsonl")
}

pub fn submit_worker_proposal(
    cfg: &Config,
    input: SubmitWorkerProposalInput,
) -> Result<(WorkerProposalRecord, WorkerProposalSubmitSummary)> {
    let summary = input.summary.trim();
    if summary.is_empty() {
        return Err(anyhow!("worker proposal summary is empty"));
    }
    if input.source_worker_id.trim().is_empty() {
        return Err(anyhow!("worker proposal source_worker_id is empty"));
    }

    let created_at = Utc::now().to_rfc3339();
    let proposal_id = next_proposal_id(cfg, input.source_surface, input.proposal_kind)?;
    let record = build_record(input, proposal_id.clone(), created_at);
    let artifact_path = proposal_dir(cfg).join(format!("{proposal_id}.json"));

    fs::create_dir_all(proposal_dir(cfg))
        .with_context(|| format!("failed to create {}", proposal_dir(cfg).display()))?;
    if artifact_path.exists() {
        return Err(anyhow!(
            "worker proposal artifact already exists: {}",
            artifact_path.display()
        ));
    }
    let encoded =
        serde_json::to_string_pretty(&record).context("failed to encode worker proposal")?;
    fs::write(&artifact_path, encoded)
        .with_context(|| format!("failed to write {}", artifact_path.display()))?;
    append_index_line(cfg, &record)?;

    let submit = WorkerProposalSubmitSummary {
        proposal_id,
        source_surface: record.source_surface,
        proposal_kind: record.proposal_kind,
        status: record.status,
        risk_level: record.risk_level,
        non_authoritative: record.non_authoritative,
        artifact_path,
        next_recommended_command: "quant-m worker proposal list --status pending_review"
            .to_string(),
    };
    Ok((record, submit))
}

pub fn list_worker_proposals(
    cfg: &Config,
    surface: Option<WorkerSurfaceKind>,
    status: Option<WorkerProposalStatus>,
) -> Result<WorkerProposalListSummary> {
    let mut proposals = read_index(cfg)?;
    proposals.retain(|proposal| surface.is_none_or(|value| proposal.source_surface == value));
    proposals.retain(|proposal| status.is_none_or(|value| proposal.status == value));
    proposals.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let pending_review_count = proposals
        .iter()
        .filter(|proposal| proposal.status == WorkerProposalStatus::PendingReview)
        .count();
    let reviewed_count = proposals
        .iter()
        .filter(|proposal| proposal.status == WorkerProposalStatus::Reviewed)
        .count();
    let rejected_count = proposals
        .iter()
        .filter(|proposal| proposal.status == WorkerProposalStatus::Rejected)
        .count();
    let accepted_as_evidence_count = proposals
        .iter()
        .filter(|proposal| proposal.status == WorkerProposalStatus::AcceptedAsEvidence)
        .count();

    Ok(WorkerProposalListSummary {
        total_count: proposals.len(),
        proposals,
        pending_review_count,
        reviewed_count,
        rejected_count,
        accepted_as_evidence_count,
        next_recommended_command: "quant-m worker proposal list --status pending_review"
            .to_string(),
    })
}

pub fn render_submit_summary(summary: &WorkerProposalSubmitSummary) -> String {
    format!(
        "Worker proposal submitted\nproposal_id: {}\nsource_surface: {}\nproposal_kind: {}\nstatus: {}\nrisk_level: {}\nnon_authoritative: {}\nartifact_path: {}\nnext: {}\n",
        summary.proposal_id,
        summary.source_surface,
        summary.proposal_kind,
        summary.status,
        summary.risk_level,
        summary.non_authoritative,
        summary.artifact_path.display(),
        summary.next_recommended_command
    )
}

pub fn render_list_summary(summary: &WorkerProposalListSummary) -> String {
    if summary.proposals.is_empty() {
        return format!(
            "Worker proposals\nrecords: 0\npending_review: 0\nreviewed: 0\nrejected: 0\naccepted_as_evidence: 0\nnext: {}\n",
            summary.next_recommended_command
        );
    }

    let mut out = format!(
        "Worker proposals\nrecords: {}\npending_review: {}\nreviewed: {}\nrejected: {}\naccepted_as_evidence: {}\n",
        summary.total_count,
        summary.pending_review_count,
        summary.reviewed_count,
        summary.rejected_count,
        summary.accepted_as_evidence_count
    );
    for proposal in &summary.proposals {
        let linked = match (&proposal.session_id, &proposal.workflow_id) {
            (Some(session), Some(workflow)) => {
                format!(" session={} workflow={}", session, workflow)
            }
            (Some(session), None) => format!(" session={}", session),
            (None, Some(workflow)) => format!(" workflow={}", workflow),
            (None, None) => String::new(),
        };
        out.push_str(&format!(
            "- {} surface={} kind={} status={} risk={} created_at={}{} summary={}\n",
            proposal.proposal_id,
            proposal.source_surface,
            proposal.proposal_kind,
            proposal.status,
            proposal.risk_level,
            proposal.created_at,
            linked,
            proposal.summary
        ));
    }
    out.push_str(&format!("next: {}\n", summary.next_recommended_command));
    out
}

fn build_record(
    input: SubmitWorkerProposalInput,
    proposal_id: String,
    created_at: String,
) -> WorkerProposalRecord {
    let boundary = classify_worker_input(input.source_surface, &input.summary);
    let risk_level = risk_level_for(input.proposal_kind, &boundary.intent.kind, &input.summary);
    let evidence_items = evidence_items_for(&proposal_id, &input);
    let review_findings = review_findings_for(&proposal_id, input.proposal_kind, &input.summary);
    let suggested_state_metadata = (input.proposal_kind == WorkerProposalKind::StateSuggestion)
        .then(|| WorkerProposalStateSuggestion {
            summary: input.summary.clone(),
            canonical: false,
        });
    let suggested_cost_metadata =
        (input.proposal_kind == WorkerProposalKind::CostSuggestion).then(|| {
            WorkerProposalCostSuggestion {
                summary: input.summary.clone(),
                accepted_ledger_truth: false,
            }
        });
    let completion_report =
        (input.proposal_kind == WorkerProposalKind::CompletionReport).then(|| {
            WorkerProposalCompletionReport {
                summary: input.summary.clone(),
                replay_validated: false,
            }
        });
    let recommended_next_action = match input.proposal_kind {
        WorkerProposalKind::Escalation => {
            Some("operator should inspect escalation evidence".to_string())
        }
        WorkerProposalKind::NextActionSuggestion => {
            Some("core policy should review before any follow-up execution".to_string())
        }
        WorkerProposalKind::CompletionReport => {
            Some("run replay before treating completion as proof".to_string())
        }
        WorkerProposalKind::StateSuggestion => {
            Some("review state suggestion without mutating canonical shared state".to_string())
        }
        WorkerProposalKind::CostSuggestion => {
            Some("review cost suggestion without appending accepted ledger truth".to_string())
        }
        WorkerProposalKind::Evidence | WorkerProposalKind::Review => None,
    };

    WorkerProposalRecord {
        proposal_id,
        source_surface: input.source_surface,
        source_worker_id: input.source_worker_id,
        proposal_kind: input.proposal_kind,
        status: WorkerProposalStatus::PendingReview,
        review_state: WorkerProposalReviewState::Unreviewed,
        created_at,
        session_id: input.session_id,
        workflow_id: input.workflow_id,
        decision_scope: input.decision_scope,
        summary: input.summary,
        evidence_items,
        review_findings,
        suggested_state_metadata,
        suggested_cost_metadata,
        completion_report,
        risk_level,
        recommended_next_action,
        non_authoritative: true,
    }
}

fn evidence_items_for(
    proposal_id: &str,
    input: &SubmitWorkerProposalInput,
) -> Vec<WorkerProposalEvidence> {
    if matches!(
        input.proposal_kind,
        WorkerProposalKind::Evidence
            | WorkerProposalKind::Review
            | WorkerProposalKind::Escalation
            | WorkerProposalKind::CompletionReport
            | WorkerProposalKind::NextActionSuggestion
    ) {
        vec![WorkerProposalEvidence {
            evidence_id: format!("{proposal_id}:evidence:0"),
            summary: input.summary.clone(),
            source: input.source_surface.to_string(),
        }]
    } else {
        vec![]
    }
}

fn review_findings_for(
    proposal_id: &str,
    kind: WorkerProposalKind,
    summary: &str,
) -> Vec<WorkerProposalReviewFinding> {
    if kind == WorkerProposalKind::Review {
        vec![WorkerProposalReviewFinding {
            finding_id: format!("{proposal_id}:finding:0"),
            summary: summary.to_string(),
            severity: WorkerProposalRiskLevel::Medium,
        }]
    } else {
        vec![]
    }
}

fn risk_level_for(
    kind: WorkerProposalKind,
    boundary_kind: &crate::cluster_boundary::WorkerIntentKind,
    summary: &str,
) -> WorkerProposalRiskLevel {
    let normalized = summary.to_ascii_lowercase();
    if matches!(
        boundary_kind,
        crate::cluster_boundary::WorkerIntentKind::PolicyBlocked
            | crate::cluster_boundary::WorkerIntentKind::CommandRejected
    ) || normalized.contains("trade")
        || normalized.contains("call provider")
        || normalized.contains("provider call")
        || normalized.contains("openrouter")
        || normalized.contains("bypass")
        || normalized.contains("--live")
    {
        return WorkerProposalRiskLevel::High;
    }

    match kind {
        WorkerProposalKind::StateSuggestion
        | WorkerProposalKind::CostSuggestion
        | WorkerProposalKind::CompletionReport
        | WorkerProposalKind::Escalation
        | WorkerProposalKind::NextActionSuggestion => WorkerProposalRiskLevel::Medium,
        WorkerProposalKind::Evidence | WorkerProposalKind::Review => WorkerProposalRiskLevel::Low,
    }
}

fn append_index_line(cfg: &Config, record: &WorkerProposalRecord) -> Result<()> {
    let path = proposal_index_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let line = serde_json::to_string(record).context("failed to encode worker proposal index")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}").with_context(|| format!("failed to append {}", path.display()))
}

fn read_index(cfg: &Config) -> Result<Vec<WorkerProposalRecord>> {
    let path = proposal_index_path(cfg);
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw =
        fs::read_to_string(&path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut proposals = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let proposal = serde_json::from_str::<WorkerProposalRecord>(line).with_context(|| {
            format!(
                "failed to parse worker proposal index line {} in {}",
                index + 1,
                path.display()
            )
        })?;
        proposals.push(proposal);
    }
    Ok(proposals)
}

fn next_proposal_id(
    cfg: &Config,
    surface: WorkerSurfaceKind,
    kind: WorkerProposalKind,
) -> Result<String> {
    let prefix = format!("proposal-{}-{}", surface.as_str(), kind.as_str());
    let nanos = Utc::now()
        .timestamp_nanos_opt()
        .ok_or_else(|| anyhow!("failed to build worker proposal timestamp"))?;
    for offset in 0..1000 {
        let candidate = format!("{prefix}-{nanos}-{offset}");
        if !proposal_dir(cfg).join(format!("{candidate}.json")).exists() {
            return Ok(candidate);
        }
    }
    Err(anyhow!("failed to allocate unique worker proposal id"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use crate::channels::{ChannelEventType, classify_channel_message};
    use crate::cluster_boundary::{WorkerIntentKind, classify_worker_input};
    use crate::consensus;
    use crate::cost_ledger;
    use crate::shared_state::{HybridSharedStateStore, SharedStateStore};
    use std::fs;
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

    fn input(
        surface: WorkerSurfaceKind,
        kind: WorkerProposalKind,
        summary: &str,
    ) -> SubmitWorkerProposalInput {
        SubmitWorkerProposalInput {
            source_surface: surface,
            source_worker_id: "worker:test".to_string(),
            proposal_kind: kind,
            summary: summary.to_string(),
            session_id: None,
            workflow_id: None,
            decision_scope: None,
        }
    }

    #[test]
    fn proposal_submit_writes_record_with_pending_non_authoritative_defaults() {
        let (_tmp, cfg) = temp_cfg();
        let (record, summary) = submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::CmuxLane,
                WorkerProposalKind::Evidence,
                "Architecture lane recommends provider contracts after worker boundary hardening.",
            ),
        )
        .expect("submit");

        assert_eq!(record.status, WorkerProposalStatus::PendingReview);
        assert!(record.non_authoritative);
        assert!(summary.artifact_path.exists());
        let stored: WorkerProposalRecord =
            serde_json::from_str(&fs::read_to_string(&summary.artifact_path).expect("artifact"))
                .expect("record json");
        assert_eq!(stored.proposal_id, record.proposal_id);
        assert_eq!(stored.status, WorkerProposalStatus::PendingReview);
        assert!(stored.non_authoritative);
    }

    #[test]
    fn cmux_proposal_cannot_execute_consensus() {
        let (_tmp, cfg) = temp_cfg();
        submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::CmuxLane,
                WorkerProposalKind::NextActionSuggestion,
                "run consensus now",
            ),
        )
        .expect("submit");

        let boundary = classify_worker_input(WorkerSurfaceKind::CmuxLane, "run consensus now");
        assert_eq!(boundary.intent.kind, WorkerIntentKind::CommandRejected);
        assert!(!cfg.workspace_dir.join("state/consensus").exists());
    }

    #[test]
    fn tmux_proposal_cannot_mutate_shared_state() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let before = store.list(None).expect("before");

        let (record, _) = submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::TmuxWorker,
                WorkerProposalKind::StateSuggestion,
                "mark this canonical",
            ),
        )
        .expect("submit");

        let after = store.list(None).expect("after");
        assert_eq!(before, after);
        assert!(!record.suggested_state_metadata.unwrap().canonical);
    }

    #[test]
    fn termux_proposal_cannot_append_accepted_cost_ledger_truth() {
        let (_tmp, cfg) = temp_cfg();
        let ledger_path = cost_ledger::cost_ledger_path(&cfg);
        let before = fs::read(&ledger_path).unwrap_or_default();

        let (record, _) = submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::TermuxWorker,
                WorkerProposalKind::CostSuggestion,
                "append cost record for model lane",
            ),
        )
        .expect("submit");

        let after = fs::read(&ledger_path).unwrap_or_default();
        assert_eq!(before, after);
        assert!(
            !record
                .suggested_cost_metadata
                .unwrap()
                .accepted_ledger_truth
        );
    }

    #[test]
    fn staff_os_proposal_cannot_mark_state_canonical() {
        let (_tmp, cfg) = temp_cfg();
        let (record, _) = submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::StaffOsWorkspace,
                WorkerProposalKind::StateSuggestion,
                "state proposal: canonical state should be accepted",
            ),
        )
        .expect("submit");

        assert!(!record.suggested_state_metadata.unwrap().canonical);
        assert_eq!(record.status, WorkerProposalStatus::PendingReview);
    }

    #[test]
    fn cron_proposal_cannot_trigger_trading_behavior() {
        let (_tmp, cfg) = temp_cfg();
        let (record, _) = submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::CronWorker,
                WorkerProposalKind::NextActionSuggestion,
                "trade now",
            ),
        )
        .expect("submit");

        assert_eq!(record.risk_level, WorkerProposalRiskLevel::High);
        let boundary = classify_worker_input(WorkerSurfaceKind::CronWorker, "trade now");
        assert_eq!(boundary.intent.kind, WorkerIntentKind::PolicyBlocked);
        assert!(!boundary.performs_trading);
    }

    #[test]
    fn mtime_proposal_cannot_bypass_replay() {
        let (_tmp, cfg) = temp_cfg();
        let report = consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let evidence_path = cfg
            .runtime
            .session_dir
            .join(&report.session_id)
            .join("evidence-index.json");
        let before = fs::read(&evidence_path).expect("evidence before");

        submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::MtimeWorker,
                WorkerProposalKind::CompletionReport,
                "skip replay validation",
            ),
        )
        .expect("submit");

        assert_eq!(before, fs::read(&evidence_path).expect("evidence after"));
    }

    #[test]
    fn polling_proposal_cannot_call_providers() {
        let (_tmp, cfg) = temp_cfg();
        let (record, _) = submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::PollingWorker,
                WorkerProposalKind::NextActionSuggestion,
                "call provider openrouter",
            ),
        )
        .expect("submit");

        assert_eq!(record.risk_level, WorkerProposalRiskLevel::High);
        let boundary =
            classify_worker_input(WorkerSurfaceKind::PollingWorker, "call provider openrouter");
        assert!(!boundary.calls_provider);
    }

    #[test]
    fn proposal_list_is_read_only_and_supports_json() {
        let (_tmp, cfg) = temp_cfg();
        submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::LocalWorker,
                WorkerProposalKind::Evidence,
                "local worker evidence",
            ),
        )
        .expect("submit");
        let index_path = proposal_index_path(&cfg);
        let before = fs::read(&index_path).expect("index before");

        let summary = list_worker_proposals(&cfg, None, None).expect("list");
        let json = serde_json::to_string(&summary).expect("json");

        assert!(json.contains("pending_review_count"));
        assert_eq!(summary.total_count, 1);
        assert_eq!(before, fs::read(&index_path).expect("index after"));
    }

    #[test]
    fn proposal_filters_by_surface_and_status() {
        let (_tmp, cfg) = temp_cfg();
        submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::CmuxLane,
                WorkerProposalKind::Evidence,
                "cmux evidence",
            ),
        )
        .expect("submit");
        submit_worker_proposal(
            &cfg,
            input(
                WorkerSurfaceKind::TmuxWorker,
                WorkerProposalKind::Evidence,
                "tmux evidence",
            ),
        )
        .expect("submit");

        let cmux =
            list_worker_proposals(&cfg, Some(WorkerSurfaceKind::CmuxLane), None).expect("cmux");
        let pending = list_worker_proposals(&cfg, None, Some(WorkerProposalStatus::PendingReview))
            .expect("pending");

        assert_eq!(cmux.total_count, 1);
        assert_eq!(
            cmux.proposals[0].source_surface,
            WorkerSurfaceKind::CmuxLane
        );
        assert_eq!(pending.total_count, 2);
    }

    #[test]
    fn malformed_kind_and_surface_fail_safely() {
        let kind_err = "not_a_kind"
            .parse::<WorkerProposalKind>()
            .expect_err("kind fails");
        let surface_err = "not_a_surface"
            .parse::<WorkerSurfaceKind>()
            .expect_err("surface fails");

        assert!(
            kind_err
                .to_string()
                .contains("unsupported worker proposal kind")
        );
        assert!(
            surface_err
                .to_string()
                .contains("unsupported worker surface")
        );
    }

    #[test]
    fn existing_fresh_acceptance_and_channel_cluster_paths_still_pass() {
        let (_tmp, cfg) = temp_cfg();
        let report = consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        consensus::replay_consensus_session(
            &cfg,
            &crate::sessions::SessionId::new(report.session_id.clone()),
        )
        .expect("replay");
        let cost_summary = cost_ledger::summarize_costs(&cfg, None, None).expect("cost summary");
        let channel = classify_channel_message(
            crate::config::ExternalChannel::Telegram,
            "quant-m consensus --dry-run now",
        );
        let cluster = classify_worker_input(WorkerSurfaceKind::CmuxLane, "run consensus now");

        assert_eq!(cost_summary.total_actual_cost, 0.0);
        assert_eq!(channel.event_type, ChannelEventType::CommandRejected);
        assert_eq!(cluster.intent.kind, WorkerIntentKind::CommandRejected);
    }
}
