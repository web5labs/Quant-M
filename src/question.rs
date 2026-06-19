use crate::config::Config;
use crate::cost_ledger::format_currency_amount;
use crate::worker_proposals::{
    SubmitWorkerProposalInput, WorkerProposalKind, WorkerProposalRecord, WorkerSurfaceKind,
    submit_worker_proposal,
};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub const UNIVERSAL_QUESTION: &str = "What should happen next, based on the available evidence, policy, cost, state, and operator goal?";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum QuantMQuestionMode {
    AgentCluster,
    StaffOsHandoff,
    Harness,
}

impl QuantMQuestionMode {
    pub fn as_str(self) -> &'static str {
        match self {
            QuantMQuestionMode::AgentCluster => "agent_cluster",
            QuantMQuestionMode::StaffOsHandoff => "staff_os_handoff",
            QuantMQuestionMode::Harness => "harness",
        }
    }
}

impl fmt::Display for QuantMQuestionMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for QuantMQuestionMode {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value.trim().to_ascii_lowercase().as_str() {
            "agent_cluster" | "agent-cluster" | "cluster" => Ok(Self::AgentCluster),
            "staff_os_handoff" | "staff-os-handoff" | "staff_os" | "handoff" => {
                Ok(Self::StaffOsHandoff)
            }
            "harness" => Ok(Self::Harness),
            other => Err(anyhow!(
                "unsupported question mode '{other}'; expected agent_cluster, staff_os_handoff, or harness"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct QuantMQuestion {
    pub question_text: String,
    pub universal_question: String,
    pub mode: QuantMQuestionMode,
    pub mode_question: String,
    pub decision_scope: String,
    pub evidence_requirements: Vec<String>,
    pub allowed_worker_surfaces: Vec<String>,
    pub policy_constraints: Vec<String>,
    pub cost_constraints: Vec<String>,
    pub replay_requirement: String,
    pub expected_output_type: String,
    pub pipeline_contract: Vec<String>,
    pub next_safe_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentClusterWorkerLane {
    pub lane_id: String,
    pub surface: WorkerSurfaceKind,
    pub proposal_kind: WorkerProposalKind,
    pub evidence_to_return: Vec<String>,
    pub risk_to_check: String,
    pub authority_denied: Vec<String>,
    pub proposal_summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QuestionCostEstimate {
    pub estimated_cost: f64,
    pub actual_cost: f64,
    pub currency: String,
    pub cost_record: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentClusterProposalPlan {
    pub question: QuantMQuestion,
    pub worker_lanes: Vec<AgentClusterWorkerLane>,
    pub proposal_kinds: Vec<WorkerProposalKind>,
    pub guardrails: Vec<String>,
    pub estimated_cost: QuestionCostEstimate,
    pub next_safe_command: String,
    pub write_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentClusterProposalWriteResult {
    pub plan: AgentClusterProposalPlan,
    pub written_proposals: Vec<WorkerProposalRecord>,
    pub next_safe_command: String,
}

pub fn build_question(mode: QuantMQuestionMode, question_text: &str) -> Result<QuantMQuestion> {
    let question_text = question_text.trim();
    if question_text.is_empty() {
        return Err(anyhow!("question text is empty"));
    }

    let template = template_for(mode);
    Ok(QuantMQuestion {
        question_text: question_text.to_string(),
        universal_question: UNIVERSAL_QUESTION.to_string(),
        mode,
        mode_question: template.mode_question.to_string(),
        decision_scope: infer_decision_scope(question_text),
        evidence_requirements: template.evidence_requirements,
        allowed_worker_surfaces: template.allowed_worker_surfaces,
        policy_constraints: common_policy_constraints(),
        cost_constraints: template.cost_constraints,
        replay_requirement: template.replay_requirement.to_string(),
        expected_output_type: template.expected_output_type.to_string(),
        pipeline_contract: vec![
            "question".to_string(),
            "evidence".to_string(),
            "proposal".to_string(),
            "policy_gate".to_string(),
            "cost_record".to_string(),
            "replayable_decision".to_string(),
            "next_safe_action".to_string(),
        ],
        next_safe_command: template.next_safe_command.to_string(),
    })
}

pub fn build_agent_cluster_proposal_plan(question_text: &str) -> Result<AgentClusterProposalPlan> {
    let question = build_question(QuantMQuestionMode::AgentCluster, question_text)?;
    let worker_lanes = agent_cluster_worker_lanes(&question);
    let proposal_kinds = worker_lanes
        .iter()
        .map(|lane| lane.proposal_kind)
        .collect::<Vec<_>>();

    Ok(AgentClusterProposalPlan {
        question,
        worker_lanes,
        proposal_kinds,
        guardrails: vec![
            "inspect-only by default".to_string(),
            "workers produce evidence and proposals only".to_string(),
            "no execution authority".to_string(),
            "no canonical shared-state mutation".to_string(),
            "no accepted cost-ledger truth".to_string(),
            "no provider calls".to_string(),
            "no trading behavior".to_string(),
            "operator approval remains separate".to_string(),
        ],
        estimated_cost: QuestionCostEstimate {
            estimated_cost: 0.0,
            actual_cost: 0.0,
            currency: "USD".to_string(),
            cost_record: "zero actual; local deterministic plan only".to_string(),
        },
        next_safe_command: "quant-m worker proposal list --status pending_review".to_string(),
        write_available: true,
    })
}

pub fn write_agent_cluster_proposal_plan(
    cfg: &Config,
    plan: AgentClusterProposalPlan,
) -> Result<AgentClusterProposalWriteResult> {
    let mut written_proposals = Vec::new();
    for lane in &plan.worker_lanes {
        let (record, _summary) = submit_worker_proposal(
            cfg,
            SubmitWorkerProposalInput {
                source_surface: lane.surface,
                source_worker_id: format!("worker:question:{}", lane.lane_id),
                proposal_kind: lane.proposal_kind,
                summary: lane.proposal_summary.clone(),
                session_id: None,
                workflow_id: None,
                decision_scope: Some(plan.question.decision_scope.clone()),
            },
        )?;
        written_proposals.push(record);
    }

    Ok(AgentClusterProposalWriteResult {
        next_safe_command: plan.next_safe_command.clone(),
        plan,
        written_proposals,
    })
}

pub fn render_question(question: &QuantMQuestion) -> String {
    format!(
        "Quant-M question\nmode: {}\nquestion: {}\nuniversal_question: {}\nmode_question: {}\ndecision_scope: {}\nexpected_output_type: {}\nreplay_requirement: {}\nnext: {}\n",
        question.mode,
        question.question_text,
        question.universal_question,
        question.mode_question,
        question.decision_scope,
        question.expected_output_type,
        question.replay_requirement,
        question.next_safe_command
    )
}

pub fn render_agent_cluster_proposal_plan(plan: &AgentClusterProposalPlan) -> String {
    let mut out = format!(
        "Question worker proposal plan\nmode: {}\nquestion: {}\ndecision_scope: {}\nestimated_cost: {}\nactual_cost: {}\nwrite_available: {}\n",
        plan.question.mode,
        plan.question.question_text,
        plan.question.decision_scope,
        format_currency_amount(
            plan.estimated_cost.estimated_cost,
            &plan.estimated_cost.currency
        ),
        format_currency_amount(
            plan.estimated_cost.actual_cost,
            &plan.estimated_cost.currency
        ),
        plan.write_available
    );
    out.push_str("worker_lanes:\n");
    for lane in &plan.worker_lanes {
        out.push_str(&format!(
            "- {} surface={} kind={} evidence={} risk={} denied={}\n",
            lane.lane_id,
            lane.surface,
            lane.proposal_kind,
            lane.evidence_to_return.join("; "),
            lane.risk_to_check,
            lane.authority_denied.join("; ")
        ));
    }
    out.push_str("guardrails:\n");
    for guardrail in &plan.guardrails {
        out.push_str(&format!("- {guardrail}\n"));
    }
    out.push_str(&format!("next: {}\n", plan.next_safe_command));
    out
}

pub fn render_agent_cluster_write_result(result: &AgentClusterProposalWriteResult) -> String {
    let mut out = format!(
        "Question worker proposals written\nrecords: {}\nmode: {}\nquestion: {}\n",
        result.written_proposals.len(),
        result.plan.question.mode,
        result.plan.question.question_text
    );
    for proposal in &result.written_proposals {
        out.push_str(&format!(
            "- {} surface={} kind={} status={} non_authoritative={}\n",
            proposal.proposal_id,
            proposal.source_surface,
            proposal.proposal_kind,
            proposal.status,
            proposal.non_authoritative
        ));
    }
    out.push_str(&format!("next: {}\n", result.next_safe_command));
    out
}

struct QuestionTemplate {
    mode_question: &'static str,
    evidence_requirements: Vec<String>,
    allowed_worker_surfaces: Vec<String>,
    cost_constraints: Vec<String>,
    replay_requirement: &'static str,
    expected_output_type: &'static str,
    next_safe_command: &'static str,
}

fn template_for(mode: QuantMQuestionMode) -> QuestionTemplate {
    match mode {
        QuantMQuestionMode::AgentCluster => QuestionTemplate {
            mode_question: "Which workers should review this, what evidence should they return, and what authority do they not have?",
            evidence_requirements: vec![
                "worker lane assignment".to_string(),
                "evidence records".to_string(),
                "risk notes".to_string(),
                "non-authority statement".to_string(),
            ],
            allowed_worker_surfaces: vec![
                "staff_os_workspace".to_string(),
                "cmux_lane".to_string(),
                "tmux_worker".to_string(),
                "termux_worker".to_string(),
                "cron_worker".to_string(),
                "mtime_worker".to_string(),
                "polling_worker".to_string(),
                "local_worker".to_string(),
            ],
            cost_constraints: vec![
                "dry-run cost records only unless a later harness gate approves otherwise"
                    .to_string(),
            ],
            replay_requirement: "worker proposals are evidence only until a replayable core decision validates them",
            expected_output_type: "worker_lanes_and_proposal_records",
            next_safe_command: "quant-m worker proposal list --status pending_review",
        },
        QuantMQuestionMode::StaffOsHandoff => QuestionTemplate {
            mode_question: "What bounded implementation packet should Staff-OS or Codex execute next?",
            evidence_requirements: vec![
                "goal".to_string(),
                "files likely touched".to_string(),
                "acceptance tests".to_string(),
                "guardrails".to_string(),
                "definition of done".to_string(),
                "do-not-build list".to_string(),
                "validation commands".to_string(),
            ],
            allowed_worker_surfaces: vec![
                "staff_os_workspace".to_string(),
                "local_worker".to_string(),
            ],
            cost_constraints: vec![
                "no provider cost unless routed through harness mode".to_string(),
            ],
            replay_requirement: "handoff packets must cite evidence and preserve validation commands",
            expected_output_type: "bounded_implementation_packet",
            next_safe_command: "quant-m worker proposal list --status pending_review",
        },
        QuantMQuestionMode::Harness => QuestionTemplate {
            mode_question: "Which model/provider/tool route should be used, under what budget, and with what policy limits?",
            evidence_requirements: vec![
                "provider route".to_string(),
                "model role".to_string(),
                "cost estimate".to_string(),
                "budget gate".to_string(),
                "capability check".to_string(),
                "fallback route".to_string(),
            ],
            allowed_worker_surfaces: vec!["local_worker".to_string()],
            cost_constraints: vec![
                "budget gate required before live provider use".to_string(),
                "fallback route required".to_string(),
                "no uncontrolled provider expansion".to_string(),
            ],
            replay_requirement: "provider choices must be replayable before use",
            expected_output_type: "provider_route_budget_and_policy_packet",
            next_safe_command: "quant-m cost summary",
        },
    }
}

fn common_policy_constraints() -> Vec<String> {
    vec![
        "workers propose; core decides".to_string(),
        "operator approval remains separate from evidence".to_string(),
        "no live trading or order generation".to_string(),
        "no dashboard or provider-specific mode expansion".to_string(),
        "only agent_cluster, staff_os_handoff, and harness modes are valid".to_string(),
    ]
}

fn infer_decision_scope(question_text: &str) -> String {
    let value = question_text.to_ascii_lowercase();
    if contains_any(&value, &["model", "provider", "budget", "harness", "route"]) {
        "provider_harness".to_string()
    } else if contains_any(&value, &["codex", "staff", "implement", "files", "build"]) {
        "implementation_handoff".to_string()
    } else if contains_any(&value, &["worker", "cluster", "review", "evidence"]) {
        "agent_cluster_review".to_string()
    } else {
        "operator_goal_review".to_string()
    }
}

fn contains_any(value: &str, needles: &[&str]) -> bool {
    needles.iter().any(|needle| value.contains(needle))
}

fn agent_cluster_worker_lanes(question: &QuantMQuestion) -> Vec<AgentClusterWorkerLane> {
    let topic = &question.question_text;
    vec![
        lane(
            "staff_os_context_lane",
            WorkerSurfaceKind::StaffOsWorkspace,
            WorkerProposalKind::Evidence,
            topic,
            &[
                "existing implementation context",
                "likely files or desks involved",
                "handoff readiness gaps",
            ],
            "missing implementation context",
        ),
        lane(
            "cmux_architecture_lane",
            WorkerSurfaceKind::CmuxLane,
            WorkerProposalKind::Review,
            topic,
            &[
                "architecture fit",
                "duplicate command-pattern risk",
                "boundary assumptions",
            ],
            "parallel-world command duplication",
        ),
        lane(
            "tmux_validation_lane",
            WorkerSurfaceKind::TmuxWorker,
            WorkerProposalKind::Review,
            topic,
            &[
                "focused acceptance checks",
                "regression coverage",
                "replay or policy validation needs",
            ],
            "untested behavior drift",
        ),
        lane(
            "termux_edge_lane",
            WorkerSurfaceKind::TermuxWorker,
            WorkerProposalKind::Evidence,
            topic,
            &[
                "edge-runtime constraints",
                "mobile shell assumptions",
                "local-only limitations",
            ],
            "edge environment mismatch",
        ),
        lane(
            "cron_cadence_lane",
            WorkerSurfaceKind::CronWorker,
            WorkerProposalKind::Evidence,
            topic,
            &[
                "scheduled-review needs",
                "cadence assumptions",
                "idle automation risks",
            ],
            "unbounded recurring work",
        ),
        lane(
            "mtime_change_lane",
            WorkerSurfaceKind::MtimeWorker,
            WorkerProposalKind::Evidence,
            topic,
            &[
                "file-change triggers",
                "stale artifact indicators",
                "change-detection boundaries",
            ],
            "stale or accidental artifact reuse",
        ),
        lane(
            "polling_state_lane",
            WorkerSurfaceKind::PollingWorker,
            WorkerProposalKind::Evidence,
            topic,
            &[
                "polling inputs",
                "state freshness",
                "external dependency assumptions",
            ],
            "unverified freshness",
        ),
        lane(
            "local_policy_lane",
            WorkerSurfaceKind::LocalWorker,
            WorkerProposalKind::Review,
            topic,
            &["policy gate", "cost gate", "next safe action"],
            "policy or cost bypass",
        ),
    ]
}

fn lane(
    lane_id: &str,
    surface: WorkerSurfaceKind,
    proposal_kind: WorkerProposalKind,
    topic: &str,
    evidence_to_return: &[&str],
    risk_to_check: &str,
) -> AgentClusterWorkerLane {
    AgentClusterWorkerLane {
        lane_id: lane_id.to_string(),
        surface,
        proposal_kind,
        evidence_to_return: evidence_to_return
            .iter()
            .map(|item| (*item).to_string())
            .collect(),
        risk_to_check: risk_to_check.to_string(),
        authority_denied: vec![
            "execute commands".to_string(),
            "mutate canonical state".to_string(),
            "write accepted cost truth".to_string(),
            "approve work".to_string(),
        ],
        proposal_summary: format!(
            "{lane_id} should review '{topic}' and return structured {proposal_kind} only; no execution authority."
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
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
    fn supports_exactly_three_modes_with_aliases() {
        assert_eq!(
            "agent-cluster".parse::<QuantMQuestionMode>().unwrap(),
            QuantMQuestionMode::AgentCluster
        );
        assert_eq!(
            "handoff".parse::<QuantMQuestionMode>().unwrap(),
            QuantMQuestionMode::StaffOsHandoff
        );
        assert_eq!(
            "harness".parse::<QuantMQuestionMode>().unwrap(),
            QuantMQuestionMode::Harness
        );
        let err = "trading".parse::<QuantMQuestionMode>().unwrap_err();
        assert!(err.to_string().contains("unsupported question mode"));
    }

    #[test]
    fn agent_cluster_question_uses_worker_proposal_contract() {
        let question = build_question(
            QuantMQuestionMode::AgentCluster,
            "How should this be reviewed?",
        )
        .unwrap();

        assert_eq!(question.mode, QuantMQuestionMode::AgentCluster);
        assert_eq!(question.universal_question, UNIVERSAL_QUESTION);
        assert!(
            question
                .allowed_worker_surfaces
                .contains(&"cmux_lane".to_string())
        );
        assert_eq!(
            question.expected_output_type,
            "worker_lanes_and_proposal_records"
        );
        assert!(
            question
                .pipeline_contract
                .contains(&"policy_gate".to_string())
        );
    }

    #[test]
    fn agent_cluster_question_builds_worker_proposal_plan_without_writes() {
        let (_tmp, cfg) = temp_cfg();
        let before = list_worker_proposals(&cfg, None, None).expect("before");

        let plan =
            build_agent_cluster_proposal_plan("Review this API design decision").expect("plan");
        let after = list_worker_proposals(&cfg, None, None).expect("after");

        assert_eq!(before.total_count, 0);
        assert_eq!(after.total_count, 0);
        assert_eq!(plan.question.mode, QuantMQuestionMode::AgentCluster);
        assert_eq!(plan.estimated_cost.actual_cost, 0.0);
        assert_eq!(plan.worker_lanes.len(), 8);
        assert!(plan.write_available);
        assert!(
            plan.worker_lanes
                .iter()
                .any(|lane| lane.surface == WorkerSurfaceKind::CmuxLane)
        );
    }

    #[test]
    fn agent_cluster_question_writes_pending_non_authoritative_proposals() {
        let (_tmp, cfg) = temp_cfg();
        let plan =
            build_agent_cluster_proposal_plan("Review this API design decision").expect("plan");

        let result = write_agent_cluster_proposal_plan(&cfg, plan).expect("write");
        let listed = list_worker_proposals(&cfg, None, Some(WorkerProposalStatus::PendingReview))
            .expect("list");

        assert_eq!(result.written_proposals.len(), 8);
        assert_eq!(listed.total_count, 8);
        assert!(
            result
                .written_proposals
                .iter()
                .all(|proposal| proposal.non_authoritative)
        );
        assert!(
            result
                .written_proposals
                .iter()
                .all(|proposal| proposal.status == WorkerProposalStatus::PendingReview)
        );
    }

    #[test]
    fn staff_os_handoff_question_outputs_bounded_packet() {
        let question = build_question(
            QuantMQuestionMode::StaffOsHandoff,
            "What should Codex implement next?",
        )
        .unwrap();

        assert_eq!(question.decision_scope, "implementation_handoff");
        assert_eq!(
            question.expected_output_type,
            "bounded_implementation_packet"
        );
        assert!(
            question
                .evidence_requirements
                .contains(&"validation commands".to_string())
        );
    }

    #[test]
    fn harness_question_keeps_provider_use_gated() {
        let question = build_question(
            QuantMQuestionMode::Harness,
            "Which model route should handle this?",
        )
        .unwrap();

        assert_eq!(
            question.expected_output_type,
            "provider_route_budget_and_policy_packet"
        );
        assert!(
            question
                .cost_constraints
                .iter()
                .any(|item| item.contains("budget gate"))
        );
        assert_eq!(question.next_safe_command, "quant-m cost summary");
    }

    #[test]
    fn empty_question_fails_safely() {
        let err = build_question(QuantMQuestionMode::AgentCluster, "  ").unwrap_err();
        assert!(err.to_string().contains("question text is empty"));
    }
}
