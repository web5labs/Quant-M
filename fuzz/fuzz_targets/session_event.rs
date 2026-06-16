#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::sessions;

const MAX_INPUT_BYTES: usize = 16 * 1024;

fn json_string(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_else(|_| "\"\"".to_string())
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let text = json_string(raw.trim());

    let candidates = [
        raw.to_string(),
        format!(
            "{{\"kind\":\"observation\",\"message\":{text},\"job_id\":null,\"detail\":null}}"
        ),
        format!(
            "{{\"kind\":\"skill_call\",\"skill_name\":\"mock.skill\",\"input_preview\":{text},\"command_preview\":null,\"status\":\"ok\"}}"
        ),
        format!(
            "{{\"kind\":\"fsm_transition\",\"machine\":\"fsm:fuzz\",\"from_state\":\"state:queued\",\"to_state\":{text},\"reason\":\"fuzz\"}}"
        ),
        format!(
            "{{\"kind\":\"operator_decision\",\"record\":{{\
                \"session_id\":\"session:fuzz\",\
                \"run_id\":\"run:fuzz\",\
                \"step_id\":\"step:fuzz\",\
                \"domain_id\":\"domain:fuzz\",\
                \"decision\":\"Approved\",\
                \"reason\":{text},\
                \"decided_at\":\"2026-05-31T00:00:00+00:00\",\
                \"decided_by\":\"operator\"\
            }}}}"
        ),
        format!(
            "{{\"kind\":\"error\",\"code\":\"fuzz\",\"message\":{text}}}"
        ),
    ];

    for candidate in candidates {
        let _ = sessions::parse_and_replay_event_for_fuzz(&candidate);
    }
});
