use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ComputeBackend {
    Scalar,
    AutoVectorizedScalar,
    ExperimentalPortableSimd,
    ArmNeon,
    X86Sse2,
    X86Avx2,
}

impl fmt::Display for ComputeBackend {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Scalar => "scalar",
            Self::AutoVectorizedScalar => "auto_vectorized_scalar",
            Self::ExperimentalPortableSimd => "experimental_portable_simd",
            Self::ArmNeon => "arm_neon",
            Self::X86Sse2 => "x86_sse2",
            Self::X86Avx2 => "x86_avx2",
        };
        f.write_str(value)
    }
}

impl FromStr for ComputeBackend {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "scalar" => Ok(Self::Scalar),
            "auto_vectorized_scalar" | "auto-vectorized-scalar" | "auto" => {
                Ok(Self::AutoVectorizedScalar)
            }
            "experimental_portable_simd" | "experimental-portable-simd" => {
                Ok(Self::ExperimentalPortableSimd)
            }
            "arm_neon" | "arm-neon" | "neon" => Ok(Self::ArmNeon),
            "x86_sse2" | "x86-sse2" | "sse2" => Ok(Self::X86Sse2),
            "x86_avx2" | "x86-avx2" | "avx2" => Ok(Self::X86Avx2),
            other => Err(anyhow!("unsupported compute backend '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ComputeBackendStatus {
    pub backend: ComputeBackend,
    pub hardware_detected: bool,
    pub compiled_available: bool,
    pub implementation_available: bool,
    pub self_test_passed: bool,
    pub last_self_test_at: Option<String>,
    pub scalar_equivalence_verified: bool,
}

impl ComputeBackendStatus {
    pub fn usable(&self) -> bool {
        self.hardware_detected
            && self.compiled_available
            && self.implementation_available
            && self.self_test_passed
            && self.scalar_equivalence_verified
    }
}
