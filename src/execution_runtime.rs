use crate::config::Config;
use crate::domain::{self, DomainRegistry};
use crate::fsm_registry::{self, FsmDescriptor, FsmTransitionDescriptor};
use crate::scheduler_registry::{self, SchedulerId};
use crate::sessions::{self, AgentId, DomainId, SessionContext, SessionEvent, SessionId};
use crate::shared_state::{
    HybridSharedStateStore, SharedStateKey, SharedStateRecord, SharedStateStore, SharedStateValue,
};
use crate::skill_registry::{self, SkillDescriptor};
use crate::workflow_registry::{self, WorkflowDescriptor, WorkflowId, WorkflowStepDescriptor};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::Serialize;

#[cfg(feature = "fuzzing_hooks")]
use serde::Deserialize;

struct ExecutionRegistries {
    domains: DomainRegistry,
    skills: skill_registry::SkillRegistry,
    workflows: workflow_registry::WorkflowRegistry,
    fsms: fsm_registry::FsmRegistry,
    schedulers: scheduler_registry::SchedulerRegistry,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkflowRunResult {
    pub session_id: SessionId,
    pub workflow_id: WorkflowId,
    pub domain_id: DomainId,
    pub status: String,
    pub steps_completed: usize,
    pub shared_state_writes: Vec<String>,
    pub related_schedulers: Vec<String>,
    pub final_summary: String,
}

#[derive(Debug)]
struct LocalSkillOutcome {
    output_summary: String,
    state_writes: Vec<SharedStateRecord>,
}

pub fn run_workflow(cfg: &Config, workflow_id: &WorkflowId) -> Result<WorkflowRunResult> {
    let registries = ExecutionRegistries {
        domains: domain::builtin_registry()?,
        skills: skill_registry::builtin_registry()?,
        workflows: workflow_registry::builtin_registry()?,
        fsms: fsm_registry::builtin_registry()?,
        schedulers: scheduler_registry::builtin_registry()?,
    };
    run_workflow_with_registries(cfg, workflow_id, &registries)
}

#[cfg(feature = "fuzzing_hooks")]
#[derive(Debug, Clone, Deserialize)]
struct WorkflowRunRequestForFuzz {
    workflow_id: String,
}

#[cfg(feature = "fuzzing_hooks")]
pub fn parse_request_for_fuzz(raw: &str) -> Result<()> {
    if let Ok(request) = serde_json::from_str::<WorkflowRunRequestForFuzz>(raw) {
        let _ = request.workflow_id.parse::<WorkflowId>()?;
        return Ok(());
    }
    let _ = raw.parse::<WorkflowId>()?;
    Ok(())
}

fn run_workflow_with_registries(
    cfg: &Config,
    workflow_id: &WorkflowId,
    registries: &ExecutionRegistries,
) -> Result<WorkflowRunResult> {
    let workflow = registries.workflows.show(workflow_id).with_context(|| {
        format!(
            "workflow '{}' could not be loaded for execution",
            workflow_id
        )
    })?;
    registries
        .domains
        .show(&workflow.domain_id)
        .with_context(|| format!("domain '{}' is not registered", workflow.domain_id))?;

    let session = SessionContext::new(
        AgentId::new(format!("agent:{}:runtime", cfg.node_id)),
        workflow.domain_id.clone(),
    );
    let state_store = HybridSharedStateStore::from_config(cfg);
    let related_schedulers = schedulers_for_workflow(registries, &workflow);

    record_event(
        cfg,
        &session,
        SessionEvent::Observation {
            message: "workflow_started".to_string(),
            job_id: None,
            detail: Some(workflow.workflow_id.to_string()),
        },
    )?;

    if !related_schedulers.is_empty() {
        record_event(
            cfg,
            &session,
            SessionEvent::AuditNote {
                note: format!(
                    "related_schedulers={}",
                    related_schedulers
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(",")
                ),
            },
        )?;
    }

    let fsm = matching_fsm(registries, &workflow);
    if let Some((fsm_descriptor, _)) = &fsm {
        record_event(
            cfg,
            &session,
            SessionEvent::Observation {
                message: "fsm_bound".to_string(),
                job_id: None,
                detail: Some(fsm_descriptor.fsm_id.to_string()),
            },
        )?;
    }

    let mut all_writes = Vec::new();
    let mut completed_steps = 0usize;

    for step in &workflow.steps {
        let skill_id = step
            .skill_id
            .as_deref()
            .ok_or_else(|| anyhow!("workflow step '{}' has no skill_id", step.step_id))?;
        let descriptor = registries
            .skills
            .show(skill_id)
            .with_context(|| format!("workflow references unknown skill '{}'", skill_id))?;
        record_event(
            cfg,
            &session,
            SessionEvent::SkillCall {
                skill_name: descriptor.skill_id.clone(),
                input_preview: step.required_inputs.join(","),
                command_preview: None,
                status: "running".to_string(),
            },
        )?;

        let outcome = match execute_local_skill(cfg, &session, &workflow, step, &descriptor) {
            Ok(outcome) => outcome,
            Err(err) => {
                record_event(
                    cfg,
                    &session,
                    SessionEvent::Error {
                        code: Some("workflow_skill_failed".to_string()),
                        message: err.to_string(),
                    },
                )?;
                record_event(
                    cfg,
                    &session,
                    SessionEvent::AuditNote {
                        note: format!(
                            "workflow_failed workflow_id={} step_id={}",
                            workflow.workflow_id, step.step_id
                        ),
                    },
                )?;
                return Err(err);
            }
        };

        record_event(
            cfg,
            &session,
            SessionEvent::SkillCall {
                skill_name: descriptor.skill_id.clone(),
                input_preview: step.required_inputs.join(","),
                command_preview: None,
                status: "ok".to_string(),
            },
        )?;

        for record in outcome.state_writes {
            state_store
                .put(record.clone())
                .with_context(|| format!("failed to persist shared-state key '{}'", record.key))?;
            record_event(
                cfg,
                &session,
                SessionEvent::Observation {
                    message: "shared_state_written".to_string(),
                    job_id: None,
                    detail: Some(record.key.to_string()),
                },
            )?;
            all_writes.push(record.key.to_string());
        }

        if let Some((fsm_descriptor, transition)) = &fsm {
            record_event(
                cfg,
                &session,
                SessionEvent::FsmTransition {
                    machine: fsm_descriptor.fsm_id.to_string(),
                    from_state: Some(transition.from_state.to_string()),
                    to_state: transition.to_state.to_string(),
                    reason: transition
                        .guard_description
                        .clone()
                        .unwrap_or_else(|| step.name.clone()),
                },
            )?;
        }

        record_event(
            cfg,
            &session,
            SessionEvent::Output {
                channel: "workflow_step".to_string(),
                summary: truncate_for_session(&outcome.output_summary),
                job_id: None,
            },
        )?;
        completed_steps = completed_steps.saturating_add(1);
    }

    let final_summary = format!(
        "workflow={} status=ok steps_completed={} shared_state_writes={}",
        workflow.workflow_id,
        completed_steps,
        all_writes.join(",")
    );
    record_event(
        cfg,
        &session,
        SessionEvent::Output {
            channel: "workflow".to_string(),
            summary: final_summary.clone(),
            job_id: None,
        },
    )?;
    record_event(
        cfg,
        &session,
        SessionEvent::Observation {
            message: "workflow_completed".to_string(),
            job_id: None,
            detail: Some(workflow.workflow_id.to_string()),
        },
    )?;

    Ok(WorkflowRunResult {
        session_id: session.session_id,
        workflow_id: workflow.workflow_id,
        domain_id: workflow.domain_id,
        status: "ok".to_string(),
        steps_completed: completed_steps,
        shared_state_writes: all_writes,
        related_schedulers: related_schedulers
            .into_iter()
            .map(|scheduler| scheduler.to_string())
            .collect(),
        final_summary,
    })
}

fn execute_local_skill(
    cfg: &Config,
    session: &SessionContext,
    workflow: &WorkflowDescriptor,
    step: &WorkflowStepDescriptor,
    descriptor: &SkillDescriptor,
) -> Result<LocalSkillOutcome> {
    match descriptor.skill_id.as_str() {
        "mock-research.capture-brief" => {
            execute_mock_research_capture_brief(cfg, session, workflow, step, descriptor)
        }
        other => Err(anyhow!(
            "local execution for skill '{}' is not implemented",
            other
        )),
    }
}

fn execute_mock_research_capture_brief(
    cfg: &Config,
    session: &SessionContext,
    workflow: &WorkflowDescriptor,
    step: &WorkflowStepDescriptor,
    descriptor: &SkillDescriptor,
) -> Result<LocalSkillOutcome> {
    let brief = read_state_as_text(cfg, "shared.research.brief")
        .unwrap_or_else(|| "Quant-M mock-research brief".to_string());
    let sources = read_state_as_text(cfg, "shared.research.sources")
        .unwrap_or_else(|| "project spec, shared state doctrine".to_string());
    let summary = format!(
        "Research summary for '{}': brief='{}'; sources='{}'; workflow='{}'; skill='{}'",
        step.name, brief, sources, workflow.workflow_id, descriptor.skill_id
    );
    let record = SharedStateRecord {
        key: SharedStateKey::new("shared.research.summary"),
        value: SharedStateValue::Text(summary.clone()),
        domain_id: workflow.domain_id.clone(),
        source: format!("workflow:{}", workflow.workflow_id),
        confidence: 0.9,
        updated_at: now_rfc3339(),
        expires_at: None,
        session_id: Some(session.session_id.clone()),
    };
    Ok(LocalSkillOutcome {
        output_summary: summary,
        state_writes: vec![record],
    })
}

fn read_state_as_text(cfg: &Config, key: &str) -> Option<String> {
    let key = SharedStateKey::new(key);
    let record = crate::shared_state::show_state(cfg, &key).ok().flatten()?;
    Some(match record.value {
        SharedStateValue::Text(value) => value,
        SharedStateValue::Json(value) => value.to_string(),
        SharedStateValue::Number(value) => value.to_string(),
        SharedStateValue::Bool(value) => value.to_string(),
        SharedStateValue::Timestamp(value) => value,
        SharedStateValue::Score(value) => value.to_string(),
        SharedStateValue::Status(value) => value,
    })
}

fn matching_fsm(
    registries: &ExecutionRegistries,
    workflow: &WorkflowDescriptor,
) -> Option<(FsmDescriptor, FsmTransitionDescriptor)> {
    registries
        .fsms
        .list(Some(&workflow.domain_id))
        .into_iter()
        .find_map(|fsm| {
            let transition = fsm
                .transitions
                .iter()
                .find(|transition| transition.workflow_id.as_ref() == Some(&workflow.workflow_id))?
                .clone();
            Some((fsm, transition))
        })
}

fn schedulers_for_workflow(
    registries: &ExecutionRegistries,
    workflow: &WorkflowDescriptor,
) -> Vec<SchedulerId> {
    registries
        .schedulers
        .list(Some(&workflow.domain_id), None)
        .into_iter()
        .filter(|scheduler| scheduler.workflow_id.as_ref() == Some(&workflow.workflow_id))
        .map(|scheduler| scheduler.scheduler_id)
        .collect()
}

fn record_event(cfg: &Config, session: &SessionContext, event: SessionEvent) -> Result<()> {
    sessions::append_event(cfg, session, event)
}

fn truncate_for_session(value: &str) -> String {
    const LIMIT: usize = 160;
    let mut chars = value.chars();
    let truncated: String = chars.by_ref().take(LIMIT).collect();
    if chars.next().is_some() {
        format!("{truncated}...")
    } else {
        truncated
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::domain::DomainPack;
    use crate::shared_state::{SharedStateKey, SharedStateValue, show_state};
    use crate::workflow_registry::{WorkflowDescriptor, WorkflowId, WorkflowStepDescriptor};
    use tempfile::TempDir;

    struct TestDomain;

    impl domain::DomainPack for TestDomain {
        fn domain_id(&self) -> DomainId {
            DomainId::new("domain:test-runtime")
        }

        fn name(&self) -> &'static str {
            "Test Runtime"
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
                skill_id: "test-runtime.missing".to_string(),
                name: "Missing".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Unsupported skill for failure-path tests.".to_string(),
                input_schema_name: "TestInput".to_string(),
                output_schema_name: "TestOutput".to_string(),
                side_effect_level: skill_registry::SideEffectLevel::ReadOnly,
                required_capabilities: vec!["observations".to_string()],
                policy_tags: vec!["test".to_string()],
            }]
        }

