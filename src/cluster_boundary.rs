use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterSurfaceKind {
    StaffOsWorkspace,
    CmuxLane,
    TmuxWorker,
    TermuxWorker,
    CronWorker,
    MtimeWorker,
    PollingWorker,
    LocalWorker,
}

impl ClusterSurfaceKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ClusterSurfaceKind::StaffOsWorkspace => "staff_os_workspace",
            ClusterSurfaceKind::CmuxLane => "cmux_lane",
            ClusterSurfaceKind::TmuxWorker => "tmux_worker",
            ClusterSurfaceKind::TermuxWorker => "termux_worker",
            ClusterSurfaceKind::CronWorker => "cron_worker",
            ClusterSurfaceKind::MtimeWorker => "mtime_worker",
            ClusterSurfaceKind::PollingWorker => "polling_worker",
            ClusterSurfaceKind::LocalWorker => "local_worker",
        }
    }
}

impl fmt::Display for ClusterSurfaceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for ClusterSurfaceKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "staff_os_workspace" | "staff-os-workspace" | "staffos" | "staff_os" => {
                Ok(Self::StaffOsWorkspace)
            }
            "cmux_lane" | "cmux-lane" | "cmux" => Ok(Self::CmuxLane),
            "tmux_worker" | "tmux-worker" | "tmux" => Ok(Self::TmuxWorker),
            "termux_worker" | "termux-worker" | "termux" => Ok(Self::TermuxWorker),
            "cron_worker" | "cron-worker" | "cron" => Ok(Self::CronWorker),
            "mtime_worker" | "mtime-worker" | "mtime" => Ok(Self::MtimeWorker),
            "polling_worker" | "polling-worker" | "polling" => Ok(Self::PollingWorker),
            "local_worker" | "local-worker" | "local" => Ok(Self::LocalWorker),
            other => Err(anyhow::anyhow!(
                "unsupported worker surface '{other}'; expected staff_os_workspace, cmux_lane, tmux_worker, termux_worker, cron_worker, mtime_worker, polling_worker, or local_worker"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerIntentKind {
    EvidenceSubmitted,
    ReviewSubmitted,
    CompletionReportSubmitted,
    StateProposalSubmitted,
    CostProposalSubmitted,
    ApprovalEvidenceSubmitted,
    DenialEvidenceSubmitted,
    EscalationRequested,
    CommandRejected,
    PolicyBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerEvidence {
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerProposal {
    pub proposal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerStateProposal {
    pub proposed_state: String,
    pub canonical: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCostProposal {
    pub proposed_record: String,
    pub accepted_ledger_truth: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerCompletionReport {
    pub report: String,
    pub replay_validated: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerIntent {
    pub surface: ClusterSurfaceKind,
    pub kind: WorkerIntentKind,
    pub raw_text: String,
    pub reason: String,
    pub evidence: Option<WorkerEvidence>,
    pub proposal: Option<WorkerProposal>,
    pub state_proposal: Option<WorkerStateProposal>,
    pub cost_proposal: Option<WorkerCostProposal>,
    pub completion_report: Option<WorkerCompletionReport>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkerBoundaryDecision {
    pub intent: WorkerIntent,
    pub evidence_only: bool,
    pub executes_runtime: bool,
    pub mutates_shared_state: bool,
    pub mutates_cost_ledger: bool,
    pub calls_provider: bool,
    pub performs_trading: bool,
    pub bypasses_policy: bool,
    pub bypasses_operator_approval: bool,
    pub bypasses_replay_validation: bool,
}

impl WorkerBoundaryDecision {
    #[allow(dead_code)]
    pub fn operator_reply(&self) -> String {
        match self.intent.kind {
            WorkerIntentKind::CommandRejected | WorkerIntentKind::PolicyBlocked => {
                format!("worker intent rejected: {}", self.intent.reason)
            }
            WorkerIntentKind::ApprovalEvidenceSubmitted => {
                "worker approval recorded as evidence only; no action executed".to_string()
            }
            WorkerIntentKind::DenialEvidenceSubmitted => {
                "worker denial recorded as evidence only; no action executed".to_string()
            }
            WorkerIntentKind::EscalationRequested => {
                "worker escalation recorded as evidence only; no action executed".to_string()
            }
            WorkerIntentKind::StateProposalSubmitted => {
                "worker state proposal recorded as non-canonical evidence only".to_string()
            }
            WorkerIntentKind::CostProposalSubmitted => {
                "worker cost proposal recorded as non-ledger evidence only".to_string()
            }
            WorkerIntentKind::CompletionReportSubmitted => {
                "worker completion report recorded as evidence only; replay remains required"
                    .to_string()
            }
            WorkerIntentKind::EvidenceSubmitted | WorkerIntentKind::ReviewSubmitted => {
                "worker message recorded as evidence only; no action executed".to_string()
            }
        }
    }
}

pub fn classify_worker_input(surface: ClusterSurfaceKind, text: &str) -> WorkerBoundaryDecision {
    let raw_text = text.trim().to_string();
    let normalized = raw_text.to_ascii_lowercase();
    let (kind, reason) = classify_kind(&normalized, raw_text.is_empty());

    let evidence = matches!(
        kind,
        WorkerIntentKind::EvidenceSubmitted
            | WorkerIntentKind::ReviewSubmitted
            | WorkerIntentKind::ApprovalEvidenceSubmitted
            | WorkerIntentKind::DenialEvidenceSubmitted
            | WorkerIntentKind::EscalationRequested
    )
    .then(|| WorkerEvidence {
        summary: raw_text.clone(),
    });

    let proposal = matches!(
        kind,
        WorkerIntentKind::StateProposalSubmitted | WorkerIntentKind::CostProposalSubmitted
    )
    .then(|| WorkerProposal {
        proposal: raw_text.clone(),
    });

    let state_proposal =
        (kind == WorkerIntentKind::StateProposalSubmitted).then(|| WorkerStateProposal {
            proposed_state: raw_text.clone(),
            canonical: false,
        });

    let cost_proposal =
        (kind == WorkerIntentKind::CostProposalSubmitted).then(|| WorkerCostProposal {
            proposed_record: raw_text.clone(),
            accepted_ledger_truth: false,
        });

    let completion_report =
        (kind == WorkerIntentKind::CompletionReportSubmitted).then(|| WorkerCompletionReport {
            report: raw_text.clone(),
            replay_validated: false,
        });

    WorkerBoundaryDecision {
        intent: WorkerIntent {
            surface,
            kind,
            raw_text,
            reason,
            evidence,
            proposal,
            state_proposal,
            cost_proposal,
            completion_report,
        },
        evidence_only: true,
        executes_runtime: false,
        mutates_shared_state: false,
        mutates_cost_ledger: false,
        calls_provider: false,
        performs_trading: false,
        bypasses_policy: false,
        bypasses_operator_approval: false,
        bypasses_replay_validation: false,
    }
}

fn classify_kind(normalized: &str, is_empty: bool) -> (WorkerIntentKind, String) {
    if is_empty {
        return (
            WorkerIntentKind::EvidenceSubmitted,
            "empty worker message is non-executing evidence".to_string(),
        );
    }

    if looks_like_trading_or_policy_block(normalized) {
        return (
            WorkerIntentKind::PolicyBlocked,
            "worker surfaces cannot trigger trading, provider calls, replay bypasses, or policy bypasses"
                .to_string(),
        );
    }

    if is_command_like(normalized) {
        return (
            WorkerIntentKind::CommandRejected,
            "workers propose; only the governed Quant-M core may execute runtime commands"
                .to_string(),
        );
    }

    if looks_like_approval(normalized) {
        return (
            WorkerIntentKind::ApprovalEvidenceSubmitted,
            "worker approval language is evidence only and cannot grant authority".to_string(),
        );
    }

    if looks_like_denial(normalized) {
        return (
            WorkerIntentKind::DenialEvidenceSubmitted,
            "worker denial language is evidence only and cannot mutate runtime state".to_string(),
        );
    }

    if looks_like_escalation(normalized) {
        return (
            WorkerIntentKind::EscalationRequested,
            "worker escalation is evidence only and requires operator follow-up".to_string(),
        );
    }

    if looks_like_cost_proposal(normalized) {
        return (
            WorkerIntentKind::CostProposalSubmitted,
            "worker cost input is a proposal only, not accepted ledger truth".to_string(),
        );
    }

    if looks_like_state_proposal(normalized) {
        return (
            WorkerIntentKind::StateProposalSubmitted,
            "worker state input is a proposal only, not canonical shared state".to_string(),
        );
    }

    if looks_like_completion(normalized) {
        return (
            WorkerIntentKind::CompletionReportSubmitted,
            "worker completion reports do not replace replay validation".to_string(),
        );
    }

    if looks_like_review(normalized) {
        return (
            WorkerIntentKind::ReviewSubmitted,
            "worker review is non-executing evidence".to_string(),
        );
    }

    (
        WorkerIntentKind::EvidenceSubmitted,
        "worker message is non-executing evidence".to_string(),
    )
}

fn is_command_like(normalized: &str) -> bool {
    let command_markers = [
        "quant-m ",
        "run consensus",
        "consensus now",
        "consensus --dry-run",
        "run workflow",
        "replay ",
        "state review",
        "cost summary",
        "worker once",
        "worker submit",
        "execute",
        "exec ",
        "shell ",
        "/run",
        "/exec",
        "/consensus",
        "/replay",
        "/state",
        "/cost",
    ];

    command_markers
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn looks_like_trading_or_policy_block(normalized: &str) -> bool {
    let blocked_markers = [
        "trade now",
        "trading",
        "buy ",
        "sell ",
        "provider call",
        "call provider",
        "call openrouter",
        "--live",
        "bypass policy",
        "skip policy",
        "bypass approval",
        "skip approval",
        "bypass replay",
        "skip replay",
        "without replay",
    ];

    blocked_markers
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn looks_like_approval(normalized: &str) -> bool {
    normalized == "approve"
        || normalized == "approved"
        || normalized.starts_with("approve ")
        || normalized.contains(" approved")
}

fn looks_like_denial(normalized: &str) -> bool {
    normalized == "deny"
        || normalized == "denied"
        || normalized.starts_with("deny ")
        || normalized.contains(" denied")
}

fn looks_like_escalation(normalized: &str) -> bool {
    normalized == "escalate"
        || normalized.starts_with("escalate ")
        || normalized.contains("needs escalation")
}

fn looks_like_cost_proposal(normalized: &str) -> bool {
    normalized.contains("cost proposal")
        || normalized.contains("proposed cost")
        || normalized.contains("append cost record")
        || normalized.contains("cost ledger")
}

fn looks_like_state_proposal(normalized: &str) -> bool {
    normalized.contains("state proposal")
        || normalized.contains("proposed state")
        || normalized.contains("mark this canonical")
        || normalized.contains("canonical state")
        || normalized.contains("mutate shared state")
}

fn looks_like_completion(normalized: &str) -> bool {
    normalized.contains("completion report")
        || normalized.contains("completed")
        || normalized.contains("task complete")
}

fn looks_like_review(normalized: &str) -> bool {
    normalized.contains("review")
        || normalized.contains("finding")
        || normalized.contains("findings")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use crate::channels::{ChannelEventType, classify_channel_message};
    use crate::config::{Config, ExternalChannel};
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

    fn assert_no_worker_authority(decision: &WorkerBoundaryDecision) {
        assert!(decision.evidence_only);
        assert!(!decision.executes_runtime);
        assert!(!decision.mutates_shared_state);
        assert!(!decision.mutates_cost_ledger);
        assert!(!decision.calls_provider);
        assert!(!decision.performs_trading);
        assert!(!decision.bypasses_policy);
        assert!(!decision.bypasses_operator_approval);
        assert!(!decision.bypasses_replay_validation);
    }

    #[test]
    fn mock_cmux_worker_cannot_directly_execute_consensus() {
        let (_tmp, cfg) = temp_cfg();
        let decision = classify_worker_input(ClusterSurfaceKind::CmuxLane, "run consensus now");

        assert_eq!(decision.intent.kind, WorkerIntentKind::CommandRejected);
        assert_no_worker_authority(&decision);
        assert!(!cfg.workspace_dir.join("state/consensus").exists());
    }

    #[test]
    fn mock_tmux_worker_cannot_mutate_shared_state_directly() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let before = store.list(None).expect("state before");

        let decision = classify_worker_input(ClusterSurfaceKind::TmuxWorker, "mark this canonical");

        let after = store.list(None).expect("state after");
        assert_eq!(
            decision.intent.kind,
            WorkerIntentKind::StateProposalSubmitted
        );
        assert!(!decision.intent.state_proposal.as_ref().unwrap().canonical);
        assert_eq!(before, after);
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn mock_termux_worker_cannot_append_accepted_cost_ledger_records_directly() {
        let (_tmp, cfg) = temp_cfg();
        let ledger_path = cost_ledger::cost_ledger_path(&cfg);
        let before = fs::read(&ledger_path).unwrap_or_default();

        let decision = classify_worker_input(
            ClusterSurfaceKind::TermuxWorker,
            "append cost record for model lane",
        );

        let after = fs::read(&ledger_path).unwrap_or_default();
        assert_eq!(
            decision.intent.kind,
            WorkerIntentKind::CostProposalSubmitted
        );
        assert!(
            !decision
                .intent
                .cost_proposal
                .as_ref()
                .unwrap()
                .accepted_ledger_truth
        );
        assert_eq!(before, after);
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn mock_staff_os_workspace_output_becomes_evidence_or_proposal_only() {
        let decision = classify_worker_input(
            ClusterSurfaceKind::StaffOsWorkspace,
            "review finding: source evidence disagrees with summary",
        );

        assert_eq!(decision.intent.kind, WorkerIntentKind::ReviewSubmitted);
        assert!(decision.intent.evidence.is_some());
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn cron_worker_cannot_trigger_trading_behavior() {
        let decision = classify_worker_input(ClusterSurfaceKind::CronWorker, "trade now");

        assert_eq!(decision.intent.kind, WorkerIntentKind::PolicyBlocked);
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn mtime_worker_cannot_bypass_replay_validation() {
        let (_tmp, cfg) = temp_cfg();
        let report = consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let session_dir = cfg.runtime.session_dir.join(&report.session_id);
        let evidence_path = session_dir.join("evidence-index.json");
        let before = fs::read(&evidence_path).expect("evidence before");

        let decision =
            classify_worker_input(ClusterSurfaceKind::MtimeWorker, "skip replay validation");

        assert_eq!(decision.intent.kind, WorkerIntentKind::PolicyBlocked);
        assert_eq!(before, fs::read(&evidence_path).expect("evidence after"));
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn polling_worker_cannot_call_providers() {
        let decision = classify_worker_input(
            ClusterSurfaceKind::PollingWorker,
            "call provider openrouter",
        );

        assert_eq!(decision.intent.kind, WorkerIntentKind::PolicyBlocked);
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn worker_approval_message_becomes_approval_evidence_only() {
        let decision = classify_worker_input(ClusterSurfaceKind::LocalWorker, "approved");

        assert_eq!(
            decision.intent.kind,
            WorkerIntentKind::ApprovalEvidenceSubmitted
        );
        assert!(decision.intent.evidence.is_some());
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn worker_denial_message_becomes_denial_evidence_only() {
        let decision = classify_worker_input(ClusterSurfaceKind::LocalWorker, "deny this plan");

        assert_eq!(
            decision.intent.kind,
            WorkerIntentKind::DenialEvidenceSubmitted
        );
        assert!(decision.intent.evidence.is_some());
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn worker_escalation_becomes_escalation_evidence_only() {
        let decision =
            classify_worker_input(ClusterSurfaceKind::LocalWorker, "escalate to operator");

        assert_eq!(decision.intent.kind, WorkerIntentKind::EscalationRequested);
        assert!(decision.intent.evidence.is_some());
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn worker_state_proposal_cannot_become_canonical_without_core_policy() {
        let decision = classify_worker_input(
            ClusterSurfaceKind::StaffOsWorkspace,
            "state proposal: canonical state should be accepted",
        );

        assert_eq!(
            decision.intent.kind,
            WorkerIntentKind::StateProposalSubmitted
        );
        assert!(!decision.intent.state_proposal.as_ref().unwrap().canonical);
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn worker_cost_proposal_cannot_become_accepted_ledger_truth_without_core_handling() {
        let decision = classify_worker_input(
            ClusterSurfaceKind::CmuxLane,
            "cost proposal: append cost record 0.25",
        );

        assert_eq!(
            decision.intent.kind,
            WorkerIntentKind::CostProposalSubmitted
        );
        assert!(
            !decision
                .intent
                .cost_proposal
                .as_ref()
                .unwrap()
                .accepted_ledger_truth
        );
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn worker_command_like_text_is_rejected_or_non_executing_intent() {
        let decision = classify_worker_input(
            ClusterSurfaceKind::LocalWorker,
            "quant-m run workflow workflow:mock-research-brief",
        );

        assert_eq!(decision.intent.kind, WorkerIntentKind::CommandRejected);
        assert_no_worker_authority(&decision);
    }

    #[test]
    fn existing_channel_isolation_classifier_still_rejects_channel_commands() {
        let record = classify_channel_message(
            ExternalChannel::Telegram,
            "quant-m consensus --dry-run \"Should we adopt this API design?\"",
        );

        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert!(record.evidence_only);
        assert!(!record.executes_runtime);
        assert!(!record.mutates_shared_state);
        assert!(!record.mutates_cost_ledger);
        assert!(!record.calls_provider);
        assert!(!record.performs_trading);
        assert!(!record.bypasses_policy);
        assert!(!record.bypasses_operator_approval);
    }

    #[test]
    fn governed_consensus_path_still_writes_only_through_core_runtime() {
        let (_tmp, cfg) = temp_cfg();

        let report = consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let replay = consensus::replay_consensus_session(
            &cfg,
            &crate::sessions::SessionId::new(report.session_id.clone()),
        )
        .expect("replay");
        let store = HybridSharedStateStore::from_config(&cfg);
        let consensus_records = store
            .list(Some(&crate::sessions::DomainId::new("domain:consensus")))
            .expect("shared consensus state");
        let summary = cost_ledger::summarize_costs(&cfg, None, None).expect("cost summary");

        assert_ne!(report.workflow_id.trim(), "");
        assert_eq!(
            replay.replay_status,
            consensus::ConsensusReplayStatus::ValidatedEvidenceOnly
        );
        assert_eq!(consensus_records.len(), 1);
        assert_eq!(summary.total_actual_cost, 0.0);
    }
}
