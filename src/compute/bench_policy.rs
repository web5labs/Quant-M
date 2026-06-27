use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct ComputeInputLimits {
    pub max_observations: usize,
    pub max_bytes: usize,
    pub max_window_size: usize,
}

#[allow(dead_code)]
impl ComputeInputLimits {
    pub fn tablet_default() -> Self {
        Self {
            max_observations: 10_000,
            max_bytes: 2 * 1024 * 1024,
            max_window_size: 500,
        }
    }

    pub fn validate(&self, observations: usize, bytes: usize, window_size: usize) -> Result<()> {
        if observations > self.max_observations {
            return Err(anyhow!("compute input has too many observations"));
        }
        if bytes > self.max_bytes {
            return Err(anyhow!("compute input exceeds byte limit"));
        }
        if window_size > self.max_window_size {
            return Err(anyhow!("compute input window is too large"));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BenchmarkPolicy {
    pub manual_only: bool,
    pub min_battery_percent: Option<u8>,
    pub max_samples_tablet: u64,
    pub deny_when_on_battery: bool,
    pub deny_during_active_role: bool,
}

impl BenchmarkPolicy {
    pub fn tablet_default() -> Self {
        Self {
            manual_only: true,
            min_battery_percent: Some(25),
            max_samples_tablet: 10_000,
            deny_when_on_battery: false,
            deny_during_active_role: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceHealthReport {
    pub battery_level: Option<u8>,
    pub charging: Option<bool>,
    pub thermal_state: Option<String>,
    pub battery_saver: Option<bool>,
}

pub fn validate_benchmark_policy(
    policy: &BenchmarkPolicy,
    health: Option<&DeviceHealthReport>,
    active_role: bool,
    manual: bool,
    samples: u64,
) -> Result<()> {
    if policy.manual_only && !manual {
        return Err(anyhow!("benchmark jobs are manual-only"));
    }
    if samples > policy.max_samples_tablet {
        return Err(anyhow!("benchmark sample count exceeds tablet limit"));
    }
    if policy.deny_during_active_role && active_role {
        return Err(anyhow!("benchmark denied during active role lease"));
    }
    if let Some(health) = health {
        if health
            .thermal_state
            .as_deref()
            .is_some_and(|state| matches!(state, "hot" | "critical"))
        {
            return Err(anyhow!("benchmark denied by thermal state"));
        }
        if health.battery_saver == Some(true) {
            return Err(anyhow!("benchmark denied while battery saver is on"));
        }
        if let (Some(min), Some(level), Some(false)) = (
            policy.min_battery_percent,
            health.battery_level,
            health.charging,
        ) && level < min
        {
            return Err(anyhow!("benchmark denied by battery policy"));
        }
        if policy.deny_when_on_battery && health.charging == Some(false) {
            return Err(anyhow!("benchmark denied while on battery"));
        }
    }
    Ok(())
}
