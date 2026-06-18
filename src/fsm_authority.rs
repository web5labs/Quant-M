use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum FsmAuthorityStatus {
    Wired,
    PartiallyWired,
    ModeledOnly,
    AuditedOnly,
    DesignOnly,
    LegacyCompatibility,
}

impl fmt::Display for FsmAuthorityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Wired => "wired",
            Self::PartiallyWired => "partially_wired",
            Self::ModeledOnly => "modeled_only",
            Self::AuditedOnly => "audited_only",
            Self::DesignOnly => "design_only",
            Self::LegacyCompatibility => "legacy_compatibility",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmAuthorityRecord {
    pub fsm_id: &'static str,
    pub human_name: &'static str,
    pub authority_status: FsmAuthorityStatus,
    pub wired_command_surfaces: Vec<&'static str>,
    pub source_module: &'static str,
    pub docs_reference: &'static str,
    pub emits_session_evidence: bool,
    pub gates_side_effects: bool,
    pub known_limitations: Vec<&'static str>,
}

pub fn authority_records() -> Vec<FsmAuthorityRecord> {
    vec![
        FsmAuthorityRecord {
            fsm_id: "context_guardian",
            human_name: "Context Guardian",
            authority_status: FsmAuthorityStatus::Wired,
            wired_command_surfaces: vec![
                "context-status",
                "context guard",
                "context packet",
                "compact",
                "boil",
                "loop --dry-run",
            ],
            source_module: "src/fsm_core.rs; src/context_status.rs; src/context_guardian.rs",
            docs_reference: "docs/fsm/rust-fsm-authority-audit.md",
            emits_session_evidence: false,
            gates_side_effects: true,
            known_limitations: vec![
                "green/yellow/red remains display compatibility",
                "transition records are derived from session and compact evidence",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "policy_approval",
            human_name: "Policy Approval",
            authority_status: FsmAuthorityStatus::PartiallyWired,
            wired_command_surfaces: vec!["skills run", "policy evaluate-skill"],
            source_module: "src/fsm_core.rs; src/skills.rs; src/policy_registry.rs",
            docs_reference: "docs/fsm/rust-fsm-authority-audit.md",
            emits_session_evidence: true,
            gates_side_effects: true,
            known_limitations: vec![
                "wired for runnable skills",
                "not yet a single gate for every provider, channel, HTTP, adapter, or trading-like surface",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "provider_tool_onboarding",
            human_name: "Provider And Tool Onboarding",
            authority_status: FsmAuthorityStatus::AuditedOnly,
            wired_command_surfaces: vec![
                "onboarding",
                "doctor",
                "capabilities",
                "provider list",
                "tool list",
            ],
            source_module: "src/capabilities.rs; src/config.rs; src/main.rs",
            docs_reference: "docs/feature-map.md",
            emits_session_evidence: false,
            gates_side_effects: true,
            known_limitations: vec![
                "capability detection is inspect-only",
                "no dedicated onboarding FSM exists",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "question_consensus_strategist",
            human_name: "Question, Consensus, And Strategist",
            authority_status: FsmAuthorityStatus::AuditedOnly,
            wired_command_surfaces: vec![
                "question ask",
                "consensus --dry-run",
                "strategist dry-run",
            ],
            source_module: "src/question.rs; src/consensus.rs; src/strategist.rs",
            docs_reference: "docs/feature-map.md",
            emits_session_evidence: true,
            gates_side_effects: true,
            known_limitations: vec![
                "dry-run and proposal boundaries exist",
                "no dedicated question/consensus runtime FSM exists",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "session_replay",
            human_name: "Session Replay",
            authority_status: FsmAuthorityStatus::PartiallyWired,
            wired_command_surfaces: vec![
                "session list",
                "session show",
                "replay",
                "resume-plan",
                "compact",
            ],
            source_module: "src/fsm_core.rs; src/sessions.rs; src/compaction.rs",
            docs_reference: "docs/fsm/product-state-machines.md",
            emits_session_evidence: false,
            gates_side_effects: true,
            known_limitations: vec![
                "typed_final_state is machine authority",
                "final_status remains legacy/display compatibility",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "shared_state_lifecycle",
            human_name: "Shared State Lifecycle",
            authority_status: FsmAuthorityStatus::AuditedOnly,
            wired_command_surfaces: vec!["state", "state-review", "state snapshot"],
            source_module: "src/shared_state.rs; src/state_sql.rs",
            docs_reference: "docs/shared_state.md",
            emits_session_evidence: false,
            gates_side_effects: false,
            known_limitations: vec![
                "store validation exists",
                "fact lifecycle transitions are not centralized in a typed FSM",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "skill_execution",
            human_name: "Skill Execution",
            authority_status: FsmAuthorityStatus::Wired,
            wired_command_surfaces: vec!["skills run", "skills list", "skills show"],
            source_module: "src/fsm_core.rs; src/skills.rs",
            docs_reference: "docs/quant-m-skills.md",
            emits_session_evidence: true,
            gates_side_effects: true,
            known_limitations: vec![
                "SessionEvent::SkillCall.status remains display compatibility",
                "shell-backed execution still requires config and policy approval",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "worker_job",
            human_name: "Worker Job",
            authority_status: FsmAuthorityStatus::Wired,
            wired_command_surfaces: vec!["worker submit", "worker once", "worker run"],
            source_module: "src/fsm_core.rs; src/worker.rs",
            docs_reference: "docs/fsm/product-state-machines.md",
            emits_session_evidence: true,
            gates_side_effects: true,
            known_limitations: vec!["WorkerResult ok/error remains outbox compatibility"],
        },
        FsmAuthorityRecord {
            fsm_id: "worker_proposal",
            human_name: "Worker Proposal Review",
            authority_status: FsmAuthorityStatus::PartiallyWired,
            wired_command_surfaces: vec!["worker proposal submit", "worker proposal list"],
            source_module: "src/worker_proposals.rs",
            docs_reference: "docs/feature-map.md",
            emits_session_evidence: false,
            gates_side_effects: true,
            known_limitations: vec![
                "proposal status transitions are validated",
                "proposal acceptance workflow is intentionally narrow and non-authoritative by default",
            ],
        },
        FsmAuthorityRecord {
            fsm_id: "workflow_cursor",
            human_name: "Workflow Runtime Cursor",
            authority_status: FsmAuthorityStatus::DesignOnly,
            wired_command_surfaces: vec!["workflow list", "workflow show", "run workflow"],
            source_module: "src/workflow_registry.rs; src/execution_runtime.rs",
            docs_reference: "docs/fsm/product-state-machines.md",
            emits_session_evidence: true,
            gates_side_effects: false,
            known_limitations: vec![
                "workflow descriptors validate metadata",
                "runtime step cursor is not enforced as a typed FSM",
            ],
        },
    ]
}

pub fn render_authority_records(records: &[FsmAuthorityRecord]) -> String {
    let mut out = String::from("FSM authority\n");
    for record in records {
        out.push_str(&format!(
            "- {}: {} | session_evidence={} | gates_side_effects={}\n",
            record.fsm_id,
            record.authority_status,
            record.emits_session_evidence,
            record.gates_side_effects
        ));
        out.push_str(&format!("  source: {}\n", record.source_module));
        out.push_str(&format!(
            "  commands: {}\n",
            record.wired_command_surfaces.join(", ")
        ));
        out.push_str(&format!("  docs: {}\n", record.docs_reference));
        out.push_str(&format!(
            "  limitations: {}\n",
            record.known_limitations.join("; ")
        ));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(id: &str) -> FsmAuthorityRecord {
        authority_records()
            .into_iter()
            .find(|record| record.fsm_id == id)
            .expect("authority record")
    }

    #[test]
    fn authority_summary_marks_core_wired_fsms() {
        assert_eq!(
            record("worker_job").authority_status,
            FsmAuthorityStatus::Wired
        );
        assert_eq!(
            record("skill_execution").authority_status,
            FsmAuthorityStatus::Wired
        );
        assert_eq!(
            record("context_guardian").authority_status,
            FsmAuthorityStatus::Wired
        );
    }

    #[test]
    fn authority_summary_does_not_overclaim_partial_or_future_fsms() {
        assert_eq!(
            record("policy_approval").authority_status,
            FsmAuthorityStatus::PartiallyWired
        );
        assert_ne!(
            record("workflow_cursor").authority_status,
            FsmAuthorityStatus::Wired
        );
    }

    #[test]
    fn authority_summary_json_is_deterministic() {
        let first = serde_json::to_string_pretty(&authority_records()).expect("json");
        let second = serde_json::to_string_pretty(&authority_records()).expect("json");
        assert_eq!(first, second);
        assert!(first.contains("\"fsm_id\": \"worker_job\""));
    }
}
