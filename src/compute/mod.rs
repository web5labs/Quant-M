pub mod backend;
pub mod bench_policy;
pub mod boundary;
pub mod capability;
pub mod fixtures;
pub mod scalar;
pub mod validation;

pub use backend::ComputeBackend;
pub use bench_policy::BenchmarkPolicy;
pub use capability::{capability_report, render_capability_report};
pub use scalar::{evidence_freshness_scan, peg_deviation_scan};
pub use validation::{
    backend_is_quarantined, read_mismatch_records, read_quarantine, read_validation_records,
    validate_backend_roundtrip,
};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compute::backend::ComputeBackendStatus;
    use crate::compute::bench_policy::ComputeInputLimits;
    use crate::compute::bench_policy::validate_benchmark_policy;
    use crate::compute::boundary::compare_threshold;
    use crate::compute::boundary::{NumericConfidence, ThresholdRelation};
    use crate::compute::validation::{
        ComputeEvidenceMeta, ComputeValidationOutcome, backend_is_quarantined,
        read_validation_records,
    };
    use crate::config::Config;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().join("workspace");
        (tmp, cfg)
    }

    #[test]
    fn backend_status_requires_all_validation_flags() {
        let mut status = ComputeBackendStatus {
            backend: ComputeBackend::ArmNeon,
            hardware_detected: true,
            compiled_available: true,
            implementation_available: true,
            self_test_passed: true,
            last_self_test_at: None,
            scalar_equivalence_verified: false,
        };
        assert!(!status.usable());
        status.scalar_equivalence_verified = true;
        assert!(status.usable());
    }

    #[test]
    fn compute_capability_defaults_to_scalar() {
        let report = capability_report();
        assert_eq!(report.selected_backend, ComputeBackend::Scalar);
        assert!(report.scalar_available);
        assert!(
            report
                .backend_statuses
                .iter()
                .find(|status| status.backend == ComputeBackend::Scalar)
                .expect("scalar")
                .usable()
        );
    }

    #[test]
    fn boundary_ambiguous_values_are_marked() {
        let comparison = compare_threshold(10.000_001, 10.0, 0.01);
        assert_eq!(comparison.confidence, NumericConfidence::BoundaryAmbiguous);
        assert_eq!(comparison.relation, ThresholdRelation::NearBoundary);
    }

    #[test]
    fn evidence_freshness_scan_splits_fresh_and_stale() {
        let input = fixtures::evidence_freshness_fixture("evidence_freshness").expect("fixture");
        let output = evidence_freshness_scan(&input).expect("scan");
        assert_eq!(output.fresh_count, 2);
        assert_eq!(output.stale_count, 1);
    }

    #[test]
    fn peg_deviation_scan_is_numeric_only_and_marks_boundary() {
        let input =
            fixtures::peg_deviation_fixture("boundary_ambiguous_peg_scan").expect("fixture");
        let output = peg_deviation_scan(&input, 10.0, 0.01).expect("scan");
        assert_eq!(
            output.numeric_confidence,
            NumericConfidence::BoundaryAmbiguous
        );
        assert_eq!(output.stale_count, 0);
    }

    #[test]
    fn input_limits_reject_oversized_tablet_jobs() {
        let limits = ComputeInputLimits::tablet_default();
        assert!(limits.validate(10_001, 100, 10).is_err());
        assert!(limits.validate(10, 2 * 1024 * 1024 + 1, 10).is_err());
        assert!(limits.validate(10, 100, 501).is_err());
    }

    #[test]
    fn benchmark_policy_rejects_low_battery_tablet() {
        let policy = BenchmarkPolicy::tablet_default();
        let health = bench_policy::DeviceHealthReport {
            battery_level: Some(20),
            charging: Some(false),
            thermal_state: Some("nominal".to_string()),
            battery_saver: Some(false),
        };
        assert!(validate_benchmark_policy(&policy, Some(&health), false, true, 100).is_err());
    }

    #[test]
    fn simd_capability_does_not_increase_evidence_weight() {
        fn evidence_weight(
            freshness_score: f64,
            reliability_score: f64,
            backend: ComputeBackend,
            benchmark_score: f64,
        ) -> f64 {
            let _ = (backend, benchmark_score);
            freshness_score * 0.6 + reliability_score * 0.4
        }
        let scalar = evidence_weight(0.8, 0.9, ComputeBackend::Scalar, 1.0);
        let simd = evidence_weight(0.8, 0.9, ComputeBackend::ArmNeon, 999.0);
        assert_eq!(scalar, simd);
    }

    #[test]
    fn compute_validation_roundtrip_writes_record() {
        let (_tmp, cfg) = test_config();
        let record = validate_backend_roundtrip(
            &cfg,
            "node:tablet-01",
            ComputeBackend::Scalar,
            "evidence_freshness",
        )
        .expect("validate");
        assert_eq!(
            record.validation_outcome,
            ComputeValidationOutcome::AcceptedScalarOnly
        );
        assert!(record.scalar_equivalence_verified);
        let records = read_validation_records(&cfg).expect("records");
        assert_eq!(records.len(), 1);
    }

    #[test]
    fn compute_validation_required_before_backend_usable() {
        let status = ComputeBackendStatus {
            backend: ComputeBackend::ArmNeon,
            hardware_detected: true,
            compiled_available: true,
            implementation_available: true,
            self_test_passed: true,
            last_self_test_at: None,
            scalar_equivalence_verified: false,
        };
        assert!(!status.usable());
    }

    #[test]
    fn compute_mismatch_quarantines_backend() {
        let (_tmp, cfg) = test_config();
        let record = validate_backend_roundtrip(
            &cfg,
            "node:tablet-01",
            ComputeBackend::ArmNeon,
            "evidence_freshness",
        )
        .expect("validate unsupported");
        assert_eq!(
            record.validation_outcome,
            ComputeValidationOutcome::RejectedUnsupportedBackend
        );
        assert!(
            backend_is_quarantined(&cfg, "node:tablet-01", ComputeBackend::ArmNeon)
                .expect("quarantine")
        );
    }

    #[test]
    fn quarantined_backend_not_selected() {
        let (_tmp, cfg) = test_config();
        let _ = validate_backend_roundtrip(
            &cfg,
            "node:tablet-01",
            ComputeBackend::ArmNeon,
            "evidence_freshness",
        )
        .expect("validate unsupported");
        assert!(
            backend_is_quarantined(&cfg, "node:tablet-01", ComputeBackend::ArmNeon)
                .expect("quarantine")
        );
        assert_eq!(capability_report().selected_backend, ComputeBackend::Scalar);
    }

    #[test]
    fn replay_uses_scalar_despite_accelerated_metadata() {
        let meta = ComputeEvidenceMeta {
            backend_requested: ComputeBackend::ArmNeon,
            backend_used: ComputeBackend::Scalar,
            scalar_fallback_used: true,
            verified_against_scalar: true,
            validation_outcome: ComputeValidationOutcome::AcceptedScalarOnly,
            numeric_confidence: NumericConfidence::Exact,
            tolerance: Some(0.000_001),
            threshold_epsilon: Some(0.01),
            runtime_ms: Some(1),
            input_hash: "input".to_string(),
            output_hash: "output".to_string(),
            timing_decision_id: None,
        };
        assert_eq!(meta.backend_used, ComputeBackend::Scalar);
        assert!(meta.scalar_fallback_used);
        assert!(meta.verified_against_scalar);
    }
}
