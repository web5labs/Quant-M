#![no_main]
use libfuzzer_sys::fuzz_target;
use quant_m::ingest;

const MAX_INPUT_BYTES: usize = 64 * 1024;
const MAX_NDJSON_LINES: usize = 128;

fuzz_target!(|data: &[u8]| {
    if data.len() > MAX_INPUT_BYTES {
        return;
    }

    let Ok(input) = std::str::from_utf8(data) else {
        return;
    };

    // Primary path: single JSON job payload
    match ingest::parse_worker_job_json(input) {
        Ok(job) => {
            let _ = ingest::validate_worker_job_minimal(&job);
            if let Ok(roundtrip) = serde_json::to_string(&job) {
                let _ = ingest::parse_worker_job_json(&roundtrip);
            }
        }
        Err(_) => {}
    }

    // Secondary path: NDJSON queue ingestion behavior
    match ingest::parse_worker_jobs_ndjson(input, MAX_NDJSON_LINES) {
        Ok(jobs) => {
            for job in jobs.iter().take(16) {
                let _ = ingest::validate_worker_job_minimal(job);
            }
        }
        Err(_) => {}
    }
});
