use crate::domain;
use crate::sessions::DomainId;
use crate::shared_state::SharedStateKey;
use crate::skill_registry::SideEffectLevel;
use crate::workflow_registry::{WorkflowId, WorkflowRegistry};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

macro_rules! typed_id {
    ($name:ident, $label:literal) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            #[allow(dead_code)]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = anyhow::Error;

            fn from_str(value: &str) -> Result<Self> {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(anyhow!(concat!($label, " is empty")));
                }
                Ok(Self::new(trimmed))
            }
        }
    };
}

typed_id!(FsmId, "FsmId");
typed_id!(FsmStateId, "FsmStateId");
typed_id!(FsmEventId, "FsmEventId");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmTransitionDescriptor {
    pub transition_id: String,
    pub from_state: FsmStateId,
    pub event: FsmEventId,
    pub to_state: FsmStateId,
    pub reads_state_keys: Vec<SharedStateKey>,
    pub writes_state_keys: Vec<SharedStateKey>,
    pub workflow_id: Option<WorkflowId>,
    pub guard_description: Option<String>,
    pub side_effect_level: SideEffectLevel,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmDescriptor {
    pub fsm_id: FsmId,
    pub name: String,
    pub version: String,
    pub domain_id: DomainId,
    pub description: String,
    pub initial_state: FsmStateId,
    pub states: Vec<FsmStateId>,
    pub events: Vec<FsmEventId>,
    pub transitions: Vec<FsmTransitionDescriptor>,
    pub tags: Vec<String>,
}

pub struct FsmRegistry {
    fsms: BTreeMap<FsmId, FsmDescriptor>,
    known_workflows: BTreeSet<WorkflowId>,
}

impl FsmRegistry {
    pub fn new() -> Self {
        Self {
            fsms: BTreeMap::new(),
            known_workflows: BTreeSet::new(),
        }
    }

    pub fn with_workflows(workflow_registry: &WorkflowRegistry) -> Self {
        let known_workflows = workflow_registry
            .list(None)
            .into_iter()
            .map(|workflow| workflow.workflow_id)
            .collect();
        Self {
            fsms: BTreeMap::new(),
            known_workflows,
        }
    }

    pub fn register(&mut self, descriptor: FsmDescriptor) -> Result<()> {
        if self.fsms.contains_key(&descriptor.fsm_id) {
            return Err(anyhow!("duplicate fsm id '{}'", descriptor.fsm_id));
        }
        validate_descriptor(&descriptor, &self.known_workflows)?;
        self.fsms.insert(descriptor.fsm_id.clone(), descriptor);
        Ok(())
    }

    pub fn list(&self, domain_id: Option<&DomainId>) -> Vec<FsmDescriptor> {
        self.fsms
            .values()
            .filter(|fsm| domain_id.is_none_or(|value| &fsm.domain_id == value))
            .cloned()
            .collect()
    }

    pub fn show(&self, fsm_id: &FsmId) -> Result<FsmDescriptor> {
        self.fsms
            .get(fsm_id)
            .cloned()
            .ok_or_else(|| anyhow!("fsm '{}' not found", fsm_id))
    }
}

impl Default for FsmRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> Result<FsmRegistry> {
    let workflows = crate::workflow_registry::builtin_registry()?;
    let domains = domain::builtin_registry()?;
    let mut registry = FsmRegistry::with_workflows(&workflows);
    for pack in domains.packs() {
        for descriptor in pack.register_fsms() {
            registry.register(descriptor)?;
        }
    }
    Ok(registry)
}

fn validate_descriptor(
    descriptor: &FsmDescriptor,
    known_workflows: &BTreeSet<WorkflowId>,
) -> Result<()> {
    let states: BTreeSet<FsmStateId> = descriptor.states.iter().cloned().collect();
    if !states.contains(&descriptor.initial_state) {
        return Err(anyhow!(
            "fsm '{}' initial_state '{}' is not declared in states",
            descriptor.fsm_id,
            descriptor.initial_state
        ));
    }

    let events: BTreeSet<FsmEventId> = descriptor.events.iter().cloned().collect();
    for transition in &descriptor.transitions {
        if !states.contains(&transition.from_state) {
            return Err(anyhow!(
                "fsm '{}' transition '{}' references unknown from_state '{}'",
                descriptor.fsm_id,
                transition.transition_id,
                transition.from_state
            ));
        }
        if !states.contains(&transition.to_state) {
            return Err(anyhow!(
                "fsm '{}' transition '{}' references unknown to_state '{}'",
                descriptor.fsm_id,
                transition.transition_id,
                transition.to_state
            ));
        }
        if !events.contains(&transition.event) {
            return Err(anyhow!(
                "fsm '{}' transition '{}' references unknown event '{}'",
                descriptor.fsm_id,
                transition.transition_id,
                transition.event
            ));
        }
        if let Some(workflow_id) = &transition.workflow_id
            && !known_workflows.is_empty()
            && !known_workflows.contains(workflow_id)
        {
            return Err(anyhow!(
                "fsm '{}' transition '{}' references unknown workflow '{}'",
                descriptor.fsm_id,
                transition.transition_id,
                workflow_id
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "fuzzing_hooks")]
#[allow(dead_code)]
pub fn validate_descriptor_for_fuzz(
    descriptor: &FsmDescriptor,
    workflow_registry: Option<&WorkflowRegistry>,
) -> Result<()> {
    let known_workflows = workflow_registry
        .map(|registry| {
            registry
                .list(None)
                .into_iter()
                .map(|workflow| workflow.workflow_id)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    validate_descriptor(descriptor, &known_workflows)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{self, DomainPack};
    use crate::policy_registry::{PolicyDecision, PolicyDescriptor};
    use crate::skill_registry::SkillDescriptor;
    use crate::workflow_registry::{WorkflowDescriptor, WorkflowStepDescriptor};

    struct TestDomain;

    impl DomainPack for TestDomain {
        fn domain_id(&self) -> DomainId {
            DomainId::new("domain:test-fsm-domain")
        }

        fn name(&self) -> &'static str {
            "Test FSM Domain"
        }

        fn version(&self) -> &'static str {
            "0.0.1"
        }

        fn capabilities(&self) -> Vec<domain::DomainCapability> {
            vec![
                domain::DomainCapability::Workflows,
                domain::DomainCapability::Reports,
            ]
        }

        fn register_skills(&self) -> Vec<SkillDescriptor> {
            vec![SkillDescriptor {
                skill_id: "test-fsm.inspect".to_string(),
                name: "Test FSM Inspect".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Read-only fsm test skill".to_string(),
                input_schema_name: "TestFsmInput".to_string(),
                output_schema_name: "TestFsmOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["read_only".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<PolicyDescriptor> {
            vec![PolicyDescriptor {
                policy_id: "test-fsm.read-only".to_string(),
                name: "Test FSM Read Only".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Allow the test fsm skill.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: PolicyDecision::Allow,
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-fsm-domain"),
                name: "Test FSM Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Workflow for test FSMs.".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "inspect".to_string(),
                    name: "Inspect".to_string(),
                    skill_id: Some("test-fsm.inspect".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-fsm.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-fsm.output")],
                    required_inputs: vec!["input".to_string()],
                    expected_outputs: vec!["output".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test fsm state.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<FsmDescriptor> {
            vec![FsmDescriptor {
                fsm_id: FsmId::new("fsm:test-fsm-domain"),
                name: "Test FSM".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Read-only test fsm.".to_string(),
                initial_state: FsmStateId::new("state:queued"),
                states: vec![
                    FsmStateId::new("state:queued"),
                    FsmStateId::new("state:done"),
                ],
                events: vec![FsmEventId::new("event:inspect")],
                transitions: vec![FsmTransitionDescriptor {
                    transition_id: "transition:queued-to-done".to_string(),
                    from_state: FsmStateId::new("state:queued"),
                    event: FsmEventId::new("event:inspect"),
                    to_state: FsmStateId::new("state:done"),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-fsm.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-fsm.output")],
                    workflow_id: Some(WorkflowId::new("workflow:test-fsm-domain")),
                    guard_description: Some("input is present".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test state and mark complete.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }
    }

    #[test]
    fn fsms_register_without_changing_core() {
        let mut registry = FsmRegistry::new();
        let descriptor = TestDomain.register_fsms().remove(0);
        registry.register(descriptor).expect("register fsm");

        let listed = registry.list(None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].fsm_id.as_str(), "fsm:test-fsm-domain");
    }

    #[test]
    fn duplicate_fsm_ids_are_rejected() {
        let mut registry = FsmRegistry::new();
        let descriptor = TestDomain.register_fsms().remove(0);
        registry
            .register(descriptor.clone())
            .expect("register first");
        let err = registry
            .register(descriptor)
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate fsm id"));
    }

    #[test]
    fn invalid_states_are_rejected() {
        let mut registry = FsmRegistry::new();
        let mut descriptor = TestDomain.register_fsms().remove(0);
        descriptor.initial_state = FsmStateId::new("state:missing");
        let err = registry
            .register(descriptor)
            .expect_err("invalid initial state should fail");
        assert!(err.to_string().contains("initial_state"));
    }

    #[test]
    fn workflow_references_validate() {
        let workflows = crate::workflow_registry::builtin_registry().expect("workflow registry");
        let mut registry = FsmRegistry::with_workflows(&workflows);
        let err = registry
            .register(TestDomain.register_fsms().remove(0))
            .expect_err("missing workflow should fail");
        assert!(err.to_string().contains("unknown workflow"));
    }

    #[test]
    fn shared_state_keys_can_be_referenced() {
        let registry = builtin_registry().expect("registry");
        let fsm = registry
            .show(&FsmId::new("fsm:mock-trading-paper-review"))
            .expect("fsm");
        let transition = fsm.transitions.last().expect("transition");
        assert!(
            transition
                .reads_state_keys
                .iter()
                .any(|key| key.as_str() == "shared.trading.score")
        );
        assert!(
            transition
                .writes_state_keys
                .iter()
                .any(|key| key.as_str() == "shared.trading.paper_review")
        );
    }

    #[test]
    fn mock_trading_remains_paper_only() {
        let registry = builtin_registry().expect("registry");
        let fsms = registry.list(Some(&DomainId::new("domain:mock-trading")));
        assert!(!fsms.is_empty());
        assert!(fsms.iter().all(|fsm| {
            fsm.transitions
                .iter()
                .all(|transition| transition.side_effect_level != SideEffectLevel::TradingAction)
        }));
    }
}
