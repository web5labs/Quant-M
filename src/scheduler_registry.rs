use crate::domain;
use crate::fsm_registry::{FsmId, FsmRegistry};
use crate::sessions::DomainId;
use crate::shared_state::SharedStateKey;
use crate::workflow_registry::{WorkflowId, WorkflowRegistry};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SchedulerId(String);

impl SchedulerId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SchedulerId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SchedulerId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("SchedulerId is empty"));
        }
        Ok(Self::new(trimmed))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum ScheduleTriggerKind {
    Cron,
    Polling,
    Mtime,
    Event,
    Manual,
}

impl fmt::Display for ScheduleTriggerKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            ScheduleTriggerKind::Cron => "cron",
            ScheduleTriggerKind::Polling => "polling",
            ScheduleTriggerKind::Mtime => "mtime",
            ScheduleTriggerKind::Event => "event",
            ScheduleTriggerKind::Manual => "manual",
        };
        value.fmt(f)
    }
}

impl FromStr for ScheduleTriggerKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "cron" => Ok(Self::Cron),
            "polling" | "poll" => Ok(Self::Polling),
            "mtime" => Ok(Self::Mtime),
            "event" => Ok(Self::Event),
            "manual" => Ok(Self::Manual),
            other => Err(anyhow!("unknown schedule trigger kind '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ScheduleCadenceDescriptor {
    pub trigger_kind: ScheduleTriggerKind,
    pub cron_expr: Option<String>,
    pub polling_interval_ms: Option<u64>,
    pub mtime_path: Option<PathBuf>,
    pub event_name: Option<String>,
    pub jitter_ms: Option<u64>,
    pub max_runs: Option<u64>,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SchedulerDescriptor {
    pub scheduler_id: SchedulerId,
    pub name: String,
    pub version: String,
    pub domain_id: DomainId,
    pub description: String,
    pub cadence: ScheduleCadenceDescriptor,
    pub workflow_id: Option<WorkflowId>,
    pub fsm_id: Option<FsmId>,
    pub reads_state_keys: Vec<SharedStateKey>,
    pub writes_state_keys: Vec<SharedStateKey>,
    pub tags: Vec<String>,
}

pub struct SchedulerRegistry {
    schedulers: BTreeMap<SchedulerId, SchedulerDescriptor>,
    known_workflows: BTreeSet<WorkflowId>,
    known_fsms: BTreeSet<FsmId>,
}

impl SchedulerRegistry {
    pub fn new() -> Self {
        Self {
            schedulers: BTreeMap::new(),
            known_workflows: BTreeSet::new(),
            known_fsms: BTreeSet::new(),
        }
    }

    pub fn with_registries(workflows: &WorkflowRegistry, fsms: &FsmRegistry) -> Self {
        Self {
            schedulers: BTreeMap::new(),
            known_workflows: workflows
                .list(None)
                .into_iter()
                .map(|workflow| workflow.workflow_id)
                .collect(),
            known_fsms: fsms.list(None).into_iter().map(|fsm| fsm.fsm_id).collect(),
        }
    }

    pub fn register(&mut self, descriptor: SchedulerDescriptor) -> Result<()> {
        if self.schedulers.contains_key(&descriptor.scheduler_id) {
            return Err(anyhow!(
                "duplicate scheduler id '{}'",
                descriptor.scheduler_id
            ));
        }
        validate_descriptor(&descriptor, &self.known_workflows, &self.known_fsms)?;
        self.schedulers
            .insert(descriptor.scheduler_id.clone(), descriptor);
        Ok(())
    }

    pub fn list(
        &self,
        domain_id: Option<&DomainId>,
        trigger_kind: Option<&ScheduleTriggerKind>,
    ) -> Vec<SchedulerDescriptor> {
        self.schedulers
            .values()
            .filter(|scheduler| {
                domain_id.is_none_or(|value| &scheduler.domain_id == value)
                    && trigger_kind.is_none_or(|value| &scheduler.cadence.trigger_kind == value)
            })
            .cloned()
            .collect()
    }

    pub fn show(&self, scheduler_id: &SchedulerId) -> Result<SchedulerDescriptor> {
        self.schedulers
            .get(scheduler_id)
            .cloned()
            .ok_or_else(|| anyhow!("scheduler '{}' not found", scheduler_id))
    }
}

impl Default for SchedulerRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> Result<SchedulerRegistry> {
    let workflows = crate::workflow_registry::builtin_registry()?;
    let fsms = crate::fsm_registry::builtin_registry()?;
    let domains = domain::builtin_registry()?;
    let mut registry = SchedulerRegistry::with_registries(&workflows, &fsms);
    for pack in domains.packs() {
        for descriptor in pack.register_schedulers() {
            registry.register(descriptor)?;
        }
    }
    Ok(registry)
}

fn validate_descriptor(
    descriptor: &SchedulerDescriptor,
    known_workflows: &BTreeSet<WorkflowId>,
    known_fsms: &BTreeSet<FsmId>,
) -> Result<()> {
    validate_cadence(&descriptor.scheduler_id, &descriptor.cadence)?;

    if let Some(workflow_id) = &descriptor.workflow_id
        && !known_workflows.is_empty()
        && !known_workflows.contains(workflow_id)
    {
        return Err(anyhow!(
            "scheduler '{}' references unknown workflow '{}'",
            descriptor.scheduler_id,
            workflow_id
        ));
    }

    if let Some(fsm_id) = &descriptor.fsm_id
        && !known_fsms.is_empty()
        && !known_fsms.contains(fsm_id)
    {
        return Err(anyhow!(
            "scheduler '{}' references unknown fsm '{}'",
            descriptor.scheduler_id,
            fsm_id
        ));
    }

    Ok(())
}

fn validate_cadence(scheduler_id: &SchedulerId, cadence: &ScheduleCadenceDescriptor) -> Result<()> {
    let cron_present = cadence
        .cron_expr
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());
    let polling_present = cadence.polling_interval_ms.is_some();
    let mtime_present = cadence.mtime_path.is_some();
    let event_present = cadence
        .event_name
        .as_deref()
        .is_some_and(|value| !value.trim().is_empty());

    match cadence.trigger_kind {
        ScheduleTriggerKind::Cron => {
            if !cron_present || polling_present || mtime_present || event_present {
                return Err(anyhow!(
                    "scheduler '{}' has invalid cadence for trigger_kind cron",
                    scheduler_id
                ));
            }
        }
        ScheduleTriggerKind::Polling => {
            if !polling_present || cron_present || mtime_present || event_present {
                return Err(anyhow!(
                    "scheduler '{}' has invalid cadence for trigger_kind polling",
                    scheduler_id
                ));
            }
        }
        ScheduleTriggerKind::Mtime => {
            if !mtime_present || cron_present || polling_present || event_present {
                return Err(anyhow!(
                    "scheduler '{}' has invalid cadence for trigger_kind mtime",
                    scheduler_id
                ));
            }
        }
        ScheduleTriggerKind::Event => {
            if !event_present || cron_present || polling_present || mtime_present {
                return Err(anyhow!(
                    "scheduler '{}' has invalid cadence for trigger_kind event",
                    scheduler_id
                ));
            }
        }
        ScheduleTriggerKind::Manual => {
            if cron_present || polling_present || mtime_present || event_present {
                return Err(anyhow!(
                    "scheduler '{}' has invalid cadence for trigger_kind manual",
                    scheduler_id
                ));
            }
        }
    }

    Ok(())
}

#[cfg(feature = "fuzzing_hooks")]
pub fn validate_descriptor_for_fuzz(
    descriptor: &SchedulerDescriptor,
    workflow_registry: Option<&WorkflowRegistry>,
    fsm_registry: Option<&FsmRegistry>,
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
    let known_fsms = fsm_registry
        .map(|registry| {
            registry
                .list(None)
                .into_iter()
                .map(|fsm| fsm.fsm_id)
                .collect::<BTreeSet<_>>()
        })
        .unwrap_or_default();
    validate_descriptor(descriptor, &known_workflows, &known_fsms)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{self, DomainPack};
    use crate::fsm_registry::{
        FsmDescriptor, FsmEventId, FsmId, FsmStateId, FsmTransitionDescriptor,
    };
    use crate::policy_registry::{PolicyDecision, PolicyDescriptor};
    use crate::skill_registry::{SideEffectLevel, SkillDescriptor};
    use crate::workflow_registry::{WorkflowDescriptor, WorkflowId, WorkflowStepDescriptor};

    struct TestDomain;

    impl DomainPack for TestDomain {
        fn domain_id(&self) -> DomainId {
            DomainId::new("domain:test-scheduler-domain")
        }

        fn name(&self) -> &'static str {
            "Test Scheduler Domain"
        }

        fn version(&self) -> &'static str {
            "0.0.1"
        }

        fn capabilities(&self) -> Vec<domain::DomainCapability> {
            vec![
                domain::DomainCapability::Workflows,
                domain::DomainCapability::Schedulers,
            ]
        }

        fn register_skills(&self) -> Vec<SkillDescriptor> {
            vec![SkillDescriptor {
                skill_id: "test-scheduler.inspect".to_string(),
                name: "Test Scheduler Inspect".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Read-only scheduler test skill".to_string(),
                input_schema_name: "TestSchedulerInput".to_string(),
                output_schema_name: "TestSchedulerOutput".to_string(),
                side_effect_level: SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["read_only".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<PolicyDescriptor> {
            vec![PolicyDescriptor {
                policy_id: "test-scheduler.read-only".to_string(),
                name: "Test Scheduler Read Only".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Allow the test scheduler skill.".to_string(),
                applies_to_side_effect_levels: vec![SideEffectLevel::ReadOnly],
                required_operator_decision: false,
                default_decision: PolicyDecision::Allow,
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-scheduler-domain"),
                name: "Test Scheduler Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Workflow for scheduler registry tests.".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "inspect".to_string(),
                    name: "Inspect".to_string(),
                    skill_id: Some("test-scheduler.inspect".to_string()),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-scheduler.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-scheduler.output")],
                    required_inputs: vec!["input".to_string()],
                    expected_outputs: vec!["output".to_string()],
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Inspect test scheduler state.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<FsmDescriptor> {
            vec![FsmDescriptor {
                fsm_id: FsmId::new("fsm:test-scheduler-domain"),
                name: "Test Scheduler FSM".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "FSM for scheduler registry tests.".to_string(),
                initial_state: FsmStateId::new("state:ready"),
                states: vec![
                    FsmStateId::new("state:ready"),
                    FsmStateId::new("state:done"),
                ],
                events: vec![FsmEventId::new("event:inspect")],
                transitions: vec![FsmTransitionDescriptor {
                    transition_id: "transition:test-scheduler".to_string(),
                    from_state: FsmStateId::new("state:ready"),
                    event: FsmEventId::new("event:inspect"),
                    to_state: FsmStateId::new("state:done"),
                    reads_state_keys: vec![SharedStateKey::new("shared.test-scheduler.input")],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-scheduler.output")],
                    workflow_id: Some(WorkflowId::new("workflow:test-scheduler-domain")),
                    guard_description: Some("input exists".to_string()),
                    side_effect_level: SideEffectLevel::ReadOnly,
                    description: "Advance test scheduler state.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_schedulers(&self) -> Vec<SchedulerDescriptor> {
            vec![SchedulerDescriptor {
                scheduler_id: SchedulerId::new("scheduler:test-scheduler-domain"),
                name: "Test Scheduler".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Scheduler for test registry coverage.".to_string(),
                cadence: ScheduleCadenceDescriptor {
                    trigger_kind: ScheduleTriggerKind::Manual,
                    cron_expr: None,
                    polling_interval_ms: None,
                    mtime_path: None,
                    event_name: None,
                    jitter_ms: Some(50),
                    max_runs: Some(1),
                    enabled: true,
                },
                workflow_id: Some(WorkflowId::new("workflow:test-scheduler-domain")),
                fsm_id: Some(FsmId::new("fsm:test-scheduler-domain")),
                reads_state_keys: vec![SharedStateKey::new("shared.test-scheduler.input")],
                writes_state_keys: vec![SharedStateKey::new("shared.test-scheduler.output")],
                tags: vec!["test".to_string()],
            }]
        }
    }

    #[test]
    fn schedulers_register_without_changing_core() {
        let mut registry = SchedulerRegistry::new();
        let descriptor = TestDomain.register_schedulers().remove(0);
        registry.register(descriptor).expect("register scheduler");

        let listed = registry.list(None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(
            listed[0].scheduler_id.as_str(),
            "scheduler:test-scheduler-domain"
        );
    }

    #[test]
    fn duplicate_scheduler_ids_are_rejected() {
        let mut registry = SchedulerRegistry::new();
        let descriptor = TestDomain.register_schedulers().remove(0);
        registry
            .register(descriptor.clone())
            .expect("register first");
        let err = registry
            .register(descriptor)
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate scheduler id"));
    }

    #[test]
    fn invalid_cadence_trigger_combinations_are_rejected() {
        let mut registry = SchedulerRegistry::new();
        let mut descriptor = TestDomain.register_schedulers().remove(0);
        descriptor.cadence.trigger_kind = ScheduleTriggerKind::Cron;
        let err = registry
            .register(descriptor)
            .expect_err("invalid cadence should fail");
        assert!(err.to_string().contains("invalid cadence"));
    }

    #[test]
    fn workflow_references_validate() {
        let workflows = crate::workflow_registry::builtin_registry().expect("workflow registry");
        let fsms = crate::fsm_registry::builtin_registry().expect("fsm registry");
        let mut registry = SchedulerRegistry::with_registries(&workflows, &fsms);
        let err = registry
            .register(TestDomain.register_schedulers().remove(0))
            .expect_err("missing workflow should fail");
        assert!(err.to_string().contains("unknown workflow"));
    }

    #[test]
    fn fsm_references_validate() {
        let mut workflows = crate::workflow_registry::WorkflowRegistry::new();
        workflows
            .register(TestDomain.register_workflows().remove(0))
            .expect("register workflow");
        let fsms = crate::fsm_registry::builtin_registry().expect("fsm registry");
        let mut registry = SchedulerRegistry::with_registries(&workflows, &fsms);
        let err = registry
            .register(TestDomain.register_schedulers().remove(0))
            .expect_err("missing fsm should fail");
        assert!(err.to_string().contains("unknown fsm"));
    }

    #[test]
    fn scheduler_state_keys_can_be_referenced() {
        let registry = builtin_registry().expect("registry");
        let scheduler = registry
            .show(&SchedulerId::new("scheduler:mock-trading-paper-review"))
            .expect("scheduler");
        assert!(
            scheduler
                .reads_state_keys
                .iter()
                .any(|key| key.as_str() == "shared.trading.score")
        );
        assert!(
            scheduler
                .writes_state_keys
                .iter()
                .any(|key| key.as_str() == "shared.trading.paper_review")
        );
    }

    #[test]
    fn mock_trading_scheduler_is_paper_only() {
        let registry = builtin_registry().expect("registry");
        let schedulers = registry.list(Some(&DomainId::new("domain:mock-trading")), None);
        assert!(!schedulers.is_empty());
        assert!(
            schedulers
                .iter()
                .all(|scheduler| scheduler.tags.iter().any(|tag| tag == "paper_trade_only"))
        );
    }

    #[test]
    fn trigger_filters_work() {
        let registry = builtin_registry().expect("registry");
        let polling = registry.list(None, Some(&ScheduleTriggerKind::Polling));
        assert!(!polling.is_empty());
        assert!(
            polling
                .iter()
                .all(|scheduler| scheduler.cadence.trigger_kind == ScheduleTriggerKind::Polling)
        );
    }
}
