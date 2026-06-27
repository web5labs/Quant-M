use crate::compute::boundary::{NumericConfidence, compare_threshold};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceFreshnessInput {
    pub evidence_ids: Vec<String>,
    pub evidence_timestamps: Vec<i64>,
    pub now_timestamp: i64,
    pub stale_after_seconds: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceFreshnessOutput {
    pub fresh_ids: Vec<String>,
    pub stale_ids: Vec<String>,
    pub fresh_count: usize,
    pub stale_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PegDeviationInput {
    pub prices: Vec<f64>,
    pub target_peg: f64,
    pub stale_flags: Vec<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PegDeviationOutput {
    pub deviations_abs: Vec<f64>,
    pub deviations_bps: Vec<f64>,
    pub max_deviation_bps: f64,
    pub stale_count: usize,
    pub numeric_confidence: NumericConfidence,
}

pub fn evidence_freshness_scan(input: &EvidenceFreshnessInput) -> Result<EvidenceFreshnessOutput> {
    if input.evidence_ids.len() != input.evidence_timestamps.len() {
        return Err(anyhow!("evidence ids and timestamps length mismatch"));
    }
    if input.stale_after_seconds < 0 {
        return Err(anyhow!("stale_after_seconds must be non-negative"));
    }
    let mut fresh_ids = Vec::new();
    let mut stale_ids = Vec::new();
    for (evidence_id, timestamp) in input.evidence_ids.iter().zip(&input.evidence_timestamps) {
        let age = input.now_timestamp.saturating_sub(*timestamp);
        if age > input.stale_after_seconds {
            stale_ids.push(evidence_id.clone());
        } else {
            fresh_ids.push(evidence_id.clone());
        }
    }
    Ok(EvidenceFreshnessOutput {
        fresh_count: fresh_ids.len(),
        stale_count: stale_ids.len(),
        fresh_ids,
        stale_ids,
    })
}

pub fn peg_deviation_scan(
    input: &PegDeviationInput,
    threshold_bps: f64,
    threshold_epsilon_bps: f64,
) -> Result<PegDeviationOutput> {
    if input.prices.len() != input.stale_flags.len() {
        return Err(anyhow!("prices and stale flags length mismatch"));
    }
    if !input.target_peg.is_finite() || input.target_peg <= 0.0 {
        return Err(anyhow!("target peg must be positive"));
    }
    let mut deviations_abs = Vec::with_capacity(input.prices.len());
    let mut deviations_bps = Vec::with_capacity(input.prices.len());
    let mut max_deviation_bps = 0.0_f64;
    let mut confidence = NumericConfidence::Exact;
    for price in &input.prices {
        if !price.is_finite() || *price <= 0.0 {
            return Err(anyhow!("price must be positive"));
        }
        let deviation_abs = (*price - input.target_peg).abs();
        let deviation_bps = (deviation_abs / input.target_peg) * 10_000.0;
        let threshold = compare_threshold(deviation_bps, threshold_bps, threshold_epsilon_bps);
        if threshold.confidence == NumericConfidence::BoundaryAmbiguous {
            confidence = NumericConfidence::BoundaryAmbiguous;
        }
        max_deviation_bps = max_deviation_bps.max(deviation_bps);
        deviations_abs.push(deviation_abs);
        deviations_bps.push(deviation_bps);
    }
    Ok(PegDeviationOutput {
        deviations_abs,
        deviations_bps,
        max_deviation_bps,
        stale_count: input.stale_flags.iter().filter(|stale| **stale).count(),
        numeric_confidence: confidence,
    })
}
