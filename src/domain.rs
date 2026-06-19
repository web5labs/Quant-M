use crate::desk_registry::{DeskCategory, DeskId, DeskPackDescriptor, DeskStorageProfile};
use crate::fsm_registry::{FsmDescriptor, FsmEventId, FsmId, FsmStateId, FsmTransitionDescriptor};
use crate::policy_registry::{PolicyDecision, PolicyDescriptor};
use crate::scheduler_registry::{
    ScheduleCadenceDescriptor, ScheduleTriggerKind, SchedulerDescriptor, SchedulerId,
};
use crate::sessions::DomainId;
use crate::shared_state::SharedStateKey;
use crate::skill_registry::{SideEffectLevel, SkillDescriptor};
use crate::workflow_registry::{WorkflowDescriptor, WorkflowId, WorkflowStepDescriptor};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub trait DomainPack {
    fn domain_id(&self) -> DomainId;
    fn name(&self) -> &'static str;
    fn version(&self) -> &'static str;
    fn capabilities(&self) -> Vec<DomainCapability>;
    fn register_skills(&self) -> Vec<SkillDescriptor>;
    fn register_policies(&self) -> Vec<PolicyDescriptor>;
    fn register_workflows(&self) -> Vec<WorkflowDescriptor>;
    fn register_fsms(&self) -> Vec<FsmDescriptor>;
    fn register_schedulers(&self) -> Vec<SchedulerDescriptor> {
        vec![]
    }
    fn register_desks(&self) -> Vec<DeskPackDescriptor> {
        vec![]
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DomainCapability {
    Observations,
    Skills,
    Policies,
    Workflows,
    Schedulers,
    Reports,
    ExternalAdapters,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DomainPackInfo {
    pub domain_id: DomainId,
    pub name: String,
    pub version: String,
    pub capabilities: Vec<DomainCapability>,
    pub skills: Vec<String>,
    pub policies: Vec<String>,
    pub workflows: Vec<String>,
    pub fsms: Vec<String>,
    pub schedulers: Vec<String>,
    pub desks: Vec<String>,
}

pub struct DomainRegistry {
    packs: BTreeMap<DomainId, Box<dyn DomainPack>>,
}

impl DomainRegistry {
    pub fn new() -> Self {
        Self {
            packs: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, pack: Box<dyn DomainPack>) -> Result<()> {
        let domain_id = pack.domain_id();
        if self.packs.contains_key(&domain_id) {
            return Err(anyhow!("duplicate domain id '{}'", domain_id));
        }
        self.packs.insert(domain_id, pack);
        Ok(())
    }

    pub fn list(&self) -> Vec<DomainPackInfo> {
        self.packs
            .values()
            .map(|pack| pack_info(pack.as_ref()))
            .collect()
    }

    pub fn show(&self, domain_id: &DomainId) -> Result<DomainPackInfo> {
        let pack = self
            .packs
            .get(domain_id)
            .ok_or_else(|| anyhow!("domain '{}' not found", domain_id))?;
        Ok(pack_info(pack.as_ref()))
    }

    pub fn packs(&self) -> Vec<&dyn DomainPack> {
        self.packs.values().map(|pack| pack.as_ref()).collect()
    }
}

impl Default for DomainRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> Result<DomainRegistry> {
    let mut registry = DomainRegistry::new();
    registry.register(Box::new(MockResearchDomain))?;
    registry.register(Box::new(MockTradingDomain))?;
    Ok(registry)
}

fn pack_info(pack: &dyn DomainPack) -> DomainPackInfo {
    let mut capabilities = pack.capabilities();
    capabilities.sort();
    capabilities.dedup();

    DomainPackInfo {
        domain_id: pack.domain_id(),
        name: pack.name().to_string(),
        version: pack.version().to_string(),
        capabilities,
        skills: pack
            .register_skills()
            .into_iter()
            .map(|descriptor| descriptor.skill_id)
            .collect(),
        policies: pack
            .register_policies()
            .into_iter()
            .map(|descriptor| descriptor.policy_id)
            .collect(),
        workflows: pack
            .register_workflows()
            .into_iter()
            .map(|descriptor| descriptor.workflow_id.to_string())
            .collect(),
        fsms: pack
            .register_fsms()
            .into_iter()
            .map(|descriptor| descriptor.fsm_id.to_string())
            .collect(),
        schedulers: pack
            .register_schedulers()
            .into_iter()
            .map(|descriptor| descriptor.scheduler_id.to_string())
            .collect(),
        desks: pack
            .register_desks()
            .into_iter()
            .map(|descriptor| descriptor.desk_id.to_string())
            .collect(),
    }
}

struct MockResearchDomain;

impl DomainPack for MockResearchDomain {
    fn domain_id(&self) -> DomainId {
        DomainId::new("domain:mock-research")
    }

    fn name(&self) -> &'static str {
        "Mock Research"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn capabilities(&self) -> Vec<DomainCapability> {
        vec![
            DomainCapability::Observations,
            DomainCapability::Skills,
            DomainCapability::Policies,
            DomainCapability::Workflows,
            DomainCapability::Schedulers,
            DomainCapability::Reports,
        ]
    }

    fn register_skills(&self) -> Vec<SkillDescriptor> {
        vec![SkillDescriptor {
            skill_id: "mock-research.capture-brief".to_string(),
            name: "Capture Research Brief".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description: "Status: mock. Capture a research brief without mutating external state."
                .to_string(),
            input_schema_name: "ResearchBriefInput".to_string(),
            output_schema_name: "ResearchBriefOutput".to_string(),
            side_effect_level: SideEffectLevel::ReadOnly,
            required_capabilities: vec!["observations".to_string(), "reports".to_string()],
            policy_tags: vec![
                "read_only".to_string(),
                "research_sources_required".to_string(),
            ],
        }]
    }

    fn register_policies(&self) -> Vec<PolicyDescriptor> {
        vec![PolicyDescriptor {
            policy_id: "mock-research.read-only".to_string(),
            name: "Mock Research Read Only".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description: "Allow read-only research inspection skills.".to_string(),
            applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
            required_operator_decision: false,
            default_decision: PolicyDecision::Allow,
            policy_tags: vec![
                "research_sources_required".to_string(),
                "analysis_is_read_only".to_string(),
            ],
        }]
    }

    fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
        vec![WorkflowDescriptor {
            workflow_id: WorkflowId::new("workflow:mock-research-brief"),
            name: "Mock Research Brief Review".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description:
                "Status: mock. Review a research brief by reading current context and producing a draft summary."
                    .to_string(),
            steps: vec![WorkflowStepDescriptor {
                step_id: "capture-brief".to_string(),
                name: "Capture Brief".to_string(),
                skill_id: Some("mock-research.capture-brief".to_string()),
                reads_state_keys: vec![
                    SharedStateKey::new("shared.research.brief"),
                    SharedStateKey::new("shared.research.sources"),
                ],
                writes_state_keys: vec![SharedStateKey::new("shared.research.summary")],
                required_inputs: vec!["brief_id".to_string(), "source_set".to_string()],
                expected_outputs: vec!["summary".to_string(), "draft_report".to_string()],
                side_effect_level: SideEffectLevel::ReadOnly,
                description: "Read current research context and produce a read-only summary draft."
                    .to_string(),
            }],
            tags: vec!["read_only".to_string(), "research".to_string()],
        }]
    }

    fn register_fsms(&self) -> Vec<FsmDescriptor> {
        vec![FsmDescriptor {
            fsm_id: FsmId::new("fsm:mock-research-brief"),
            name: "Mock Research Brief FSM".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description:
                "Status: mock. Describe deterministic research brief transitions without executing them."
                    .to_string(),
            initial_state: FsmStateId::new("state:brief_received"),
            states: vec![
                FsmStateId::new("state:brief_received"),
                FsmStateId::new("state:summary_drafted"),
            ],
            events: vec![FsmEventId::new("event:capture_brief")],
            transitions: vec![FsmTransitionDescriptor {
                transition_id: "transition:brief-to-summary".to_string(),
                from_state: FsmStateId::new("state:brief_received"),
                event: FsmEventId::new("event:capture_brief"),
                to_state: FsmStateId::new("state:summary_drafted"),
                reads_state_keys: vec![
                    SharedStateKey::new("shared.research.brief"),
                    SharedStateKey::new("shared.research.sources"),
                ],
                writes_state_keys: vec![SharedStateKey::new("shared.research.summary")],
                workflow_id: Some(WorkflowId::new("workflow:mock-research-brief")),
                guard_description: Some("research brief and sources are present".to_string()),
                side_effect_level: SideEffectLevel::ReadOnly,
                description: "Capture the current research brief and move to drafted summary."
                    .to_string(),
            }],
            tags: vec!["read_only".to_string(), "research".to_string()],
        }]
    }

    fn register_schedulers(&self) -> Vec<SchedulerDescriptor> {
        vec![SchedulerDescriptor {
            scheduler_id: SchedulerId::new("scheduler:mock-research-brief"),
            name: "Mock Research Brief Scheduler".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description:
                "Status: mock. Describe a cron-style research brief cadence without executing it."
                    .to_string(),
            cadence: ScheduleCadenceDescriptor {
                trigger_kind: ScheduleTriggerKind::Cron,
                cron_expr: Some("0 */6 * * *".to_string()),
                polling_interval_ms: None,
                mtime_path: None,
                event_name: None,
                jitter_ms: Some(30_000),
                max_runs: None,
                enabled: true,
            },
            workflow_id: Some(WorkflowId::new("workflow:mock-research-brief")),
            fsm_id: Some(FsmId::new("fsm:mock-research-brief")),
            reads_state_keys: vec![
                SharedStateKey::new("shared.research.brief"),
                SharedStateKey::new("shared.research.sources"),
            ],
            writes_state_keys: vec![SharedStateKey::new("shared.research.summary")],
            tags: vec!["research".to_string(), "read_only".to_string()],
        }]
    }

    fn register_desks(&self) -> Vec<DeskPackDescriptor> {
        vec![DeskPackDescriptor {
            desk_id: DeskId::new("desk:mock-research-brief"),
            name: "Mock Research Brief Desk".to_string(),
            version: "0.1.0".to_string(),
            category: DeskCategory::Research,
            domain_id: self.domain_id(),
            description:
                "Status: mock. Package the research brief flow as a reusable desk-shaped use case."
                    .to_string(),
            skill_ids: vec!["mock-research.capture-brief".to_string()],
            workflow_ids: vec![WorkflowId::new("workflow:mock-research-brief")],
            fsm_ids: vec![FsmId::new("fsm:mock-research-brief")],
            scheduler_ids: vec![SchedulerId::new("scheduler:mock-research-brief")],
            reads_state_keys: vec![
                SharedStateKey::new("shared.research.brief"),
                SharedStateKey::new("shared.research.sources"),
            ],
            writes_state_keys: vec![SharedStateKey::new("shared.research.summary")],
            storage_profile: DeskStorageProfile {
                hot_state_required: true,
                durable_history_required: true,
                external_adapters_required: false,
                paper_only: false,
                notes: "Research desk pack stays local-first and metadata-only.".to_string(),
            },
            tags: vec!["research".to_string(), "read_only".to_string()],
        }]
    }
}

struct MockTradingDomain;

impl DomainPack for MockTradingDomain {
    fn domain_id(&self) -> DomainId {
        DomainId::new("domain:mock-trading")
    }

    fn name(&self) -> &'static str {
        "Mock Trading"
    }

    fn version(&self) -> &'static str {
        "0.1.0"
    }

    fn capabilities(&self) -> Vec<DomainCapability> {
        vec![
            DomainCapability::Observations,
            DomainCapability::Skills,
            DomainCapability::Policies,
            DomainCapability::Workflows,
            DomainCapability::Schedulers,
            DomainCapability::Reports,
        ]
    }

    fn register_skills(&self) -> Vec<SkillDescriptor> {
        vec![
            SkillDescriptor {
                skill_id: "mock-trading.score-handoff".to_string(),
                name: "Score Mock Handoff".to_string(),
                version: "0.1.0".to_string(),
                domain_id: self.domain_id(),
                description: "Status: mock. Score a paper-trade handoff without touching live venues, brokers, exchanges, or live orders."
                    .to_string(),
                input_schema_name: "MockHandoffInput".to_string(),
                output_schema_name: "MockScoreOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string(), "reports".to_string()],
                policy_tags: vec!["paper_trade_only".to_string()],
            },
            SkillDescriptor {
                skill_id: "mock-trading.prepare-paper-review".to_string(),
                name: "Prepare Paper Trade Review".to_string(),
                version: "0.1.0".to_string(),
                domain_id: self.domain_id(),
                description: "Status: mock/guarded. Write local paper-trade review artifacts without broker/exchange integration or live execution."
                    .to_string(),
                input_schema_name: "PaperTradeReviewInput".to_string(),
                output_schema_name: "PaperTradeReviewOutput".to_string(),
                side_effect_level: SideEffectLevel::LocalWrite,
                required_capabilities: vec!["workflows".to_string(), "reports".to_string()],
                policy_tags: vec![
                    "paper_trade_only".to_string(),
                    "operator_approval_required".to_string(),
                ],
            },
        ]
    }

    fn register_policies(&self) -> Vec<PolicyDescriptor> {
        vec![
            PolicyDescriptor {
                policy_id: "mock-trading.read-only".to_string(),
                name: "Mock Trading Read Only".to_string(),
                version: "0.1.0".to_string(),
                domain_id: self.domain_id(),
                description: "Allow read-only paper-trade inspection skills.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: PolicyDecision::Allow,
                policy_tags: vec!["paper_trade_only".to_string()],
            },
            PolicyDescriptor {
                policy_id: "mock-trading.local-write".to_string(),
                name: "Mock Trading Local Write Approval".to_string(),
                version: "0.1.0".to_string(),
                domain_id: self.domain_id(),
                description: "Require operator approval for local paper-trade artifacts."
                    .to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::LocalWrite],
                required_operator_decision: true,
                default_decision: PolicyDecision::RequireApproval,
                policy_tags: vec![
                    "paper_trade_only".to_string(),
                    "operator_approval_required".to_string(),
                ],
            },
            PolicyDescriptor {
                policy_id: "mock-trading.trading-action-deny".to_string(),
                name: "Mock Trading Deny Live Trading".to_string(),
                version: "0.1.0".to_string(),
                domain_id: self.domain_id(),
                description: "Deny any live trading action in the mock trading domain.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::TradingAction],
                required_operator_decision: false,
                default_decision: PolicyDecision::Deny,
                policy_tags: vec![
                    "paper_trade_only".to_string(),
                    "deny_live_trading".to_string(),
                ],
            },
        ]
    }

    fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
        vec![WorkflowDescriptor {
            workflow_id: WorkflowId::new("workflow:mock-trading-paper-review"),
            name: "Mock Trading Paper Review".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description:
                "Status: mock. Review a paper-trade candidate without placing live orders."
                    .to_string(),
            steps: vec![
                WorkflowStepDescriptor {
                    step_id: "score-handoff".to_string(),
                    name: "Score Handoff".to_string(),
                    skill_id: Some("mock-trading.score-handoff".to_string()),
                    reads_state_keys: vec![
                        SharedStateKey::new("shared.trading.handoff"),
                        SharedStateKey::new("shared.trading.context"),
                    ],
                    writes_state_keys: vec![SharedStateKey::new("shared.trading.score")],
                    required_inputs: vec!["handoff_id".to_string()],
                    expected_outputs: vec!["score".to_string(), "review_notes".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Read a paper-trade handoff and produce a score without external execution.".to_string(),
                },
                WorkflowStepDescriptor {
                    step_id: "prepare-paper-review".to_string(),
                    name: "Prepare Paper Review".to_string(),
                    skill_id: Some("mock-trading.prepare-paper-review".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.trading.score")],
                    writes_state_keys: vec![SharedStateKey::new("shared.trading.paper_review")],
                    required_inputs: vec!["review_template".to_string()],
                    expected_outputs: vec!["paper_review".to_string()],
                    side_effect_level: SideEffectLevel::LocalWrite,
                    description: "Prepare local paper-trade review artifacts only; no live trading surface is exposed.".to_string(),
                },
            ],
            tags: vec!["paper_trade_only".to_string(), "mock_trading".to_string()],
        }]
    }

    fn register_fsms(&self) -> Vec<FsmDescriptor> {
        vec![FsmDescriptor {
            fsm_id: FsmId::new("fsm:mock-trading-paper-review"),
            name: "Mock Trading Paper Review FSM".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description: "Status: mock. Describe a paper-only trading review path without broker/exchange integration or live execution."
                .to_string(),
            initial_state: FsmStateId::new("state:handoff_received"),
            states: vec![
                FsmStateId::new("state:handoff_received"),
                FsmStateId::new("state:scored"),
                FsmStateId::new("state:paper_review_ready"),
            ],
            events: vec![
                FsmEventId::new("event:score_handoff"),
                FsmEventId::new("event:prepare_review"),
            ],
            transitions: vec![
                FsmTransitionDescriptor {
                    transition_id: "transition:handoff-to-scored".to_string(),
                    from_state: FsmStateId::new("state:handoff_received"),
                    event: FsmEventId::new("event:score_handoff"),
                    to_state: FsmStateId::new("state:scored"),
                    reads_state_keys: vec![
                        SharedStateKey::new("shared.trading.handoff"),
                        SharedStateKey::new("shared.trading.context"),
                    ],
                    writes_state_keys: vec![SharedStateKey::new("shared.trading.score")],
                    workflow_id: Some(WorkflowId::new("workflow:mock-trading-paper-review")),
                    guard_description: Some("paper-trade handoff is available".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Score the paper-trade handoff and keep the runtime in read-only mode."
                        .to_string(),
                },
                FsmTransitionDescriptor {
                    transition_id: "transition:scored-to-paper-review".to_string(),
                    from_state: FsmStateId::new("state:scored"),
                    event: FsmEventId::new("event:prepare_review"),
                    to_state: FsmStateId::new("state:paper_review_ready"),
                    reads_state_keys: vec![SharedStateKey::new("shared.trading.score")],
                    writes_state_keys: vec![SharedStateKey::new("shared.trading.paper_review")],
                    workflow_id: Some(WorkflowId::new("workflow:mock-trading-paper-review")),
                    guard_description: Some("paper review template is available".to_string()),
                    side_effect_level: SideEffectLevel::LocalWrite,
                    description: "Prepare local paper-review artifacts only; no live trading transition exists."
                        .to_string(),
                },
            ],
            tags: vec!["paper_trade_only".to_string(), "mock_trading".to_string()],
        }]
    }

    fn register_schedulers(&self) -> Vec<SchedulerDescriptor> {
        vec![SchedulerDescriptor {
            scheduler_id: SchedulerId::new("scheduler:mock-trading-paper-review"),
            name: "Mock Trading Paper Review Scheduler".to_string(),
            version: "0.1.0".to_string(),
            domain_id: self.domain_id(),
            description:
                "Status: mock. Describe a paper-only polling cadence for trading review prep."
                    .to_string(),
            cadence: ScheduleCadenceDescriptor {
                trigger_kind: ScheduleTriggerKind::Polling,
                cron_expr: None,
                polling_interval_ms: Some(30_000),
                mtime_path: None,
                event_name: None,
                jitter_ms: Some(5_000),
                max_runs: None,
                enabled: true,
            },
            workflow_id: Some(WorkflowId::new("workflow:mock-trading-paper-review")),
            fsm_id: Some(FsmId::new("fsm:mock-trading-paper-review")),
            reads_state_keys: vec![SharedStateKey::new("shared.trading.score")],
            writes_state_keys: vec![SharedStateKey::new("shared.trading.paper_review")],
            tags: vec!["paper_trade_only".to_string(), "mock_trading".to_string()],
        }]
    }

    fn register_desks(&self) -> Vec<DeskPackDescriptor> {
        vec![DeskPackDescriptor {
            desk_id: DeskId::new("desk:mock-trading-paper"),
            name: "Mock Trading Paper Desk".to_string(),
            version: "0.1.0".to_string(),
            category: DeskCategory::Forex,
            domain_id: self.domain_id(),
            description: "Status: mock. Package a paper-only trading review desk without broker/exchange integration, live orders, or live trading."
                .to_string(),
            skill_ids: vec![
                "mock-trading.score-handoff".to_string(),
                "mock-trading.prepare-paper-review".to_string(),
            ],
            workflow_ids: vec![WorkflowId::new("workflow:mock-trading-paper-review")],
            fsm_ids: vec![FsmId::new("fsm:mock-trading-paper-review")],
            scheduler_ids: vec![SchedulerId::new("scheduler:mock-trading-paper-review")],
            reads_state_keys: vec![
                SharedStateKey::new("shared.trading.handoff"),
                SharedStateKey::new("shared.trading.context"),
                SharedStateKey::new("shared.trading.score"),
            ],
            writes_state_keys: vec![
                SharedStateKey::new("shared.trading.score"),
                SharedStateKey::new("shared.trading.paper_review"),
            ],
            storage_profile: DeskStorageProfile {
                hot_state_required: true,
                durable_history_required: true,
                external_adapters_required: false,
                paper_only: true,
                notes: "Paper-only desk pack for review and handoff prep; no live adapters."
                    .to_string(),
            },
            tags: vec!["paper_trade_only".to_string(), "mock_trading".to_string()],
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::sessions::{self, AgentId, SessionContext, SessionEvent};
    use tempfile::TempDir;

    struct TestDomain {
        id: DomainId,
    }

    impl DomainPack for TestDomain {
        fn domain_id(&self) -> DomainId {
            self.id.clone()
        }

        fn name(&self) -> &'static str {
            "Test Domain"
        }

        fn version(&self) -> &'static str {
            "0.0.1"
        }

        fn capabilities(&self) -> Vec<DomainCapability> {
            vec![DomainCapability::Observations, DomainCapability::Policies]
        }

        fn register_skills(&self) -> Vec<SkillDescriptor> {
            vec![SkillDescriptor {
                skill_id: "test-domain.inspect".to_string(),
                name: "Test Domain Inspect".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Inspect test-domain state".to_string(),
                input_schema_name: "TestInput".to_string(),
                output_schema_name: "TestOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["test_policy".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<PolicyDescriptor> {
            vec![PolicyDescriptor {
                policy_id: "test-domain.read-only".to_string(),
                name: "Test Domain Read Only".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Allow test-domain read-only inspection.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: PolicyDecision::Allow,
                policy_tags: vec!["test_policy".to_string()],
            }]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-domain"),
                name: "Test Domain Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Inspect test-domain state.".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "inspect".to_string(),
                    name: "Inspect".to_string(),
                    skill_id: Some("test-domain.inspect".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-domain.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-domain.output")],
                    required_inputs: vec!["input".to_string()],
                    expected_outputs: vec!["output".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test-domain state.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<FsmDescriptor> {
            vec![FsmDescriptor {
                fsm_id: FsmId::new("fsm:test-domain"),
                name: "Test Domain FSM".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Inspect test-domain state transitions.".to_string(),
                initial_state: FsmStateId::new("state:test-start"),
                states: vec![
                    FsmStateId::new("state:test-start"),
                    FsmStateId::new("state:test-finished"),
                ],
                events: vec![FsmEventId::new("event:test-inspect")],
                transitions: vec![FsmTransitionDescriptor {
                    transition_id: "transition:test-inspect".to_string(),
                    from_state: FsmStateId::new("state:test-start"),
                    event: FsmEventId::new("event:test-inspect"),
                    to_state: FsmStateId::new("state:test-finished"),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-domain.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-domain.output")],
                    workflow_id: Some(WorkflowId::new("workflow:test-domain")),
                    guard_description: Some("test input exists".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test-domain state and mark the fsm complete.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }
    }

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        (tmp, cfg)
    }

    #[test]
    fn domains_register_without_touching_core() {
        let mut registry = DomainRegistry::new();
        registry
            .register(Box::new(TestDomain {
                id: DomainId::new("domain:test-domain"),
            }))
            .expect("register domain");

        let listed = registry.list();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].domain_id.as_str(), "domain:test-domain");
        assert_eq!(listed[0].name, "Test Domain");
    }

    #[test]
    fn duplicate_domain_ids_are_rejected() {
        let mut registry = DomainRegistry::new();
        registry
            .register(Box::new(TestDomain {
                id: DomainId::new("domain:duplicate"),
            }))
            .expect("register first");
        let err = registry
            .register(Box::new(TestDomain {
                id: DomainId::new("domain:duplicate"),
            }))
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate domain id"));
    }

    #[test]
    fn mock_trading_domain_does_not_enable_live_trading() {
        let registry = builtin_registry().expect("builtin registry");
        let detail = registry
            .show(&DomainId::new("domain:mock-trading"))
            .expect("show mock trading");
        assert!(
            detail
                .policies
                .iter()
                .any(|value| value == "mock-trading.trading-action-deny")
        );
        assert!(
            detail
                .desks
                .iter()
                .any(|value| value == "desk:mock-trading-paper")
        );
        assert!(
            !detail
                .capabilities
                .contains(&DomainCapability::ExternalAdapters)
        );
    }

    #[test]
    fn domain_capabilities_are_inspectable() {
        let registry = builtin_registry().expect("builtin registry");
        let detail = registry
            .show(&DomainId::new("domain:mock-research"))
            .expect("show mock research");
        assert!(
            detail
                .capabilities
                .contains(&DomainCapability::Observations)
        );
        assert!(detail.capabilities.contains(&DomainCapability::Skills));
        assert!(
            detail
                .skills
                .iter()
                .any(|value| value == "mock-research.capture-brief")
        );
        assert!(
            detail
                .fsms
                .iter()
                .any(|value| value == "fsm:mock-research-brief")
        );
        assert!(
            detail
                .desks
                .iter()
                .any(|value| value == "desk:mock-research-brief")
        );
    }

    #[test]
    fn session_events_can_reference_registered_domain_ids() {
        let (_tmp, cfg) = temp_cfg();
        let registry = builtin_registry().expect("builtin registry");
        let domain = registry
            .show(&DomainId::new("domain:mock-research"))
            .expect("show domain");
        let context = SessionContext::new(AgentId::new("agent:test"), domain.domain_id.clone());

        sessions::append_event(
            &cfg,
            &context,
            SessionEvent::Observation {
                message: "domain-bound event".to_string(),
                job_id: None,
                detail: Some("registered domain".to_string()),
            },
        )
        .expect("append event");

        let detail = sessions::show_session(&cfg, &context.session_id).expect("show session");
        assert_eq!(detail.summary.domain_id, domain.domain_id);
    }
}
