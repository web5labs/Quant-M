use crate::compute::scalar::{EvidenceFreshnessInput, PegDeviationInput};
use anyhow::{Result, anyhow};

pub fn evidence_freshness_fixture(name: &str) -> Result<EvidenceFreshnessInput> {
    match name {
        "evidence_freshness" | "evidence_freshness_scan" => Ok(EvidenceFreshnessInput {
            evidence_ids: vec![
                "evidence:fresh-01".to_string(),
                "evidence:stale-01".to_string(),
                "evidence:fresh-02".to_string(),
            ],
            evidence_timestamps: vec![1_700_000_000, 1_699_999_000, 1_699_999_950],
            now_timestamp: 1_700_000_000,
            stale_after_seconds: 120,
        }),
        other => Err(anyhow!("unknown evidence freshness fixture '{other}'")),
    }
}

pub fn peg_deviation_fixture(name: &str) -> Result<PegDeviationInput> {
    match name {
        "stablecoin_peg" | "stablecoin_peg_deviation" => Ok(PegDeviationInput {
            prices: vec![1.0, 0.999, 1.0015, 0.9975],
            target_peg: 1.0,
            stale_flags: vec![false, false, false, true],
        }),
        "boundary_ambiguous_peg_scan" => Ok(PegDeviationInput {
            prices: vec![0.999, 1.001],
            target_peg: 1.0,
            stale_flags: vec![false, false],
        }),
        other => Err(anyhow!("unknown peg deviation fixture '{other}'")),
    }
}
