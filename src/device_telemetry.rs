#![allow(dead_code)]

use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;

#[cfg(feature = "device-telemetry-termux")]
use std::process::{Command, Stdio};
#[cfg(feature = "device-telemetry-termux")]
use std::thread;
#[cfg(feature = "device-telemetry-termux")]
use std::time::{Duration as StdDuration, Instant};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceTelemetry {
    pub device_display_name: Option<String>,
    pub hostname: Option<String>,
    pub model_hint: Option<String>,
    pub os: String,
    pub arch: String,
    pub storage: Option<StorageTelemetry>,
    pub battery: Option<BatteryTelemetry>,
    pub collection_errors: Vec<String>,
    pub collected_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct StorageTelemetry {
    pub path: String,
    pub total_bytes: Option<u64>,
    pub available_bytes: Option<u64>,
    pub used_percent: Option<f64>,
    pub source: StorageTelemetrySource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum StorageTelemetrySource {
    UnixStatvfs,
    DfCommandFallback,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BatteryTelemetry {
    pub percent: Option<f64>,
    pub charging: Option<bool>,
    pub status: Option<String>,
    pub temperature_c: Option<f64>,
    pub health: Option<String>,
    pub source: BatteryTelemetrySource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BatteryTelemetrySource {
    TermuxBatteryStatus,
    LinuxPowerSupplySysfs,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceTelemetryWarningPolicy {
    pub low_storage_available_bytes: u64,
    pub low_battery_percent: f64,
}

impl Default for DeviceTelemetryWarningPolicy {
    fn default() -> Self {
        Self {
            low_storage_available_bytes: 1024 * 1024 * 1024,
            low_battery_percent: 20.0,
        }
    }
}

pub fn collect_device_telemetry(
    workspace: &Path,
    device_display_name: Option<String>,
) -> DeviceTelemetry {
    let mut collection_errors = Vec::new();
    let hostname = read_hostname().or_else(|| env::var("HOSTNAME").ok());
    let model_hint = read_first_trimmed(&[
        Path::new("/proc/device-tree/model"),
        Path::new("/sys/devices/virtual/dmi/id/product_name"),
    ]);
    let storage = collect_storage(workspace, &mut collection_errors);
    let battery = collect_battery(&mut collection_errors);
    DeviceTelemetry {
        device_display_name: device_display_name
            .or_else(|| hostname.clone())
            .filter(|value| !value.trim().is_empty()),
        hostname,
        model_hint,
        os: env::consts::OS.to_string(),
        arch: env::consts::ARCH.to_string(),
        storage: Some(storage),
        battery: Some(battery),
        collection_errors,
        collected_at: Utc::now().to_rfc3339(),
    }
}

fn collect_storage(path: &Path, collection_errors: &mut Vec<String>) -> StorageTelemetry {
    #[cfg(feature = "device-telemetry-df-fallback")]
    {
        if let Some(storage) = df_storage(path, collection_errors) {
            return storage;
        }
    }
    #[cfg(not(feature = "device-telemetry-df-fallback"))]
    {
        collection_errors
            .push("storage capacity unavailable without device-telemetry-df-fallback".to_string());
    }
    StorageTelemetry {
        path: path.display().to_string(),
        total_bytes: None,
        available_bytes: None,
        used_percent: None,
        source: StorageTelemetrySource::Unknown,
    }
}

#[cfg(feature = "device-telemetry-df-fallback")]
fn df_storage(path: &Path, collection_errors: &mut Vec<String>) -> Option<StorageTelemetry> {
    let output = std::process::Command::new("df")
        .arg("-Pk")
        .arg(path)
        .output();
    let output = match output {
        Ok(output) if output.status.success() => output,
        Ok(output) => {
            collection_errors.push(format!("df failed with status {}", output.status));
            return None;
        }
        Err(err) => {
            collection_errors.push(format!("df unavailable: {err}"));
            return None;
        }
    };
    let text = String::from_utf8_lossy(&output.stdout);
    let Some(line) = text.lines().nth(1) else {
        collection_errors.push("df output missing data line".to_string());
        return None;
    };
    let fields: Vec<_> = line.split_whitespace().collect();
    if fields.len() < 4 {
        collection_errors.push("df output has too few columns".to_string());
        return None;
    }
    let total_k = fields.get(1)?.parse::<u64>().ok()?;
    let used_k = fields.get(2)?.parse::<u64>().ok()?;
    let available_k = fields.get(3)?.parse::<u64>().ok()?;
    let total_bytes = total_k.saturating_mul(1024);
    let available_bytes = available_k.saturating_mul(1024);
    let used_percent = storage_used_percent(total_bytes, used_k.saturating_mul(1024));
    Some(StorageTelemetry {
        path: path.display().to_string(),
        total_bytes: Some(total_bytes),
        available_bytes: Some(available_bytes),
        used_percent,
        source: StorageTelemetrySource::DfCommandFallback,
    })
}

fn collect_battery(collection_errors: &mut Vec<String>) -> BatteryTelemetry {
    #[cfg(feature = "device-telemetry-termux")]
    {
        if let Some(battery) = termux_battery_status(collection_errors) {
            return battery;
        }
    }
    if let Some(battery) = linux_power_supply_battery(collection_errors) {
        return battery;
    }
    BatteryTelemetry {
        percent: None,
        charging: None,
        status: None,
        temperature_c: None,
        health: None,
        source: BatteryTelemetrySource::Unknown,
    }
}

fn linux_power_supply_battery(collection_errors: &mut Vec<String>) -> Option<BatteryTelemetry> {
    let root = Path::new("/sys/class/power_supply");
    let entries = fs::read_dir(root).ok()?;
    for entry in entries.flatten() {
        let path = entry.path();
        let kind = read_trimmed(&path.join("type")).unwrap_or_default();
        if !kind.eq_ignore_ascii_case("battery") {
            continue;
        }
        let percent = read_trimmed(&path.join("capacity")).and_then(|value| value.parse().ok());
        let status = read_trimmed(&path.join("status"));
        let charging = status.as_ref().map(|value| {
            matches!(
                value.to_ascii_lowercase().as_str(),
                "charging" | "full" | "not charging"
            )
        });
        let temperature_c = read_trimmed(&path.join("temp"))
            .and_then(|value| value.parse::<f64>().ok())
            .map(|raw| if raw > 1000.0 { raw / 10.0 } else { raw });
        let health = read_trimmed(&path.join("health"));
        return Some(BatteryTelemetry {
            percent,
            charging,
            status,
            temperature_c,
            health,
            source: BatteryTelemetrySource::LinuxPowerSupplySysfs,
        });
    }
    collection_errors.push("battery unavailable in /sys/class/power_supply".to_string());
    None
}

#[cfg(feature = "device-telemetry-termux")]
fn termux_battery_status(collection_errors: &mut Vec<String>) -> Option<BatteryTelemetry> {
    let mut child = match Command::new("termux-battery-status")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(err) => {
            collection_errors.push(format!("termux-battery-status unavailable: {err}"));
            return None;
        }
    };
    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed() < StdDuration::from_secs(2) => {
                thread::sleep(StdDuration::from_millis(50));
            }
            Ok(None) => {
                let _ = child.kill();
                collection_errors.push("termux-battery-status timed out".to_string());
                return None;
            }
            Err(err) => {
                collection_errors.push(format!("termux-battery-status wait failed: {err}"));
                return None;
            }
        }
    }
    match child.wait_with_output() {
        Ok(output) if output.status.success() => {
            let raw = String::from_utf8_lossy(&output.stdout);
            parse_termux_battery_status(&raw)
                .map_err(|err| {
                    collection_errors.push(err);
                })
                .ok()
        }
        Ok(output) => {
            collection_errors.push(format!(
                "termux-battery-status failed with status {}",
                output.status
            ));
            None
        }
        Err(err) => {
            collection_errors.push(format!("termux-battery-status output failed: {err}"));
            None
        }
    }
}

pub fn parse_termux_battery_status(raw: &str) -> Result<BatteryTelemetry, String> {
    let value: serde_json::Value =
        serde_json::from_str(raw).map_err(|err| format!("termux battery JSON invalid: {err}"))?;
    let percent = value
        .get("percentage")
        .or_else(|| value.get("percent"))
        .and_then(serde_json::Value::as_f64);
    let status = value
        .get("status")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    let charging = status.as_ref().map(|status| {
        matches!(
            status.to_ascii_lowercase().as_str(),
            "charging" | "full" | "not charging"
        )
    });
    let temperature_c = value
        .get("temperature")
        .or_else(|| value.get("temperature_c"))
        .and_then(serde_json::Value::as_f64);
    let health = value
        .get("health")
        .and_then(serde_json::Value::as_str)
        .map(str::to_string);
    Ok(BatteryTelemetry {
        percent,
        charging,
        status,
        temperature_c,
        health,
        source: BatteryTelemetrySource::TermuxBatteryStatus,
    })
}

pub fn storage_used_percent(total_bytes: u64, used_bytes: u64) -> Option<f64> {
    if total_bytes == 0 {
        return None;
    }
    Some((used_bytes as f64 / total_bytes as f64) * 100.0)
}

pub fn telemetry_warnings(
    node_id: &str,
    telemetry: &DeviceTelemetry,
    policy: &DeviceTelemetryWarningPolicy,
) -> Vec<String> {
    let mut warnings = Vec::new();
    if let Some(battery) = telemetry.battery.as_ref()
        && let Some(percent) = battery.percent
        && percent < policy.low_battery_percent
    {
        warnings.push(format!(
            "Warning: {node_id} battery below {:.0}%.",
            policy.low_battery_percent
        ));
    }
    if let Some(storage) = telemetry.storage.as_ref()
        && let Some(available) = storage.available_bytes
        && available < policy.low_storage_available_bytes
    {
        warnings.push(format!("Warning: {node_id} storage below 1 GiB."));
    }
    warnings
}

pub fn format_bytes(bytes: Option<u64>) -> String {
    let Some(bytes) = bytes else {
        return "unknown".to_string();
    };
    let gib = bytes as f64 / 1024.0 / 1024.0 / 1024.0;
    if gib >= 1.0 {
        format!("{gib:.1}GB")
    } else {
        let mib = bytes as f64 / 1024.0 / 1024.0;
        format!("{mib:.1}MB")
    }
}

pub fn format_battery(battery: Option<&BatteryTelemetry>) -> String {
    let Some(battery) = battery else {
        return "unknown".to_string();
    };
    let percent = battery
        .percent
        .map(|value| format!("{value:.0}%"))
        .unwrap_or_else(|| "unknown".to_string());
    let charging = battery
        .charging
        .map(|value| if value { " charging" } else { " not_charging" })
        .unwrap_or("");
    format!("{percent}{charging}")
}

fn read_hostname() -> Option<String> {
    read_trimmed(Path::new("/proc/sys/kernel/hostname"))
}

fn read_first_trimmed(paths: &[&Path]) -> Option<String> {
    paths.iter().find_map(|path| read_trimmed(path))
}

fn read_trimmed(path: &Path) -> Option<String> {
    fs::read_to_string(path)
        .ok()
        .map(|value| value.trim_matches(char::from(0)).trim().to_string())
        .filter(|value| !value.is_empty())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn device_telemetry_collects_os_arch() {
        let telemetry = collect_device_telemetry(Path::new("."), Some("tablet-01".to_string()));
        assert_eq!(telemetry.os, env::consts::OS);
        assert_eq!(telemetry.arch, env::consts::ARCH);
    }

    #[test]
    fn device_telemetry_reads_hostname_best_effort() {
        let telemetry = collect_device_telemetry(Path::new("."), None);
        assert!(!telemetry.os.is_empty());
    }

    #[test]
    fn device_telemetry_storage_unknown_does_not_fail() {
        let telemetry = collect_device_telemetry(&PathBuf::from("/definitely/not/quantm"), None);
        assert!(telemetry.storage.is_some());
    }

    #[test]
    fn device_telemetry_battery_unknown_does_not_fail() {
        let telemetry = collect_device_telemetry(Path::new("."), None);
        assert!(telemetry.battery.is_some());
    }

    #[test]
    fn termux_battery_status_fixture_parses_percent() {
        let battery = parse_termux_battery_status(
            r#"{"health":"GOOD","percentage":82,"plugged":"PLUGGED_AC","status":"CHARGING","temperature":29.5}"#,
        )
        .expect("battery");
        assert_eq!(battery.percent, Some(82.0));
        assert_eq!(battery.source, BatteryTelemetrySource::TermuxBatteryStatus);
    }

    #[test]
    fn termux_battery_status_fixture_parses_charging() {
        let battery =
            parse_termux_battery_status(r#"{"percentage":82,"status":"Charging"}"#).expect("ok");
        assert_eq!(battery.charging, Some(true));
    }

    #[test]
    fn termux_battery_status_malformed_json_is_error_not_panic() {
        let err = parse_termux_battery_status("not json").expect_err("invalid");
        assert!(err.contains("invalid"));
    }

    #[test]
    fn storage_used_percent_calculates_correctly() {
        assert_eq!(storage_used_percent(100, 25), Some(25.0));
    }

    #[test]
    fn storage_zero_total_does_not_panic() {
        assert_eq!(storage_used_percent(0, 25), None);
    }

    #[test]
    fn low_battery_warning_is_advisory_only() {
        let telemetry = DeviceTelemetry {
            device_display_name: Some("tablet-01".to_string()),
            hostname: None,
            model_hint: None,
            os: "android".to_string(),
            arch: "aarch64".to_string(),
            storage: None,
            battery: Some(BatteryTelemetry {
                percent: Some(10.0),
                charging: Some(false),
                status: Some("Discharging".to_string()),
                temperature_c: None,
                health: None,
                source: BatteryTelemetrySource::Unknown,
            }),
            collection_errors: vec![],
            collected_at: Utc::now().to_rfc3339(),
        };
        assert_eq!(
            telemetry_warnings(
                "node:tablet-01",
                &telemetry,
                &DeviceTelemetryWarningPolicy::default()
            )
            .len(),
            1
        );
    }
}
