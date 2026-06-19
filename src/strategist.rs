use crate::config::Config;
use crate::cost_ledger::{self, CostLedgerRecord, format_currency_amount};
use crate::sessions::{self, AgentId, DomainId, SessionContext, SessionEvent};
use crate::worker_proposals::{
    SubmitWorkerProposalInput, WorkerProposalKind, WorkerProposalRecord, WorkerSurfaceKind,
    submit_worker_proposal,
};
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const STRATEGIST_DOMAIN: &str = "domain:strategist";
const DEFAULT_STRATEGY_SCOPE: &str =
    "Evaluate whether current mock market context is suitable for further research, not execution.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StrategistPolicyResult {
    ResearchOnlyAllowed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategistLaneFinding {
    pub lane_id: String,
    pub domain: String,
    pub source_surface: WorkerSurfaceKind,
    pub proposal_kind: WorkerProposalKind,
    pub summary: String,
    pub risk_notes: Vec<String>,
    pub recommended_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategistEvidenceItem {
    pub id: String,
    pub lane_id: String,
    pub proposal_id: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategistCostEstimate {
    pub provider: String,
    pub model: String,
    pub estimated_cost: f64,
    pub actual_cost: f64,
    pub currency: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategistMetadata {
    pub confidence: f64,
    pub freshness: f64,
    pub source_count: usize,
    pub contradiction_count: usize,
    pub memory_class: String,
    pub last_verified_at: String,
    pub decision_scope: String,
    pub session_id: String,
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StrategistArtifactPaths {
    pub session_dir: PathBuf,
    pub report_markdown: PathBuf,
    pub report_json: PathBuf,
    pub evidence_index_json: PathBuf,
    pub state_record_json: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategistReport {
    pub status: String,
    pub session_id: String,
    pub workflow_id: String,
    pub strategy_scope: String,
    pub lanes: Vec<StrategistLaneFinding>,
    pub proposal_ids: Vec<String>,
    pub proposal_count: usize,
    pub domain_summaries: Vec<String>,
    pub agreement_summary: String,
    pub contradiction_summary: String,
    pub risk_summary: String,
    pub confidence: f64,
    pub policy_result: StrategistPolicyResult,
    pub cost_estimate: StrategistCostEstimate,
    pub replay_recommendation: String,
    pub state_review_recommendation: String,
    pub worker_proposal_review_recommendation: String,
    pub next_recommended_command: String,
    pub artifact_paths: StrategistArtifactPaths,
    pub metadata: StrategistMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategistStateRecord {
    pub workflow_id: String,
    pub session_id: String,
    pub strategy_scope: String,
    pub proposal_ids: Vec<String>,
    pub agreement_summary: String,
    pub contradiction_summary: String,
    pub risk_summary: String,
    pub confidence: f64,
    pub policy_result: StrategistPolicyResult,
    pub metadata: StrategistMetadata,
    pub artifact_paths: StrategistArtifactPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategistEvidenceIndex {
    pub session_id: String,
    pub workflow_id: String,
    pub evidence_items: Vec<StrategistEvidenceItem>,
    pub proposal_ids: Vec<String>,
    pub contradictions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StrategistJsonOutput {
    pub session_id: String,
    pub workflow_id: String,
    pub lane_count: usize,
    pub proposal_ids: Vec<String>,
    pub proposal_count: usize,
    pub agreement_summary: String,
    pub contradiction_summary: String,
    pub confidence: f64,
    pub policy_result: StrategistPolicyResult,
    pub estimated_cost: f64,
    pub actual_cost: f64,
    pub dry_run: bool,
    pub artifact_paths: StrategistArtifactPaths,
    pub next_recommended_command: String,
}

impl StrategistReport {
    pub fn json_output(&self) -> StrategistJsonOutput {
        StrategistJsonOutput {
            session_id: self.session_id.clone(),
            workflow_id: self.workflow_id.clone(),
            lane_count: self.lanes.len(),
            proposal_ids: self.proposal_ids.clone(),
            proposal_count: self.proposal_count,
            agreement_summary: self.agreement_summary.clone(),
            contradiction_summary: self.contradiction_summary.clone(),
            confidence: self.confidence,
            policy_result: self.policy_result.clone(),
            estimated_cost: self.cost_estimate.estimated_cost,
            actual_cost: self.cost_estimate.actual_cost,
            dry_run: self.cost_estimate.dry_run,
            artifact_paths: self.artifact_paths.clone(),
            next_recommended_command: self.next_recommended_command.clone(),
        }
    }
}

impl StrategistStateRecord {
    fn from_report(report: &StrategistReport) -> Self {
        Self {
            workflow_id: report.workflow_id.clone(),
            session_id: report.session_id.clone(),
            strategy_scope: report.strategy_scope.clone(),
            proposal_ids: report.proposal_ids.clone(),
            agreement_summary: report.agreement_summary.clone(),
            contradiction_summary: report.contradiction_summary.clone(),
            risk_summary: report.risk_summary.clone(),
            confidence: report.confidence,
            policy_result: report.policy_result.clone(),
            metadata: report.metadata.clone(),
            artifact_paths: report.artifact_paths.clone(),
        }
    }
}

pub fn run_strategist_dry_run(cfg: &Config) -> Result<StrategistReport> {
    let strategy_scope = DEFAULT_STRATEGY_SCOPE.to_string();
    let workflow_id = format!(
        "workflow:strategist-dry-run:{}",
        stable_hex(&strategy_scope)
    );
    let context = SessionContext::new(
        AgentId::new(format!("agent:{}:strategist", cfg.node_id)),
        DomainId::new(STRATEGIST_DOMAIN),
    );
    let session_id = context.session_id.to_string();

    sessions::append_event(
        cfg,
        &context,
        SessionEvent::Observation {
            message: "strategist_dry_run_started".to_string(),
            job_id: None,
            detail: Some(strategy_scope.clone()),
        },
    )?;

    let lanes = mock_lanes();
    let mut proposals = Vec::new();
    for lane in &lanes {
        sessions::append_event(
            cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: format!("strategist::{}", lane.lane_id),
                input_preview: strategy_scope.clone(),
                command_preview: None,
                status: "mock_ok".to_string(),
            },
        )?;
        let (proposal, _summary) = submit_worker_proposal(
            cfg,
            SubmitWorkerProposalInput {
                source_surface: lane.source_surface,
                source_worker_id: format!("worker:strategist:{}", lane.lane_id),
                proposal_kind: lane.proposal_kind,
                summary: lane.summary.clone(),
                session_id: Some(session_id.clone()),
                workflow_id: Some(workflow_id.clone()),
                decision_scope: Some("multi_domain_research".to_string()),
            },
        )?;
        proposals.push(proposal);
    }

    let proposal_ids = proposals
        .iter()
        .map(|proposal| proposal.proposal_id.clone())
        .collect::<Vec<_>>();
    let confidence = 0.74;
    let agreement_summary = "All mock lanes agree this packet supports further research only; no lane recommends execution or live market action.".to_string();
    let contradiction_summary =
        "No hard cross-domain contradiction in the mock packet; risk lanes require more evidence before any execution discussion."
            .to_string();
    let risk_summary = "Macro uncertainty, peg/liquidity checks, options event risk, and event-window timing remain research blockers."
        .to_string();
    let now = Utc::now().to_rfc3339();
    let artifact_paths = artifact_paths(cfg, &session_id, &workflow_id);
    let cost_estimate = StrategistCostEstimate {
        provider: "mock".to_string(),
        model: "deterministic-strategist-lanes".to_string(),
        estimated_cost: 0.0,
        actual_cost: 0.0,
        currency: "USD".to_string(),
        dry_run: true,
    };
    let metadata = StrategistMetadata {
        confidence,
        freshness: 1.0,
        source_count: lanes.len(),
        contradiction_count: 0,
        memory_class: "tactical".to_string(),
        last_verified_at: now,
        decision_scope: "multi_domain_research".to_string(),
        session_id: session_id.clone(),
        workflow_id: workflow_id.clone(),
    };

    let report = StrategistReport {
        status: "ok".to_string(),
        session_id: session_id.clone(),
        workflow_id: workflow_id.clone(),
        strategy_scope,
        domain_summaries: domain_summaries(&lanes),
        lanes,
        proposal_ids,
        proposal_count: proposals.len(),
        agreement_summary,
        contradiction_summary,
        risk_summary,
        confidence,
        policy_result: StrategistPolicyResult::ResearchOnlyAllowed,
        cost_estimate,
        replay_recommendation: "STRATEGIST_REPLAY_01 should validate this packet before any future resume execution."
            .to_string(),
        state_review_recommendation:
            "Inspect strategist artifacts directly until strategist replay/state-review support lands."
                .to_string(),
        worker_proposal_review_recommendation:
            "quant-m worker proposal list --status pending_review".to_string(),
        next_recommended_command: "quant-m worker proposal list --status pending_review"
            .to_string(),
        artifact_paths,
        metadata,
    };

    write_artifacts(&report, &proposals)?;
    write_cost_record(cfg, &report)?;

    sessions::append_event(
        cfg,
        &context,
        SessionEvent::PolicyDecision {
            policy: "strategist_research_only_not_execution".to_string(),
            allowed: false,
            reason: "strategist dry-run is evidence only and cannot recommend trades or execution"
                .to_string(),
        },
    )?;
    sessions::append_event(
        cfg,
        &context,
        SessionEvent::Output {
            channel: "terminal".to_string(),
            summary: format!(
                "strategist_dry_run_complete lanes={} proposals={} confidence={:.2}",
                report.lanes.len(),
                report.proposal_count,
                report.confidence
            ),
            job_id: None,
        },
    )?;

    Ok(report)
}

pub fn render_terminal_summary(report: &StrategistReport) -> String {
    format!(
        "Strategist dry-run status: {}\n\
         session_id: {}\n\
         workflow_id: {}\n\
         lanes_created: {}\n\
         proposal_count: {}\n\
         cross_domain_agreement: {}\n\
         contradictions: {}\n\
         confidence: {:.2}\n\
         policy_result: {:?}\n\
         actual_cost: {}\n\
         report: {}\n\
         report_json: {}\n\
         evidence_index: {}\n\
         state_record: {}\n\
         next: {}\n",
        report.status,
        report.session_id,
        report.workflow_id,
        report.lanes.len(),
        report.proposal_count,
        report.agreement_summary,
        report.contradiction_summary,
        report.confidence,
        report.policy_result,
        format_currency_amount(
            report.cost_estimate.actual_cost,
            &report.cost_estimate.currency
        ),
        report.artifact_paths.report_markdown.display(),
        report.artifact_paths.report_json.display(),
        report.artifact_paths.evidence_index_json.display(),
        report.artifact_paths.state_record_json.display(),
        report.next_recommended_command
    )
}

fn mock_lanes() -> Vec<StrategistLaneFinding> {
    vec![
        StrategistLaneFinding {
            lane_id: "macro_lane".to_string(),
            domain: "macro".to_string(),
            source_surface: WorkerSurfaceKind::CmuxLane,
            proposal_kind: WorkerProposalKind::Review,
            summary:
                "Macro lane reviews a mock macro regime and suggests caution because context is intentionally incomplete."
                    .to_string(),
            risk_notes: vec!["macro context is unclear".to_string()],
            recommended_next_action: "continue research only".to_string(),
        },
        StrategistLaneFinding {
            lane_id: "forex_carry_lane".to_string(),
            domain: "forex_carry".to_string(),
            source_surface: WorkerSurfaceKind::TmuxWorker,
            proposal_kind: WorkerProposalKind::Review,
            summary:
                "Forex carry lane reviews mock carry trend conditions and suggests research continuation only."
                    .to_string(),
            risk_notes: vec!["carry conditions are mock-only".to_string()],
            recommended_next_action: "continue research only".to_string(),
        },
        StrategistLaneFinding {
            lane_id: "crypto_peg_risk_lane".to_string(),
            domain: "crypto_peg_risk".to_string(),
            source_surface: WorkerSurfaceKind::TermuxWorker,
            proposal_kind: WorkerProposalKind::Review,
            summary:
                "Crypto peg risk lane reviews mock peg and liquidity risk and requires peg checks before any future execution discussion."
                    .to_string(),
            risk_notes: vec!["peg and liquidity checks required".to_string()],
            recommended_next_action: "collect more evidence".to_string(),
        },
        StrategistLaneFinding {
            lane_id: "equity_options_risk_lane".to_string(),
            domain: "equity_options_risk".to_string(),
            source_surface: WorkerSurfaceKind::PollingWorker,
            proposal_kind: WorkerProposalKind::Review,
            summary:
                "Equity options risk lane reviews mock volatility and event risk and requires strict risk checks."
                    .to_string(),
            risk_notes: vec!["options exposure requires strict risk checks".to_string()],
            recommended_next_action: "collect more evidence".to_string(),
        },
        StrategistLaneFinding {
            lane_id: "sports_event_timing_lane".to_string(),
            domain: "sports_event_timing".to_string(),
            source_surface: WorkerSurfaceKind::CronWorker,
            proposal_kind: WorkerProposalKind::Evidence,
            summary:
                "Sports event timing lane reviews mock event-window timing and notes cron-heavy event-window specificity."
                    .to_string(),
            risk_notes: vec!["event windows are timing-sensitive".to_string()],
            recommended_next_action: "keep timing research bounded".to_string(),
        },
        StrategistLaneFinding {
            lane_id: "operator_audit_lane".to_string(),
            domain: "operator_audit".to_string(),
            source_surface: WorkerSurfaceKind::LocalWorker,
            proposal_kind: WorkerProposalKind::Evidence,
            summary:
                "Operator audit lane confirms the dry run stayed local with no live adapters, provider access, or order behavior."
                    .to_string(),
            risk_notes: vec!["operator approval remains required".to_string()],
            recommended_next_action: "review proposals before any follow-up".to_string(),
        },
    ]
}

fn domain_summaries(lanes: &[StrategistLaneFinding]) -> Vec<String> {
    lanes
        .iter()
        .map(|lane| format!("{}: {}", lane.domain, lane.summary))
        .collect()
}

fn write_artifacts(report: &StrategistReport, proposals: &[WorkerProposalRecord]) -> Result<()> {
    fs::create_dir_all(&report.artifact_paths.session_dir).with_context(|| {
        format!(
            "failed to create {}",
            report.artifact_paths.session_dir.display()
        )
    })?;
    if let Some(parent) = report.artifact_paths.state_record_json.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    fs::write(
        &report.artifact_paths.report_markdown,
        render_markdown_report(report),
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.report_markdown.display()
        )
    })?;
    fs::write(
        &report.artifact_paths.report_json,
        serde_json::to_string_pretty(report).context("failed to encode strategist report")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.report_json.display()
        )
    })?;
    let evidence_index = StrategistEvidenceIndex {
        session_id: report.session_id.clone(),
        workflow_id: report.workflow_id.clone(),
        proposal_ids: report.proposal_ids.clone(),
        evidence_items: proposals
            .iter()
            .enumerate()
            .map(|(index, proposal)| StrategistEvidenceItem {
                id: format!("strategist-evidence-{index}"),
                lane_id: report.lanes[index].lane_id.clone(),
                proposal_id: proposal.proposal_id.clone(),
                summary: proposal.summary.clone(),
            })
            .collect(),
        contradictions: vec![],
    };
    fs::write(
        &report.artifact_paths.evidence_index_json,
        serde_json::to_string_pretty(&evidence_index)
            .context("failed to encode strategist evidence index")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.evidence_index_json.display()
        )
    })?;
    fs::write(
        &report.artifact_paths.state_record_json,
        serde_json::to_string_pretty(&StrategistStateRecord::from_report(report))
            .context("failed to encode strategist state record")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.state_record_json.display()
        )
    })?;
    Ok(())
}

