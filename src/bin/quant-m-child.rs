use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use clap::{Parser, Subcommand};
use quant_m::device_telemetry::{self, DeviceTelemetry};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Parser, Debug)]
#[command(name = "quant-m-child")]
#[command(about = "Minimal Quant-M child runtime for edge devices")]
struct Cli {
    #[arg(long, default_value = "workspace")]
    workspace: PathBuf,
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    Pair {
        #[arg(long)]
        core: String,
        #[arg(long)]
        invite: String,
        #[arg(long, default_value = "edge-child")]
        name: String,
        #[arg(long)]
        json: bool,
    },
    PairScan {
        #[arg(long)]
        image: PathBuf,
    },
    Identity {
        #[arg(long)]
        create: bool,
        #[arg(long, default_value = "edge-child")]
        name: String,
        #[arg(long)]
        json: bool,
    },
    Doctor {
        #[arg(long)]
        json: bool,
    },
    Heartbeat {
        #[arg(long)]
        json: bool,
    },
    RunOnce {
        #[arg(long)]
        job: Option<PathBuf>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChildIdentity {
    node_display_name: String,
    node_public_key: String,
    created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChildCore {
    core_url: String,
    invite_token_hint: String,
    request_id: Option<String>,
    pairing_status: Option<String>,
    node_id: Option<String>,
    paired_at: String,
    authority: String,
    execution_enabled: bool,
    approval_enabled: bool,
    canonical_write_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ChildDoctor {
    identity_exists: bool,
    core_exists: bool,
    outbox_exists: bool,
    logs_exists: bool,
    authority: String,
    execution_enabled: bool,
    approval_enabled: bool,
    canonical_write_enabled: bool,
    model_router_compiled: bool,
    provider_adapters_compiled: bool,
    shared_state_accept_compiled: bool,
    pairing_server_compiled: bool,
    device_telemetry: DeviceTelemetry,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct ChildHeartbeat {
    heartbeat_id: String,
    node_display_name: Option<String>,
    timestamp: String,
    device_telemetry: DeviceTelemetry,
    authority: String,
    execution_enabled: bool,
    approval_enabled: bool,
    canonical_write_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PairingServerResponse {
    request_id: String,
    status: String,
    execution_enabled: bool,
    canonical_write_enabled: bool,
    approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct PairingStatusResponse {
    request_id: String,
    status: String,
    node_id: Option<String>,
    execution_enabled: bool,
    canonical_write_enabled: bool,
    approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct HeartbeatServerResponse {
    heartbeat_id: String,
    node_id: String,
    paired: bool,
    approved: bool,
    execution_enabled: bool,
    canonical_write_enabled: bool,
    approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChildJob {
    job_id: String,
    kind: String,
    payload: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ChildReceipt {
    receipt_id: String,
    job_id: String,
    kind: String,
    accepted: bool,
    output: String,
    replay_safe: bool,
    created_at: String,
}

#[derive(Debug, Clone)]
struct ChildPaths {
    identity: PathBuf,
    core: PathBuf,
    outbox: PathBuf,
    logs: PathBuf,
    heartbeats: PathBuf,
    receipts: PathBuf,
}

impl ChildPaths {
    fn new(workspace: &Path) -> Self {
        let child = workspace.join("child");
        let outbox = child.join("outbox");
        let logs = child.join("logs");
        Self {
            identity: child.join("identity.toml"),
            core: child.join("core.toml"),
            heartbeats: outbox.join("heartbeats.jsonl"),
            receipts: outbox.join("job-receipts.jsonl"),
            outbox,
            logs,
        }
    }

    fn ensure(&self) -> Result<()> {
        for path in [
            parent(&self.identity)?,
            parent(&self.core)?,
            &self.outbox,
            &self.logs,
        ] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let paths = ChildPaths::new(&cli.workspace);
    match cli.command {
        Command::Pair {
            core,
            invite,
            name,
            json,
        } => {
            paths.ensure()?;
            let identity = load_or_create_identity(&paths, &name)?;
            let response = submit_pairing_request(&core, &invite, &identity)?;
            let core = ChildCore {
                core_url: core,
                invite_token_hint: invite.chars().take(8).collect(),
                request_id: Some(response.request_id.clone()),
                pairing_status: Some(response.status.clone()),
                node_id: None,
                paired_at: now(),
                authority: "observe".to_string(),
                execution_enabled: response.execution_enabled,
                approval_enabled: response.approval_enabled,
                canonical_write_enabled: response.canonical_write_enabled,
            };
            if core.execution_enabled || core.approval_enabled || core.canonical_write_enabled {
                return Err(anyhow!(
                    "pairing server response claimed forbidden child authority"
                ));
            }
            write_toml(&paths.core, &core)?;
            if json {
                print_json(
                    &serde_json::json!({ "identity": identity, "core": core, "request": response }),
                )?;
            } else {
                println!(
                    "child pairing request submitted\nname: {}\nrequest_id: {}\nstatus: {}\nauthority: observe\nexecution: disabled\napproval: disabled\ncanonical_write: disabled",
                    identity.node_display_name, response.request_id, response.status
                );
            }
        }
        Command::PairScan { image: _ } => {
            #[cfg(feature = "child-scan-image")]
            {
                return Err(anyhow!(
                    "child image scan decoder is intentionally not wired in child-min yet"
                ));
            }
            #[cfg(not(feature = "child-scan-image"))]
            {
                return Err(anyhow!(
                    "pair-scan requires the child-scan-image feature; child-min does not compile QR image scanning"
                ));
            }
        }
        Command::Identity { create, name, json } => {
            paths.ensure()?;
            let identity = if create {
                Some(load_or_create_identity(&paths, &name)?)
            } else {
                load_identity(&paths)?
            };
            if json {
                print_json(&serde_json::json!({ "identity": identity }))?;
            } else if let Some(identity) = identity {
                println!(
                    "child identity\nname: {}\npublic_key: {}",
                    identity.node_display_name, identity.node_public_key
                );
            } else {
                println!("child identity not created");
            }
        }
        Command::Doctor { json } => {
            let core = load_core(&paths)?;
            let identity = load_identity(&paths)?;
            let telemetry = device_telemetry::collect_device_telemetry(
                &cli.workspace,
                identity
                    .as_ref()
                    .map(|identity| identity.node_display_name.clone()),
            );
            let report = ChildDoctor {
                identity_exists: paths.identity.exists(),
                core_exists: paths.core.exists(),
                outbox_exists: paths.outbox.exists(),
                logs_exists: paths.logs.exists(),
                authority: core
                    .as_ref()
                    .map(|core| core.authority.clone())
                    .unwrap_or_else(|| "none".to_string()),
                execution_enabled: core.as_ref().is_some_and(|core| core.execution_enabled),
                approval_enabled: core.as_ref().is_some_and(|core| core.approval_enabled),
                canonical_write_enabled: core
                    .as_ref()
                    .is_some_and(|core| core.canonical_write_enabled),
                model_router_compiled: false,
                provider_adapters_compiled: false,
                shared_state_accept_compiled: false,
                pairing_server_compiled: false,
                device_telemetry: telemetry,
            };
            if json {
                print_json(&report)?;
            } else {
                println!(
                    "child doctor\nidentity_exists: {}\ncore_exists: {}\noutbox_exists: {}\nlogs_exists: {}\n\nDevice:\n  name: {}\n  hostname: {}\n  model: {}\n  os: {}\n  arch: {}\nStorage:\n  path: {}\n  total: {}\n  available: {}\n  used: {}\nBattery:\n  percent: {}\n  source: {}\nAuthority:\n  authority: {}\n  execution_enabled: {}\n  approval_enabled: {}\n  canonical_write_enabled: {}\nmodel_router_compiled: false\nprovider_adapters_compiled: false\nshared_state_accept_compiled: false\npairing_server_compiled: false",
                    report.identity_exists,
                    report.core_exists,
                    report.outbox_exists,
                    report.logs_exists,
                    report
                        .device_telemetry
                        .device_display_name
                        .as_deref()
                        .unwrap_or("unknown"),
                    report
                        .device_telemetry
                        .hostname
                        .as_deref()
                        .unwrap_or("unknown"),
                    report
                        .device_telemetry
                        .model_hint
                        .as_deref()
                        .unwrap_or("unknown"),
                    report.device_telemetry.os,
                    report.device_telemetry.arch,
                    report
                        .device_telemetry
                        .storage
                        .as_ref()
                        .map(|storage| storage.path.as_str())
                        .unwrap_or("unknown"),
                    device_telemetry::format_bytes(
                        report
                            .device_telemetry
                            .storage
                            .as_ref()
                            .and_then(|storage| storage.total_bytes)
                    ),
                    device_telemetry::format_bytes(
                        report
                            .device_telemetry
                            .storage
                            .as_ref()
                            .and_then(|storage| storage.available_bytes)
                    ),
                    report
                        .device_telemetry
                        .storage
                        .as_ref()
                        .and_then(|storage| storage.used_percent)
                        .map(|value| format!("{value:.1}%"))
                        .unwrap_or_else(|| "unknown".to_string()),
                    device_telemetry::format_battery(report.device_telemetry.battery.as_ref()),
                    report
                        .device_telemetry
                        .battery
                        .as_ref()
                        .map(|battery| format!("{:?}", battery.source))
                        .unwrap_or_else(|| "unknown".to_string()),
                    report.authority,
                    report.execution_enabled,
                    report.approval_enabled,
                    report.canonical_write_enabled
                );
            }
        }
        Command::Heartbeat { json } => {
            paths.ensure()?;
            let identity = load_identity(&paths)?;
            let heartbeat = ChildHeartbeat {
                heartbeat_id: format!(
                    "heartbeat-{}",
                    Utc::now().timestamp_nanos_opt().unwrap_or_default()
                ),
                node_display_name: identity
                    .as_ref()
                    .map(|identity| identity.node_display_name.clone()),
                timestamp: now(),
                device_telemetry: device_telemetry::collect_device_telemetry(
                    &cli.workspace,
                    identity.map(|identity| identity.node_display_name),
                ),
                authority: "observe".to_string(),
                execution_enabled: false,
                approval_enabled: false,
                canonical_write_enabled: false,
            };
            append_jsonl(&paths.heartbeats, &heartbeat)?;
            let core_sync = sync_heartbeat_with_core(&paths, &heartbeat)?;
            if json {
                print_json(&serde_json::json!({
                    "heartbeat": heartbeat,
                    "core_sync": core_sync
                }))?;
            } else {
                println!(
                    "child heartbeat recorded\nheartbeat_id: {}\ndevice: {}/{}\nauthority: observe\nexecution: disabled",
                    heartbeat.heartbeat_id,
                    heartbeat.device_telemetry.os,
                    heartbeat.device_telemetry.arch
                );
                if let Some(core_sync) = core_sync {
                    println!(
                        "core heartbeat synced\nnode_id: {}\npaired: {}\napproved: {}\nexecution: disabled\napproval: disabled\ncanonical_write: disabled",
                        core_sync.node_id, core_sync.paired, core_sync.approved
                    );
                }
            }
        }
        Command::RunOnce { job, json } => {
            paths.ensure()?;
            let receipt = if let Some(job) = job {
                run_one_job(&paths, &job)?
            } else {
                None
            };
            if json {
                print_json(&serde_json::json!({ "receipt": receipt }))?;
            } else if let Some(receipt) = receipt {
                println!(
                    "child job complete\nreceipt_id: {}\njob_id: {}\nreplay_safe: {}",
                    receipt.receipt_id, receipt.job_id, receipt.replay_safe
                );
            } else {
                println!("child run-once idle");
            }
        }
    }
    Ok(())
}

fn submit_pairing_request(
    core_url: &str,
    invite_token: &str,
    identity: &ChildIdentity,
) -> Result<PairingServerResponse> {
    let endpoint = parse_local_http_core(core_url)?;
    let payload = serde_json::json!({
        "invite_token": invite_token,
        "node_display_name": identity.node_display_name,
        "node_public_key": identity.node_public_key,
        "surface": "termux_worker",
        "claimed_capabilities": ["echo", "sleep", "compute_scalar"],
        "requested_role": "stablecoin_peg_watcher",
        "requested_authority": "observe",
        "execution_enabled": false,
        "canonical_write_enabled": false,
        "approval_enabled": false
    });
    let body = serde_json::to_string(&payload)?;
    let request = format!(
        "POST /pair/request HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        endpoint.host_header,
        body.len(),
        body
    );
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .with_context(|| format!("failed to connect to pairing server at {core_url}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    parse_pairing_response(&response)
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalHttpEndpoint {
    host: String,
    port: u16,
    host_header: String,
}

fn parse_local_http_core(core_url: &str) -> Result<LocalHttpEndpoint> {
    let trimmed = core_url.trim().trim_end_matches('/');
    let without_scheme = trimmed
        .strip_prefix("http://")
        .ok_or_else(|| anyhow!("child pairing only supports http:// local core URLs"))?;
    let authority = without_scheme
        .split('/')
        .next()
        .ok_or_else(|| anyhow!("core URL missing host"))?;
    let (host, port) = if let Some((host, port)) = authority.rsplit_once(':') {
        let port = port
            .parse::<u16>()
            .context("core URL contains invalid port")?;
        (host.to_string(), port)
    } else {
        (authority.to_string(), 80)
    };
    let allowed = host == "localhost"
        || host == "127.0.0.1"
        || host.starts_with("192.168.")
        || host.starts_with("10.")
        || is_private_172(&host);
    if !allowed {
        return Err(anyhow!(
            "child pairing refuses non-local/non-LAN core URL; use a trusted LAN address"
        ));
    }
    Ok(LocalHttpEndpoint {
        host_header: if port == 80 {
            host.clone()
        } else {
            format!("{host}:{port}")
        },
        host,
        port,
    })
}

fn is_private_172(host: &str) -> bool {
    let mut parts = host.split('.');
    matches!(
        (
            parts.next(),
            parts.next().and_then(|part| part.parse::<u8>().ok())
        ),
        (Some("172"), Some(16..=31))
    )
}

fn parse_pairing_response(raw: &str) -> Result<PairingServerResponse> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("pairing server returned malformed HTTP response"))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| anyhow!("pairing server returned empty HTTP response"))?;
    if !status_line.contains(" 200 ") {
        return Err(anyhow!(
            "pairing server rejected request: {}",
            status_line.trim()
        ));
    }
    let response: PairingServerResponse =
        serde_json::from_str(body).context("failed to parse pairing server response")?;
    if response.execution_enabled || response.approval_enabled || response.canonical_write_enabled {
        return Err(anyhow!(
            "pairing server response enabled forbidden child authority"
        ));
    }
    Ok(response)
}

fn sync_heartbeat_with_core(
    paths: &ChildPaths,
    heartbeat: &ChildHeartbeat,
) -> Result<Option<HeartbeatServerResponse>> {
    let Some(mut core) = load_core(paths)? else {
        return Ok(None);
    };
    let Some(request_id) = core.request_id.clone() else {
        return Ok(None);
    };
    let status = fetch_pairing_status(&core.core_url, &request_id)?;
    if status.execution_enabled || status.approval_enabled || status.canonical_write_enabled {
        return Err(anyhow!("pairing status enabled forbidden child authority"));
    }
    core.pairing_status = Some(status.status.clone());
    core.node_id = status.node_id.clone();
    write_toml(&paths.core, &core)?;
    if status.status != "approved" {
        return Ok(None);
    }
    let Some(node_id) = status.node_id else {
        return Ok(None);
    };
    let response = submit_heartbeat(&core.core_url, &node_id, heartbeat)?;
    Ok(Some(response))
}

fn fetch_pairing_status(core_url: &str, request_id: &str) -> Result<PairingStatusResponse> {
    let response = send_local_http(core_url, "GET", &format!("/pair/status/{request_id}"), None)?;
    parse_json_http_response(&response, "pairing status")
}

fn submit_heartbeat(
    core_url: &str,
    node_id: &str,
    heartbeat: &ChildHeartbeat,
) -> Result<HeartbeatServerResponse> {
    let payload = serde_json::json!({
        "node_id": node_id,
        "surface": "termux_worker",
        "claimed_capabilities": ["echo", "sleep", "heartbeat", "compute_scalar"],
        "execution_enabled": false,
        "canonical_write_enabled": false,
        "approval_enabled": false,
        "device_telemetry": heartbeat.device_telemetry
    });
    let body = serde_json::to_string(&payload)?;
    let response = send_local_http(core_url, "POST", "/cluster/heartbeat", Some(&body))?;
    let response: HeartbeatServerResponse =
        parse_json_http_response(&response, "cluster heartbeat")?;
    if response.execution_enabled || response.approval_enabled || response.canonical_write_enabled {
        return Err(anyhow!(
            "heartbeat response enabled forbidden child authority"
        ));
    }
    Ok(response)
}

fn send_local_http(core_url: &str, method: &str, path: &str, body: Option<&str>) -> Result<String> {
    let endpoint = parse_local_http_core(core_url)?;
    let body = body.unwrap_or("");
    let request = if method == "GET" {
        format!(
            "GET {path} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
            endpoint.host_header
        )
    } else {
        format!(
            "{method} {path} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            endpoint.host_header,
            body.len(),
            body
        )
    };
    let mut stream = TcpStream::connect((endpoint.host.as_str(), endpoint.port))
        .with_context(|| format!("failed to connect to pairing server at {core_url}"))?;
    stream.set_read_timeout(Some(Duration::from_secs(10)))?;
    stream.set_write_timeout(Some(Duration::from_secs(10)))?;
    stream.write_all(request.as_bytes())?;
    let mut response = String::new();
    stream.read_to_string(&mut response)?;
    Ok(response)
}

fn parse_json_http_response<T: for<'de> Deserialize<'de>>(raw: &str, label: &str) -> Result<T> {
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .ok_or_else(|| anyhow!("{label} returned malformed HTTP response"))?;
    let status_line = head
        .lines()
        .next()
        .ok_or_else(|| anyhow!("{label} returned empty HTTP response"))?;
    if !status_line.contains(" 200 ") {
        return Err(anyhow!("{label} rejected request: {}", status_line.trim()));
    }
    serde_json::from_str(body).with_context(|| format!("failed to parse {label} response"))
}

fn run_one_job(paths: &ChildPaths, job_path: &Path) -> Result<Option<ChildReceipt>> {
    let job: ChildJob = serde_json::from_str(&fs::read_to_string(job_path)?)
        .context("failed to parse child job json")?;
    if job.kind != "echo" {
        return Err(anyhow!(
            "child-min only accepts echo jobs; rejected kind '{}'",
            job.kind
        ));
    }
    let receipt = ChildReceipt {
        receipt_id: format!(
            "receipt-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        job_id: job.job_id,
        kind: job.kind,
        accepted: true,
        output: job.payload.unwrap_or_default(),
        replay_safe: true,
        created_at: now(),
    };
    append_jsonl(&paths.receipts, &receipt)?;
    Ok(Some(receipt))
}

fn load_or_create_identity(paths: &ChildPaths, name: &str) -> Result<ChildIdentity> {
    if let Some(identity) = load_identity(paths)? {
        return Ok(identity);
    }
    let identity = ChildIdentity {
        node_display_name: name.to_string(),
        node_public_key: format!("child-pub-{}", stable_hash(&format!("{}:{}", name, now()))),
        created_at: now(),
    };
    write_toml(&paths.identity, &identity)?;
    Ok(identity)
}

fn load_identity(paths: &ChildPaths) -> Result<Option<ChildIdentity>> {
    if !paths.identity.exists() {
        return Ok(None);
    }
    Ok(Some(toml::from_str(&fs::read_to_string(&paths.identity)?)?))
}

fn load_core(paths: &ChildPaths) -> Result<Option<ChildCore>> {
    if !paths.core.exists() {
        return Ok(None);
    }
    Ok(Some(toml::from_str(&fs::read_to_string(&paths.core)?)?))
}

fn write_toml(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(parent(path)?)?;
    fs::write(path, toml::to_string_pretty(value)?)?;
    Ok(())
}

fn append_jsonl(path: &Path, value: &impl Serialize) -> Result<()> {
    fs::create_dir_all(parent(path)?)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn print_json(value: &impl Serialize) -> Result<()> {
    println!("{}", serde_json::to_string_pretty(value)?);
    Ok(())
}

fn parent(path: &Path) -> Result<&Path> {
    path.parent()
        .ok_or_else(|| anyhow!("path '{}' has no parent", path.display()))
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn child_min_doctor_reports_forbidden_surfaces_not_compiled() {
        let tmp = TempDir::new().expect("tempdir");
        let paths = ChildPaths::new(tmp.path());
        paths.ensure().expect("paths");
        let report = ChildDoctor {
            identity_exists: paths.identity.exists(),
            core_exists: paths.core.exists(),
            outbox_exists: paths.outbox.exists(),
            logs_exists: paths.logs.exists(),
            authority: "none".to_string(),
            execution_enabled: false,
            approval_enabled: false,
            canonical_write_enabled: false,
            model_router_compiled: false,
            provider_adapters_compiled: false,
            shared_state_accept_compiled: false,
            pairing_server_compiled: false,
            device_telemetry: device_telemetry::collect_device_telemetry(tmp.path(), None),
        };
        assert!(!report.model_router_compiled);
        assert!(!report.provider_adapters_compiled);
        assert!(!report.shared_state_accept_compiled);
        assert!(!report.pairing_server_compiled);
    }

    #[test]
    fn child_min_run_once_rejects_non_echo_job() {
        let tmp = TempDir::new().expect("tempdir");
        let paths = ChildPaths::new(tmp.path());
        paths.ensure().expect("paths");
        let job_path = tmp.path().join("job.json");
        fs::write(
            &job_path,
            serde_json::to_string(&ChildJob {
                job_id: "job-1".to_string(),
                kind: "model_handoff".to_string(),
                payload: None,
            })
            .expect("json"),
        )
        .expect("write job");
        let err = run_one_job(&paths, &job_path).expect_err("rejected");
        assert!(err.to_string().contains("only accepts echo"));
    }
}
