use crate::domain;
use crate::sessions::DomainId;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum SideEffectLevel {
    ReadOnly,
    LocalWrite,
    NetworkRead,
    NetworkWrite,
    ExternalAction,
    TradingAction,
}

impl fmt::Display for SideEffectLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            SideEffectLevel::ReadOnly => "read_only",
            SideEffectLevel::LocalWrite => "local_write",
            SideEffectLevel::NetworkRead => "network_read",
            SideEffectLevel::NetworkWrite => "network_write",
            SideEffectLevel::ExternalAction => "external_action",
            SideEffectLevel::TradingAction => "trading_action",
        };
        value.fmt(f)
    }
}

impl FromStr for SideEffectLevel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "read_only" | "readonly" => Ok(Self::ReadOnly),
            "local_write" | "localwrite" => Ok(Self::LocalWrite),
            "network_read" | "networkread" => Ok(Self::NetworkRead),
            "network_write" | "networkwrite" => Ok(Self::NetworkWrite),
            "external_action" | "externalaction" => Ok(Self::ExternalAction),
            "trading_action" | "tradingaction" => Ok(Self::TradingAction),
            other => Err(anyhow!("unknown side effect level '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillDescriptor {
    pub skill_id: String,
    pub name: String,
    pub version: String,
    pub domain_id: DomainId,
    pub description: String,
    pub input_schema_name: String,
    pub output_schema_name: String,
    pub side_effect_level: SideEffectLevel,
    pub required_capabilities: Vec<String>,
    pub policy_tags: Vec<String>,
}

pub struct SkillRegistry {
    skills: BTreeMap<String, SkillDescriptor>,
}

impl SkillRegistry {
    pub fn new() -> Self {
        Self {
            skills: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, descriptor: SkillDescriptor) -> Result<()> {
        if self.skills.contains_key(&descriptor.skill_id) {
            return Err(anyhow!("duplicate skill id '{}'", descriptor.skill_id));
        }
        self.skills.insert(descriptor.skill_id.clone(), descriptor);
        Ok(())
    }

    pub fn list(
        &self,
        domain_id: Option<&DomainId>,
        side_effect_level: Option<&SideEffectLevel>,
    ) -> Vec<SkillDescriptor> {
        self.skills
            .values()
            .filter(|skill| {
                domain_id.is_none_or(|value| &skill.domain_id == value)
                    && side_effect_level.is_none_or(|value| &skill.side_effect_level == value)
            })
            .cloned()
            .collect()
    }

    pub fn show(&self, skill_id: &str) -> Result<SkillDescriptor> {
        self.skills
            .get(skill_id)
            .cloned()
            .ok_or_else(|| anyhow!("skill '{}' not found", skill_id))
    }
}

impl Default for SkillRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> Result<SkillRegistry> {
    let domains = domain::builtin_registry()?;
    let mut registry = SkillRegistry::new();
    for pack in domains.packs() {
        for descriptor in pack.register_skills() {
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
            DomainId::new("domain:test-skill-domain")
        }

        fn name(&self) -> &'static str {
            "Test Skill Domain"
        }

        fn version(&self) -> &'static str {
            "0.0.1"
        }

        fn capabilities(&self) -> Vec<domain::DomainCapability> {
            vec![domain::DomainCapability::Skills]
        }

        fn register_skills(&self) -> Vec<SkillDescriptor> {
            vec![SkillDescriptor {
                skill_id: "test.inspect".to_string(),
                name: "Test Inspect".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Read-only test skill".to_string(),
                input_schema_name: "TestInspectInput".to_string(),
                output_schema_name: "TestInspectOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["read_only".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<crate::policy_registry::PolicyDescriptor> {
            vec![crate::policy_registry::PolicyDescriptor {
                policy_id: "test-skill.read-only".to_string(),
                name: "Test Skill Read Only".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Allow the test skill.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: crate::policy_registry::PolicyDecision::Allow,
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-skill-domain"),
                name: "Test Skill Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Workflow for testing skill registry integration.".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "inspect".to_string(),
                    name: "Inspect".to_string(),
                    skill_id: Some("test.inspect".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-skill.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-skill.output")],
                    required_inputs: vec!["input".to_string()],
                    expected_outputs: vec!["output".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test workflow state.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<FsmDescriptor> {
            vec![FsmDescriptor {
                fsm_id: FsmId::new("fsm:test-skill-domain"),
                name: "Test Skill FSM".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "FSM for skill registry tests.".to_string(),
                initial_state: FsmStateId::new("state:input_ready"),
                states: vec![
                    FsmStateId::new("state:input_ready"),
                    FsmStateId::new("state:inspected"),
                ],
                events: vec![FsmEventId::new("event:inspect")],
                transitions: vec![FsmTransitionDescriptor {
                    transition_id: "transition:inspect".to_string(),
                    from_state: FsmStateId::new("state:input_ready"),
                    event: FsmEventId::new("event:inspect"),
                    to_state: FsmStateId::new("state:inspected"),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-skill.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-skill.output")],
                    workflow_id: Some(WorkflowId::new("workflow:test-skill-domain")),
                    guard_description: Some("input exists".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Run the test inspect transition.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }
    }

    #[test]
    fn skills_register_without_changing_core() {
        let mut registry = SkillRegistry::new();
        let descriptor = TestDomain.register_skills().remove(0);
        registry.register(descriptor).expect("register skill");

        let listed = registry.list(None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].skill_id, "test.inspect");
    }

    #[test]
    fn duplicate_skill_ids_are_rejected() {
        let mut registry = SkillRegistry::new();
        let descriptor = TestDomain.register_skills().remove(0);
        registry
            .register(descriptor.clone())
            .expect("register first");
        let err = registry
            .register(descriptor)
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate skill id"));
    }

    #[test]
    fn mock_trading_skills_do_not_expose_trading_action() {
        let registry = builtin_registry().expect("registry");
        let skills = registry.list(Some(&DomainId::new("domain:mock-trading")), None);
        assert!(!skills.is_empty());
        assert!(
            !skills
                .iter()
                .any(|skill| skill.side_effect_level == SideEffectLevel::TradingAction)
        );
    }

    #[test]
    fn side_effect_filters_work() {
        let registry = builtin_registry().expect("registry");
        let readonly = registry.list(None, Some(&SideEffectLevel::ReadOnly));
        assert!(!readonly.is_empty());
        assert!(
            readonly
                .iter()
                .all(|skill| skill.side_effect_level == SideEffectLevel::ReadOnly)
        );
    }
}
