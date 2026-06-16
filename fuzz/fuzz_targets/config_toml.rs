#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::config;

const MAX_INPUT_BYTES: usize = 16 * 1024;

fn toml_single_quoted(input: &str) -> String {
    input.replace('\'', " ")
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let cleaned = toml_single_quoted(raw.trim());

    let candidates = [
        raw.to_string(),
        format!(
            "node_id = 'fuzz-node'\nworkspace_dir = 'workspace'\n[runtime]\nprofile = 'laptop'\n"
        ),
        format!(
            "node_id = 'fuzz-node'\nworkspace_dir = '{}'\n[runtime]\nprofile = 'edge'\nsession_dir = 'workspace/state/sessions'\nexternal_network_enabled = false\n",
            cleaned
        ),
        format!(
            "[preferences]\npreferred_openrouter_model = '{}'\n",
            cleaned
        ),
        format!(
            "provider = 'openrouter'\nmodel = '{}'\n",
            cleaned
        ),
    ];

    for candidate in candidates {
        let _ = config::parse_and_validate_toml_for_fuzz(&candidate);
    }
});
