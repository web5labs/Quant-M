#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::execution_runtime;

const MAX_INPUT_BYTES: usize = 8 * 1024;

fn json_string(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_else(|_| "\"\"".to_string())
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let token = raw.trim();
    let text = json_string(token);

    let candidates = [
        raw.to_string(),
        "workflow:mock-research-brief".to_string(),
        format!("{{\"workflow_id\":{text}}}"),
        format!("{{\"workflow_id\":\"workflow:{token}\"}}"),
        "{\"workflow_id\":\"\"}".to_string(),
    ];

    for candidate in candidates {
        let _ = execution_runtime::parse_request_for_fuzz(&candidate);
    }
});