fn write_cost_record(cfg: &Config, report: &StrategistReport) -> Result<()> {
    let record = CostLedgerRecord {
        cost_record_id: format!(
            "cost:{}:{}",
            stable_hex(&report.session_id),
            stable_hex(&report.workflow_id)
        ),
        session_id: report.session_id.clone(),
        workflow_id: report.workflow_id.clone(),
        workflow_kind: "strategist_dry_run".to_string(),
        command: "quant-m strategist --dry-run".to_string(),
        provider: report.cost_estimate.provider.clone(),
        model: report.cost_estimate.model.clone(),
        estimated_cost: report.cost_estimate.estimated_cost,
        actual_cost: report.cost_estimate.actual_cost,
        currency: report.cost_estimate.currency.clone(),
        dry_run: report.cost_estimate.dry_run,
        created_at: Utc::now().to_rfc3339(),
        input_units: Some(report.strategy_scope.split_whitespace().count() as u64),
        output_units: Some(
            report
                .lanes
                .iter()
                .map(|lane| lane.summary.split_whitespace().count())
                .sum::<usize>() as u64,
        ),
        notes: "$0.00 actual, mock-only strategist lanes".to_string(),
    };
    cost_ledger::append_cost_record(cfg, &record)
}

fn render_markdown_report(report: &StrategistReport) -> String {
    let lane_lines = report
        .lanes
        .iter()
        .zip(report.proposal_ids.iter())
        .map(|(lane, proposal_id)| {
            format!(
                "- {} ({}) proposal={} summary={}",
                lane.lane_id, lane.domain, proposal_id, lane.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# Strategist Dry Run\n\n\
         - session_id: {}\n\
         - workflow_id: {}\n\
         - strategy_scope: {}\n\
         - policy_result: {:?}\n\
         - confidence: {:.2}\n\
         - actual_cost: {}\n\n\
         ## Lanes\n\n{}\n\n\
         ## Agreement\n\n{}\n\n\
         ## Contradictions\n\n{}\n\n\
         ## Risk Summary\n\n{}\n\n\
         ## Next\n\n{}\n",
        report.session_id,
        report.workflow_id,
        report.strategy_scope,
        report.policy_result,
        report.confidence,
        format_currency_amount(
            report.cost_estimate.actual_cost,
            &report.cost_estimate.currency
        ),
        lane_lines,
        report.agreement_summary,
        report.contradiction_summary,
        report.risk_summary,
        report.next_recommended_command
    )
}

fn artifact_paths(cfg: &Config, session_id: &str, workflow_id: &str) -> StrategistArtifactPaths {
    let session_dir = cfg.runtime.session_dir.join(session_id);
    let state_dir = cfg.workspace_dir.join("state/strategist");
    let state_filename = format!("{}.json", filename_safe(workflow_id));
    StrategistArtifactPaths {
        report_markdown: session_dir.join("strategist-report.md"),
        report_json: session_dir.join("strategist-report.json"),
        evidence_index_json: session_dir.join("strategist-evidence-index.json"),
        state_record_json: state_dir.join(state_filename),
        session_dir,
    }
}

fn filename_safe(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn stable_hex(value: &str) -> String {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("{hash:016x}")
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
    use crate::worker_proposals::{WorkerProposalStatus, list_worker_proposals};
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
    fn dry_run_creates_lanes_proposals_artifacts_and_zero_cost_record() {
        let (_tmp, cfg) = temp_cfg();

        let report = run_strategist_dry_run(&cfg).expect("strategist");
        let proposals = list_worker_proposals(&cfg, None, None).expect("proposals");
        let costs =
            cost_ledger::summarize_costs(&cfg, Some(&report.workflow_id), Some(&report.session_id))
                .expect("costs");

        assert_eq!(report.lanes.len(), 6);
        assert_eq!(report.proposal_count, 6);
        assert_eq!(proposals.total_count, 6);
        assert!(report.artifact_paths.report_markdown.exists());
        assert!(report.artifact_paths.report_json.exists());
        assert!(report.artifact_paths.evidence_index_json.exists());
        assert!(report.artifact_paths.state_record_json.exists());
        assert_eq!(costs.record_count, 1);
        assert_eq!(costs.total_actual_cost, 0.0);
        assert_eq!(costs.latest_records[0].provider, "mock");
        assert_eq!(
            costs.latest_records[0].model,
            "deterministic-strategist-lanes"
        );
    }

    #[test]
    fn every_lane_proposal_is_pending_review_and_non_authoritative() {
        let (_tmp, cfg) = temp_cfg();
        let report = run_strategist_dry_run(&cfg).expect("strategist");
        let proposals = list_worker_proposals(&cfg, None, None).expect("proposals");

        assert_eq!(proposals.total_count, report.lanes.len());
        for proposal in proposals.proposals {
            assert_eq!(proposal.status, WorkerProposalStatus::PendingReview);
            assert!(proposal.non_authoritative);
            assert_eq!(
                proposal.session_id.as_deref(),
                Some(report.session_id.as_str())
            );
            assert_eq!(
                proposal.workflow_id.as_deref(),
                Some(report.workflow_id.as_str())
            );
        }
    }

    #[test]
    fn policy_result_blocks_execution_and_json_output_is_complete() {
        let (_tmp, cfg) = temp_cfg();
        let report = run_strategist_dry_run(&cfg).expect("strategist");
        let json = report.json_output();

        assert_eq!(
            report.policy_result,
            StrategistPolicyResult::ResearchOnlyAllowed
        );
        assert_eq!(json.lane_count, 6);
        assert_eq!(json.proposal_count, 6);
        assert_eq!(json.actual_cost, 0.0);
        assert!(json.dry_run);
        assert!(
            json.next_recommended_command
                .contains("worker proposal list")
        );
    }

    #[test]
    fn dry_run_requires_no_provider_keys_network_or_trading_behavior() {
        let (_tmp, mut cfg) = temp_cfg();
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        let store = HybridSharedStateStore::from_config(&cfg);
        let before = store.list(None).expect("state before");

        let report = run_strategist_dry_run(&cfg).expect("strategist");

        let after = store.list(None).expect("state after");
        assert_eq!(before, after);
        assert_eq!(report.cost_estimate.provider, "mock");
        assert_eq!(report.cost_estimate.model, "deterministic-strategist-lanes");
        assert_eq!(report.cost_estimate.actual_cost, 0.0);
        assert!(
            !report
                .risk_summary
                .to_ascii_lowercase()
                .contains("trade now")
        );
    }

    #[test]
    fn existing_consensus_replay_cost_channel_cluster_and_worker_proposal_paths_still_pass() {
        let (_tmp, cfg) = temp_cfg();

        let consensus_report =
            consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
                .expect("consensus");
        consensus::replay_consensus_session(
            &cfg,
            &sessions::SessionId::new(consensus_report.session_id.clone()),
        )
        .expect("replay");
        let costs = cost_ledger::summarize_costs(&cfg, None, None).expect("cost summary");
        let channel = classify_channel_message(
            crate::config::ExternalChannel::Telegram,
            "quant-m consensus --dry-run now",
        );
        let cluster = classify_worker_input(WorkerSurfaceKind::CmuxLane, "run consensus now");
        run_strategist_dry_run(&cfg).expect("strategist");
        let proposals = list_worker_proposals(&cfg, None, None).expect("proposals");

        assert!(costs.record_count >= 1);
        assert_eq!(channel.event_type, ChannelEventType::CommandRejected);
        assert_eq!(cluster.intent.kind, WorkerIntentKind::CommandRejected);
        assert!(proposals.total_count >= 6);
    }

    #[test]
    fn strategist_artifacts_are_replay_friendly_json() {
        let (_tmp, cfg) = temp_cfg();
        let report = run_strategist_dry_run(&cfg).expect("strategist");

        let report_json: StrategistReport = serde_json::from_str(
            &fs::read_to_string(&report.artifact_paths.report_json).expect("report json"),
        )
        .expect("decode report");
        let evidence_index: StrategistEvidenceIndex = serde_json::from_str(
            &fs::read_to_string(&report.artifact_paths.evidence_index_json).expect("evidence json"),
        )
        .expect("decode evidence");
        let state_record: StrategistStateRecord = serde_json::from_str(
            &fs::read_to_string(&report.artifact_paths.state_record_json).expect("state json"),
        )
        .expect("decode state");

        assert_eq!(report_json.session_id, report.session_id);
        assert_eq!(evidence_index.proposal_ids.len(), 6);
        assert_eq!(
            state_record.policy_result,
            StrategistPolicyResult::ResearchOnlyAllowed
        );
    }
}
