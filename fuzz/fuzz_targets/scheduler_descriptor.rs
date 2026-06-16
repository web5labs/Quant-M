#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::fsm_registry::{
    FsmDescriptor, FsmEventId, FsmId, FsmRegistry, FsmStateId, FsmTransitionDescriptor,
};
use quant_m::scheduler_registry::{self, SchedulerDescriptor};
use quant_m::sessions::DomainId;
use quant_m::shared_state::SharedStateKey;
use quant_m::skill_registry::SideEffectLevel;
use quant_m::workflow_registry::{
    WorkflowDescriptor, WorkflowId, WorkflowRegistry, WorkflowStepDescriptor,
};

const MAX_INPUT_BYTES: usize = 24 * 1024;

fn fixture_registries() -> (WorkflowRegistry, FsmRegistry) {
    let mut workflows = WorkflowRegistry::new();
    let _ = workflows.register(WorkflowDescriptor {
        workflow_id: WorkflowId::new("workflow:fixture"),
        name: "fixture".to_string(),
        version: "0.0.1".to_string(),
        domain_id: DomainId::new("domain:fuzz"),
        description: "fixture".to_string(),
        steps: vec![WorkflowStepDescriptor {
            step_id: "step-1".to_string(),
            name: "step".to_string(),
            skill_id: Some("skill.fuzz".to_string()),
            reads_state_keys: vec![SharedStateKey::new("shared.input")],
            writes_state_keys: vec![SharedStateKey::new("shared.output")],
            required_inputs: vec![],
            expected_outputs: vec![],
            side_effect_level: SideEffectLevel::ReadOnly,
            description: "fixture".to_string(),
        }],
        tags: vec!["fuzz".to_string()],
    });

    let mut fsms = FsmRegistry::with_workflows(&workflows);
    let _ = fsms.register(FsmDescriptor {
        fsm_id: FsmId::new("fsm:fixture"),
        name: "fixture".to_string(),
        version: "0.0.1".to_string(),
        domain_id: DomainId::new("domain:fuzz"),
        description: "fixture".to_string(),
        initial_state: FsmStateId::new("state:queued"),
        states: vec![FsmStateId::new("state:queued"), FsmStateId::new("state:done")],
        events: vec![FsmEventId::new("event:go")],
        transitions: vec![FsmTransitionDescriptor {
            transition_id: "transition:go".to_string(),
            from_state: FsmStateId::new("state:queued"),
            event: FsmEventId::new("event:go"),
            to_state: FsmStateId::new("state:done"),
            reads_state_keys: vec![SharedStateKey::new("shared.input")],
            writes_state_keys: vec![SharedStateKey::new("shared.output")],
            workflow_id: Some(WorkflowId::new("workflow:fixture")),
            guard_description: Some("ready".to_string()),
            side_effect_level: SideEffectLevel::ReadOnly,
            description: "fixture".to_string(),
        }],
        tags: vec!["fuzz".to_string()],
    });

    (workflows, fsms)
}

fn exercise_candidate(candidate: &str, workflows: &WorkflowRegistry, fsms: &FsmRegistry) {
    if let Ok(descriptor) = serde_json::from_str::<SchedulerDescriptor>(candidate) {
        let _ = scheduler_registry::validate_descriptor_for_fuzz(
            &descriptor,
            Some(workflows),
            Some(fsms),
        );
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let (workflows, fsms) = fixture_registries();

    let validish = concat!(
        "{\"scheduler_id\":\"scheduler:fuzz\",\"name\":\"fuzz\",\"version\":\"0.0.1\",\"domain_id\":\"domain:fuzz\",",
        "\"description\":\"fuzz\",\"cadence\":{\"trigger_kind\":\"Polling\",\"cron_expr\":null,\"polling_interval_ms\":1000,",
        "\"mtime_path\":null,\"event_name\":null,\"jitter_ms\":0,\"max_runs\":1,\"enabled\":true},",
        "\"workflow_id\":\"workflow:fixture\",\"fsm_id\":\"fsm:fixture\",\"reads_state_keys\":[\"shared.input\"],",
        "\"writes_state_keys\":[\"shared.output\"],\"tags\":[\"fuzz\"]}"
    );

    let invalidish = format!(
        concat!(
            "{{\"scheduler_id\":\"scheduler:{token}\",\"name\":\"bad\",\"version\":\"0.0.1\",\"domain_id\":\"domain:fuzz\",",
            "\"description\":\"bad\",\"cadence\":{{\"trigger_kind\":\"Manual\",\"cron_expr\":\"* * * * *\",\"polling_interval_ms\":1,",
            "\"mtime_path\":null,\"event_name\":null,\"jitter_ms\":0,\"max_runs\":1,\"enabled\":true}},",
            "\"workflow_id\":\"workflow:unknown\",\"fsm_id\":\"fsm:unknown\",\"reads_state_keys\":[],\"writes_state_keys\":[],\"tags\":[]}}"
        ),
        token = raw.trim()
    );

    exercise_candidate(&raw, &workflows, &fsms);
    exercise_candidate(validish, &workflows, &fsms);
    exercise_candidate(&invalidish, &workflows, &fsms);
});
