#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::forex;

const MAX_INPUT_BYTES: usize = 128 * 1024;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    if let Ok(events) = forex::parse_mql5_rows_for_fuzz(input) {
        for event in events.iter().take(32) {
            let _ = serde_json::to_string(event);
        }
    }
});
