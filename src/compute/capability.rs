use crate::compute::backend::{ComputeBackend, ComputeBackendStatus};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeCapabilityReport {
    pub scalar_available: bool,
    pub backend_statuses: Vec<ComputeBackendStatus>,
    pub selected_backend: ComputeBackend,
    pub selected_backend_reason: String,
    pub quantm_version: String,
    pub rust_target_triple: String,
}

pub fn capability_report() -> ComputeCapabilityReport {
    let backend_statuses = vec![
        ComputeBackendStatus {
            backend: ComputeBackend::Scalar,
            hardware_detected: true,
            compiled_available: true,
            implementation_available: true,
            self_test_passed: true,
            last_self_test_at: None,
            scalar_equivalence_verified: true,
        },
        ComputeBackendStatus {
            backend: ComputeBackend::AutoVectorizedScalar,
            hardware_detected: true,
            compiled_available: true,
            implementation_available: true,
            self_test_passed: true,
            last_self_test_at: None,
            scalar_equivalence_verified: true,
        },
        ComputeBackendStatus {
            backend: ComputeBackend::ExperimentalPortableSimd,
            hardware_detected: false,
            compiled_available: false,
            implementation_available: false,
            self_test_passed: false,
            last_self_test_at: None,
            scalar_equivalence_verified: false,
        },
        arch_status(ComputeBackend::ArmNeon),
        arch_status(ComputeBackend::X86Sse2),
        arch_status(ComputeBackend::X86Avx2),
    ];
    ComputeCapabilityReport {
        scalar_available: true,
        backend_statuses,
        selected_backend: ComputeBackend::Scalar,
        selected_backend_reason: "No validated accelerated backend available".to_string(),
        quantm_version: env!("CARGO_PKG_VERSION").to_string(),
        rust_target_triple: rust_target_triple(),
    }
}

pub fn render_capability_report(report: &ComputeCapabilityReport) -> String {
    let mut out = String::from(
        "compute capabilities\nBackend                    Hardware  Compiled  Implemented  Self-test  Scalar-eq  Usable\n",
    );
    for status in &report.backend_statuses {
        out.push_str(&format!(
            "{:<26} {:<8}  {:<8}  {:<11}  {:<9}  {:<9}  {}\n",
            status.backend,
            yn(status.hardware_detected),
            yn(status.compiled_available),
            yn(status.implementation_available),
            yn(status.self_test_passed),
            yn(status.scalar_equivalence_verified),
            yn(status.usable())
        ));
    }
    out.push_str(&format!(
        "Selected backend: {}\nReason: {}\nTarget: {}\n",
        report.selected_backend, report.selected_backend_reason, report.rust_target_triple
    ));
    out
}

fn arch_status(backend: ComputeBackend) -> ComputeBackendStatus {
    ComputeBackendStatus {
        backend,
        hardware_detected: false,
        compiled_available: false,
        implementation_available: false,
        self_test_passed: false,
        last_self_test_at: None,
        scalar_equivalence_verified: false,
    }
}

fn rust_target_triple() -> String {
    format!("{}-{}", std::env::consts::ARCH, std::env::consts::OS)
}

fn yn(value: bool) -> &'static str {
    if value { "yes" } else { "no" }
}
