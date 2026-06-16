#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::forex;

const MAX_INPUT_BYTES: usize = 64 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok(mapped) = forex::map_stonex_payload_for_fuzz(input) {
        let _ = serde_json::to_string(&mapped.shared_signal);
        let _ = serde_json::to_string(&mapped.handoff);
    }
});