        fn register_policies(&self) -> Vec<crate::policy_registry::PolicyDescriptor> {
            vec![]
        }

        fn register_workflows(&self) -> Vec<WorkflowDescriptor> {
            vec![WorkflowDescriptor {
                workflow_id: WorkflowId::new("workflow:test-runtime-fail"),
                name: "Failing Workflow".to_string(),
                version: "0.0.1".to_string(),
                domain_id: self.domain_id(),
                description: "Failure workflow".to_string(),
                steps: vec![WorkflowStepDescriptor {
                    step_id: "missing".to_string(),
                    name: "Missing".to_string(),
                    skill_id: Some("test-runtime.missing".to_string()),
                    reads_state_keys: vec![],
                    writes_state_keys: vec![SharedStateKey::new("shared.test-runtime.output")],
                    required_inputs: vec![],
                    expected_outputs: vec!["output".to_string()],
                    side_effect_level: skill_registry::SideEffectLevel::ReadOnly,
                    description: "Triggers unsupported skill.".to_string(),
                }],
                tags: vec!["test".to_string()],
            }]
        }

        fn register_fsms(&self) -> Vec<fsm_registry::FsmDescriptor> {
            vec![]
        }
    }

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        (tmp, cfg)
    }

    #[test]
    fn mock_research_workflow_executes_end_to_end() {
        let (_tmp, cfg) = temp_cfg();
        let result =
            run_workflow(&cfg, &WorkflowId::new("workflow:mock-research-brief")).expect("run");

        assert_eq!(result.status, "ok");
        assert_eq!(result.steps_completed, 1);
        assert_eq!(result.shared_state_writes, vec!["shared.research.summary"]);
        assert_eq!(result.domain_id, DomainId::new("domain:mock-research"));
    }

    #[test]
    fn shared_state_is_updated() {
        let (_tmp, cfg) = temp_cfg();
        let result =
            run_workflow(&cfg, &WorkflowId::new("workflow:mock-research-brief")).expect("run");

        let record = show_state(&cfg, &SharedStateKey::new("shared.research.summary"))
            .expect("show state")
            .expect("summary record");
        assert_eq!(record.session_id, Some(result.session_id));
        match record.value {
            SharedStateValue::Text(value) => assert!(value.contains("Research summary")),
            other => panic!("unexpected shared-state value: {other:?}"),
        }
    }

    #[test]
    fn session_events_are_recorded_and_replay_works() {
        let (_tmp, cfg) = temp_cfg();
        let result =
            run_workflow(&cfg, &WorkflowId::new("workflow:mock-research-brief")).expect("run");

        let detail = sessions::show_session(&cfg, &result.session_id).expect("show session");
        assert!(
            detail
                .events
                .iter()
                .any(|entry| matches!(entry.event, SessionEvent::Observation { ref message, .. } if message == "workflow_started"))
        );
        assert!(
            detail
                .events
                .iter()
                .any(|entry| matches!(entry.event, SessionEvent::SkillCall { .. }))
        );
        assert!(
            detail
                .events
                .iter()
                .any(|entry| matches!(entry.event, SessionEvent::Observation { ref message, .. } if message == "shared_state_written"))
        );
        let replay = sessions::replay_session(&cfg, &result.session_id).expect("replay");
        assert_eq!(replay.summary.final_status, "ok");
        assert!(!replay.state.side_effects_replayed);
    }

    #[test]
    fn failure_paths_are_recorded_as_session_evidence() {
        let (_tmp, cfg) = temp_cfg();
        let domain = TestDomain;
        let mut domains = DomainRegistry::new();
        domains.register(Box::new(TestDomain)).expect("domain");
        let mut skills = skill_registry::SkillRegistry::new();
        for descriptor in domain.register_skills() {
            skills.register(descriptor).expect("skill");
        }
        let mut workflows = workflow_registry::WorkflowRegistry::new();
        for descriptor in domain.register_workflows() {
            workflows.register(descriptor).expect("workflow");
        }
        let registries = ExecutionRegistries {
            domains,
            skills,
            workflows,
            fsms: fsm_registry::FsmRegistry::new(),
            schedulers: scheduler_registry::SchedulerRegistry::new(),
        };

        let err = run_workflow_with_registries(
            &cfg,
            &WorkflowId::new("workflow:test-runtime-fail"),
            &registries,
        )
        .expect_err("run should fail");
        assert!(err.to_string().contains("not implemented"));

        let listed = sessions::list_sessions(&cfg).expect("list sessions");
        let session = listed.first().expect("failure session");
        let detail = sessions::show_session(&cfg, &session.session_id).expect("show session");
        assert!(
            detail
                .events
                .iter()
                .any(|entry| matches!(entry.event, SessionEvent::Error { .. }))
        );
    }

    #[test]
    fn no_external_network_action_occurs() {
        let (_tmp, cfg) = temp_cfg();
        let result =
            run_workflow(&cfg, &WorkflowId::new("workflow:mock-research-brief")).expect("run");
        let replay = sessions::replay_session(&cfg, &result.session_id).expect("replay");
        assert!(!replay.state.side_effects_replayed);
        assert_eq!(replay.state.policy_denials, 0);
    }
}
