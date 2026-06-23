use crate::domain;
use crate::sessions::DomainId;
use crate::shared_state::SharedStateKey;
use crate::skill_registry::SideEffectLevel;
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct WorkflowId(String);

impl WorkflowId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for WorkflowId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for WorkflowId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("WorkflowId is empty"));
        }
        Ok(Self::new(trimmed))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowStepDescriptor {
    pub step_id: String,
    pub name: String,
    pub skill_id: Option<String>,
    pub reads_state_keys: Vec<SharedStateKey>,
    pub writes_state_keys: Vec<SharedStateKey>,
    pub required_inputs: Vec<String>,
    pub expected_outputs: Vec<String>,
    pub side_effect_level: SideEffectLevel,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WorkflowDescriptor {
    pub workflow_id: WorkflowId,
    pub name: String,
    pub version: String,
    pub domain_id: DomainId,
    pub description: String,
    pub steps: Vec<WorkflowStepDescriptor>,
    pub tags: Vec<String>,
}

pub struct WorkflowRegistry {
    workflows: BTreeMap<WorkflowId, WorkflowDescriptor>,
}

impl WorkflowRegistry {
    pub fn new() -> Self {
        Self {
            workflows: BTreeMap::new(),
        }
    }

    pub fn register(&mut self, descriptor: WorkflowDescriptor) -> Result<()> {
        if self.workflows.contains_key(&descriptor.workflow_id) {
            return Err(anyhow!(
                "duplicate workflow id '{}'",
                descriptor.workflow_id
            ));
        }
        validate_descriptor(&descriptor)?;
        self.workflows
            .insert(descriptor.workflow_id.clone(), descriptor);
        Ok(())
    }

    pub fn list(&self, domain_id: Option<&DomainId>) -> Vec<WorkflowDescriptor> {
        self.workflows
            .values()
            .filter(|workflow| domain_id.is_none_or(|value| &workflow.domain_id == value))
            .cloned()
            .collect()
    }

    pub fn show(&self, workflow_id: &WorkflowId) -> Result<WorkflowDescriptor> {
        self.workflows
            .get(workflow_id)
            .cloned()
            .ok_or_else(|| anyhow!("workflow '{}' not found", workflow_id))
    }

    #[allow(dead_code)]
    pub fn inspect_steps(&self, workflow_id: &WorkflowId) -> Result<Vec<WorkflowStepDescriptor>> {
        Ok(self.show(workflow_id)?.steps)
    }
}

impl Default for WorkflowRegistry {
    fn default() -> Self {
        Self::new()
    }
}

fn validate_descriptor(descriptor: &WorkflowDescriptor) -> Result<()> {
    let _ = WorkflowId::from_str(descriptor.workflow_id.as_str())?;
    let _ = DomainId::from_str(descriptor.domain_id.as_str())?;
    if descriptor.name.trim().is_empty() {
        return Err(anyhow!(
            "workflow '{}' name is empty",
            descriptor.workflow_id
        ));
    }
    if descriptor.version.trim().is_empty() {
        return Err(anyhow!(
            "workflow '{}' version is empty",
            descriptor.workflow_id
        ));
    }
    if descriptor.steps.is_empty() {
        return Err(anyhow!(
            "workflow '{}' has no steps",
            descriptor.workflow_id
        ));
    }

    let mut seen_step_ids = BTreeMap::<&str, usize>::new();
    for step in &descriptor.steps {
        if step.step_id.trim().is_empty() {
            return Err(anyhow!(
                "workflow '{}' has step with empty step_id",
                descriptor.workflow_id
            ));
        }
        if step.name.trim().is_empty() {
            return Err(anyhow!(
                "workflow '{}' step '{}' has empty name",
                descriptor.workflow_id,
                step.step_id
            ));
        }
        if let Some(skill_id) = &step.skill_id
            && skill_id.trim().is_empty()
        {
            return Err(anyhow!(
                "workflow '{}' step '{}' has empty skill_id",
                descriptor.workflow_id,
                step.step_id
            ));
        }
        if seen_step_ids.insert(step.step_id.as_str(), 1).is_some() {
            return Err(anyhow!(
                "workflow '{}' has duplicate step_id '{}'",
                descriptor.workflow_id,
                step.step_id
            ));
        }
        for key in step
            .reads_state_keys
            .iter()
            .chain(step.writes_state_keys.iter())
        {
            let _ = SharedStateKey::from_str(key.as_str())?;
        }
    }

    Ok(())
}

#[cfg(feature = "fuzzing_hooks")]
#[allow(dead_code)]
pub fn validate_descriptor_for_fuzz(descriptor: &WorkflowDescriptor) -> Result<()> {
    validate_descriptor(descriptor)
}

pub fn builtin_registry() -> Result<WorkflowRegistry> {
    let domains = domain::builtin_registry()?;
    let mut registry = WorkflowRegistry::new();
    for pack in domains.packs() {
        for descriptor in pack.register_workflows() {
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
    use crate::policy_registry::{PolicyDecision, PolicyDescriptor};
    use crate::skill_registry::SkillDescriptor;

    struct TestDomain;

    impl DomainPack for TestDomain {
        fn domain_id(&self) -> DomainId {
            DomainId::new("domain:test-workflow-domain")
        }

        fn name(&self) -> &'static str {
            "Test Workflow Domain"
        }

        fn version(&self) -> &'static str {
            "0.0.1"
        }

        fn capabilities(&self) -> Vec<domain::DomainCapability> {
            vec![
                domain::DomainCapability::Skills,
                domain::DomainCapability::Workflows,
            ]
        }

        fn register_skills(&self) -> Vec<SkillDescriptor> {
            vec![SkillDescriptor {
                skill_id: "test-workflow.inspect".to_string(),
                name: "Test Workflow Inspect".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Read-only workflow test skill".to_string(),
                input_schema_name: "TestWorkflowInput".to_string(),
                output_schema_name: "TestWorkflowOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["read_only".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<PolicyDescriptor> {
            vec![PolicyDescriptor {
                policy_id: "test-workflow.read-only".to_string(),
                name: "Test Workflow Read Only".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Allow the test workflow skill.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: PolicyDecision::Allow,
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-workflow"),
                name: "Test Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Inspect test state through a read-only plan.".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "collect".to_string(),
                    name: "Collect".to_string(),
                    skill_id: Some("test-workflow.inspect".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.test.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test.output")],
                    required_inputs: vec!["brief_id".to_string()],
                    expected_outputs: vec!["inspection".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Read current state and prepare an inspection output.".to_string(),
                }],
                tags: vec!["test".to_string(), "read_only".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<FsmDescriptor> {
            vec![FsmDescriptor {
                fsm_id: FsmId::new("fsm:test-workflow-domain"),
                name: "Test Workflow FSM".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "FSM for workflow registry tests.".to_string(),
                initial_state: FsmStateId::new("state:queued"),
                states: vec![
                    FsmStateId::new("state:queued"),
                    FsmStateId::new("state:done"),
                ],
                events: vec![FsmEventId::new("event:collect")],
                transitions: vec![FsmTransitionDescriptor {
                    transition_id: "transition:queued-to-done".to_string(),
                    from_state: FsmStateId::new("state:queued"),
                    event: FsmEventId::new("event:collect"),
                    to_state: FsmStateId::new("state:done"),
                    reads_state_keys: vec![SharedStateKey::new("shared.test.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test.output")],
                    workflow_id: Some(WorkflowId::new("workflow:test-workflow")),
                    guard_description: Some("brief is ready".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Collect test workflow output.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }
    }

    #[test]
    fn workflows_register_without_changing_core() {
        let mut registry = WorkflowRegistry::new();
        let descriptor = TestDomain.register_workflows().remove(0);
        registry.register(descriptor).expect("register workflow");

        let listed = registry.list(None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].workflow_id.as_str(), "workflow:test-workflow");
    }

    #[test]
    fn duplicate_workflow_ids_are_rejected() {
        let mut registry = WorkflowRegistry::new();
        let descriptor = TestDomain.register_workflows().remove(0);
        registry
            .register(descriptor.clone())
            .expect("register first workflow");
        let err = registry
            .register(descriptor)
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate workflow id"));
    }

    #[test]
    fn workflow_steps_can_reference_skill_ids() {
        let registry = builtin_registry().expect("registry");
        let steps = registry
            .inspect_steps(&WorkflowId::new("workflow:mock-research-brief"))
            .expect("steps");
        assert_eq!(steps.len(), 1);
        assert_eq!(
            steps[0].skill_id.as_deref(),
            Some("mock-research.capture-brief")
        );
    }

    #[test]
    fn workflow_steps_can_read_and_write_shared_state_keys() {
        let registry = builtin_registry().expect("registry");
        let workflow = registry
            .show(&WorkflowId::new("workflow:mock-trading-paper-review"))
            .expect("workflow");
        let step = workflow.steps.last().expect("workflow step");
        assert!(
            step.reads_state_keys
                .iter()
                .any(|key| key.as_str() == "shared.trading.score")
        );
        assert!(
            step.writes_state_keys
                .iter()
                .any(|key| key.as_str() == "shared.trading.paper_review")
        );
    }

    #[test]
    fn mock_trading_workflow_does_not_expose_live_trading() {
        let registry = builtin_registry().expect("registry");
        let workflows = registry.list(Some(&DomainId::new("domain:mock-trading")));
        assert!(!workflows.is_empty());
        assert!(workflows.iter().all(|workflow| {
            workflow
                .steps
                .iter()
                .all(|step| step.side_effect_level != SideEffectLevel::TradingAction)
        }));
    }
}
