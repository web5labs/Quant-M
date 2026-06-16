#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::workflow_registry::{self, WorkflowDescriptor, WorkflowRegistry};

const MAX_INPUT_BYTES: usize = 24 * 1024;

fn json_string(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_else(|_| "\"\"".to_string())
}

fn exercise_candidate(candidate: &str) {
    if let Ok(descriptor) = serde_json::from_str::<WorkflowDescriptor>(candidate) {
        let _ = workflow_registry::validate_descriptor_for_fuzz(&descriptor);
        let mut registry = WorkflowRegistry::new();
        if registry.register(descriptor.clone()).is_ok() {
            let _ = registry.register(descriptor);
        }
    }
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let token = raw.trim();
    let text = json_string(token);

    let validish = format!(
        concat!(
            "{{",
            "\"workflow_id\":\"workflow:fuzz\",",
            "\"name\":\"fuzz-workflow\",",
            "\"version\":\"0.0.1\",",
            "\"domain_id\":\"domain:fuzz\",",
            "\"description\":{text},",
            "\"steps\":[{{",
            "\"step_id\":\"step-a\",",
            "\"name\":\"collect\",",
            "\"skill_id\":\"skill.fuzz\",",
            "\"reads_state_keys\":[\"shared.input\"],",
            "\"writes_state_keys\":[\"shared.output\"],",
            "\"required_inputs\":[\"brief\"],",
            "\"expected_outputs\":[\"summary\"],",
            "\"side_effect_level\":\"ReadOnly\",",
            "\"description\":{text}",
            "}}],",
            "\"tags\":[\"fuzz\"]",
            "}}"
        ),
        text = text
    );

    let duplicate_step = format!(
        concat!(
            "{{",
            "\"workflow_id\":\"workflow:{token}\",",
            "\"name\":\"{token}\",",
            "\"version\":\"0.0.1\",",
            "\"domain_id\":\"domain:fuzz\",",
            "\"description\":\"dup\",",
            "\"steps\":[",
            "{{\"step_id\":\"dup\",\"name\":\"a\",\"skill_id\":\"skill.fuzz\",\"reads_state_keys\":[],\"writes_state_keys\":[],\"required_inputs\":[],\"expected_outputs\":[],\"side_effect_level\":\"ReadOnly\",\"description\":\"a\"}},",
            "{{\"step_id\":\"dup\",\"name\":\"b\",\"skill_id\":\"skill.fuzz\",\"reads_state_keys\":[],\"writes_state_keys\":[],\"required_inputs\":[],\"expected_outputs\":[],\"side_effect_level\":\"ReadOnly\",\"description\":\"b\"}}",
            "],",
            "\"tags\":[]",
            "}}"
        ),
        token = token
    );

    exercise_candidate(&raw);
    exercise_candidate(&validish);
    exercise_candidate(&duplicate_step);
});
