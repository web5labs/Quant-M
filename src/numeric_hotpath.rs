use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::time::Instant;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum NumericBackend {
    Scalar,
    Auto,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PegSnapshot {
    pub price: f64,
    pub fee_bps: f64,
    pub spread_bps: f64,
    pub slippage_bps: f64,
    pub venue_risk_bps: f64,
    pub withdrawal_friction_bps: f64,
    pub age_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PegScanConfig {
    pub peg_price: f64,
    pub deviation_threshold_bps: f64,
    pub stale_after_seconds: u64,
    pub net_edge_threshold_bps: f64,
}

impl Default for PegScanConfig {
    fn default() -> Self {
        Self {
            peg_price: 1.0,
            deviation_threshold_bps: 10.0,
            stale_after_seconds: 120,
            net_edge_threshold_bps: 5.0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PegScanRow {
    pub index: usize,
    pub price: f64,
    pub deviation_bps: f64,
    pub net_edge_bps: f64,
    pub stale: bool,
    pub candidate: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PegScanReport {
    pub backend: NumericBackend,
    pub scalar_source_of_truth: bool,
    pub samples: usize,
    pub stale_rejections: usize,
    pub candidates: Vec<PegScanRow>,
    pub elapsed_nanos: u128,
    pub authority_changed: bool,
}

pub fn scan_stablecoin_peg_scalar(
    snapshots: &[PegSnapshot],
    config: &PegScanConfig,
) -> Result<Vec<PegScanRow>> {
    validate_config(config)?;
    snapshots
        .iter()
        .enumerate()
        .map(|(index, snapshot)| {
            if !snapshot.price.is_finite() || snapshot.price <= 0.0 {
                return Err(anyhow!("snapshot {index} has invalid price"));
            }
            let deviation_bps = ((snapshot.price - config.peg_price) / config.peg_price) * 10_000.0;
            let gross_edge_bps = deviation_bps.abs();
            let net_edge_bps = gross_edge_bps
                - snapshot.fee_bps
                - snapshot.spread_bps
                - snapshot.slippage_bps
                - snapshot.venue_risk_bps
                - snapshot.withdrawal_friction_bps;
            let stale = snapshot.age_seconds > config.stale_after_seconds;
            Ok(PegScanRow {
                index,
                price: snapshot.price,
                deviation_bps,
                net_edge_bps,
                stale,
                candidate: !stale
                    && gross_edge_bps >= config.deviation_threshold_bps
                    && net_edge_bps >= config.net_edge_threshold_bps,
            })
        })
        .collect()
}

pub fn scan_stablecoin_peg(
    snapshots: &[PegSnapshot],
    config: &PegScanConfig,
    backend: NumericBackend,
) -> Result<Vec<PegScanRow>> {
    match backend {
        NumericBackend::Scalar => scan_stablecoin_peg_scalar(snapshots, config),
        NumericBackend::Auto => scan_stablecoin_peg_scalar(snapshots, config),
    }
}

pub fn compare_with_scalar(
    snapshots: &[PegSnapshot],
    config: &PegScanConfig,
    backend: NumericBackend,
    tolerance_bps: f64,
) -> Result<()> {
    let scalar = scan_stablecoin_peg_scalar(snapshots, config)?;
    let accelerated = scan_stablecoin_peg(snapshots, config, backend)?;
    if scalar.len() != accelerated.len() {
        return Err(anyhow!("accelerated output length differs from scalar"));
    }
    for (left, right) in scalar.iter().zip(accelerated.iter()) {
        if left.index != right.index
            || left.stale != right.stale
            || left.candidate != right.candidate
        {
            return Err(anyhow!("accelerated classification differs from scalar"));
        }
        if (left.deviation_bps - right.deviation_bps).abs() > tolerance_bps
            || (left.net_edge_bps - right.net_edge_bps).abs() > tolerance_bps
        {
            return Err(anyhow!("accelerated numeric output exceeds tolerance"));
        }
    }
    Ok(())
}

pub fn run_peg_scan_benchmark(samples: usize, backend: NumericBackend) -> Result<PegScanReport> {
    let snapshots = synthetic_peg_snapshots(samples);
    let config = PegScanConfig::default();
    compare_with_scalar(&snapshots, &config, backend, 0.000_001)?;
    let started = Instant::now();
    let rows = scan_stablecoin_peg(&snapshots, &config, backend)?;
    let elapsed_nanos = started.elapsed().as_nanos();
    let stale_rejections = rows.iter().filter(|row| row.stale).count();
    let candidates = rows.into_iter().filter(|row| row.candidate).collect();
    Ok(PegScanReport {
        backend,
        scalar_source_of_truth: true,
        samples,
        stale_rejections,
        candidates,
        elapsed_nanos,
        authority_changed: false,
    })
}

pub fn render_peg_scan_report(report: &PegScanReport) -> String {
    format!(
        "numeric hot path peg scan\nbackend: {:?}\nsamples: {}\ncandidates: {}\nstale_rejections: {}\nelapsed_nanos: {}\nscalar_source_of_truth: {}\nauthority_changed: {}\n",
        report.backend,
        report.samples,
        report.candidates.len(),
        report.stale_rejections,
        report.elapsed_nanos,
        report.scalar_source_of_truth,
        report.authority_changed
    )
}

fn validate_config(config: &PegScanConfig) -> Result<()> {
    if !config.peg_price.is_finite() || config.peg_price <= 0.0 {
        return Err(anyhow!("peg_price must be positive"));
    }
    if config.deviation_threshold_bps < 0.0 || config.net_edge_threshold_bps < 0.0 {
        return Err(anyhow!("thresholds must be non-negative"));
    }
    Ok(())
}

fn synthetic_peg_snapshots(samples: usize) -> Vec<PegSnapshot> {
    (0..samples)
        .map(|index| {
            let drift = match index % 9 {
                0 => -0.0020,
                1 => 0.0016,
                2 => -0.0007,
                _ => 0.0002,
            };
            PegSnapshot {
                price: 1.0 + drift,
                fee_bps: 1.0,
                spread_bps: 1.5,
                slippage_bps: 1.0,
                venue_risk_bps: 1.0,
                withdrawal_friction_bps: 0.5,
                age_seconds: if index % 17 == 0 { 180 } else { 30 },
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stablecoin_peg_scanner_flags_net_edge_and_stale_rows() {
        let snapshots = vec![
            PegSnapshot {
                price: 0.998,
                fee_bps: 1.0,
                spread_bps: 1.0,
                slippage_bps: 1.0,
                venue_risk_bps: 1.0,
                withdrawal_friction_bps: 1.0,
                age_seconds: 10,
            },
            PegSnapshot {
                price: 0.998,
                fee_bps: 1.0,
                spread_bps: 1.0,
                slippage_bps: 1.0,
                venue_risk_bps: 1.0,
                withdrawal_friction_bps: 1.0,
                age_seconds: 999,
            },
        ];
        let rows = scan_stablecoin_peg_scalar(&snapshots, &PegScanConfig::default()).expect("scan");
        assert!(rows[0].candidate);
        assert!(rows[1].stale);
        assert!(!rows[1].candidate);
    }

    #[test]
    fn auto_backend_matches_scalar_with_strict_tolerance() {
        let snapshots = synthetic_peg_snapshots(256);
        compare_with_scalar(
            &snapshots,
            &PegScanConfig::default(),
            NumericBackend::Auto,
            0.000_001,
        )
        .expect("auto matches scalar");
    }

    #[test]
    fn benchmark_report_never_changes_authority() {
        let report = run_peg_scan_benchmark(128, NumericBackend::Auto).expect("benchmark");
        assert!(report.scalar_source_of_truth);
        assert!(!report.authority_changed);
        assert_eq!(report.samples, 128);
    }
}
