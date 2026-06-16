use crate::domain;
use crate::sessions::DomainId;
use crate::skill_registry::{SideEffectLevel, SkillDescriptor};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PolicyDecision {
    Allow,
    RequireApproval,
    Deny,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PolicyDescriptor {
    pub policy_id: String,
    pub name: String,
    pub version: String,
    pub domain_id: DomainId,
    pub description: String,
    pub applies_to_side_effect_levels: Vec<SideEffectLevel>,
    pub required_operator_decision: bool,
    pub default_decision: PolicyDecision,
    pub policy_tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillPolicyEvaluation {
    pub skill_id: String,
    pub domain_id: DomainId,
    pub side_effect_level: SideEffectLevel,
    pub decision: PolicyDecision,
    pub matched_policy_ids: Vec<String>,
}

pub struct PolicyRegistry {
    policies: BTreeMap<String, PolicyDescriptor>,
}

impl PolicyRegistry {
    pub fn new() -> Self {
        Self {
            policies: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, descriptor: PolicyDescriptor) -> Result<()> {
        if self.policies.contains_key(&descriptor.policy_id) {
            return Err(anyhow!("duplicate policy id '{}'", descriptor.policy_id));
        }
        self.policies
            .insert(descriptor.policy_id.clone(), descriptor);
        Ok(())
    }

    pub fn list(
        &self,
        domain_id: Option<&DomainId>,
        side_effect_level: Option<&SideEffectLevel>,
    ) -> Vec<PolicyDescriptor> {
        self.policies
            .values()
            .filter(|policy| {
                domain_id.is_none_or(|value| &policy.domain_id == value)
                    && side_effect_level
                        .is_none_or(|value| policy.applies_to_side_effect_levels.contains(value))
            })
            .cloned()
            .collect()
    }

    pub fn show(&self, policy_id: &str) -> Result<PolicyDescriptor> {
        self.policies
            .get(policy_id)
            .cloned()
            .ok_or_else(|| anyhow!("policy '{}' not found", policy_id))
    }

    pub fn evaluate_skill(&self, skill: &SkillDescriptor) -> SkillPolicyEvaluation {
        let matches: Vec<PolicyDescriptor> = self
            .policies
            .values()
            .filter(|policy| {
                policy.domain_id == skill.domain_id
                    && policy
                        .applies_to_side_effect_levels
                        .contains(&skill.side_effect_level)
            })
            .cloned()
            .collect();
        let matched_policy_ids = matches
            .iter()
            .map(|policy| policy.policy_id.clone())
            .collect();

        let decision = if matches.iter().any(|policy| {
            policy.default_decision == PolicyDecision::Deny && !policy.required_operator_decision
        }) {
            PolicyDecision::Deny
        } else if matches.iter().any(|policy| {
            policy.default_decision == PolicyDecision::RequireApproval
                || policy.required_operator_decision
        }) {
            PolicyDecision::RequireApproval
        } else if matches
            .iter()
            .any(|policy| policy.default_decision == PolicyDecision::Allow)
        {
            PolicyDecision::Allow
        } else {
            PolicyDecision::RequireApproval
        };

        SkillPolicyEvaluation {
            skill_id: skill.skill_id.clone(),
            domain_id: skill.domain_id.clone(),
            side_effect_level: skill.side_effect_level.clone(),
            decision,
            matched_policy_ids,
        }
    }
}

impl Default for PolicyRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> Result<PolicyRegistry> {
    let domains = domain::builtin_registry()?;
    let mut registry = PolicyRegistry::new();
    for pack in domains.packs() {
        for descriptor in pack.register_policies() {
            registry.register(descriptor)?;
        }
    }
    Ok(registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{self, DomainPack};
    use crate::fsm_registry::{
        FsmDescriptor, FsmEventId, FsmId, FsmStateId, FsmTransitionDescriptor,
    };
    use crate::shared_state::SharedStateKey;
    use crate::workflow_registry::{WorkflowDescriptor, WorkflowId, WorkflowStepDescriptor};

    struct TestDomain;

    impl DomainPack for TestDomain {
        fn domain_id(&self) -> DomainId {
            DomainId::new("domain:test-policy-domain")
        }

        fn name(&self) -> &'static str {
            "Test Policy Domain"
        }

        fn version(&self) -> &'static str {
            "0.0.1"
        }

        fn capabilities(&self) -> Vec<domain::DomainCapability> {
            vec![domain::DomainCapability::Policies]
        }

        fn register_skills(&self) -> Vec<SkillDescriptor> {
            vec![SkillDescriptor {
                skill_id: "test-policy.inspect".to_string(),
                name: "Test Policy Inspect".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Inspect test policy domain".to_string(),
                input_schema_name: "TestPolicyInput".to_string(),
                output_schema_name: "TestPolicyOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<PolicyDescriptor> {
            vec![PolicyDescriptor {
                policy_id: "test-policy.read-only".to_string(),
                name: "Test Read Only Policy".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Allow read-only inspection".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: PolicyDecision::Allow,
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-policy-domain"),
                name: "Test Policy Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Workflow for testing policy registry integration.".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "inspect".to_string(),
                    name: "Inspect".to_string(),
                    skill_id: Some("test-policy.inspect".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-policy.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-policy.output")],
                    required_inputs: vec!["input".to_string()],
                    expected_outputs: vec!["output".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test policy workflow state.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<FsmDescriptor> {
            vec![FsmDescriptor {
                fsm_id: FsmId::new("fsm:test-policy-domain"),
                name: "Test Policy FSM".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "FSM for policy registry tests.".to_string(),
                initial_state: FsmStateId::new("state:policy-ready"),
                states: vec![
                    FsmStateId::new("state:policy-ready"),
                    FsmStateId::new("state:policy-complete"),
                ],
                events: vec![FsmEventId::new("event:inspect")],
                transitions: vec![FsmTransitionDescriptor {
                    transition_id: "transition:policy-inspect".to_string(),
                    from_state: FsmStateId::new("state:policy-ready"),
                    event: FsmEventId::new("event:inspect"),
                    to_state: FsmStateId::new("state:policy-complete"),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-policy.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-policy.output")],
                    workflow_id: Some(WorkflowId::new("workflow:test-policy-domain")),
                    guard_description: Some("input exists".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Run the policy test transition.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }
    }

    #[test]
    fn policies_register_without_changing_core() {
        let mut registry = PolicyRegistry::new();
        let descriptor = TestDomain.register_policies().remove(0);
        registry.register(descriptor).expect("register policy");
        let listed = registry.list(None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].policy_id, "test-policy.read-only");
    }

    #[test]
    fn duplicate_policy_ids_are_rejected() {
        let mut registry = PolicyRegistry::new();
        let descriptor = TestDomain.register_policies().remove(0);
        registry
            .register(descriptor.clone())
            .expect("register first policy");
        let err = registry
            .register(descriptor)
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate policy id"));
    }

    #[test]
    fn read_only_research_skill_evaluates_allow() {
        let policies = builtin_registry().expect("policy registry");
        let skills = crate::skill_registry::builtin_registry().expect("skill registry");
        let skill = skills
            .show("mock-research.capture-brief")
            .expect("show research skill");
        assert_eq!(
            policies.evaluate_skill(&skill).decision,
            PolicyDecision::Allow
        );
    }

    #[test]
    fn mock_trading_local_write_evaluates_require_approval() {
        let policies = builtin_registry().expect("policy registry");
        let skills = crate::skill_registry::builtin_registry().expect("skill registry");
        let skill = skills
            .show("mock-trading.prepare-paper-review")
            .expect("show trading skill");
        assert_eq!(
            policies.evaluate_skill(&skill).decision,
            PolicyDecision::RequireApproval
        );
    }

    #[test]
    fn trading_action_evaluates_deny() {
        let policies = builtin_registry().expect("policy registry");
        let skill = SkillDescriptor {
            skill_id: "mock-trading.live-order".to_string(),
            name: "Mock Trading Live Order".to_string(),
            version: "0.1.0".to_string(),
            domain_id: DomainId::new("domain:mock-trading"),
            description: "Hypothetical live trade".to_string(),
            input_schema_name: "LiveOrderInput".to_string(),
            output_schema_name: "LiveOrderOutput".to_string(),
            side_effect_level: SideEffectLevel::TradingAction,
            required_capabilities: vec!["external_adapters".to_string()],
            policy_tags: vec!["live".to_string()],
        };
        assert_eq!(
            policies.evaluate_skill(&skill).decision,
            PolicyDecision::Deny
        );
    }

    #[test]
    fn side_effect_filters_work() {
        let registry = builtin_registry().expect("policy registry");
        let readonly = registry.list(None, Some(&SideEffectLevel::ReadOnly));
        assert!(!readonly.is_empty());
        assert!(readonly.iter().all(|policy| {
            policy
                .applies_to_side_effect_levels
                .contains(&SideEffectLevel::ReadOnly)
        }));
    }
}
