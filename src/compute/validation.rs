use crate::compute::backend::ComputeBackend;
use crate::compute::boundary::NumericConfidence;
use crate::compute::fixtures;
use crate::compute::scalar::{evidence_freshness_scan, peg_deviation_scan};
use crate::config::Config;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ComputeValidationOutcome {
    Accepted,
    AcceptedScalarOnly,
    AcceptedWithScalarFallback,
    RejectedUnsupportedBackend,
    RejectedScalarMismatch,
    RejectedBoundaryAmbiguous,
    RejectedRuntimeExceeded,
    RejectedInputTooLarge,
    RejectedPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeValidationRecord {
    pub validation_id: String,
    pub node_id: String,
    pub backend: ComputeBackend,
    pub hardware_detected: bool,
    pub compiled_available: bool,
    pub implementation_available: bool,
    pub self_test_passed: bool,
    pub scalar_equivalence_verified: bool,
    pub fixture_hash: String,
    pub scalar_output_hash: String,
    pub backend_output_hash: String,
    pub validation_outcome: ComputeValidationOutcome,
    pub quantm_version: String,
    pub rust_target_triple: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ComputeMismatchRecord {
    pub mismatch_id: String,
    pub node_id: String,
    pub backend: ComputeBackend,
    pub workload_id: String,
    pub input_hash: String,
    pub scalar_output_hash: String,
    pub backend_output_hash: String,
    pub tolerance: Option<f64>,
    pub reason: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeBackendQuarantine {
    pub node_id: String,
    pub backend: ComputeBackend,
    pub reason: String,
    pub quarantined_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[allow(dead_code)]
pub struct ComputeEvidenceMeta {
    pub backend_requested: ComputeBackend,
    pub backend_used: ComputeBackend,
    pub scalar_fallback_used: bool,
    pub verified_against_scalar: bool,
    pub validation_outcome: ComputeValidationOutcome,
    pub numeric_confidence: NumericConfidence,
    pub tolerance: Option<f64>,
    pub threshold_epsilon: Option<f64>,
    pub runtime_ms: Option<u64>,
    pub input_hash: String,
    pub output_hash: String,
    pub timing_decision_id: Option<String>,
}

#[allow(dead_code)]
pub struct ComputePaths {
    pub validations: PathBuf,
    pub mismatches: PathBuf,
    pub backend_quarantine: PathBuf,
}

impl ComputePaths {
    pub fn new(cfg: &Config) -> Self {
        Self {
            validations: cfg
                .workspace_dir
                .join("state/cluster/compute-validations.jsonl"),
            mismatches: cfg.workspace_dir.join("state/compute/mismatches.jsonl"),
            backend_quarantine: cfg
                .workspace_dir
                .join("state/compute/backend-quarantine.json"),
        }
    }
}

pub fn append_validation_record(cfg: &Config, record: &ComputeValidationRecord) -> Result<()> {
    append_json_line(&ComputePaths::new(cfg).validations, record)
}

pub fn append_mismatch_record(cfg: &Config, record: &ComputeMismatchRecord) -> Result<()> {
    append_json_line(&ComputePaths::new(cfg).mismatches, record)
}

pub fn read_validation_records(cfg: &Config) -> Result<Vec<ComputeValidationRecord>> {
    read_jsonl(&ComputePaths::new(cfg).validations)
}

pub fn read_mismatch_records(cfg: &Config) -> Result<Vec<ComputeMismatchRecord>> {
    read_jsonl(&ComputePaths::new(cfg).mismatches)
}

pub fn read_quarantine(cfg: &Config) -> Result<Vec<ComputeBackendQuarantine>> {
    let path = ComputePaths::new(cfg).backend_quarantine;
    if !path.exists() {
        return Ok(vec![]);
    }
    serde_json::from_str(&fs::read_to_string(&path)?).context("failed to parse compute quarantine")
}

pub fn write_quarantine(cfg: &Config, quarantine: &[ComputeBackendQuarantine]) -> Result<()> {
    let path = ComputePaths::new(cfg).backend_quarantine;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(quarantine)?)?;
    Ok(())
}

#[allow(dead_code)]
pub fn backend_is_quarantined(
    cfg: &Config,
    node_id: &str,
    backend: ComputeBackend,
) -> Result<bool> {
    Ok(read_quarantine(cfg)?
        .into_iter()
        .any(|entry| entry.node_id == node_id && entry.backend == backend))
}

pub fn quarantine_backend(
    cfg: &Config,
    node_id: &str,
    backend: ComputeBackend,
    reason: &str,
) -> Result<ComputeBackendQuarantine> {
    let mut quarantine = read_quarantine(cfg)?;
    quarantine.retain(|entry| !(entry.node_id == node_id && entry.backend == backend));
    let entry = ComputeBackendQuarantine {
        node_id: node_id.to_string(),
        backend,
        reason: reason.to_string(),
        quarantined_at: Utc::now().to_rfc3339(),
    };
    quarantine.push(entry.clone());
    write_quarantine(cfg, &quarantine)?;
    Ok(entry)
}

pub fn append_mismatch_and_quarantine(
    cfg: &Config,
    mismatch: &ComputeMismatchRecord,
) -> Result<ComputeBackendQuarantine> {
    append_mismatch_record(cfg, mismatch)?;
    quarantine_backend(cfg, &mismatch.node_id, mismatch.backend, &mismatch.reason)
}

pub fn validate_backend_roundtrip(
    cfg: &Config,
    node_id: &str,
    backend: ComputeBackend,
    fixture: &str,
) -> Result<ComputeValidationRecord> {
    let (fixture_hash, scalar_output_hash, backend_output_hash, outcome) = match backend {
        ComputeBackend::Scalar => {
            let (input_hash, output_hash) = run_scalar_fixture(fixture)?;
            (
                input_hash,
                output_hash.clone(),
                output_hash,
                ComputeValidationOutcome::AcceptedScalarOnly,
            )
        }
        other => {
            let fixture_hash = stable_hash(fixture);
            let record = ComputeMismatchRecord {
                mismatch_id: format!(
                    "compute-mismatch-{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                node_id: node_id.to_string(),
                backend: other,
                workload_id: fixture.to_string(),
                input_hash: fixture_hash.clone(),
                scalar_output_hash: "unavailable".to_string(),
                backend_output_hash: "unsupported".to_string(),
                tolerance: None,
                reason: "unsupported backend in scalar-first checkpoint".to_string(),
                timestamp: Utc::now().to_rfc3339(),
            };
            let _ = append_mismatch_and_quarantine(cfg, &record)?;
            (
                fixture_hash,
                "unavailable".to_string(),
                "unsupported".to_string(),
                ComputeValidationOutcome::RejectedUnsupportedBackend,
            )
        }
    };
    let accepted = matches!(
        outcome,
        ComputeValidationOutcome::Accepted | ComputeValidationOutcome::AcceptedScalarOnly
    );
    let record = ComputeValidationRecord {
        validation_id: format!(
            "compute-validation-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        node_id: node_id.to_string(),
        backend,
        hardware_detected: accepted,
        compiled_available: accepted,
        implementation_available: accepted,
        self_test_passed: accepted,
        scalar_equivalence_verified: accepted,
        fixture_hash,
        scalar_output_hash,
        backend_output_hash,
        validation_outcome: outcome,
        quantm_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_target_triple: rust_target_triple(),
        timestamp: Utc::now().to_rfc3339(),
    };
    append_validation_record(cfg, &record)?;
    Ok(record)
}

#[allow(dead_code)]
pub fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    fs::read_to_string(path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("failed to parse compute jsonl record"))
        .collect()
}

fn run_scalar_fixture(fixture: &str) -> Result<(String, String)> {
    match fixture {
        "evidence_freshness" | "evidence_freshness_scan" => {
            let input = fixtures::evidence_freshness_fixture(fixture)?;
            let input_raw = serde_json::to_string(&input)?;
            let output = evidence_freshness_scan(&input)?;
            let output_raw = serde_json::to_string(&output)?;
            Ok((stable_hash(&input_raw), stable_hash(&output_raw)))
        }
        "stablecoin_peg_deviation"
        | "stablecoin_peg_deviation_scan"
        | "boundary_ambiguous_peg_scan" => {
            let input = fixtures::peg_deviation_fixture(fixture)?;
            let input_raw = serde_json::to_string(&input)?;
            let output = peg_deviation_scan(&input, 10.0, 0.01)?;
            let output_raw = serde_json::to_string(&output)?;
            Ok((stable_hash(&input_raw), stable_hash(&output_raw)))
        }
        other => fixtures::evidence_freshness_fixture(other)
            .map(|_| unreachable!())
            .with_context(|| format!("unknown compute validation fixture '{other}'")),
    }
}

fn rust_target_triple() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}
