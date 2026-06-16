#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::fsm_registry::{self, FsmDescriptor};
use quant_m::sessions::DomainId;
use quant_m::shared_state::SharedStateKey;
use quant_m::skill_registry::SideEffectLevel;
use quant_m::workflow_registry::{
    WorkflowDescriptor, WorkflowId, WorkflowRegistry, WorkflowStepDescriptor,
};

const MAX_INPUT_BYTES: usize = 24 * 1024;

fn fixture_workflows() -> WorkflowRegistry {
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
    workflows
}

fn exercise_candidate(candidate: &str, workflows: &WorkflowRegistry) {
    if let Ok(descriptor) = serde_json::from_str::<FsmDescriptor>(candidate) {
        let _ = fsm_registry::validate_descriptor_for_fuzz(&descriptor, Some(workflows));
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let workflows = fixture_workflows();

    let validish = concat!(
        "{\"fsm_id\":\"fsm:fuzz\",\"name\":\"fuzz\",\"version\":\"0.0.1\",\"domain_id\":\"domain:fuzz\",",
        "\"description\":\"fuzz\",\"initial_state\":\"state:queued\",\"states\":[\"state:queued\",\"state:done\"],",
        "\"events\":[\"event:go\"],\"transitions\":[{\"transition_id\":\"transition:go\",",
        "\"from_state\":\"state:queued\",\"event\":\"event:go\",\"to_state\":\"state:done\",",
        "\"reads_state_keys\":[\"shared.input\"],\"writes_state_keys\":[\"shared.output\"],",
        "\"workflow_id\":\"workflow:fixture\",\"guard_description\":\"ready\",\"side_effect_level\":\"ReadOnly\",",
        "\"description\":\"advance\"}],\"tags\":[\"fuzz\"]}"
    );

    let invalid_ref = format!(
        concat!(
            "{{\"fsm_id\":\"fsm:{token}\",\"name\":\"bad\",\"version\":\"0.0.1\",\"domain_id\":\"domain:fuzz\",",
            "\"description\":\"bad\",\"initial_state\":\"state:missing\",\"states\":[\"state:a\"],\"events\":[\"event:a\"],",
            "\"transitions\":[{{\"transition_id\":\"transition:a\",\"from_state\":\"state:missing\",\"event\":\"event:oops\",",
            "\"to_state\":\"state:b\",\"reads_state_keys\":[],\"writes_state_keys\":[],\"workflow_id\":\"workflow:unknown\",",
            "\"guard_description\":null,\"side_effect_level\":\"ReadOnly\",\"description\":\"bad\"}}],\"tags\":[]}}"
        ),
        token = raw.trim()
    );

    exercise_candidate(&raw, &workflows);
    exercise_candidate(validish, &workflows);
    exercise_candidate(&invalid_ref, &workflows);
});
