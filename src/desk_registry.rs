use crate::domain;
use crate::fsm_registry::{FsmId, FsmRegistry};
use crate::scheduler_registry::{SchedulerId, SchedulerRegistry};
use crate::sessions::DomainId;
use crate::shared_state::SharedStateKey;
use crate::skill_registry::SkillRegistry;
use crate::workflow_registry::{WorkflowId, WorkflowRegistry};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct DeskId(String);

impl DeskId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DeskId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for DeskId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("DeskId is empty"));
        }
        Ok(Self::new(trimmed))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub enum DeskCategory {
    Forex,
    Crypto,
    StocksOptions,
    PredictionMarkets,
    Sports,
    Research,
    Custom,
}

impl fmt::Display for DeskCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            DeskCategory::Forex => "forex",
            DeskCategory::Crypto => "crypto",
            DeskCategory::StocksOptions => "stocks_options",
            DeskCategory::PredictionMarkets => "prediction_markets",
            DeskCategory::Sports => "sports",
            DeskCategory::Research => "research",
            DeskCategory::Custom => "custom",
        };
        value.fmt(f)
    }
}

impl FromStr for DeskCategory {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "forex" => Ok(Self::Forex),
            "crypto" => Ok(Self::Crypto),
            "stocks_options" | "stocks-options" | "stocksoptions" => Ok(Self::StocksOptions),
            "prediction_markets" | "prediction-markets" | "predictionmarkets" => {
                Ok(Self::PredictionMarkets)
            }
            "sports" => Ok(Self::Sports),
            "research" => Ok(Self::Research),
            "custom" => Ok(Self::Custom),
            other => Err(anyhow!("unknown desk category '{}'", other)),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeskStorageProfile {
    pub hot_state_required: bool,
    pub durable_history_required: bool,
    pub external_adapters_required: bool,
    pub paper_only: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeskPackDescriptor {
    pub desk_id: DeskId,
    pub name: String,
    pub version: String,
    pub category: DeskCategory,
    pub domain_id: DomainId,
    pub description: String,
    pub skill_ids: Vec<String>,
    pub workflow_ids: Vec<WorkflowId>,
    pub fsm_ids: Vec<FsmId>,
    pub scheduler_ids: Vec<SchedulerId>,
    pub reads_state_keys: Vec<SharedStateKey>,
    pub writes_state_keys: Vec<SharedStateKey>,
    pub storage_profile: DeskStorageProfile,
    pub tags: Vec<String>,
}

pub struct DeskPackRegistry {
    desks: BTreeMap<DeskId, DeskPackDescriptor>,
    known_skill_ids: BTreeSet<String>,
    known_workflow_ids: BTreeSet<WorkflowId>,
    known_fsm_ids: BTreeSet<FsmId>,
    known_scheduler_ids: BTreeSet<SchedulerId>,
}

impl DeskPackRegistry {
    pub fn new() -> Self {
        Self {
            desks: BTreeMap::new(),
            known_skill_ids: BTreeSet::new(),
            known_workflow_ids: BTreeSet::new(),
            known_fsm_ids: BTreeSet::new(),
            known_scheduler_ids: BTreeSet::new(),
        }
    }

    pub fn with_registries(
        skills: &SkillRegistry,
        workflows: &WorkflowRegistry,
        fsms: &FsmRegistry,
        schedulers: &SchedulerRegistry,
    ) -> Self {
        Self {
            desks: BTreeMap::new(),
            known_skill_ids: skills
                .list(None, None)
                .into_iter()
                .map(|descriptor| descriptor.skill_id)
                .collect(),
            known_workflow_ids: workflows
                .list(None)
                .into_iter()
                .map(|descriptor| descriptor.workflow_id)
                .collect(),
            known_fsm_ids: fsms
                .list(None)
                .into_iter()
                .map(|descriptor| descriptor.fsm_id)
                .collect(),
            known_scheduler_ids: schedulers
                .list(None, None)
                .into_iter()
                .map(|descriptor| descriptor.scheduler_id)
                .collect(),
        }
    }

    pub fn register(&mut self, descriptor: DeskPackDescriptor) -> Result<()> {
        if self.desks.contains_key(&descriptor.desk_id) {
            return Err(anyhow!("duplicate desk id '{}'", descriptor.desk_id));
        }
        validate_descriptor(
            &descriptor,
            &self.known_skill_ids,
            &self.known_workflow_ids,
            &self.known_fsm_ids,
            &self.known_scheduler_ids,
        )?;
        self.desks.insert(descriptor.desk_id.clone(), descriptor);
        Ok(())
    }

    pub fn list(
        &self,
        category: Option<&DeskCategory>,
        domain_id: Option<&DomainId>,
    ) -> Vec<DeskPackDescriptor> {
        self.desks
            .values()
            .filter(|descriptor| {
                category.is_none_or(|value| &descriptor.category == value)
                    && domain_id.is_none_or(|value| &descriptor.domain_id == value)
            })
            .cloned()
            .collect()
    }

    pub fn show(&self, desk_id: &DeskId) -> Result<DeskPackDescriptor> {
        self.desks
            .get(desk_id)
            .cloned()
            .ok_or_else(|| anyhow!("desk '{}' not found", desk_id))
    }
}

impl Default for DeskPackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

pub fn builtin_registry() -> Result<DeskPackRegistry> {
    let skills = crate::skill_registry::builtin_registry()?;
    let workflows = crate::workflow_registry::builtin_registry()?;
    let fsms = crate::fsm_registry::builtin_registry()?;
    let schedulers = crate::scheduler_registry::builtin_registry()?;
    let domains = domain::builtin_registry()?;
    let mut registry = DeskPackRegistry::with_registries(&skills, &workflows, &fsms, &schedulers);
    for pack in domains.packs() {
        for descriptor in pack.register_desks() {
            registry.register(descriptor)?;
        }
    }
    Ok(registry)
}

fn validate_descriptor(
    descriptor: &DeskPackDescriptor,
    known_skill_ids: &BTreeSet<String>,
    known_workflow_ids: &BTreeSet<WorkflowId>,
    known_fsm_ids: &BTreeSet<FsmId>,
    known_scheduler_ids: &BTreeSet<SchedulerId>,
) -> Result<()> {
    for skill_id in &descriptor.skill_ids {
        if !known_skill_ids.is_empty() && !known_skill_ids.contains(skill_id) {
            return Err(anyhow!(
                "desk '{}' references unknown skill '{}'",
                descriptor.desk_id,
                skill_id
            ));
        }
    }

    for workflow_id in &descriptor.workflow_ids {
        if !known_workflow_ids.is_empty() && !known_workflow_ids.contains(workflow_id) {
            return Err(anyhow!(
                "desk '{}' references unknown workflow '{}'",
                descriptor.desk_id,
                workflow_id
            ));
        }
    }

    for fsm_id in &descriptor.fsm_ids {
        if !known_fsm_ids.is_empty() && !known_fsm_ids.contains(fsm_id) {
            return Err(anyhow!(
                "desk '{}' references unknown fsm '{}'",
                descriptor.desk_id,
                fsm_id
            ));
        }
    }

    for scheduler_id in &descriptor.scheduler_ids {
        if !known_scheduler_ids.is_empty() && !known_scheduler_ids.contains(scheduler_id) {
            return Err(anyhow!(
                "desk '{}' references unknown scheduler '{}'",
                descriptor.desk_id,
                scheduler_id
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_descriptor() -> DeskPackDescriptor {
        DeskPackDescriptor {
            desk_id: DeskId::new("desk:test-paper"),
            name: "Test Paper Desk".to_string(),
            version: "0.0.1".to_string(),
            category: DeskCategory::Custom,
            domain_id: DomainId::new("domain:test-desk"),
            description: "A test desk pack descriptor.".to_string(),
            skill_ids: vec!["mock-trading.score-handoff".to_string()],
            workflow_ids: vec![WorkflowId::new("workflow:mock-trading-paper-review")],
            fsm_ids: vec![FsmId::new("fsm:mock-trading-paper-review")],
            scheduler_ids: vec![SchedulerId::new("scheduler:mock-trading-paper-review")],
            reads_state_keys: vec![SharedStateKey::new("shared.trading.handoff")],
            writes_state_keys: vec![SharedStateKey::new("shared.trading.paper_review")],
            storage_profile: DeskStorageProfile {
                hot_state_required: true,
                durable_history_required: true,
                external_adapters_required: false,
                paper_only: true,
                notes: "Test paper-only desk profile.".to_string(),
            },
            tags: vec!["paper_trade_only".to_string(), "test".to_string()],
        }
    }

    fn registry_with_known_refs() -> DeskPackRegistry {
        let skills = crate::skill_registry::builtin_registry().expect("skills");
        let workflows = crate::workflow_registry::builtin_registry().expect("workflows");
        let fsms = crate::fsm_registry::builtin_registry().expect("fsms");
        let schedulers = crate::scheduler_registry::builtin_registry().expect("schedulers");
        DeskPackRegistry::with_registries(&skills, &workflows, &fsms, &schedulers)
    }

    #[test]
    fn desk_packs_register_without_changing_core() {
        let mut registry = registry_with_known_refs();
        let descriptor = sample_descriptor();
        registry.register(descriptor).expect("register desk");

        let listed = registry.list(None, None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].desk_id.as_str(), "desk:test-paper");
    }

    #[test]
    fn duplicate_desk_ids_are_rejected() {
        let mut registry = registry_with_known_refs();
        let descriptor = sample_descriptor();
        registry
            .register(descriptor.clone())
            .expect("register first desk");
        let err = registry
            .register(descriptor)
            .expect_err("duplicate should fail");
        assert!(err.to_string().contains("duplicate desk id"));
    }

    #[test]
    fn category_filtering_works() {
        let registry = builtin_registry().expect("registry");
        let listed = registry.list(Some(&DeskCategory::Research), None);
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].desk_id.as_str(), "desk:mock-research-brief");
    }

    #[test]
    fn domain_filtering_works() {
        let registry = builtin_registry().expect("registry");
        let listed = registry.list(None, Some(&DomainId::new("domain:mock-trading")));
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].desk_id.as_str(), "desk:mock-trading-paper");
    }

    #[test]
    fn references_validate_against_registries() {
        let mut registry = registry_with_known_refs();
        let mut descriptor = sample_descriptor();
        descriptor.skill_ids = vec!["missing.skill".to_string()];

        let err = registry
            .register(descriptor)
            .expect_err("missing skill should fail");
        assert!(err.to_string().contains("references unknown skill"));
    }

    #[test]
    fn mock_trading_desk_pack_is_paper_only() {
        let registry = builtin_registry().expect("registry");
        let descriptor = registry
            .show(&DeskId::new("desk:mock-trading-paper"))
            .expect("show desk");
        assert_eq!(descriptor.category, DeskCategory::Forex);
        assert!(descriptor.storage_profile.paper_only);
        assert!(!descriptor.storage_profile.external_adapters_required);
    }
}
