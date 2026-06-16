use crate::config::Config;
use crate::cost_ledger;
use crate::sessions::{self, AgentId, DomainId, SessionContext, SessionEvent};
use crate::shared_state::{
    HybridSharedStateStore, SharedStateKey, SharedStateRecord, SharedStateStore, SharedStateValue,
};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const CONSENSUS_DOMAIN: &str = "domain:consensus";
const WORKFLOW_PREFIX: &str = "workflow:consensus-dry-run";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerLaneKind {
    Architecture,
    Risk,
    Operator,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ReviewerStance {
    Support,
    Caution,
    Oppose,
    Neutral,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClass {
    Ephemeral,
    Tactical,
    Strategic,
    Canonical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PolicyResult {
    EvidenceOnly,
    RequiresOperatorApproval,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusReviewerFinding {
    pub lane: ReviewerLaneKind,
    pub lane_name: String,
    pub stance: ReviewerStance,
    pub confidence: f64,
    pub key_findings: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub risks: Vec<String>,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsensusEvidenceItem {
    pub id: String,
    pub source: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ConsensusContradiction {
    pub id: String,
    pub summary: String,
    pub severity: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusScore {
    pub agreement_score: f64,
    pub confidence: f64,
    pub reviewer_confidence_average: f64,
    pub evidence_count: usize,
    pub contradiction_count: usize,
    pub freshness: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusCostEstimate {
    pub actual_cost_usd: f64,
    pub estimated_cost_usd: f64,
    pub mode: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusMetadata {
    pub confidence: f64,
    pub freshness: f64,
    pub source_count: usize,
    pub contradiction_count: usize,
    pub memory_class: MemoryClass,
    pub last_verified_at: String,
    pub decision_scope: String,
    pub session_id: String,
    pub workflow_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusArtifactPaths {
    pub session_dir: PathBuf,
    pub report_markdown: PathBuf,
    pub report_json: PathBuf,
    pub evidence_index_json: PathBuf,
    pub state_record_json: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusReport {
    pub decision_question: String,
    pub workflow_id: String,
    pub session_id: String,
    pub reviewer_lanes: Vec<ConsensusReviewerFinding>,
    pub agreement_score: f64,
    pub disagreement_summary: String,
    pub evidence_used: Vec<ConsensusEvidenceItem>,
    pub contradictions: Vec<ConsensusContradiction>,
    pub score: ConsensusScore,
    pub policy_result: PolicyResult,
    pub recommended_next_action: String,
    pub next_recommended_command: String,
    pub cost_estimate: ConsensusCostEstimate,
    pub metadata: ConsensusMetadata,
    pub artifact_paths: ConsensusArtifactPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusStateRecord {
    pub workflow_id: String,
    pub session_id: String,
    pub decision_question: String,
    pub agreement_score: f64,
    pub disagreement_summary: String,
    pub policy_result: PolicyResult,
    pub recommended_next_action: String,
    pub metadata: ConsensusMetadata,
    pub artifact_paths: ConsensusArtifactPaths,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusIntegrityStatus {
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SharedStateMatchStatus {
    Ok,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ConsensusReplayStatus {
    ValidatedEvidenceOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConsensusReplaySummary {
    pub session_id: String,
    pub workflow_id: String,
    pub decision_question: String,
    pub reviewer_lanes: Vec<String>,
    pub agreement_score: f64,
    pub disagreement_summary: String,
    pub evidence_count: usize,
    pub contradiction_count: usize,
    pub confidence: f64,
    pub policy_result: PolicyResult,
    pub cost_estimate: ConsensusCostEstimate,
    pub artifact_status: ConsensusIntegrityStatus,
    pub shared_state_status: SharedStateMatchStatus,
    pub replay_status: ConsensusReplayStatus,
    pub next_recommended_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ConsensusEvidenceIndex {
    session_id: String,
    workflow_id: String,
    evidence_items: Vec<ConsensusEvidenceItem>,
    contradictions: Vec<ConsensusContradiction>,
}

pub fn run_consensus_dry_run(cfg: &Config, question: &str) -> Result<ConsensusReport> {
    let question = normalize_question(question)?;
    let workflow_id = format!("{WORKFLOW_PREFIX}:{}", stable_hex(&question));
    let context = SessionContext::new(
        AgentId::new(format!("agent:{}:consensus", cfg.node_id)),
        DomainId::new(CONSENSUS_DOMAIN),
    );
    let session_id = context.session_id.to_string();

    sessions::append_event(
        cfg,
        &context,
        SessionEvent::Observation {
            message: "consensus_dry_run_started".to_string(),
            job_id: None,
            detail: Some(question.clone()),
        },
    )?;

    let reviewer_lanes = mock_reviewers(&question);
    for lane in &reviewer_lanes {
        sessions::append_event(
            cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: format!("consensus::{:?}", lane.lane).to_ascii_lowercase(),
                input_preview: question.clone(),
                command_preview: None,
                status: "mock_ok".to_string(),
            },
        )?;
    }

    let evidence_used = evidence_items(&question);
    let contradictions = contradictions_for(&question);
    let score = score_consensus(&reviewer_lanes, evidence_used.len(), contradictions.len());
    let policy_result = PolicyResult::RequiresOperatorApproval;
    let disagreement_summary = summarize_disagreement(&reviewer_lanes, contradictions.len());
    let recommended_next_action =
        "Review the consensus packet, inspect the session evidence, then approve or deny any follow-up action explicitly."
            .to_string();
    let next_recommended_command = format!("quant-m session show {session_id}");
    let cost_estimate = ConsensusCostEstimate {
        actual_cost_usd: 0.0,
        estimated_cost_usd: 0.0,
        mode: "dry_run_mock".to_string(),
        note: "$0.00 actual, mock-only".to_string(),
    };
    let now = Utc::now().to_rfc3339();
    let metadata = ConsensusMetadata {
        confidence: score.confidence,
        freshness: score.freshness,
        source_count: evidence_used.len(),
        contradiction_count: contradictions.len(),
        memory_class: MemoryClass::Tactical,
        last_verified_at: now.clone(),
        decision_scope: classify_decision_scope(&question),
        session_id: session_id.clone(),
        workflow_id: workflow_id.clone(),
    };
    let artifact_paths = artifact_paths(cfg, &session_id, &workflow_id);

    let report = ConsensusReport {
        decision_question: question,
        workflow_id: workflow_id.clone(),
        session_id: session_id.clone(),
        reviewer_lanes,
        agreement_score: score.agreement_score,
        disagreement_summary,
        evidence_used,
        contradictions,
        score,
        policy_result: policy_result.clone(),
        recommended_next_action,
        next_recommended_command,
        cost_estimate,
        metadata,
        artifact_paths,
    };

    write_artifacts(&report)?;
    write_state_record(cfg, &report)?;
    write_cost_record(cfg, &report)?;

    sessions::append_event(
        cfg,
        &context,
        SessionEvent::PolicyDecision {
            policy: "consensus_is_evidence_not_authority".to_string(),
            allowed: false,
            reason: "dry-run consensus may recommend a follow-up but cannot execute it".to_string(),
        },
    )?;
    sessions::append_event(
        cfg,
        &context,
        SessionEvent::Output {
            channel: "terminal".to_string(),
            summary: format!(
                "consensus_dry_run_complete agreement={:.2} confidence={:.2}",
                report.agreement_score, report.score.confidence
            ),
            job_id: None,
        },
    )?;

    Ok(report)
}

pub fn replay_consensus_session(
    cfg: &Config,
    session_id: &sessions::SessionId,
) -> Result<ConsensusReplaySummary> {
    let session_dir = cfg.runtime.session_dir.join(session_id.as_str());
    let report_path = session_dir.join("consensus-report.json");
    let evidence_path = session_dir.join("evidence-index.json");

    if !report_path.exists() {
        return Err(anyhow!(
            "missing consensus report: {}",
            report_path.display()
        ));
    }
    if !evidence_path.exists() {
        return Err(anyhow!(
            "missing consensus evidence index: {}",
            evidence_path.display()
        ));
    }

    let report = read_consensus_report(&report_path)?;
    let evidence_index = read_json::<ConsensusEvidenceIndex>(&evidence_path)?;
    validate_report_required_fields(&report)?;

    if report.session_id != session_id.as_str() {
        return Err(anyhow!(
            "consensus report session_id mismatch: expected {}, got {}",
            session_id,
            report.session_id
        ));
    }
    if evidence_index.session_id != report.session_id {
        return Err(anyhow!(
            "evidence index session_id mismatch: expected {}, got {}",
            report.session_id,
            evidence_index.session_id
        ));
    }
    if evidence_index.workflow_id != report.workflow_id {
        return Err(anyhow!(
            "evidence index workflow_id mismatch: expected {}, got {}",
            report.workflow_id,
            evidence_index.workflow_id
        ));
    }
    if evidence_index.evidence_items.len() != report.evidence_used.len()
        || evidence_index.evidence_items.len() != report.metadata.source_count
    {
        return Err(anyhow!(
            "evidence count mismatch: report={}, evidence_index={}, metadata={}",
            report.evidence_used.len(),
            evidence_index.evidence_items.len(),
            report.metadata.source_count
        ));
    }
    if evidence_index.contradictions.len() != report.contradictions.len()
        || evidence_index.contradictions.len() != report.metadata.contradiction_count
        || report.score.contradiction_count != report.metadata.contradiction_count
    {
        return Err(anyhow!(
            "contradiction count mismatch: report={}, evidence_index={}, metadata={}, score={}",
            report.contradictions.len(),
            evidence_index.contradictions.len(),
            report.metadata.contradiction_count,
            report.score.contradiction_count
        ));
    }

    let state_record_path = cfg
        .workspace_dir
        .join("state/consensus")
        .join(format!("{}.json", filename_safe(&report.workflow_id)));
    if !state_record_path.exists() {
        return Err(anyhow!(
            "missing consensus state record: {}",
            state_record_path.display()
        ));
    }
    let state_record = read_json::<ConsensusStateRecord>(&state_record_path)?;
    validate_state_record(&report, &state_record)?;
    validate_shared_state_record(cfg, &report, &state_record)?;

    Ok(ConsensusReplaySummary {
        session_id: report.session_id.clone(),
        workflow_id: report.workflow_id.clone(),
        decision_question: report.decision_question.clone(),
        reviewer_lanes: report
            .reviewer_lanes
            .iter()
            .map(|lane| format!("{}:{:?}", lane.lane_name, lane.stance))
            .collect(),
        agreement_score: report.agreement_score,
        disagreement_summary: report.disagreement_summary.clone(),
        evidence_count: report.evidence_used.len(),
        contradiction_count: report.contradictions.len(),
        confidence: report.score.confidence,
        policy_result: report.policy_result.clone(),
        cost_estimate: report.cost_estimate.clone(),
        artifact_status: ConsensusIntegrityStatus::Ok,
        shared_state_status: SharedStateMatchStatus::Ok,
        replay_status: ConsensusReplayStatus::ValidatedEvidenceOnly,
        next_recommended_command: report.next_recommended_command.clone(),
    })
}

pub fn render_replay_summary(summary: &ConsensusReplaySummary) -> String {
    format!(
        "Consensus replay validated\n\
         session_id: {}\n\
         workflow_id: {}\n\
         decision_question: {}\n\
         reviewer_lanes: {}\n\
         agreement_score: {:.2}\n\
         disagreement: {}\n\
         evidence_count: {}\n\
         contradiction_count: {}\n\
         confidence: {:.2}\n\
         policy_result: {:?}\n\
         cost_estimate: {}\n\
         artifact_integrity: {:?}\n\
         shared_state_match: {:?}\n\
         replay_status: {:?}\n\
         next: {}\n",
        summary.session_id,
        summary.workflow_id,
        summary.decision_question,
        summary.reviewer_lanes.join(", "),
        summary.agreement_score,
        summary.disagreement_summary,
        summary.evidence_count,
        summary.contradiction_count,
        summary.confidence,
        summary.policy_result,
        summary.cost_estimate.note,
        summary.artifact_status,
        summary.shared_state_status,
        summary.replay_status,
        summary.next_recommended_command,
    )
}

pub fn render_terminal_summary(report: &ConsensusReport) -> String {
    let lanes = report
        .reviewer_lanes
        .iter()
        .map(|lane| {
            format!(
                "- {}: {:?} ({:.2})",
                lane.lane_name, lane.stance, lane.confidence
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Consensus dry-run complete\n\
         question: {}\n\
         workflow_id: {}\n\
         session_id: {}\n\
         reviewer_lanes:\n{}\n\
         agreement_score: {:.2}\n\
         disagreement: {}\n\
         confidence: {:.2}\n\
         policy_result: {:?}\n\
         estimated_cost: {}\n\
         report_markdown: {}\n\
         report_json: {}\n\
         evidence_index: {}\n\
         state_record: {}\n\
         next: {}\n",
        report.decision_question,
        report.workflow_id,
        report.session_id,
        lanes,
        report.agreement_score,
        report.disagreement_summary,
        report.score.confidence,
        report.policy_result,
        report.cost_estimate.note,
        report.artifact_paths.report_markdown.display(),
        report.artifact_paths.report_json.display(),
        report.artifact_paths.evidence_index_json.display(),
        report.artifact_paths.state_record_json.display(),
        report.next_recommended_command,
    )
}

fn normalize_question(question: &str) -> Result<String> {
    let question = question.trim();
    if question.is_empty() {
        return Err(anyhow!("consensus question is empty"));
    }
    Ok(question.to_string())
}

fn mock_reviewers(question: &str) -> Vec<ConsensusReviewerFinding> {
    vec![
        architecture_reviewer(question),
        risk_reviewer(question),
        operator_reviewer(question),
    ]
}

fn architecture_reviewer(question: &str) -> ConsensusReviewerFinding {
    let abstraction = contains_any(question, &["abstraction", "api", "provider", "contract"]);
    ConsensusReviewerFinding {
        lane: ReviewerLaneKind::Architecture,
        lane_name: "Architecture reviewer".to_string(),
        stance: if abstraction {
            ReviewerStance::Support
        } else {
            ReviewerStance::Caution
        },
        confidence: if abstraction { 0.86 } else { 0.74 },
        key_findings: vec![
            "Prefer typed contracts before expanding runtime surfaces.".to_string(),
            "Keep deterministic orchestration in Rust and use models only behind explicit boundaries."
                .to_string(),
        ],
        evidence_refs: vec![
            "docs/wiki/MANIFEST.md#Current goal context router".to_string(),
            "docs/wiki/ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md#Provider normalization".to_string(),
        ],
        risks: vec!["Provider-specific branches can leak into workflow logic.".to_string()],
        recommendation: "Support the decision if it preserves a small typed runtime contract."
            .to_string(),
    }
}

fn risk_reviewer(question: &str) -> ConsensusReviewerFinding {
    let risky = contains_any(
        question,
        &["live", "execute", "trading", "network", "provider"],
    );
    ConsensusReviewerFinding {
        lane: ReviewerLaneKind::Risk,
        lane_name: "Risk reviewer".to_string(),
        stance: if risky {
            ReviewerStance::Caution
        } else {
            ReviewerStance::Support
        },
        confidence: if risky { 0.82 } else { 0.78 },
        key_findings: vec![
            "Consensus output is evidence, not authority.".to_string(),
            "Follow-up actions must remain gated by policy and operator approval.".to_string(),
        ],
        evidence_refs: vec![
            "docs/definition-of-shippable.md#Not shippable if".to_string(),
            "docs/wiki/ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md#Risks / constraints".to_string(),
        ],
        risks: vec![
            "Hidden network behavior or provider sprawl would weaken the local-first boundary."
                .to_string(),
        ],
        recommendation: "Proceed only as a local dry-run until provider contracts and cost gates exist."
            .to_string(),
    }
}

fn operator_reviewer(question: &str) -> ConsensusReviewerFinding {
    let user_facing = contains_any(question, &["cli", "output", "user", "onboarding", "api"]);
    ConsensusReviewerFinding {
        lane: ReviewerLaneKind::Operator,
        lane_name: "Operator reviewer".to_string(),
        stance: if user_facing {
            ReviewerStance::Support
        } else {
            ReviewerStance::Caution
        },
        confidence: if user_facing { 0.84 } else { 0.76 },
        key_findings: vec![
            "The result must be understandable without reading architecture docs.".to_string(),
            "Artifacts should point to the next safe inspection command.".to_string(),
        ],
        evidence_refs: vec![
            "README.md#Quick Start".to_string(),
            "docs/wiki/ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md#First-run onboarding".to_string(),
        ],
        risks: vec!["Too many knobs can hide the value of the signature workflow.".to_string()],
        recommendation: "Support if the terminal summary is concise and every artifact is inspectable."
            .to_string(),
    }
}

fn evidence_items(question: &str) -> Vec<ConsensusEvidenceItem> {
    let mut items = vec![
        ConsensusEvidenceItem {
            id: "evidence:wiki-adversarial-review".to_string(),
            source: "docs/wiki/ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md"
                .to_string(),
            summary:
                "Roadmap pressure favors onboarding, signature consensus, provider contracts, evidence, and state quality."
                    .to_string(),
        },
        ConsensusEvidenceItem {
            id: "evidence:shippable-definition".to_string(),
            source: "docs/definition-of-shippable.md".to_string(),
            summary:
                "Optional integrations must stay opt-in, default-safe, and clear about validation."
                    .to_string(),
        },
        ConsensusEvidenceItem {
            id: "evidence:wiki-manifest-router".to_string(),
            source: "docs/wiki/MANIFEST.md".to_string(),
            summary:
                "Shared state, session evidence, and wiki doctrine must remain separate lanes."
                    .to_string(),
        },
    ];
    if contains_any(question, &["provider", "api", "openrouter", "openai"]) {
        items.push(ConsensusEvidenceItem {
            id: "evidence:provider-onboarding-config".to_string(),
            source: "src/config.rs".to_string(),
            summary:
                "Provider onboarding metadata exists separately from runtime model-call permission."
                    .to_string(),
        });
    }
    items
}

fn contradictions_for(question: &str) -> Vec<ConsensusContradiction> {
    if contains_any(question, &["live", "trading", "execute now", "production"]) {
        vec![ConsensusContradiction {
            id: "contradiction:execution-readiness".to_string(),
            summary:
                "The question hints at execution while current roadmap evidence recommends research-only dry-runs."
                    .to_string(),
            severity: "medium".to_string(),
        }]
    } else {
        vec![]
    }
}

fn score_consensus(
    reviewers: &[ConsensusReviewerFinding],
    evidence_count: usize,
    contradiction_count: usize,
) -> ConsensusScore {
    let supportish = reviewers
        .iter()
        .filter(|reviewer| {
            matches!(
                reviewer.stance,
                ReviewerStance::Support | ReviewerStance::Caution
            )
        })
        .count();
    let agreement_score = round2(supportish as f64 / reviewers.len().max(1) as f64);
    let reviewer_confidence_average = round2(
        reviewers
            .iter()
            .map(|reviewer| reviewer.confidence)
            .sum::<f64>()
            / reviewers.len().max(1) as f64,
    );
    let evidence_bonus = (evidence_count as f64 * 0.03).min(0.12);
    let contradiction_penalty = contradiction_count as f64 * 0.12;
    let freshness = 1.0;
    let confidence = round2(
        (reviewer_confidence_average + evidence_bonus - contradiction_penalty).clamp(0.0, 1.0),
    );
    ConsensusScore {
        agreement_score,
        confidence,
        reviewer_confidence_average,
        evidence_count,
        contradiction_count,
        freshness,
    }
}

fn summarize_disagreement(
    reviewers: &[ConsensusReviewerFinding],
    contradiction_count: usize,
) -> String {
    let cautious = reviewers
        .iter()
        .filter(|reviewer| reviewer.stance == ReviewerStance::Caution)
        .map(|reviewer| reviewer.lane_name.as_str())
        .collect::<Vec<_>>();
    if contradiction_count > 0 {
        return "Reviewers agree the question is useful, but execution readiness conflicts with current research-only safety posture.".to_string();
    }
    if cautious.is_empty() {
        "No material disagreement; all lanes support the decision as a dry-run evidence packet."
            .to_string()
    } else {
        format!(
            "Broad support with sequencing caution from {}.",
            cautious.join(", ")
        )
    }
}

fn write_artifacts(report: &ConsensusReport) -> Result<()> {
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
        serde_json::to_string_pretty(report).context("failed to encode consensus report")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.report_json.display()
        )
    })?;
    let evidence_index = ConsensusEvidenceIndex {
        session_id: report.session_id.clone(),
        workflow_id: report.workflow_id.clone(),
        evidence_items: report.evidence_used.clone(),
        contradictions: report.contradictions.clone(),
    };
    fs::write(
        &report.artifact_paths.evidence_index_json,
        serde_json::to_string_pretty(&evidence_index)
            .context("failed to encode consensus evidence index")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.evidence_index_json.display()
        )
    })?;
    fs::write(
        &report.artifact_paths.state_record_json,
        serde_json::to_string_pretty(&ConsensusStateRecord::from_report(report))
            .context("failed to encode consensus state record")?,
    )
    .with_context(|| {
        format!(
            "failed to write {}",
            report.artifact_paths.state_record_json.display()
        )
    })?;
    Ok(())
}

fn write_state_record(cfg: &Config, report: &ConsensusReport) -> Result<()> {
    let state = ConsensusStateRecord::from_report(report);
    let store = HybridSharedStateStore::from_config(cfg);
    store.put(SharedStateRecord {
        key: SharedStateKey::new(format!(
            "shared.consensus.{}",
            stable_hex(&report.workflow_id)
        )),
        value: SharedStateValue::Json(
            serde_json::to_value(&state).context("failed to encode consensus state value")?,
        ),
        domain_id: DomainId::new(CONSENSUS_DOMAIN),
        source: "consensus_dry_run".to_string(),
        confidence: report.score.confidence,
        updated_at: report.metadata.last_verified_at.clone(),
        expires_at: None,
        session_id: Some(sessions::SessionId::new(report.session_id.clone())),
    })
}

fn write_cost_record(cfg: &Config, report: &ConsensusReport) -> Result<()> {
    let record = cost_ledger::consensus_dry_run_record(
        &report.session_id,
        &report.workflow_id,
        &format!(
            "quant-m consensus --dry-run \"{}\"",
            report.decision_question
        ),
        Some(report.decision_question.split_whitespace().count() as u64),
        Some(
            report
                .reviewer_lanes
                .iter()
                .map(|lane| {
                    lane.key_findings.len() + lane.evidence_refs.len() + lane.risks.len() + 1
                })
                .sum::<usize>() as u64,
        ),
    );
    cost_ledger::append_cost_record(cfg, &record)
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &std::path::Path) -> Result<T> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("failed to decode {}", path.display()))
}

fn read_consensus_report(path: &std::path::Path) -> Result<ConsensusReport> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let value = serde_json::from_str::<serde_json::Value>(&raw)
        .with_context(|| format!("failed to decode {}", path.display()))?;
    if value.get("policy_result").is_none() {
        return Err(anyhow!("consensus report missing policy_result"));
    }
    serde_json::from_value(value).with_context(|| format!("failed to decode {}", path.display()))
}

fn validate_report_required_fields(report: &ConsensusReport) -> Result<()> {
    if report.workflow_id.trim().is_empty() {
        return Err(anyhow!("consensus report missing workflow_id"));
    }
    if report.decision_question.trim().is_empty() {
        return Err(anyhow!("consensus report missing decision_question"));
    }
    if report.metadata.session_id != report.session_id {
        return Err(anyhow!(
            "metadata session_id mismatch: expected {}, got {}",
            report.session_id,
            report.metadata.session_id
        ));
    }
    if report.metadata.workflow_id != report.workflow_id {
        return Err(anyhow!(
            "metadata workflow_id mismatch: expected {}, got {}",
            report.workflow_id,
            report.metadata.workflow_id
        ));
    }
    if report.metadata.last_verified_at.trim().is_empty() {
        return Err(anyhow!("consensus report missing last_verified_at"));
    }
    if report.metadata.decision_scope.trim().is_empty() {
        return Err(anyhow!("consensus report missing decision_scope"));
    }
    if !(0.0..=1.0).contains(&report.metadata.confidence) {
        return Err(anyhow!(
            "consensus report metadata confidence is out of range"
        ));
    }
    match report.metadata.memory_class {
        MemoryClass::Ephemeral
        | MemoryClass::Tactical
        | MemoryClass::Strategic
        | MemoryClass::Canonical => {}
    }
    Ok(())
}

fn validate_state_record(report: &ConsensusReport, state: &ConsensusStateRecord) -> Result<()> {
    if state.session_id != report.session_id {
        return Err(anyhow!(
            "consensus state session_id mismatch: expected {}, got {}",
            report.session_id,
            state.session_id
        ));
    }
    if state.workflow_id != report.workflow_id {
        return Err(anyhow!(
            "consensus state workflow_id mismatch: expected {}, got {}",
            report.workflow_id,
            state.workflow_id
        ));
    }
    if state.decision_question != report.decision_question {
        return Err(anyhow!("consensus state decision question mismatch"));
    }
    if state.metadata.source_count != report.metadata.source_count {
        return Err(anyhow!("consensus state source_count mismatch"));
    }
    if state.metadata.contradiction_count != report.metadata.contradiction_count {
        return Err(anyhow!("consensus state contradiction_count mismatch"));
    }
    if state.policy_result != report.policy_result {
        return Err(anyhow!("consensus state policy_result mismatch"));
    }
    Ok(())
}

fn validate_shared_state_record(
    cfg: &Config,
    report: &ConsensusReport,
    state: &ConsensusStateRecord,
) -> Result<()> {
    let store = HybridSharedStateStore::from_config(cfg);
    let records = store.list(Some(&DomainId::new(CONSENSUS_DOMAIN)))?;
    let matched = records.into_iter().any(|record| {
        record
            .session_id
            .as_ref()
            .is_some_and(|value| value.as_str() == report.session_id)
            && matches!(
                record.value,
                SharedStateValue::Json(ref value)
                    if value
                        .get("workflow_id")
                        .and_then(|value| value.as_str())
                        == Some(state.workflow_id.as_str())
                        && value
                            .get("decision_question")
                            .and_then(|value| value.as_str())
                            == Some(state.decision_question.as_str())
                        && value
                            .get("metadata")
                            .and_then(|value| value.get("source_count"))
                            .and_then(|value| value.as_u64())
                            == Some(state.metadata.source_count as u64)
                        && value
                            .get("metadata")
                            .and_then(|value| value.get("contradiction_count"))
                            .and_then(|value| value.as_u64())
                            == Some(state.metadata.contradiction_count as u64)
            )
    });
    if !matched {
        return Err(anyhow!(
            "missing or mismatched shared-state consensus record for session {}",
            report.session_id
        ));
    }
    Ok(())
}

impl ConsensusStateRecord {
    fn from_report(report: &ConsensusReport) -> Self {
        Self {
            workflow_id: report.workflow_id.clone(),
            session_id: report.session_id.clone(),
            decision_question: report.decision_question.clone(),
            agreement_score: report.agreement_score,
            disagreement_summary: report.disagreement_summary.clone(),
            policy_result: report.policy_result.clone(),
            recommended_next_action: report.recommended_next_action.clone(),
            metadata: report.metadata.clone(),
            artifact_paths: report.artifact_paths.clone(),
        }
    }
}

fn render_markdown_report(report: &ConsensusReport) -> String {
    let lanes = report
        .reviewer_lanes
        .iter()
        .map(|lane| {
            format!(
                "### {}\n\n- Stance: `{:?}`\n- Confidence: `{:.2}`\n- Findings:\n{}\n- Risks:\n{}\n- Recommendation: {}\n",
                lane.lane_name,
                lane.stance,
                lane.confidence,
                bullet_list(&lane.key_findings),
                bullet_list(&lane.risks),
                lane.recommendation
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "# Consensus Dry-Run Report\n\n\
         ## Decision Question\n\n{}\n\n\
         ## Summary\n\n\
         - Workflow ID: `{}`\n\
         - Session ID: `{}`\n\
         - Agreement score: `{:.2}`\n\
         - Confidence: `{:.2}`\n\
         - Policy result: `{:?}`\n\
         - Cost: `{}`\n\
         - Next command: `{}`\n\n\
         ## Reviewer Lanes\n\n{}\n\
         ## Disagreement Summary\n\n{}\n\n\
         ## Evidence Used\n\n{}\n\n\
         ## Contradictions\n\n{}\n\n\
         ## Next Recommended Action\n\n{}\n",
        report.decision_question,
        report.workflow_id,
        report.session_id,
        report.agreement_score,
        report.score.confidence,
        report.policy_result,
        report.cost_estimate.note,
        report.next_recommended_command,
        lanes,
        report.disagreement_summary,
        bullet_list(
            &report
                .evidence_used
                .iter()
                .map(|item| format!("{}: {} ({})", item.id, item.summary, item.source))
                .collect::<Vec<_>>()
        ),
        if report.contradictions.is_empty() {
            "- None.".to_string()
        } else {
            bullet_list(
                &report
                    .contradictions
                    .iter()
                    .map(|item| format!("{}: {} ({})", item.id, item.summary, item.severity))
                    .collect::<Vec<_>>(),
            )
        },
        report.recommended_next_action,
    )
}

fn artifact_paths(cfg: &Config, session_id: &str, workflow_id: &str) -> ConsensusArtifactPaths {
    let session_dir = cfg.runtime.session_dir.join(session_id);
    let state_dir = cfg.workspace_dir.join("state/consensus");
    let state_filename = format!("{}.json", filename_safe(workflow_id));
    ConsensusArtifactPaths {
        report_markdown: session_dir.join("consensus-report.md"),
        report_json: session_dir.join("consensus-report.json"),
        evidence_index_json: session_dir.join("evidence-index.json"),
        state_record_json: state_dir.join(state_filename),
        session_dir,
    }
}

fn classify_decision_scope(question: &str) -> String {
    if contains_any(
        question,
        &["provider", "api", "model", "openrouter", "openai"],
    ) {
        "provider_runtime".to_string()
    } else if contains_any(question, &["trading", "risk", "market", "forex"]) {
        "trading_research".to_string()
    } else if contains_any(question, &["setup", "onboarding", "cli", "user"]) {
        "operator_experience".to_string()
    } else {
        "technical_decision".to_string()
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    let value = value.to_ascii_lowercase();
    needles.iter().any(|needle| value.contains(needle))
}

fn stable_hex(value: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
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
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn bullet_list(items: &[String]) -> String {
    if items.is_empty() {
        return "- None.".to_string();
    }
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use serde_json::Value;
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

    #[test]
    fn dry_run_creates_artifacts_and_state() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");

        assert!(report.artifact_paths.report_markdown.exists());
        assert!(report.artifact_paths.report_json.exists());
        assert!(report.artifact_paths.evidence_index_json.exists());
        assert!(report.artifact_paths.state_record_json.exists());
        assert!(crate::cost_ledger::cost_ledger_path(&cfg).exists());
        assert_eq!(report.policy_result, PolicyResult::RequiresOperatorApproval);
        assert_eq!(report.metadata.memory_class, MemoryClass::Tactical);
        assert!(!report.evidence_used.is_empty());

        let sessions = sessions::list_sessions(&cfg).expect("sessions");
        assert_eq!(sessions.len(), 1);
        let state = crate::shared_state::list_state(&cfg, Some(&DomainId::new(CONSENSUS_DOMAIN)))
            .expect("state");
        assert_eq!(state.len(), 1);
    }

    #[test]
    fn dry_run_does_not_require_provider_keys_or_network() {
        let (_tmp, mut cfg) = temp_cfg();
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        let report = run_consensus_dry_run(&cfg, "Should we add provider runtime contracts?")
            .expect("consensus");

        assert_eq!(report.cost_estimate.actual_cost_usd, 0.0);
        assert_eq!(report.cost_estimate.mode, "dry_run_mock");
        let summary = crate::cost_ledger::summarize_costs(&cfg, None, None).expect("cost summary");
        assert_eq!(summary.record_count, 1);
        assert_eq!(summary.total_actual_cost, 0.0);
    }

    #[test]
    fn same_question_has_deterministic_structure_and_scores() {
        let (_tmp_a, cfg_a) = temp_cfg();
        let (_tmp_b, cfg_b) = temp_cfg();
        let a =
            run_consensus_dry_run(&cfg_a, "Should we adopt this API design?").expect("consensus a");
        let b =
            run_consensus_dry_run(&cfg_b, "Should we adopt this API design?").expect("consensus b");

        assert_eq!(a.workflow_id, b.workflow_id);
        assert_eq!(a.agreement_score, b.agreement_score);
        assert_eq!(a.disagreement_summary, b.disagreement_summary);
        assert_eq!(a.reviewer_lanes, b.reviewer_lanes);
        assert_eq!(a.evidence_used, b.evidence_used);
        assert_eq!(a.contradictions, b.contradictions);
    }

    #[test]
    fn empty_question_fails_safely() {
        let (_tmp, cfg) = temp_cfg();
        let err = run_consensus_dry_run(&cfg, "   ").expect_err("empty question fails");
        assert!(err.to_string().contains("consensus question is empty"));
    }

    #[test]
    fn replay_succeeds_for_valid_consensus_session() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        let summary =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect("replay");

        assert_eq!(summary.session_id, report.session_id);
        assert_eq!(summary.workflow_id, report.workflow_id);
        assert_eq!(summary.evidence_count, report.evidence_used.len());
        assert_eq!(summary.contradiction_count, report.contradictions.len());
        assert_eq!(summary.artifact_status, ConsensusIntegrityStatus::Ok);
        assert_eq!(summary.shared_state_status, SharedStateMatchStatus::Ok);
        assert_eq!(
            summary.replay_status,
            ConsensusReplayStatus::ValidatedEvidenceOnly
        );
    }

    #[test]
    fn replay_fails_when_report_is_missing() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        fs::remove_file(&report.artifact_paths.report_json).expect("remove report");

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("missing report fails");
        assert!(err.to_string().contains("missing consensus report"));
    }

    #[test]
    fn replay_fails_when_evidence_index_is_missing() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        fs::remove_file(&report.artifact_paths.evidence_index_json).expect("remove evidence");

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("missing evidence fails");
        assert!(err.to_string().contains("missing consensus evidence index"));
    }

    #[test]
    fn replay_fails_when_state_record_is_missing() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        fs::remove_file(&report.artifact_paths.state_record_json).expect("remove state");

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("missing state fails");
        assert!(err.to_string().contains("missing consensus state record"));
    }

    #[test]
    fn replay_detects_session_id_mismatch() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        mutate_json(&report.artifact_paths.report_json, |value| {
            value["session_id"] = Value::String("session:other".to_string());
        });

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("session mismatch fails");
        assert!(err.to_string().contains("session_id mismatch"));
    }

    #[test]
    fn replay_detects_workflow_id_mismatch() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        mutate_json(&report.artifact_paths.evidence_index_json, |value| {
            value["workflow_id"] = Value::String("workflow:other".to_string());
        });

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("workflow mismatch fails");
        assert!(err.to_string().contains("workflow_id mismatch"));
    }

    #[test]
    fn replay_detects_decision_question_mismatch() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        mutate_json(&report.artifact_paths.state_record_json, |value| {
            value["decision_question"] = Value::String("Different question?".to_string());
        });

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("question mismatch fails");
        assert!(err.to_string().contains("decision question mismatch"));
    }

    #[test]
    fn replay_detects_evidence_count_mismatch() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        mutate_json(&report.artifact_paths.evidence_index_json, |value| {
            value["evidence_items"] = Value::Array(vec![]);
        });

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("evidence count mismatch fails");
        assert!(err.to_string().contains("evidence count mismatch"));
    }

    #[test]
    fn replay_detects_contradiction_count_mismatch() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we execute live trading in production now?")
                .expect("consensus");
        mutate_json(&report.artifact_paths.evidence_index_json, |value| {
            value["contradictions"] = Value::Array(vec![]);
        });

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("contradiction count mismatch fails");
        assert!(err.to_string().contains("contradiction count mismatch"));
    }

    #[test]
    fn replay_fails_when_policy_result_is_missing() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        mutate_json(&report.artifact_paths.report_json, |value| {
            value
                .as_object_mut()
                .expect("object")
                .remove("policy_result");
        });

        let err =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect_err("missing policy fails");
        assert!(err.to_string().contains("policy_result"));
    }

    #[test]
    fn replay_does_not_mutate_artifacts() {
        let (_tmp, cfg) = temp_cfg();
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        let before_report = fs::read(&report.artifact_paths.report_json).expect("read report");
        let before_evidence =
            fs::read(&report.artifact_paths.evidence_index_json).expect("read evidence");
        let before_state = fs::read(&report.artifact_paths.state_record_json).expect("read state");

        replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
            .expect("replay");

        assert_eq!(
            before_report,
            fs::read(&report.artifact_paths.report_json).expect("read report after")
        );
        assert_eq!(
            before_evidence,
            fs::read(&report.artifact_paths.evidence_index_json).expect("read evidence after")
        );
        assert_eq!(
            before_state,
            fs::read(&report.artifact_paths.state_record_json).expect("read state after")
        );
    }

    #[test]
    fn replay_does_not_require_provider_keys_or_network() {
        let (_tmp, mut cfg) = temp_cfg();
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        let report =
            run_consensus_dry_run(&cfg, "Should we adopt this API design?").expect("consensus");
        let summary =
            replay_consensus_session(&cfg, &sessions::SessionId::new(report.session_id.clone()))
                .expect("replay");

        assert_eq!(summary.cost_estimate.actual_cost_usd, 0.0);
        assert_eq!(
            summary.replay_status,
            ConsensusReplayStatus::ValidatedEvidenceOnly
        );
    }

    fn mutate_json(path: &std::path::Path, mutate: impl FnOnce(&mut Value)) {
        let raw = fs::read_to_string(path).expect("read json");
        let mut value = serde_json::from_str::<Value>(&raw).expect("parse json");
        mutate(&mut value);
        fs::write(
            path,
            serde_json::to_string_pretty(&value).expect("encode json"),
        )
        .expect("write json");
    }
}
