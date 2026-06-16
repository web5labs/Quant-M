#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::shared_state;

const MAX_INPUT_BYTES: usize = 16 * 1024;

fn json_string(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_else(|_| "\"\"".to_string())
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let value = json_string(raw.trim());

    let candidates = [
        raw.to_string(),
        format!(
            concat!(
                "{{",
                "\"key\":\"shared.fuzz\",",
                "\"value\":{{\"Text\":{value}}},",
                "\"domain_id\":\"domain:fuzz\",",
                "\"source\":\"fuzz\",",
                "\"confidence\":0.5,",
                "\"updated_at\":\"2026-05-31T00:00:00+00:00\",",
                "\"expires_at\":null,",
                "\"session_id\":\"session:fuzz\"",
                "}}"
            ),
            value = value
        ),
        format!(
            concat!(
                "{{",
                "\"key\":{value},",
                "\"value\":{{\"Status\":\"ready\"}},",
                "\"domain_id\":\"domain:fuzz\",",
                "\"source\":{value},",
                "\"confidence\":1.5,",
                "\"updated_at\":\"bad-timestamp\",",
                "\"expires_at\":\"2026-05-30T00:00:00+00:00\",",
                "\"session_id\":\"session:fuzz\"",
                "}}"
            ),
            value = value
        ),
    ];

    for candidate in candidates {
        let _ = shared_state::parse_and_validate_record_for_fuzz(&candidate);
    }
});
