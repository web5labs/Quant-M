#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::agent_shell;

const MAX_INPUT_BYTES: usize = 8 * 1024;

fn escaped(input: &str) -> String {
    serde_json::to_string(input).unwrap_or_else(|_| "\"\"".to_string())
}

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let raw = String::from_utf8_lossy(data);
    let trimmed = raw.trim();
    let token = if trimmed.is_empty() { "x" } else { trimmed };
    let quoted = escaped(token);

    let candidates = [
        raw.to_string(),
        "help".to_string(),
        "doctor".to_string(),
        "run demo".to_string(),
        format!("run workflow {}", token),
        "state summary".to_string(),
        format!("state show {}", token),
        "session recent".to_string(),
        format!("session show {}", token),
        format!("session replay {}", token),
        "config show".to_string(),
        "quit".to_string(),
        format!("unknown {}", quoted),
    ];

    for candidate in candidates {
        let _ = agent_shell::parse_command_for_fuzz(&candidate);
    }
});
