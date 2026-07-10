use anyhow::{Context, Result};
use if_addrs::get_if_addrs;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::config::Config;

const DEFAULT_BIND: &str = "0.0.0.0:8787";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PairRequestStatus {
    Pending,
    Approved,
    Denied,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChildStatus {
    Approved,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildAuthority {
    pub authority: String,
    pub provider_calls_allowed: bool,
    pub execution_allowed: bool,
    pub canonical_write_allowed: bool,
    pub approval_allowed: bool,
}

impl ChildAuthority {
    pub fn observe_only() -> Self {
        Self {
            authority: "observe-only".to_string(),
            provider_calls_allowed: false,
            execution_allowed: false,
            canonical_write_allowed: false,
            approval_allowed: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairInvite {
    pub invite_id: String,
    pub core_name: String,
    pub core_fingerprint: String,
    pub bind: String,
    pub local_url: String,
    pub created_at: u64,
    pub expires_at: u64,
    pub manual_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairRequest {
    pub request_id: String,
    pub invite_id: String,
    pub claimed_device_name: String,
    pub claimed_role: String,
    pub claimed_surface: String,
    pub runtime_kind: String,
    pub device_class: String,
    pub os: Option<String>,
    pub architecture: Option<String>,
    pub requested_authority: String,
    pub requested_at: u64,
    pub core_url: String,
    pub child_fingerprint: Option<String>,
    pub status: PairRequestStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildRecord {
    pub node_id: String,
    pub request_id: String,
    pub display_name: String,
    pub role: String,
    pub surface: String,
    pub authority: ChildAuthority,
    pub approved_at: u64,
    pub approved_by: String,
    pub revoked_at: Option<u64>,
    pub last_heartbeat: Option<u64>,
    pub active_pack_hash: Option<String>,
    pub status: ChildStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatHealth {
    Healthy,
    Stale,
    Pending,
    Revoked,
    Unknown,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildHeartbeatPayload {
    pub node_id: String,
    pub request_id: Option<String>,
    pub child_fingerprint: String,
    pub device_name: String,
    pub claimed_role: String,
    pub authority: ChildAuthority,
    pub timestamp: u64,
    pub os: String,
    pub architecture: String,
    pub runtime_surface: String,
    pub child_binary_version: String,
    pub core_url: String,
    pub active_pack_hash: Option<String>,
    pub battery_status: Option<String>,
    pub storage_status: Option<String>,
    pub network_status: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildHeartbeatRecord {
    pub node_id: String,
    pub request_id: Option<String>,
    pub child_fingerprint: String,
    pub last_heartbeat: u64,
    pub heartbeat_hash: String,
    pub health: HeartbeatHealth,
    pub active_pack_hash: Option<String>,
    pub authority: ChildAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChildHeartbeatReport {
    pub accepted: bool,
    pub health: HeartbeatHealth,
    pub record: Option<ChildHeartbeatRecord>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairAuditEvent {
    pub event_id: String,
    pub event_type: String,
    pub timestamp: u64,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairStatusReport {
    pub server_status: String,
    pub bind: String,
    pub port: u16,
    pub pending_request_count: usize,
    pub approved_child_count: usize,
    pub revoked_child_count: usize,
    pub healthy_child_count: usize,
    pub stale_child_count: usize,
    pub pending_child_count: usize,
    pub unknown_child_count: usize,
    pub denied_child_count: usize,
    pub last_audit_event: Option<PairAuditEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairCockpitReport {
    pub role: String,
    pub workspace: PathBuf,
    pub bind: String,
    pub port: u16,
    pub local_url: String,
    pub selected_advertise_host: String,
    pub detected_addresses: Vec<AdvertiseCandidate>,
    pub ignored_addresses: Vec<AdvertiseCandidate>,
    pub child_test_command: String,
    pub qr_rendered: bool,
    pub qr_warning: Option<String>,
    pub pending_request_count: usize,
    pub approved_child_count: usize,
    pub revoked_child_count: usize,
    pub safety: ChildAuthority,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeviceAddReport {
    pub invite: PairInvite,
    pub selected_advertise_host: String,
    pub detected_addresses: Vec<AdvertiseCandidate>,
    pub ignored_addresses: Vec<AdvertiseCandidate>,
    pub child_test_command: String,
    pub qr_rendered: bool,
    pub qr_warning: Option<String>,
    pub manual_fallback_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdvertiseCandidate {
    pub interface: String,
    pub host: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairDoctorReport {
    pub bind: String,
    pub port: u16,
    pub selected_advertise_host: Option<String>,
    pub selected_url: Option<String>,
    pub detected_addresses: Vec<AdvertiseCandidate>,
    pub ignored_addresses: Vec<AdvertiseCandidate>,
    pub wifi_addresses_found: bool,
    pub only_loopback_found: bool,
    pub port_available: bool,
    pub firewall_warning: String,
    pub same_network_explanation: String,
    pub child_test_command: Option<String>,
    pub guidance: Vec<String>,
}

#[derive(Debug, Clone, Default)]
pub struct AdvertiseOptions {
    pub host: Option<String>,
    pub interface: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildListReport {
    pub pending: Vec<PairRequest>,
    pub approved: Vec<ChildRecord>,
    pub revoked: Vec<ChildRecord>,
    pub health: Vec<ChildHeartbeatRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct JoinMetadata {
    pub core_url: String,
    pub invite_id: String,
    pub core_name: String,
    pub core_fingerprint: String,
    pub expires_at: u64,
    pub pair_request_url: String,
    pub max_authority: String,
    pub manual_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildIdentity {
    pub identity_id: String,
    pub child_fingerprint: String,
    pub display_name: String,
    pub os: String,
    pub architecture: String,
    pub runtime_surface: String,
    pub created_at: u64,
    pub last_joined_core_url: Option<String>,
    pub last_invite_id: Option<String>,
    #[serde(default)]
    pub last_request_id: Option<String>,
    #[serde(default)]
    pub approved_node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildIdentityReport {
    pub identity: ChildIdentity,
    pub identity_file: PathBuf,
    pub secret_scan_status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildJoinReport {
    pub metadata: JoinMetadata,
    pub identity: ChildIdentity,
    pub request: PairRequest,
    pub approval_command: String,
    pub camera_scanning_available: bool,
    pub safety: ChildAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairRequestInput {
    pub invite_id: String,
    pub claimed_device_name: String,
    pub claimed_role: String,
    pub claimed_surface: String,
    pub runtime_kind: String,
    pub device_class: String,
    pub os: Option<String>,
    pub architecture: Option<String>,
    pub requested_authority: String,
    pub core_url: String,
    pub child_fingerprint: Option<String>,
}

#[derive(Debug, Clone)]
pub struct PairPaths {
    root: PathBuf,
    invites: PathBuf,
    requests: PathBuf,
    children: PathBuf,
    heartbeats: PathBuf,
    audit: PathBuf,
}

#[derive(Debug, Clone)]
struct ChildPaths {
    root: PathBuf,
    identity: PathBuf,
    audit: PathBuf,
}

impl ChildPaths {
    fn new(cfg: &Config) -> Self {
        let root = cfg.workspace_dir.join("state/child");
        Self {
            identity: root.join("identity.json"),
            audit: root.join("audit.ndjson"),
            root,
        }
    }
}

impl PairPaths {
    pub fn new(cfg: &Config) -> Self {
        let root = cfg.workspace_dir.join("state/pairing");
        Self {
            invites: root.join("invites"),
            requests: root.join("requests"),
            children: root.join("children"),
            heartbeats: root.join("heartbeats"),
            audit: root.join("audit.ndjson"),
            root,
        }
    }

    fn ensure(&self) -> Result<()> {
        fs::create_dir_all(&self.invites)
            .with_context(|| format!("failed to create {}", self.invites.display()))?;
        fs::create_dir_all(&self.requests)
            .with_context(|| format!("failed to create {}", self.requests.display()))?;
        fs::create_dir_all(&self.children)
            .with_context(|| format!("failed to create {}", self.children.display()))?;
        fs::create_dir_all(&self.heartbeats)
            .with_context(|| format!("failed to create {}", self.heartbeats.display()))?;
        Ok(())
    }

    fn invite_path(&self, invite_id: &str) -> PathBuf {
        self.invites.join(format!("{}.json", safe_id(invite_id)))
    }

    fn request_path(&self, request_id: &str) -> PathBuf {
        self.requests.join(format!("{}.json", safe_id(request_id)))
    }

    fn child_path(&self, node_id: &str) -> PathBuf {
        self.children.join(format!("{}.json", safe_id(node_id)))
    }

    fn heartbeat_path(&self, node_id: &str) -> PathBuf {
        self.heartbeats.join(format!("{}.json", safe_id(node_id)))
    }
}

#[allow(dead_code)]
pub fn default_bind() -> &'static str {
    DEFAULT_BIND
}

#[allow(dead_code)]
pub fn create_invite(
    cfg: &Config,
    bind: &str,
    ttl_minutes: u64,
    qr: bool,
    dry_run: bool,
) -> Result<DeviceAddReport> {
    create_invite_with_options(
        cfg,
        bind,
        ttl_minutes,
        qr,
        dry_run,
        &AdvertiseOptions::default(),
    )
}

pub fn create_invite_with_options(
    cfg: &Config,
    bind: &str,
    ttl_minutes: u64,
    qr: bool,
    dry_run: bool,
    options: &AdvertiseOptions,
) -> Result<DeviceAddReport> {
    let paths = PairPaths::new(cfg);
    let selection = resolve_advertise_host(bind, options)?;
    let invite = build_invite(cfg, bind, &selection.selected_host, ttl_minutes.max(1));
    let (qr_rendered, qr_warning) = render_advertise_qr_status(&invite.local_url, qr);
    if !dry_run {
        paths.ensure()?;
        write_json(&paths.invite_path(&invite.invite_id), &invite)?;
        append_audit(
            &paths,
            "invite_created",
            &format!("invite_id={}", invite.invite_id),
        )?;
        if qr {
            append_audit(
                &paths,
                if qr_rendered {
                    "qr_rendered"
                } else {
                    "qr_render_failed"
                },
                qr_warning.as_deref().unwrap_or(&invite.local_url),
            )?;
        }
    }
    Ok(DeviceAddReport {
        manual_fallback_command: invite.manual_command.clone(),
        selected_advertise_host: selection.selected_host.clone(),
        detected_addresses: selection.detected,
        ignored_addresses: selection.ignored,
        child_test_command: format!(
            "curl -fsS {}/join/{}.json",
            base_url_for_host(&selection.selected_host, parse_port(bind)),
            invite.invite_id
        ),
        invite,
        qr_rendered,
        qr_warning,
    })
}

#[allow(dead_code)]
pub fn cockpit(cfg: &Config, bind: &str, qr: bool, dry_run: bool) -> Result<PairCockpitReport> {
    cockpit_with_options(cfg, bind, qr, dry_run, &AdvertiseOptions::default())
}

pub fn cockpit_with_options(
    cfg: &Config,
    bind: &str,
    qr: bool,
    dry_run: bool,
    options: &AdvertiseOptions,
) -> Result<PairCockpitReport> {
    if !dry_run {
        preflight_pairing_writes(cfg)?;
    }
    let paths = PairPaths::new(cfg);
    let pending = list_pending_requests(&paths)?;
    let children = list_child_records(&paths)?;
    let selection = resolve_advertise_host(bind, options)?;
    let local_url = base_url_for_host(&selection.selected_host, parse_port(bind));
    let (qr_rendered, qr_warning) = render_advertise_qr_status(&local_url, qr);
    if !dry_run && qr {
        append_audit(
            &paths,
            if qr_rendered {
                "qr_rendered"
            } else {
                "qr_render_failed"
            },
            qr_warning.as_deref().unwrap_or(&local_url),
        )?;
    }
    Ok(PairCockpitReport {
        role: "agent-cluster-core".to_string(),
        workspace: cfg.workspace_dir.clone(),
        bind: bind.to_string(),
        port: parse_port(bind),
        local_url: local_url.clone(),
        selected_advertise_host: selection.selected_host.clone(),
        detected_addresses: selection.detected,
        ignored_addresses: selection.ignored,
        child_test_command: format!("curl -fsS {local_url}/"),
        qr_rendered,
        qr_warning,
        pending_request_count: pending.len(),
        approved_child_count: children
            .iter()
            .filter(|child| child.status == ChildStatus::Approved)
            .count(),
        revoked_child_count: children
            .iter()
            .filter(|child| child.status == ChildStatus::Revoked)
            .count(),
        safety: ChildAuthority::observe_only(),
    })
}

pub fn doctor(_cfg: &Config, bind: &str, options: &AdvertiseOptions) -> Result<PairDoctorReport> {
    let port = parse_port(bind);
    let port_available = TcpListener::bind(bind).is_ok();
    let resolved = resolve_advertise_host(bind, options);
    let (selected_advertise_host, selected_url, detected_addresses, ignored_addresses, guidance) =
        match resolved {
            Ok(selection) => {
                let url = base_url_for_host(&selection.selected_host, port);
                let guidance = if url_contains_local_only_host(&url) {
                    vec![
                        "This URL only works on the core device. Use --host <your-wifi-ip> or bind a private interface before pairing a phone/tablet.".to_string(),
                        "Wi-Fi is supported and Ethernet is optional.".to_string(),
                    ]
                } else {
                    vec![
                        "Open the URL on the phone/tablet while connected to the same Wi-Fi or local network.".to_string(),
                        "If it does not open, check Wi-Fi network, VPN, firewall, guest Wi-Fi isolation, and port blocking.".to_string(),
                    ]
                };
                (
                    Some(selection.selected_host),
                    Some(url.clone()),
                    selection.detected,
                    selection.ignored,
                    guidance,
                )
            }
            Err(err) => {
                let candidates = local_advertise_candidates();
                let has_private_address = candidates
                    .iter()
                    .any(|candidate| is_private_ipv4_host(&candidate.host));
                let recovery = if has_private_address {
                    "Use one of the detected private IPv4 addresses with --host, select its exact interface name, or remove the invalid override."
                } else {
                    "No reachable local Wi-Fi/LAN address was detected automatically. Connect Wi-Fi or use --host <your-wifi-ip>."
                };
                let ignored = candidates
                    .iter()
                    .filter_map(|candidate| {
                        let reason = classify_advertise_candidate(candidate, None);
                        (reason != "usable").then_some(AdvertiseCandidate {
                            reason,
                            ..candidate.clone()
                        })
                    })
                    .collect();
                (
                    None,
                    None,
                    candidates,
                    ignored,
                    vec![
                        format!("{err}"),
                        recovery.to_string(),
                        "Wi-Fi is supported and Ethernet is optional.".to_string(),
                    ],
                )
            }
        };
    let wifi_addresses_found = detected_addresses
        .iter()
        .any(|candidate| is_private_ipv4_host(&candidate.host));
    let only_loopback_found = !detected_addresses.is_empty()
        && detected_addresses
            .iter()
            .all(|candidate| is_loopback_host(&candidate.host));
    let child_test_command = selected_url.as_ref().map(|url| format!("curl -fsS {url}/"));
    Ok(PairDoctorReport {
        bind: bind.to_string(),
        port,
        selected_advertise_host,
        selected_url,
        detected_addresses,
        ignored_addresses,
        wifi_addresses_found,
        only_loopback_found,
        port_available,
        firewall_warning: "Use only on a same trusted local network. Wi-Fi is supported. Ethernet is optional. Do not expose this port to the public internet.".to_string(),
        same_network_explanation: "Same trusted local network means the core and child are on the same Wi-Fi or LAN and can reach each other directly.".to_string(),
        child_test_command,
        guidance,
    })
}

pub fn status(cfg: &Config, bind: &str) -> Result<PairStatusReport> {
    let paths = PairPaths::new(cfg);
    let pending = list_pending_requests(&paths)?;
    let children = list_child_records(&paths)?;
    let health = health_records(&paths)?;
    Ok(PairStatusReport {
        server_status: "stopped_or_unverified".to_string(),
        bind: bind.to_string(),
        port: parse_port(bind),
        pending_request_count: pending.len(),
        approved_child_count: children
            .iter()
            .filter(|child| child.status == ChildStatus::Approved)
            .count(),
        revoked_child_count: children
            .iter()
            .filter(|child| child.status == ChildStatus::Revoked)
            .count(),
        healthy_child_count: health
            .iter()
            .filter(|record| record.health == HeartbeatHealth::Healthy)
            .count(),
        stale_child_count: health
            .iter()
            .filter(|record| record.health == HeartbeatHealth::Stale)
            .count(),
        pending_child_count: pending.len(),
        unknown_child_count: health
            .iter()
            .filter(|record| record.health == HeartbeatHealth::Unknown)
            .count(),
        denied_child_count: health
            .iter()
            .filter(|record| record.health == HeartbeatHealth::Denied)
            .count(),
        last_audit_event: last_audit_event(&paths)?,
    })
}

pub fn list_children(
    cfg: &Config,
    include_pending: bool,
    include_revoked: bool,
) -> Result<ChildListReport> {
    let paths = PairPaths::new(cfg);
    let mut children = list_child_records(&paths)?;
    let revoked = if include_revoked {
        children
            .iter()
            .filter(|child| child.status == ChildStatus::Revoked)
            .cloned()
            .collect()
    } else {
        Vec::new()
    };
    children.retain(|child| child.status == ChildStatus::Approved);
    Ok(ChildListReport {
        pending: if include_pending {
            list_pending_requests(&paths)?
        } else {
            Vec::new()
        },
        approved: children,
        revoked,
        health: health_records(&paths)?,
    })
}

pub fn child_identity(cfg: &Config) -> Result<ChildIdentityReport> {
    preflight_child_writes(cfg)?;
    let paths = ChildPaths::new(cfg);
    let identity = load_or_create_child_identity(cfg, &paths)?;
    Ok(ChildIdentityReport {
        identity,
        identity_file: paths.identity,
        secret_scan_status: "no provider or execution credentials found".to_string(),
    })
}

pub fn child_join_by_url(
    child_cfg: &Config,
    core_cfg: Option<&Config>,
    join_url: &str,
    requested_authority: Option<&str>,
) -> Result<ChildJoinReport> {
    preflight_child_writes(child_cfg)?;
    let metadata = fetch_join_metadata(join_url, core_cfg)?;
    child_join_from_metadata(child_cfg, core_cfg, metadata, requested_authority)
}

pub fn child_join_from_metadata(
    child_cfg: &Config,
    core_cfg: Option<&Config>,
    metadata: JoinMetadata,
    requested_authority: Option<&str>,
) -> Result<ChildJoinReport> {
    preflight_child_writes(child_cfg)?;
    if metadata.expires_at <= now_secs() {
        anyhow::bail!(
            "invite {} is expired; expires_at={}",
            metadata.invite_id,
            metadata.expires_at
        );
    }
    if metadata.max_authority.trim() != "observe-only" && metadata.max_authority.trim() != "observe"
    {
        anyhow::bail!(
            "join metadata max_authority must be observe-only; got {}",
            metadata.max_authority
        );
    }
    let paths = ChildPaths::new(child_cfg);
    let mut identity = load_or_create_child_identity(child_cfg, &paths)?;
    identity.last_joined_core_url = Some(metadata.core_url.clone());
    identity.last_invite_id = Some(metadata.invite_id.clone());
    write_json(&paths.identity, &identity)?;
    append_child_audit(
        &paths,
        "child_join_identity_ready",
        &format!("invite_id={}", metadata.invite_id),
    )?;

    let authority = requested_authority.unwrap_or("observe");
    let input = PairRequestInput {
        invite_id: metadata.invite_id.clone(),
        claimed_device_name: identity.display_name.clone(),
        claimed_role: "agent-cluster-child-worker".to_string(),
        claimed_surface: identity.runtime_surface.clone(),
        runtime_kind: "quant-m-child-local".to_string(),
        device_class: device_class(),
        os: Some(identity.os.clone()),
        architecture: Some(identity.architecture.clone()),
        requested_authority: authority.to_string(),
        core_url: metadata.core_url.clone(),
        child_fingerprint: Some(identity.child_fingerprint.clone()),
    };
    let request = if let Some(core_cfg) = core_cfg {
        submit_pair_request(core_cfg, input)?
    } else {
        submit_pair_request_http(&metadata.pair_request_url, &input)?
    };
    if request.status != PairRequestStatus::Pending {
        anyhow::bail!(
            "pair request was not accepted as pending; status={:?}",
            request.status
        );
    }
    identity.last_request_id = Some(request.request_id.clone());
    identity.approved_node_id = Some(node_id_for_request(&request.request_id));
    write_json(&paths.identity, &identity)?;
    append_child_audit(
        &paths,
        "child_pair_request_submitted",
        &format!("request_id={}", request.request_id),
    )?;
    Ok(ChildJoinReport {
        metadata,
        approval_command: format!("quant-m child approve {}", request.request_id),
        identity,
        request,
        camera_scanning_available: false,
        safety: ChildAuthority::observe_only(),
    })
}

pub fn child_heartbeat(
    child_cfg: &Config,
    core_cfg: Option<&Config>,
    core_url: Option<&str>,
    claimed_authority: Option<ChildAuthority>,
) -> Result<ChildHeartbeatReport> {
    preflight_child_writes(child_cfg)?;
    let paths = ChildPaths::new(child_cfg);
    let identity = load_or_create_child_identity(child_cfg, &paths)?;
    let request_id = identity.last_request_id.clone();
    let node_id = identity
        .approved_node_id
        .clone()
        .or_else(|| request_id.as_deref().map(node_id_for_request))
        .with_context(|| "child heartbeat requires a prior child join request")?;
    let core_url = core_url
        .map(ToOwned::to_owned)
        .or(identity.last_joined_core_url.clone())
        .with_context(|| "child heartbeat requires --core <core-url> or prior join metadata")?;
    let payload = ChildHeartbeatPayload {
        node_id,
        request_id,
        child_fingerprint: identity.child_fingerprint.clone(),
        device_name: identity.display_name.clone(),
        claimed_role: "agent-cluster-child-worker".to_string(),
        authority: claimed_authority.unwrap_or_else(ChildAuthority::observe_only),
        timestamp: now_secs(),
        os: identity.os.clone(),
        architecture: identity.architecture.clone(),
        runtime_surface: identity.runtime_surface.clone(),
        child_binary_version: env!("CARGO_PKG_VERSION").to_string(),
        core_url,
        active_pack_hash: None,
        battery_status: None,
        storage_status: None,
        network_status: None,
    };
    let report = if let Some(core_cfg) = core_cfg {
        submit_heartbeat(core_cfg, payload)?
    } else {
        submit_heartbeat_http(&payload)?
    };
    append_child_audit(
        &paths,
        "child_heartbeat_sent",
        &format!(
            "node_id={} health={:?}",
            report
                .record
                .as_ref()
                .map(|record| record.node_id.as_str())
                .unwrap_or("none"),
            report.health
        ),
    )?;
    Ok(report)
}

pub fn submit_heartbeat(
    cfg: &Config,
    payload: ChildHeartbeatPayload,
) -> Result<ChildHeartbeatReport> {
    let paths = PairPaths::new(cfg);
    paths.ensure()?;
    let mut health = classify_heartbeat(&paths, &payload)?;
    let mut accepted = health == HeartbeatHealth::Healthy;
    if payload.authority != ChildAuthority::observe_only() {
        append_audit(
            &paths,
            "heartbeat_authority_claim_rejected",
            &format!("node_id={}", payload.node_id),
        )?;
        accepted = false;
        if health == HeartbeatHealth::Healthy {
            health = HeartbeatHealth::Unknown;
        }
    }
    let record = ChildHeartbeatRecord {
        node_id: payload.node_id.clone(),
        request_id: payload.request_id.clone(),
        child_fingerprint: payload.child_fingerprint.clone(),
        last_heartbeat: payload.timestamp,
        heartbeat_hash: heartbeat_hash(&payload),
        health: health.clone(),
        active_pack_hash: payload.active_pack_hash.clone(),
        authority: ChildAuthority::observe_only(),
    };
    write_json(&paths.heartbeat_path(&payload.node_id), &record)?;
    match health {
        HeartbeatHealth::Healthy => {
            let mut child: ChildRecord = read_json(&paths.child_path(&payload.node_id))?;
            child.last_heartbeat = Some(payload.timestamp);
            child.active_pack_hash = payload.active_pack_hash.clone();
            child.authority = ChildAuthority::observe_only();
            write_json(&paths.child_path(&payload.node_id), &child)?;
            append_audit(&paths, "heartbeat_accepted", &payload.node_id)?;
        }
        HeartbeatHealth::Pending => {
            append_audit(&paths, "heartbeat_rejected_pending_child", &payload.node_id)?;
        }
        HeartbeatHealth::Revoked => {
            append_audit(&paths, "heartbeat_rejected_revoked_child", &payload.node_id)?;
        }
        HeartbeatHealth::Denied => {
            append_audit(&paths, "heartbeat_rejected_denied_child", &payload.node_id)?;
        }
        HeartbeatHealth::Stale => {
            append_audit(&paths, "heartbeat_stale_classified", &payload.node_id)?;
        }
        HeartbeatHealth::Unknown => {
            append_audit(&paths, "heartbeat_rejected_unknown_child", &payload.node_id)?;
        }
    }
    Ok(ChildHeartbeatReport {
        accepted,
        health,
        record: Some(record),
        message: if accepted {
            "heartbeat accepted".to_string()
        } else {
            "heartbeat recorded as not healthy".to_string()
        },
    })
}

pub fn submit_pair_request(cfg: &Config, input: PairRequestInput) -> Result<PairRequest> {
    let paths = PairPaths::new(cfg);
    paths.ensure()?;
    let invite: PairInvite = read_json(&paths.invite_path(&input.invite_id))?;
    let now = now_secs();
    let mut request = PairRequest {
        request_id: make_id("req"),
        invite_id: input.invite_id,
        claimed_device_name: input.claimed_device_name,
        claimed_role: input.claimed_role,
        claimed_surface: input.claimed_surface,
        runtime_kind: input.runtime_kind,
        device_class: input.device_class,
        os: input.os,
        architecture: input.architecture,
        requested_authority: "observe-only".to_string(),
        requested_at: now,
        core_url: input.core_url,
        child_fingerprint: input.child_fingerprint,
        status: PairRequestStatus::Pending,
    };
    if invite.expires_at <= now {
        request.status = PairRequestStatus::Expired;
        write_json(&paths.request_path(&request.request_id), &request)?;
        append_audit(&paths, "expired_request_rejected", &request.request_id)?;
        return Ok(request);
    }
    if input.requested_authority.trim() != "observe-only"
        && input.requested_authority.trim() != "observe"
    {
        append_audit(
            &paths,
            "invalid_authority_request_rejected",
            &request.request_id,
        )?;
    }
    write_json(&paths.request_path(&request.request_id), &request)?;
    append_audit(&paths, "pair_request_received", &request.request_id)?;
    Ok(request)
}

pub fn approve_request(cfg: &Config, request_id: &str) -> Result<ChildRecord> {
    let paths = PairPaths::new(cfg);
    paths.ensure()?;
    let mut request: PairRequest = read_json(&paths.request_path(request_id))?;
    if request.status != PairRequestStatus::Pending {
        anyhow::bail!(
            "request {} is not pending; current status is {:?}",
            request_id,
            request.status
        );
    }
    let invite: PairInvite = read_json(&paths.invite_path(&request.invite_id))?;
    if invite.expires_at <= now_secs() {
        request.status = PairRequestStatus::Expired;
        write_json(&paths.request_path(request_id), &request)?;
        append_audit(&paths, "expired_request_rejected", request_id)?;
        anyhow::bail!("request {} is expired", request_id);
    }
    request.status = PairRequestStatus::Approved;
    write_json(&paths.request_path(request_id), &request)?;
    let node_id = node_id_for_request(request_id);
    let child = ChildRecord {
        node_id: node_id.clone(),
        request_id: request.request_id.clone(),
        display_name: request.claimed_device_name.clone(),
        role: request.claimed_role.clone(),
        surface: request.claimed_surface.clone(),
        authority: ChildAuthority::observe_only(),
        approved_at: now_secs(),
        approved_by: "local-operator".to_string(),
        revoked_at: None,
        last_heartbeat: None,
        active_pack_hash: None,
        status: ChildStatus::Approved,
    };
    write_json(&paths.child_path(&node_id), &child)?;
    append_audit(&paths, "pair_request_approved", request_id)?;
    Ok(child)
}

pub fn deny_request(cfg: &Config, request_id: &str) -> Result<PairRequest> {
    let paths = PairPaths::new(cfg);
    let mut request: PairRequest = read_json(&paths.request_path(request_id))?;
    if request.status != PairRequestStatus::Pending {
        anyhow::bail!(
            "request {} is not pending; current status is {:?}",
            request_id,
            request.status
        );
    }
    request.status = PairRequestStatus::Denied;
    write_json(&paths.request_path(request_id), &request)?;
    append_audit(&paths, "pair_request_denied", request_id)?;
    Ok(request)
}

pub fn revoke_child(cfg: &Config, node_id: &str) -> Result<ChildRecord> {
    let paths = PairPaths::new(cfg);
    let mut child: ChildRecord = read_json(&paths.child_path(node_id))?;
    child.status = ChildStatus::Revoked;
    child.revoked_at = Some(now_secs());
    write_json(&paths.child_path(node_id), &child)?;
    append_audit(&paths, "child_revoked", node_id)?;
    Ok(child)
}

#[allow(dead_code)]
pub fn child_can_receive_work(child: &ChildRecord) -> bool {
    child.status == ChildStatus::Approved
        && !child.authority.execution_allowed
        && !child.authority.provider_calls_allowed
        && !child.authority.canonical_write_allowed
        && !child.authority.approval_allowed
}

pub fn preflight_pairing_writes(cfg: &Config) -> Result<()> {
    let paths = PairPaths::new(cfg);
    for (path, operation) in [
        (&paths.invites, "write pairing invite"),
        (&paths.requests, "write pair request registry"),
        (&paths.children, "write child registry"),
        (&paths.root, "write pairing state"),
    ] {
        fs::create_dir_all(path).with_context(|| format!("{operation}: {}", path.display()))?;
        let probe = path.join(".quantm-pairing-write-check");
        fs::write(&probe, b"ok").with_context(|| format!("{operation}: {}", probe.display()))?;
        fs::remove_file(&probe).with_context(|| format!("{operation}: {}", probe.display()))?;
    }
    if let Some(parent) = cfg.logging.file.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("write audit log: {}", parent.display()))?;
    }
    paths.ensure()?;
    append_audit(&paths, "pairing_preflight_ok", "pairing workspace writable")?;
    Ok(())
}

pub fn serve(cfg: &Config, bind: &str, allow_public_bind: bool) -> Result<()> {
    if bind.starts_with("0.0.0.0") && !allow_public_bind {
        println!("{}", trusted_lan_warning());
    }
    preflight_pairing_writes(cfg)?;
    let listener = TcpListener::bind(bind).with_context(|| format!("failed to bind {bind}"))?;
    let local = listener
        .local_addr()
        .context("failed to inspect listener addr")?;
    let paths = PairPaths::new(cfg);
    append_audit(&paths, "pairing_server_started", &local.to_string())?;
    println!("Quant-M pairing server listening on http://{local}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_connection(cfg, &mut stream, bind) {
                    eprintln!("pairing connection failed: {err}");
                }
            }
            Err(err) => eprintln!("pairing accept failed: {err}"),
        }
    }
    Ok(())
}

fn handle_connection(cfg: &Config, stream: &mut TcpStream, bind: &str) -> Result<()> {
    let mut buffer = [0_u8; 8192];
    let read = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..read]);
    let mut lines = request.lines();
    let request_line = lines.next().unwrap_or("");
    let method = request_line.split_whitespace().next().unwrap_or("GET");
    let path = request_line.split_whitespace().nth(1).unwrap_or("/");
    let request_body = request.split("\r\n\r\n").nth(1).unwrap_or("");
    let (status, content_type, body) = if method == "POST" && path == "/api/pair-requests" {
        match serde_json::from_str::<PairRequestInput>(request_body)
            .context("invalid pair request JSON")
            .and_then(|input| submit_pair_request(cfg, input))
        {
            Ok(pair_request) => (
                "200 OK",
                "application/json",
                serde_json::to_string_pretty(&pair_request)?,
            ),
            Err(err) => (
                "400 Bad Request",
                "text/plain; charset=utf-8",
                format!("{err:#}\n"),
            ),
        }
    } else if method == "POST" && path == "/api/heartbeats" {
        match serde_json::from_str::<ChildHeartbeatPayload>(request_body)
            .context("invalid heartbeat JSON")
            .and_then(|payload| submit_heartbeat(cfg, payload))
        {
            Ok(report) => (
                "200 OK",
                "application/json",
                serde_json::to_string_pretty(&report)?,
            ),
            Err(err) => (
                "400 Bad Request",
                "text/plain; charset=utf-8",
                format!("{err:#}\n"),
            ),
        }
    } else if method == "GET" && path == "/" {
        (
            "200 OK",
            "text/plain; charset=utf-8",
            render_pair_root(cfg, bind),
        )
    } else if method == "GET" {
        if let Some(invite_id) = path.strip_prefix("/join/") {
            if let Some(invite_id) = invite_id.strip_suffix(".json") {
                match join_metadata(cfg, bind, invite_id) {
                    Ok(metadata) => (
                        "200 OK",
                        "application/json",
                        serde_json::to_string_pretty(&metadata)?,
                    ),
                    Err(err) => (
                        "404 Not Found",
                        "text/plain; charset=utf-8",
                        format!("{err:#}\n"),
                    ),
                }
            } else {
                match render_join_page(cfg, bind, invite_id) {
                    Ok(page) => ("200 OK", "text/plain; charset=utf-8", page),
                    Err(err) => (
                        "404 Not Found",
                        "text/plain; charset=utf-8",
                        format!("{err:#}\n"),
                    ),
                }
            }
        } else {
            (
                "404 Not Found",
                "text/plain; charset=utf-8",
                "not found\n".to_string(),
            )
        }
    } else {
        (
            "404 Not Found",
            "text/plain; charset=utf-8",
            "not found\n".to_string(),
        )
    };
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    stream.write_all(response.as_bytes())?;
    Ok(())
}

pub fn render_cockpit(report: &PairCockpitReport) -> String {
    let mut out = String::new();
    out.push_str("Quant-M Agent Cluster Pairing Cockpit\n");
    out.push_str(&format!("role: {}\n", report.role));
    out.push_str(&format!("workspace: {}\n", report.workspace.display()));
    out.push_str(&format!("bind: {}\n", report.bind));
    out.push_str(&format!("port: {}\n", report.port));
    out.push_str(&format!("local_url: {}\n", report.local_url));
    out.push_str(&format!(
        "selected_advertise_host: {}\n",
        report.selected_advertise_host
    ));
    out.push_str("same_network: same trusted local network required; Wi-Fi is supported; Ethernet is optional\n");
    out.push_str(&format!(
        "child_test_command: {}\n",
        report.child_test_command
    ));
    render_advertise_candidates(&mut out, "detected_addresses", &report.detected_addresses);
    render_advertise_candidates(&mut out, "ignored_addresses", &report.ignored_addresses);
    if report.qr_rendered {
        out.push_str("qr: rendered\n");
        out.push_str(&terminal_qr_placeholder(&report.local_url));
    } else {
        out.push_str("qr: fallback_url\n");
        if let Some(warning) = &report.qr_warning {
            out.push_str(&format!("qr_warning: {warning}\n"));
        }
    }
    out.push_str(&format!(
        "pending_requests: {}\n",
        report.pending_request_count
    ));
    out.push_str(&format!(
        "approved_children: {}\n",
        report.approved_child_count
    ));
    out.push_str(&format!(
        "revoked_children: {}\n",
        report.revoked_child_count
    ));
    out.push_str("commands:\n");
    out.push_str("  quant-m device add --qr\n");
    out.push_str("  quant-m device add --watch\n");
    out.push_str("  quant-m child approve <request_id>\n");
    out.push_str("  quant-m child deny <request_id>\n");
    out.push_str("  quant-m child revoke <node_id>\n");
    out.push_str(&render_safety_status());
    out
}

pub fn render_device_add(report: &DeviceAddReport) -> String {
    let mut out = String::new();
    out.push_str("Quant-M pairing invite\n");
    out.push_str(&format!("invite_id: {}\n", report.invite.invite_id));
    out.push_str(&format!("expires_at: {}\n", report.invite.expires_at));
    out.push_str(&format!(
        "core_fingerprint: {}\n",
        report.invite.core_fingerprint
    ));
    out.push_str(&format!("local_url: {}\n", report.invite.local_url));
    out.push_str(&format!(
        "selected_advertise_host: {}\n",
        report.selected_advertise_host
    ));
    out.push_str("same_network: open this URL on a phone/tablet connected to the same Wi-Fi or local network; Ethernet is optional\n");
    out.push_str(&format!(
        "child_test_command: {}\n",
        report.child_test_command
    ));
    render_advertise_candidates(&mut out, "detected_addresses", &report.detected_addresses);
    render_advertise_candidates(&mut out, "ignored_addresses", &report.ignored_addresses);
    if report.qr_rendered {
        out.push_str("qr: rendered\n");
        out.push_str(&terminal_qr_placeholder(&report.invite.local_url));
    } else if let Some(warning) = &report.qr_warning {
        out.push_str(&format!("qr_warning: {warning}\n"));
    } else {
        out.push_str("qr_hint: run quant-m device add --qr\n");
    }
    out.push_str(&format!(
        "manual_fallback: {}\n",
        report.manual_fallback_command
    ));
    out.push_str(&trusted_lan_warning());
    out
}

pub fn render_doctor(report: &PairDoctorReport) -> String {
    let mut out = String::new();
    out.push_str("Quant-M pairing doctor\n");
    out.push_str(&format!("bind: {}\n", report.bind));
    out.push_str(&format!("port: {}\n", report.port));
    out.push_str(&format!("port_available: {}\n", report.port_available));
    out.push_str(&format!(
        "selected_advertise_host: {}\n",
        report.selected_advertise_host.as_deref().unwrap_or("none")
    ));
    out.push_str(&format!(
        "core_pairing_url: {}\n",
        report.selected_url.as_deref().unwrap_or("none")
    ));
    out.push_str(&format!(
        "wifi_addresses_found: {}\n",
        report.wifi_addresses_found
    ));
    out.push_str(&format!(
        "only_loopback_found: {}\n",
        report.only_loopback_found
    ));
    render_advertise_candidates(&mut out, "detected_addresses", &report.detected_addresses);
    render_advertise_candidates(&mut out, "ignored_addresses", &report.ignored_addresses);
    out.push_str(&format!("firewall_warning: {}\n", report.firewall_warning));
    out.push_str(&format!(
        "same_network: {}\n",
        report.same_network_explanation
    ));
    if let Some(command) = &report.child_test_command {
        out.push_str(&format!("child_test_command: {command}\n"));
    }
    out.push_str("guidance:\n");
    for item in &report.guidance {
        out.push_str(&format!("  - {item}\n"));
    }
    out
}

fn render_advertise_candidates(out: &mut String, label: &str, candidates: &[AdvertiseCandidate]) {
    out.push_str(&format!("{label}:\n"));
    if candidates.is_empty() {
        out.push_str("  none\n");
        return;
    }
    for candidate in candidates {
        out.push_str(&format!(
            "  - interface={} host={} reason={}\n",
            candidate.interface, candidate.host, candidate.reason
        ));
    }
}

pub fn render_status(report: &PairStatusReport) -> String {
    format!(
        "pairing_status: {}\nbind: {}\nport: {}\npending_requests: {}\napproved_children: {}\nrevoked_children: {}\nhealthy_children: {}\nstale_children: {}\npending_children: {}\nunknown_children: {}\ndenied_children: {}\nlast_audit_event: {}\n",
        report.server_status,
        report.bind,
        report.port,
        report.pending_request_count,
        report.approved_child_count,
        report.revoked_child_count,
        report.healthy_child_count,
        report.stale_child_count,
        report.pending_child_count,
        report.unknown_child_count,
        report.denied_child_count,
        report
            .last_audit_event
            .as_ref()
            .map(|event| format!("{} {}", event.event_type, event.detail))
            .unwrap_or_else(|| "none".to_string())
    )
}

pub fn render_child_list(report: &ChildListReport) -> String {
    let mut out = String::new();
    out.push_str("Quant-M children\n");
    for request in &report.pending {
        out.push_str(&format!(
            "pending request_id={} name={} role={} authority={} age_seconds={}\n",
            request.request_id,
            request.claimed_device_name,
            request.claimed_role,
            request.requested_authority,
            now_secs().saturating_sub(request.requested_at)
        ));
    }
    for child in &report.approved {
        out.push_str(&format!(
            "approved node_id={} name={} role={} authority={} last_heartbeat={}\n",
            child.node_id,
            child.display_name,
            child.role,
            child.authority.authority,
            child
                .last_heartbeat
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
    }
    for child in &report.revoked {
        out.push_str(&format!(
            "revoked node_id={} name={} role={} revoked_at={}\n",
            child.node_id,
            child.display_name,
            child.role,
            child
                .revoked_at
                .map(|value| value.to_string())
                .unwrap_or_else(|| "none".to_string())
        ));
    }
    for record in &report.health {
        out.push_str(&format!(
            "health node_id={} status={:?} last_heartbeat={} active_pack_hash={}\n",
            record.node_id,
            record.health,
            record.last_heartbeat,
            record
                .active_pack_hash
                .clone()
                .unwrap_or_else(|| "none".to_string())
        ));
    }
    out
}

pub fn render_pending_watch(cfg: &Config) -> Result<String> {
    let list = list_children(cfg, true, false)?;
    let mut out = render_child_list(&list);
    out.push_str("approve: quant-m child approve <request_id>\n");
    out.push_str("deny: quant-m child deny <request_id>\n");
    Ok(out)
}

pub fn render_child_identity(report: &ChildIdentityReport) -> String {
    format!(
        "Quant-M child identity\nidentity_id: {}\nchild_fingerprint: {}\ndisplay_name: {}\nos: {}\narchitecture: {}\nruntime_surface: {}\nidentity_file: {}\nsecret_scan_status: {}\n",
        report.identity.identity_id,
        report.identity.child_fingerprint,
        report.identity.display_name,
        report.identity.os,
        report.identity.architecture,
        report.identity.runtime_surface,
        report.identity_file.display(),
        report.secret_scan_status,
    )
}

pub fn render_child_join(report: &ChildJoinReport) -> String {
    format!(
        "Quant-M child join\nrequest_id: {}\nstatus: pending\ninvite_id: {}\ncore_url: {}\ncore_name: {}\ncore_fingerprint: {}\nidentity_id: {}\nchild_fingerprint: {}\nrequested_authority: observe-only\ncamera_qr_scanning: unavailable\nmanual_url_flow: ok\n\nPair request submitted and waiting for manual approval on the core. On the core, run: {}\n\nSafety:\n  child stores no provider keys\n  child provider calls: blocked\n  child approval authority: blocked\n  child canonical shared-state writes: blocked\n  child execution authority: blocked\n  child live trading/broker/exchange/sportsbook execution: blocked\n",
        report.request.request_id,
        report.metadata.invite_id,
        report.metadata.core_url,
        report.metadata.core_name,
        report.metadata.core_fingerprint,
        report.identity.identity_id,
        report.identity.child_fingerprint,
        report.approval_command,
    )
}

pub fn render_child_join_manual() -> String {
    "Camera QR scanning is not implemented in this runtime yet. Open the QR URL manually or paste it with quant-m child join --url <url>.\n\nTermux/manual fallback:\n  pkg update\n  pkg install curl openssh termux-api\n  quant-m child join --url http://<core-wifi-or-lan-ip>:8787/join/<invite_id>\n\nUse a phone/tablet on the same trusted local network. Wi-Fi is supported and Ethernet is optional. The child requests observe-only authority, stores no provider keys, and waits for manual core approval.\n".to_string()
}

pub fn render_child_heartbeat(report: &ChildHeartbeatReport) -> String {
    let node_id = report
        .record
        .as_ref()
        .map(|record| record.node_id.as_str())
        .unwrap_or("none");
    let last_heartbeat = report
        .record
        .as_ref()
        .map(|record| record.last_heartbeat.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "Quant-M child heartbeat\naccepted: {}\nhealth: {:?}\nnode_id: {}\nlast_heartbeat: {}\nmessage: {}\n\nSafety:\n  heartbeat visibility only\n  heartbeat does not grant authority\n  child authority remains observe-only\n  provider calls: blocked\n  execution: blocked\n  approval: blocked\n  canonical shared-state writes: blocked\n  broker/exchange/sportsbook execution: blocked\n",
        report.accepted, report.health, node_id, last_heartbeat, report.message
    )
}

pub fn join_metadata(cfg: &Config, _bind: &str, invite_id: &str) -> Result<JoinMetadata> {
    let paths = PairPaths::new(cfg);
    let invite: PairInvite = read_json(&paths.invite_path(invite_id))?;
    if invite.expires_at <= now_secs() {
        anyhow::bail!(
            "invite {} is expired; expires_at={}",
            invite.invite_id,
            invite.expires_at
        );
    }
    let core_url = parse_join_url(&invite.local_url)
        .with_context(|| format!("invite {} has an invalid advertised URL", invite.invite_id))?
        .core_url;
    Ok(JoinMetadata {
        pair_request_url: format!("{core_url}/api/pair-requests"),
        core_url,
        invite_id: invite.invite_id,
        core_name: invite.core_name,
        core_fingerprint: invite.core_fingerprint,
        expires_at: invite.expires_at,
        max_authority: "observe-only".to_string(),
        manual_command: invite.manual_command,
    })
}

fn build_invite(cfg: &Config, bind: &str, advertise_host: &str, ttl_minutes: u64) -> PairInvite {
    let invite_id = make_id("inv");
    let local_url = format!(
        "{}/join/{}",
        base_url_for_host(advertise_host, parse_port(bind)),
        invite_id
    );
    PairInvite {
        invite_id: invite_id.clone(),
        core_name: cfg.node_id.clone(),
        core_fingerprint: core_fingerprint(cfg),
        bind: bind.to_string(),
        local_url: local_url.clone(),
        created_at: now_secs(),
        expires_at: now_secs() + ttl_minutes * 60,
        manual_command: format!("quant-m child join --url {local_url}"),
    }
}

fn preflight_child_writes(cfg: &Config) -> Result<()> {
    let paths = ChildPaths::new(cfg);
    for (path, operation) in [
        (&paths.root, "write child state"),
        (&paths.identity, "write child identity"),
        (&paths.audit, "write child audit"),
    ] {
        if path.extension().is_some() {
            let parent = path
                .parent()
                .with_context(|| format!("{operation}: {}", path.display()))?;
            fs::create_dir_all(parent)
                .with_context(|| format!("{operation}: {}", parent.display()))?;
            let probe = parent.join(".quantm-child-write-check");
            fs::write(&probe, b"ok")
                .with_context(|| format!("{operation}: {}", probe.display()))?;
            fs::remove_file(&probe).with_context(|| format!("{operation}: {}", probe.display()))?;
        } else {
            fs::create_dir_all(path).with_context(|| format!("{operation}: {}", path.display()))?;
            let probe = path.join(".quantm-child-write-check");
            fs::write(&probe, b"ok")
                .with_context(|| format!("{operation}: {}", probe.display()))?;
            fs::remove_file(&probe).with_context(|| format!("{operation}: {}", probe.display()))?;
        }
    }
    Ok(())
}

fn load_or_create_child_identity(cfg: &Config, paths: &ChildPaths) -> Result<ChildIdentity> {
    if paths.identity.exists() {
        return read_json(&paths.identity);
    }
    let identity_id = make_id("child-id");
    let identity = ChildIdentity {
        child_fingerprint: child_fingerprint(cfg, &identity_id),
        identity_id,
        display_name: cfg.node_id.clone(),
        os: std::env::consts::OS.to_string(),
        architecture: std::env::consts::ARCH.to_string(),
        runtime_surface: runtime_surface(),
        created_at: now_secs(),
        last_joined_core_url: None,
        last_invite_id: None,
        last_request_id: None,
        approved_node_id: None,
    };
    write_json(&paths.identity, &identity)?;
    append_child_audit(paths, "child_identity_created", &identity.identity_id)?;
    Ok(identity)
}

fn append_child_audit(paths: &ChildPaths, event_type: &str, detail: &str) -> Result<()> {
    fs::create_dir_all(&paths.root)
        .with_context(|| format!("write child audit: {}", paths.root.display()))?;
    let event = PairAuditEvent {
        event_id: make_id("child-audit"),
        event_type: event_type.to_string(),
        timestamp: now_secs(),
        detail: detail.to_string(),
    };
    let raw = serde_json::to_string(&event).context("failed to serialize child audit event")?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.audit)
        .with_context(|| format!("failed to open {}", paths.audit.display()))?;
    writeln!(file, "{raw}").with_context(|| format!("failed to write {}", paths.audit.display()))
}

fn fetch_join_metadata(join_url: &str, core_cfg: Option<&Config>) -> Result<JoinMetadata> {
    let parsed = parse_join_url(join_url)?;
    if let Some(core_cfg) = core_cfg {
        return join_metadata(core_cfg, &parsed.bind, &parsed.invite_id);
    }
    let metadata_url = format!(
        "{}/join/{}.json",
        parsed.core_url.trim_end_matches('/'),
        parsed.invite_id
    );
    let raw = http_get_text(&metadata_url)?;
    serde_json::from_str(&raw).with_context(|| format!("invalid join metadata from {metadata_url}"))
}

fn submit_pair_request_http(url: &str, input: &PairRequestInput) -> Result<PairRequest> {
    let raw = serde_json::to_string(input).context("failed to serialize pair request")?;
    let response = http_post_json(url, &raw)?;
    serde_json::from_str(&response)
        .with_context(|| format!("invalid pair request response from {url}"))
}

fn submit_heartbeat_http(payload: &ChildHeartbeatPayload) -> Result<ChildHeartbeatReport> {
    let raw = serde_json::to_string(payload).context("failed to serialize heartbeat")?;
    let url = format!("{}/api/heartbeats", payload.core_url.trim_end_matches('/'));
    let response = http_post_json(&url, &raw)?;
    serde_json::from_str(&response)
        .with_context(|| format!("invalid heartbeat response from {url}"))
}

fn health_records(paths: &PairPaths) -> Result<Vec<ChildHeartbeatRecord>> {
    let mut records = read_dir_json::<ChildHeartbeatRecord>(&paths.heartbeats)?;
    let children = list_child_records(paths)?;
    let now = now_secs();
    for child in children {
        if child.status == ChildStatus::Approved {
            if let Some(record) = records
                .iter_mut()
                .find(|record| record.node_id == child.node_id)
            {
                if record.health == HeartbeatHealth::Healthy
                    && now.saturating_sub(record.last_heartbeat) > heartbeat_fresh_seconds()
                {
                    record.health = HeartbeatHealth::Stale;
                }
            } else {
                records.push(ChildHeartbeatRecord {
                    node_id: child.node_id.clone(),
                    request_id: Some(child.request_id.clone()),
                    child_fingerprint: String::new(),
                    last_heartbeat: 0,
                    heartbeat_hash: String::new(),
                    health: HeartbeatHealth::Stale,
                    active_pack_hash: child.active_pack_hash.clone(),
                    authority: child.authority.clone(),
                });
            }
        }
    }
    Ok(records)
}

fn classify_heartbeat(
    paths: &PairPaths,
    payload: &ChildHeartbeatPayload,
) -> Result<HeartbeatHealth> {
    if let Ok(child) = read_json::<ChildRecord>(&paths.child_path(&payload.node_id)) {
        return Ok(if child.status == ChildStatus::Revoked {
            HeartbeatHealth::Revoked
        } else if payload.timestamp <= now_secs().saturating_sub(heartbeat_fresh_seconds()) {
            HeartbeatHealth::Stale
        } else {
            HeartbeatHealth::Healthy
        });
    }
    if let Some(request_id) = &payload.request_id
        && let Ok(request) = read_json::<PairRequest>(&paths.request_path(request_id))
    {
        return Ok(match request.status {
            PairRequestStatus::Pending => HeartbeatHealth::Pending,
            PairRequestStatus::Denied => HeartbeatHealth::Denied,
            PairRequestStatus::Revoked => HeartbeatHealth::Revoked,
            PairRequestStatus::Expired => HeartbeatHealth::Unknown,
            PairRequestStatus::Approved => HeartbeatHealth::Unknown,
        });
    }
    Ok(HeartbeatHealth::Unknown)
}

fn heartbeat_hash(payload: &ChildHeartbeatPayload) -> String {
    let raw = serde_json::to_string(payload).unwrap_or_default();
    let mut hash: u64 = 14_695_981_039_346_656_037;
    for byte in raw.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("heartbeat-{hash:016x}")
}

fn heartbeat_fresh_seconds() -> u64 {
    120
}

fn list_pending_requests(paths: &PairPaths) -> Result<Vec<PairRequest>> {
    Ok(read_dir_json::<PairRequest>(&paths.requests)?
        .into_iter()
        .filter(|request| request.status == PairRequestStatus::Pending)
        .collect())
}

fn list_child_records(paths: &PairPaths) -> Result<Vec<ChildRecord>> {
    read_dir_json::<ChildRecord>(&paths.children)
}

fn read_dir_json<T: for<'de> Deserialize<'de>>(dir: &Path) -> Result<Vec<T>> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    let mut values = Vec::new();
    for entry in fs::read_dir(dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let path = entry?.path();
        if path.extension().and_then(|value| value.to_str()) == Some("json") {
            values.push(read_json(&path)?);
        }
    }
    Ok(values)
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(value).context("failed to serialize pairing record")?;
    fs::write(path, raw).with_context(|| format!("failed to write {}", path.display()))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str(&raw).with_context(|| format!("invalid JSON at {}", path.display()))
}

fn append_audit(paths: &PairPaths, event_type: &str, detail: &str) -> Result<()> {
    paths.ensure()?;
    let event = PairAuditEvent {
        event_id: make_id("audit"),
        event_type: event_type.to_string(),
        timestamp: now_secs(),
        detail: detail.to_string(),
    };
    let raw = serde_json::to_string(&event).context("failed to serialize audit event")?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&paths.audit)
        .with_context(|| format!("failed to open {}", paths.audit.display()))?;
    writeln!(file, "{raw}").with_context(|| format!("failed to write {}", paths.audit.display()))
}

fn last_audit_event(paths: &PairPaths) -> Result<Option<PairAuditEvent>> {
    if !paths.audit.exists() {
        return Ok(None);
    }
    let raw = fs::read_to_string(&paths.audit)
        .with_context(|| format!("failed to read {}", paths.audit.display()))?;
    Ok(raw
        .lines()
        .rev()
        .find_map(|line| serde_json::from_str::<PairAuditEvent>(line).ok()))
}

fn render_pair_root(cfg: &Config, bind: &str) -> String {
    format!(
        "Quant-M pairing core\ncore_name: {}\ncore_fingerprint: {}\nlocal_url: {}\n{}\n",
        cfg.node_id,
        core_fingerprint(cfg),
        base_url(bind),
        render_safety_status()
    )
}

fn render_join_page(cfg: &Config, bind: &str, invite_id: &str) -> Result<String> {
    let metadata = join_metadata(cfg, bind, invite_id)?;
    Ok(format!(
        "Quant-M child join\ncore_name: {}\ncore_fingerprint: {}\ninvite_id: {}\nmanual_command: {}\npair_command: {}\n{}\n",
        metadata.core_name,
        metadata.core_fingerprint,
        metadata.invite_id,
        metadata.manual_command,
        metadata.manual_command,
        render_safety_status()
    ))
}

fn render_qr_status(url: &str, qr: bool) -> (bool, Option<String>) {
    render_qr_status_for(url, qr, false)
}

fn render_advertise_qr_status(url: &str, qr: bool) -> (bool, Option<String>) {
    if url_contains_local_only_host(url) {
        return (
            false,
            Some(
                "QR disabled because this URL only works on this computer. Use --host <your-wifi-ip>; Wi-Fi is supported and Ethernet is optional."
                    .to_string(),
            ),
        );
    }
    render_qr_status(url, qr)
}

fn render_qr_status_for(url: &str, qr: bool, force_failure: bool) -> (bool, Option<String>) {
    if !qr {
        return (false, None);
    }
    if force_failure {
        return (
            false,
            Some(format!(
                "terminal QR unavailable; use local URL fallback: {url}"
            )),
        );
    }
    (true, None)
}

fn terminal_qr_placeholder(url: &str) -> String {
    format!(
        "qr_terminal:\n  +------------------------------+\n  | Quant-M QR local URL         |\n  | {} |\n  +------------------------------+\n",
        truncate_middle(url, 28)
    )
}

fn render_safety_status() -> String {
    "Safety:\n  child authority: observe-only\n  child provider calls: blocked\n  child approval authority: blocked\n  child canonical shared-state writes: blocked\n  child execution authority: blocked\n  child live trading/broker/exchange/sportsbook execution: blocked\n".to_string()
}

fn trusted_lan_warning() -> String {
    "warning: same trusted local network required; Wi-Fi is supported; Ethernet is optional; do not expose pairing to the public internet; no secrets should be placed on children; children remain observe-only\n".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedJoinUrl {
    core_url: String,
    invite_id: String,
    bind: String,
}

fn parse_join_url(join_url: &str) -> Result<ParsedJoinUrl> {
    let trimmed = join_url.trim();
    let without_scheme = trimmed
        .strip_prefix("http://")
        .or_else(|| trimmed.strip_prefix("https://"))
        .with_context(|| format!("join URL must start with http:// or https://: {trimmed}"))?;
    let (authority, path) = without_scheme
        .split_once('/')
        .with_context(|| format!("join URL must include /join/<invite_id>: {trimmed}"))?;
    let invite_id = path
        .strip_prefix("join/")
        .with_context(|| format!("join URL must include /join/<invite_id>: {trimmed}"))?
        .strip_suffix(".json")
        .unwrap_or_else(|| path.strip_prefix("join/").unwrap_or(path))
        .trim_matches('/');
    if invite_id.is_empty() {
        anyhow::bail!("join URL invite id is empty: {trimmed}");
    }
    let scheme = if trimmed.starts_with("https://") {
        "https"
    } else {
        "http"
    };
    let core_url = format!("{scheme}://{authority}");
    Ok(ParsedJoinUrl {
        bind: authority.to_string(),
        core_url,
        invite_id: invite_id.to_string(),
    })
}

fn base_url(bind: &str) -> String {
    let port = parse_port(bind);
    let host = bind
        .split(':')
        .next()
        .filter(|host| !host.is_empty())
        .unwrap_or("127.0.0.1");
    let host = if host == "0.0.0.0" { "127.0.0.1" } else { host };
    format!("http://{host}:{port}")
}

fn base_url_for_host(host: &str, port: u16) -> String {
    format!("http://{}:{port}", host.trim())
}

#[derive(Debug, Clone)]
struct AdvertiseSelection {
    selected_host: String,
    detected: Vec<AdvertiseCandidate>,
    ignored: Vec<AdvertiseCandidate>,
}

fn resolve_advertise_host(bind: &str, options: &AdvertiseOptions) -> Result<AdvertiseSelection> {
    if let Some(host) = options.host.as_deref() {
        validate_advertise_host(host)?;
        let advertised_host = host.trim();
        let explicit_bind_host = match bind_host(bind) {
            host if host.eq_ignore_ascii_case("localhost") => "127.0.0.1".to_string(),
            host => host,
        };
        if !explicit_bind_host.is_empty()
            && explicit_bind_host != "0.0.0.0"
            && explicit_bind_host != advertised_host
        {
            anyhow::bail!(
                "cannot advertise {advertised_host} while bound only to {explicit_bind_host}; bind 0.0.0.0 for same-network access or use matching --bind and --host values"
            );
        }
        return Ok(AdvertiseSelection {
            selected_host: advertised_host.to_string(),
            detected: vec![AdvertiseCandidate {
                interface: "manual".to_string(),
                host: advertised_host.to_string(),
                reason: "manual --host override".to_string(),
            }],
            ignored: Vec::new(),
        });
    }
    let candidates = local_advertise_candidates();
    select_advertise_host(bind, options.interface.as_deref(), candidates)
}

fn select_advertise_host(
    bind: &str,
    interface: Option<&str>,
    candidates: Vec<AdvertiseCandidate>,
) -> Result<AdvertiseSelection> {
    let bind_host = bind_host(bind);
    if !bind_host.is_empty() && bind_host != "0.0.0.0" {
        let selected_host = if bind_host.eq_ignore_ascii_case("localhost") {
            "127.0.0.1".to_string()
        } else {
            bind_host.clone()
        };
        if is_loopback_host(&selected_host) {
            return Ok(AdvertiseSelection {
                selected_host: selected_host.clone(),
                detected: vec![AdvertiseCandidate {
                    interface: "loopback".to_string(),
                    host: selected_host,
                    reason: "local-only explicit bind".to_string(),
                }],
                ignored: candidates,
            });
        }
        validate_advertise_host(&selected_host).with_context(|| {
            "same trusted local network required. Wi-Fi is supported. Ethernet is optional. Try --host <your-wifi-ip>"
        })?;
        if let Some(requested_interface) = interface {
            let belongs_to_interface = candidates.iter().any(|candidate| {
                candidate.interface == requested_interface && candidate.host == selected_host
            });
            if !belongs_to_interface {
                anyhow::bail!(
                    "bind host {selected_host} is not assigned to interface {requested_interface}; choose a matching --bind/--interface pair or use --host <your-wifi-ip>"
                );
            }
        }
        return Ok(AdvertiseSelection {
            selected_host: selected_host.clone(),
            detected: vec![AdvertiseCandidate {
                interface: interface.unwrap_or("bind").to_string(),
                host: selected_host,
                reason: "explicit bind host".to_string(),
            }],
            ignored: candidates,
        });
    }

    let mut detected = Vec::new();
    let mut ignored = Vec::new();
    for candidate in candidates {
        let classification = classify_advertise_candidate(&candidate, interface);
        if classification == "usable" {
            detected.push(candidate);
        } else {
            ignored.push(AdvertiseCandidate {
                reason: classification,
                ..candidate
            });
        }
    }
    detected.sort_by_key(advertise_priority);
    if let Some(selected) = detected.first() {
        return Ok(AdvertiseSelection {
            selected_host: selected.host.clone(),
            detected,
            ignored,
        });
    }
    if let Some(interface) = interface {
        anyhow::bail!(
            "interface {interface} has no private IPv4 address suitable for same-network pairing; run `quant-m pair doctor` to list detected interfaces or use --host <your-wifi-ip>"
        );
    }
    if let Some(loopback) = ignored
        .iter()
        .find(|candidate| is_loopback_host(&candidate.host))
        .cloned()
    {
        return Ok(AdvertiseSelection {
            selected_host: loopback.host.clone(),
            detected: vec![AdvertiseCandidate {
                reason: "local-only fallback; use --host <your-wifi-ip> for phone/tablet pairing"
                    .to_string(),
                ..loopback
            }],
            ignored,
        });
    }
    anyhow::bail!(
        "No reachable local Wi-Fi/LAN address was detected automatically. Same trusted local network required. Wi-Fi is supported. Ethernet is optional. Use --host <your-wifi-ip>."
    )
}

fn classify_advertise_candidate(
    candidate: &AdvertiseCandidate,
    requested_interface: Option<&str>,
) -> String {
    if let Some(interface) = requested_interface
        && candidate.interface != interface
    {
        return format!("ignored because --interface {interface} was selected");
    }
    if is_zero_host(&candidate.host) {
        return "0.0.0.0 is a bind address, not a child-reachable URL".to_string();
    }
    if is_loopback_host(&candidate.host) {
        return "loopback only works on this computer".to_string();
    }
    if is_link_local_host(&candidate.host) {
        return "link-local address is fallback-only and often not reachable from another device"
            .to_string();
    }
    if is_docker_or_vm_interface(&candidate.interface) {
        return "Docker/VM/tunnel interface is not preferred for phone/tablet pairing".to_string();
    }
    if !is_private_ipv4_host(&candidate.host) {
        return "not a private same-Wi-Fi/LAN IPv4 address".to_string();
    }
    "usable".to_string()
}

fn local_advertise_candidates() -> Vec<AdvertiseCandidate> {
    if let Ok(raw) = std::env::var("QUANT_M_PAIR_HOSTS") {
        return raw
            .split(',')
            .filter_map(|entry| {
                let entry = entry.trim();
                if entry.is_empty() {
                    return None;
                }
                let (interface, host) = entry.split_once('=').unwrap_or(("manual", entry));
                Some(AdvertiseCandidate {
                    interface: interface.trim().to_string(),
                    host: host.trim().to_string(),
                    reason: "environment candidate".to_string(),
                })
            })
            .collect();
    }
    let default_route_ip = default_route_ipv4();
    let mut candidates = get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            IpAddr::V4(ip) => Some(AdvertiseCandidate {
                interface: interface.name,
                host: ip.to_string(),
                reason: if Some(ip) == default_route_ip {
                    "default local network route".to_string()
                } else {
                    "system network interface".to_string()
                },
            }),
            IpAddr::V6(_) => None,
        })
        .collect::<Vec<_>>();
    if candidates.is_empty()
        && let Some(ip) = default_route_ip
    {
        candidates.push(AdvertiseCandidate {
            interface: "default-route".to_string(),
            host: ip.to_string(),
            reason: "default local network route fallback".to_string(),
        });
    }
    if !candidates
        .iter()
        .any(|candidate| is_loopback_host(&candidate.host))
    {
        candidates.push(AdvertiseCandidate {
            interface: "loopback".to_string(),
            host: "127.0.0.1".to_string(),
            reason: "local-only diagnostic".to_string(),
        });
    }
    let mut seen = BTreeSet::new();
    candidates
        .retain(|candidate| seen.insert((candidate.interface.clone(), candidate.host.clone())));
    candidates
}

fn default_route_ipv4() -> Option<Ipv4Addr> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("192.0.2.1:80").ok()?;
    match socket.local_addr().ok()? {
        SocketAddr::V4(addr) => Some(*addr.ip()),
        SocketAddr::V6(_) => None,
    }
}

fn validate_advertise_host(host: &str) -> Result<()> {
    let host = host.trim();
    if host.is_empty() {
        anyhow::bail!("advertise host is empty; use --host <your-wifi-ip>");
    }
    if is_zero_host(host) {
        anyhow::bail!("0.0.0.0 is only for binding; use --host <your-wifi-ip> for the QR/join URL");
    }
    if is_loopback_host(host) || host.eq_ignore_ascii_case("localhost") {
        anyhow::bail!(
            "This URL only works on this computer. Choose a same Wi-Fi/LAN IP with --host <your-wifi-ip>."
        );
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        match ip {
            IpAddr::V4(ipv4) => {
                if !is_private_ipv4(ipv4) {
                    anyhow::bail!(
                        "advertise host {host} is not a private same-Wi-Fi/LAN IPv4 address; use --host <your-wifi-ip>"
                    );
                }
            }
            IpAddr::V6(_) => {
                anyhow::bail!("IPv6 pairing URLs are not supported yet; use --host <your-wifi-ip>");
            }
        }
    } else {
        anyhow::bail!(
            "advertise host {host} must be a private IPv4 address; public or unresolved hostnames are not allowed"
        );
    }
    Ok(())
}

fn advertise_priority(candidate: &AdvertiseCandidate) -> u8 {
    if candidate.reason.contains("default local network route") {
        return 0;
    }
    let interface = candidate.interface.to_ascii_lowercase();
    if interface.contains("wi-fi")
        || interface.contains("wifi")
        || interface.starts_with("wlan")
        || interface.starts_with("en")
    {
        1
    } else if is_private_ipv4_host(&candidate.host) {
        2
    } else {
        9
    }
}

fn bind_host(bind: &str) -> String {
    bind.rsplit_once(':')
        .map(|(host, _)| host)
        .unwrap_or(bind)
        .trim()
        .to_string()
}

fn is_zero_host(host: &str) -> bool {
    host.trim() == "0.0.0.0"
}

fn is_loopback_host(host: &str) -> bool {
    host.parse::<IpAddr>().is_ok_and(|ip| ip.is_loopback())
}

fn is_link_local_host(host: &str) -> bool {
    host.parse::<Ipv4Addr>()
        .is_ok_and(|ip| ip.octets()[0] == 169 && ip.octets()[1] == 254)
}

fn is_private_ipv4_host(host: &str) -> bool {
    host.parse::<Ipv4Addr>().is_ok_and(is_private_ipv4)
}

fn is_private_ipv4(ip: Ipv4Addr) -> bool {
    ip.is_private()
}

fn is_docker_or_vm_interface(interface: &str) -> bool {
    let interface = interface.to_ascii_lowercase();
    interface.contains("docker")
        || interface.contains("vmnet")
        || interface.contains("vbox")
        || interface.contains("bridge")
        || interface.contains("tun")
        || interface.contains("tap")
        || interface.starts_with("awdl")
        || interface.starts_with("llw")
}

fn url_contains_local_only_host(url: &str) -> bool {
    parse_http_url(url)
        .map(|parsed| {
            is_loopback_host(&parsed.host)
                || is_zero_host(&parsed.host)
                || parsed.host.eq_ignore_ascii_case("localhost")
        })
        .unwrap_or(false)
}

fn http_get_text(url: &str) -> Result<String> {
    let parsed = parse_http_url(url)?;
    let mut stream = TcpStream::connect(&parsed.bind)
        .with_context(|| format!("failed to connect to {}", parsed.bind))?;
    let request = format!(
        "GET {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
        parsed.path, parsed.host
    );
    stream.write_all(request.as_bytes())?;
    read_http_body(stream)
}

fn http_post_json(url: &str, body: &str) -> Result<String> {
    let parsed = parse_http_url(url)?;
    let mut stream = TcpStream::connect(&parsed.bind)
        .with_context(|| format!("failed to connect to {}", parsed.bind))?;
    let request = format!(
        "POST {} HTTP/1.1\r\nHost: {}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        parsed.path,
        parsed.host,
        body.len(),
        body
    );
    stream.write_all(request.as_bytes())?;
    read_http_body(stream)
}

#[derive(Debug, Clone)]
struct ParsedHttpUrl {
    bind: String,
    host: String,
    path: String,
}

fn parse_http_url(url: &str) -> Result<ParsedHttpUrl> {
    let without_scheme = url
        .strip_prefix("http://")
        .with_context(|| format!("only http:// LAN URLs are supported in P0C-A: {url}"))?;
    let (authority, path) = without_scheme
        .split_once('/')
        .unwrap_or((without_scheme, ""));
    let host = authority.split(':').next().unwrap_or(authority).to_string();
    Ok(ParsedHttpUrl {
        bind: authority.to_string(),
        host,
        path: format!("/{}", path),
    })
}

fn read_http_body(mut stream: TcpStream) -> Result<String> {
    let mut raw = String::new();
    stream.read_to_string(&mut raw)?;
    let (head, body) = raw
        .split_once("\r\n\r\n")
        .with_context(|| "invalid HTTP response: missing header/body separator")?;
    if !head.starts_with("HTTP/1.1 200") && !head.starts_with("HTTP/1.0 200") {
        anyhow::bail!("HTTP request failed: {head}");
    }
    Ok(body.to_string())
}

fn parse_port(bind: &str) -> u16 {
    bind.rsplit(':')
        .next()
        .and_then(|value| value.parse::<u16>().ok())
        .unwrap_or(8787)
}

fn core_fingerprint(cfg: &Config) -> String {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    let workspace = cfg.workspace_dir.to_string_lossy();
    for byte in cfg.node_id.as_bytes().iter().chain(workspace.as_bytes()) {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("core-{hash:016x}")
}

fn child_fingerprint(cfg: &Config, identity_id: &str) -> String {
    let mut hash: u64 = 14_695_981_039_346_656_037;
    let workspace = cfg.workspace_dir.to_string_lossy();
    for byte in identity_id
        .as_bytes()
        .iter()
        .chain(cfg.node_id.as_bytes())
        .chain(workspace.as_bytes())
    {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(1_099_511_628_211);
    }
    format!("child-{hash:016x}")
}

fn runtime_surface() -> String {
    if std::env::var("TERMUX_VERSION").is_ok()
        || std::env::var("PREFIX").is_ok_and(|v| v.contains("com.termux"))
    {
        "termux".to_string()
    } else {
        "local-cli".to_string()
    }
}

fn device_class() -> String {
    if runtime_surface() == "termux" {
        "android-termux".to_string()
    } else {
        format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
    }
}

fn make_id(prefix: &str) -> String {
    format!("{}-{}-{}", prefix, now_nanos(), std::process::id())
}

fn node_id_for_request(request_id: &str) -> String {
    format!("child-{}", safe_id(request_id))
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '-'
            }
        })
        .collect()
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}

fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0)
}

fn truncate_middle(value: &str, max: usize) -> String {
    if value.len() <= max {
        value.to_string()
    } else {
        let keep = max.saturating_sub(3) / 2;
        format!("{}...{}", &value[..keep], &value[value.len() - keep..])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn test_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.node_id = "core-test".to_string();
        cfg.workspace_dir = tmp.path().join("workspace");
        cfg.logging.file = cfg.workspace_dir.join("logs/quant-m.log");
        (tmp, cfg)
    }

    #[allow(clippy::field_reassign_with_default)]
    fn child_cfg(root: &Path) -> Config {
        let mut cfg = Config::default();
        cfg.node_id = "android-tablet-01".to_string();
        cfg.workspace_dir = root.join("child-workspace");
        cfg.logging.file = cfg.workspace_dir.join("logs/quant-m.log");
        cfg
    }

    #[test]
    fn device_add_creates_expiring_invite() {
        let (_tmp, cfg) = test_cfg();
        let report = create_invite(&cfg, "127.0.0.1:8787", 5, false, false).expect("invite");
        assert!(report.invite.expires_at > report.invite.created_at);
        assert!(report.invite.local_url.contains(&report.invite.invite_id));
        assert!(
            PairPaths::new(&cfg)
                .invite_path(&report.invite.invite_id)
                .exists()
        );
    }

    #[test]
    fn qr_failure_falls_back_to_local_url() {
        let (rendered, warning) =
            render_qr_status_for("http://127.0.0.1:8787/join/invite", true, true);
        assert!(!rendered);
        assert!(warning.unwrap().contains("fallback"));
    }

    #[test]
    fn pair_cockpit_prints_qr_or_url() {
        let (_tmp, cfg) = test_cfg();
        let report = cockpit(&cfg, "127.0.0.1:8787", true, true).expect("cockpit");
        let rendered = render_cockpit(&report);
        assert!(rendered.contains("local_url: http://127.0.0.1:8787"));
        assert!(rendered.contains("child authority: observe-only"));
    }

    #[test]
    fn wifi_ip_is_valid_pairing_advertise_host() {
        let selection = select_advertise_host(
            "0.0.0.0:8787",
            None,
            vec![AdvertiseCandidate {
                interface: "wlan0".to_string(),
                host: "192.168.1.42".to_string(),
                reason: "test wifi".to_string(),
            }],
        )
        .expect("select wifi");

        assert_eq!(selection.selected_host, "192.168.1.42");
        assert_eq!(
            base_url_for_host(&selection.selected_host, 8787),
            "http://192.168.1.42:8787"
        );
    }

    #[test]
    fn loopback_is_not_used_for_child_join_url() {
        let selection = select_advertise_host(
            "0.0.0.0:8787",
            None,
            vec![
                AdvertiseCandidate {
                    interface: "lo0".to_string(),
                    host: "127.0.0.1".to_string(),
                    reason: "loopback".to_string(),
                },
                AdvertiseCandidate {
                    interface: "en0".to_string(),
                    host: "192.168.1.42".to_string(),
                    reason: "wifi".to_string(),
                },
            ],
        )
        .expect("select wifi over loopback");

        assert_eq!(selection.selected_host, "192.168.1.42");
        assert!(selection.ignored.iter().any(|candidate| {
            candidate.host == "127.0.0.1" && candidate.reason.contains("loopback")
        }));
    }

    #[test]
    fn explicit_bind_is_authoritative_over_detected_routes() {
        let selection = select_advertise_host(
            "127.0.0.1:8787",
            None,
            vec![AdvertiseCandidate {
                interface: "en0".to_string(),
                host: "192.168.1.42".to_string(),
                reason: "detected wifi".to_string(),
            }],
        )
        .expect("explicit bind");

        assert_eq!(selection.selected_host, "127.0.0.1");
        assert!(selection.detected[0].reason.contains("explicit bind"));
    }

    #[test]
    fn interface_override_selects_matching_system_interface() {
        let selection = select_advertise_host(
            "0.0.0.0:8787",
            Some("en0"),
            vec![
                AdvertiseCandidate {
                    interface: "en0".to_string(),
                    host: "192.168.1.42".to_string(),
                    reason: "system network interface".to_string(),
                },
                AdvertiseCandidate {
                    interface: "en7".to_string(),
                    host: "10.0.0.24".to_string(),
                    reason: "system network interface".to_string(),
                },
            ],
        )
        .expect("select requested interface");

        assert_eq!(selection.selected_host, "192.168.1.42");
        assert_eq!(selection.detected[0].interface, "en0");
    }

    #[test]
    fn unknown_interface_does_not_fall_back_to_loopback() {
        let err = select_advertise_host(
            "0.0.0.0:8787",
            Some("missing0"),
            vec![AdvertiseCandidate {
                interface: "lo0".to_string(),
                host: "127.0.0.1".to_string(),
                reason: "loopback".to_string(),
            }],
        )
        .expect_err("unknown interface must fail");

        assert!(format!("{err:#}").contains("missing0"));
    }

    #[test]
    fn zero_bind_not_used_as_advertised_url() {
        let (_tmp, cfg) = test_cfg();
        let report = create_invite_with_options(
            &cfg,
            "0.0.0.0:8787",
            5,
            true,
            true,
            &AdvertiseOptions {
                host: Some("192.168.1.50".to_string()),
                interface: None,
            },
        )
        .expect("invite");

        assert!(
            report
                .invite
                .local_url
                .starts_with("http://192.168.1.50:8787/join/")
        );
        assert!(!report.invite.local_url.contains("0.0.0.0"));
    }

    #[test]
    fn manual_host_override_generates_valid_url() {
        let (_tmp, cfg) = test_cfg();
        let report = cockpit_with_options(
            &cfg,
            "0.0.0.0:8787",
            true,
            true,
            &AdvertiseOptions {
                host: Some("192.168.1.50".to_string()),
                interface: None,
            },
        )
        .expect("cockpit");

        assert_eq!(report.selected_advertise_host, "192.168.1.50");
        assert_eq!(report.local_url, "http://192.168.1.50:8787");
    }

    #[test]
    fn manual_host_cannot_claim_an_address_outside_explicit_bind() {
        let err = resolve_advertise_host(
            "127.0.0.1:8787",
            &AdvertiseOptions {
                host: Some("192.168.1.50".to_string()),
                interface: None,
            },
        )
        .expect_err("mismatched host and bind must fail");

        assert!(format!("{err:#}").contains("while bound only to 127.0.0.1"));
    }

    #[test]
    fn ethernet_not_required_for_pairing() {
        let selection = select_advertise_host(
            "0.0.0.0:8787",
            None,
            vec![AdvertiseCandidate {
                interface: "wlan0".to_string(),
                host: "10.0.0.24".to_string(),
                reason: "wifi without ethernet".to_string(),
            }],
        )
        .expect("wifi should be enough");

        assert_eq!(selection.selected_host, "10.0.0.24");
    }

    #[test]
    fn same_wifi_language_replaces_lan_required_error() {
        let err = select_advertise_host("0.0.0.0:8787", None, vec![])
            .expect_err("no candidate should explain same network");
        let rendered = format!("{err:#}");

        assert!(rendered.contains("Same trusted local network required"));
        assert!(rendered.contains("Wi-Fi is supported"));
        assert!(rendered.contains("Ethernet is optional"));
        assert!(!rendered.contains("Ethernet required"));
    }

    #[test]
    fn docker_or_vm_interface_not_preferred_over_wifi() {
        let selection = select_advertise_host(
            "0.0.0.0:8787",
            None,
            vec![
                AdvertiseCandidate {
                    interface: "docker0".to_string(),
                    host: "172.17.0.2".to_string(),
                    reason: "docker".to_string(),
                },
                AdvertiseCandidate {
                    interface: "wlan0".to_string(),
                    host: "192.168.1.42".to_string(),
                    reason: "wifi".to_string(),
                },
            ],
        )
        .expect("select wifi");

        assert_eq!(selection.selected_host, "192.168.1.42");
        assert!(selection.ignored.iter().any(|candidate| {
            candidate.interface == "docker0" && candidate.reason.contains("Docker/VM")
        }));
    }

    #[test]
    fn pair_doctor_reports_detected_interfaces() {
        let (_tmp, cfg) = test_cfg();
        let report = doctor(
            &cfg,
            "0.0.0.0:0",
            &AdvertiseOptions {
                host: Some("192.168.1.42".to_string()),
                interface: None,
            },
        )
        .expect("doctor");
        let rendered = render_doctor(&report);

        assert!(rendered.contains("core_pairing_url: http://192.168.1.42:0"));
        assert!(rendered.contains("detected_addresses"));
        assert!(rendered.contains("same trusted local network"));
    }

    #[test]
    fn invalid_advertise_host_is_rejected_clearly() {
        let err = validate_advertise_host("127.0.0.1").expect_err("loopback rejected");
        assert!(format!("{err:#}").contains("--host <your-wifi-ip>"));

        let err = validate_advertise_host("0.0.0.0").expect_err("zero rejected");
        assert!(format!("{err:#}").contains("only for binding"));

        let err = validate_advertise_host("example.com").expect_err("public hostname rejected");
        assert!(format!("{err:#}").contains("private IPv4"));
    }

    #[test]
    fn qr_uses_reachable_advertised_url() {
        let (_tmp, cfg) = test_cfg();
        let report = create_invite_with_options(
            &cfg,
            "0.0.0.0:8787",
            5,
            true,
            true,
            &AdvertiseOptions {
                host: Some("192.168.1.77".to_string()),
                interface: None,
            },
        )
        .expect("invite");
        let rendered = terminal_qr_placeholder(&report.invite.local_url);

        assert!(report.qr_rendered);
        assert!(
            report
                .invite
                .local_url
                .starts_with("http://192.168.1.77:8787")
        );
        assert!(!report.invite.local_url.contains("127.0.0.1"));
        assert!(!rendered.contains("0.0.0.0"));
    }

    #[test]
    fn pair_request_starts_pending() {
        let (_tmp, cfg) = test_cfg();
        let invite = create_invite(&cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let request =
            submit_pair_request(&cfg, request_input(&invite, "observe-only")).expect("request");
        assert_eq!(request.status, PairRequestStatus::Pending);
    }

    #[test]
    fn child_join_url_parses_invite_metadata() {
        let (_tmp, cfg) = test_cfg();
        let invite = create_invite(&cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let parsed = parse_join_url(&invite.local_url).expect("parse");
        let metadata = join_metadata(&cfg, "127.0.0.1:8787", &invite.invite_id).expect("metadata");

        assert_eq!(parsed.invite_id, invite.invite_id);
        assert_eq!(parsed.core_url, "http://127.0.0.1:8787");
        assert_eq!(
            metadata.pair_request_url,
            "http://127.0.0.1:8787/api/pair-requests"
        );
        assert!(metadata.expires_at > now_secs());
        assert_eq!(metadata.max_authority, "observe-only");
    }

    #[test]
    fn join_metadata_preserves_advertised_host_for_remote_child_callback() {
        let (_tmp, cfg) = test_cfg();
        let report = create_invite_with_options(
            &cfg,
            "0.0.0.0:8787",
            5,
            false,
            false,
            &AdvertiseOptions {
                host: Some("192.168.1.77".to_string()),
                interface: None,
            },
        )
        .expect("invite");
        let metadata =
            join_metadata(&cfg, "0.0.0.0:8787", &report.invite.invite_id).expect("metadata");

        assert_eq!(metadata.core_url, "http://192.168.1.77:8787");
        assert_eq!(
            metadata.pair_request_url,
            "http://192.168.1.77:8787/api/pair-requests"
        );
        assert!(!metadata.pair_request_url.contains("127.0.0.1"));
        assert!(!metadata.pair_request_url.contains("0.0.0.0"));
    }

    #[test]
    fn child_identity_created_without_secrets() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = child_cfg(tmp.path());
        let first = child_identity(&cfg).expect("identity");
        let second = child_identity(&cfg).expect("identity reload");
        let raw = fs::read_to_string(&first.identity_file).expect("identity raw");

        assert_eq!(
            first.identity.child_fingerprint,
            second.identity.child_fingerprint
        );
        for forbidden in [
            "openrouter",
            "openai",
            "codex",
            "claude",
            "gemini",
            "broker",
            "exchange",
            "sportsbook",
            "api_key",
            "token",
            "secret",
        ] {
            assert!(!raw.to_lowercase().contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn child_join_submits_pending_request() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let invite = create_invite(&core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let report =
            child_join_by_url(&child_cfg, Some(&core_cfg), &invite.local_url, None).expect("join");
        let list = list_children(&core_cfg, true, false).expect("children");

        assert_eq!(report.request.status, PairRequestStatus::Pending);
        assert_eq!(list.pending.len(), 1);
        assert!(list.approved.is_empty());
        assert!(render_child_join(&report).contains("quant-m child approve"));
    }

    #[test]
    fn child_requested_authority_above_observe_is_rejected_or_downgraded() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let invite = create_invite(&core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let report = child_join_by_url(
            &child_cfg,
            Some(&core_cfg),
            &invite.local_url,
            Some("provider-calls execution canonical-write approval"),
        )
        .expect("join");
        let audit = fs::read_to_string(PairPaths::new(&core_cfg).audit).expect("audit");

        assert_eq!(report.request.status, PairRequestStatus::Pending);
        assert_eq!(report.request.requested_authority, "observe-only");
        assert!(audit.contains("invalid_authority_request_rejected"));
    }

    #[test]
    fn child_join_without_provider_keys() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let invite = create_invite(&core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        child_join_by_url(&child_cfg, Some(&core_cfg), &invite.local_url, None).expect("join");
        let identity_path = ChildPaths::new(&child_cfg).identity;
        let raw = fs::read_to_string(identity_path).expect("identity raw");

        for forbidden in [
            "openrouter",
            "openai",
            "codex",
            "claude",
            "gemini",
            "broker",
            "exchange",
            "sportsbook",
            "api_key",
            "token",
            "secret",
        ] {
            assert!(!raw.to_lowercase().contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn expired_invite_blocks_child_join() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let mut invite = create_invite(&core_cfg, "127.0.0.1:8787", 1, false, false)
            .expect("invite")
            .invite;
        invite.expires_at = now_secs().saturating_sub(1);
        write_json(
            &PairPaths::new(&core_cfg).invite_path(&invite.invite_id),
            &invite,
        )
        .expect("expired invite");
        let err = child_join_by_url(&child_cfg, Some(&core_cfg), &invite.local_url, None)
            .expect_err("expired join");
        let list = list_children(&core_cfg, true, false).expect("children");

        assert!(err.to_string().contains("expired"));
        assert!(list.pending.is_empty());
        assert!(list.approved.is_empty());
    }

    #[test]
    fn manual_url_flow_works_without_camera() {
        let manual = render_child_join_manual();

        assert!(manual.contains("Camera QR scanning is not implemented"));
        assert!(manual.contains("quant-m child join --url"));
        assert!(manual.contains("stores no provider keys"));
    }

    #[test]
    fn child_join_read_only_workspace_blocks_before_request() {
        let (tmp, core_cfg) = test_cfg();
        let mut child_cfg = child_cfg(tmp.path());
        child_cfg.workspace_dir = tmp.path().join("blocked-child");
        fs::create_dir_all(&child_cfg.workspace_dir).expect("child workspace");
        fs::write(child_cfg.workspace_dir.join("state"), b"not a directory").expect("blocker");
        let invite = create_invite(&core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let err = child_join_by_url(&child_cfg, Some(&core_cfg), &invite.local_url, None)
            .expect_err("read only");
        let list = list_children(&core_cfg, true, false).expect("children");
        let message = format!("{err:#}");

        assert!(message.contains("write child state"));
        assert!(message.contains("state/child"));
        assert!(list.pending.is_empty());
    }

    #[test]
    fn child_join_prints_core_approval_instruction() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let invite = create_invite(&core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let report =
            child_join_by_url(&child_cfg, Some(&core_cfg), &invite.local_url, None).expect("join");
        let rendered = render_child_join(&report);

        assert!(rendered.contains("Pair request submitted"));
        assert!(rendered.contains("quant-m child approve"));
        assert!(rendered.contains(&report.request.request_id));
    }

    #[test]
    fn approved_child_can_send_heartbeat() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let (_join, child) = approved_child_fixture(&core_cfg, &child_cfg);

        let report = child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            None,
        )
        .expect("heartbeat");
        let updated: ChildRecord =
            read_json(&PairPaths::new(&core_cfg).child_path(&child.node_id)).expect("child");

        assert!(report.accepted);
        assert_eq!(report.health, HeartbeatHealth::Healthy);
        assert!(updated.last_heartbeat.is_some());
    }

    #[test]
    fn child_list_shows_last_heartbeat() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        approved_child_fixture(&core_cfg, &child_cfg);
        child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            None,
        )
        .expect("heartbeat");

        let list = list_children(&core_cfg, true, true).expect("list");
        let rendered = render_child_list(&list);

        assert!(rendered.contains("last_heartbeat="));
        assert!(
            list.health
                .iter()
                .any(|record| record.health == HeartbeatHealth::Healthy)
        );
    }

    #[test]
    fn pair_status_summarizes_child_health() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        approved_child_fixture(&core_cfg, &child_cfg);
        child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            None,
        )
        .expect("heartbeat");

        let status = status(&core_cfg, "127.0.0.1:8787").expect("status");

        assert_eq!(status.healthy_child_count, 1);
        assert_eq!(status.stale_child_count, 0);
        assert_eq!(status.revoked_child_count, 0);
    }

    #[test]
    fn heartbeat_does_not_grant_authority() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let (_join, child) = approved_child_fixture(&core_cfg, &child_cfg);
        let mut authority = ChildAuthority::observe_only();
        authority.provider_calls_allowed = true;
        authority.execution_allowed = true;
        authority.approval_allowed = true;
        authority.canonical_write_allowed = true;

        let report = child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            Some(authority),
        )
        .expect("heartbeat");
        let updated: ChildRecord =
            read_json(&PairPaths::new(&core_cfg).child_path(&child.node_id)).expect("child");

        assert!(!report.accepted);
        assert!(!updated.authority.provider_calls_allowed);
        assert!(!updated.authority.execution_allowed);
        assert!(!updated.authority.approval_allowed);
        assert!(!updated.authority.canonical_write_allowed);
    }

    #[test]
    fn heartbeat_claiming_execution_is_rejected_or_downgraded() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        approved_child_fixture(&core_cfg, &child_cfg);
        let mut authority = ChildAuthority::observe_only();
        authority.execution_allowed = true;

        let report = child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            Some(authority),
        )
        .expect("heartbeat");
        let audit = fs::read_to_string(PairPaths::new(&core_cfg).audit).expect("audit");

        assert!(!report.accepted);
        assert!(audit.contains("heartbeat_authority_claim_rejected"));
    }

    #[test]
    fn pending_child_heartbeat_not_healthy() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let invite = create_invite(&core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        child_join_by_url(&child_cfg, Some(&core_cfg), &invite.local_url, None).expect("join");

        let report = child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            None,
        )
        .expect("heartbeat");

        assert!(!report.accepted);
        assert_eq!(report.health, HeartbeatHealth::Pending);
        assert!(
            list_children(&core_cfg, true, false)
                .expect("list")
                .approved
                .is_empty()
        );
    }

    #[test]
    fn unknown_child_heartbeat_rejected_or_untrusted() {
        let (_tmp, core_cfg) = test_cfg();
        let report = submit_heartbeat(
            &core_cfg,
            heartbeat_payload("child-unknown", None, "unknown-fp", now_secs()),
        )
        .expect("heartbeat");

        assert!(!report.accepted);
        assert_eq!(report.health, HeartbeatHealth::Unknown);
        assert!(
            list_children(&core_cfg, true, false)
                .expect("list")
                .approved
                .is_empty()
        );
    }

    #[test]
    fn revoked_child_heartbeat_is_not_healthy() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let (_join, child) = approved_child_fixture(&core_cfg, &child_cfg);
        revoke_child(&core_cfg, &child.node_id).expect("revoke");

        let report = child_heartbeat(
            &child_cfg,
            Some(&core_cfg),
            Some("http://127.0.0.1:8787"),
            None,
        )
        .expect("heartbeat");
        let status = status(&core_cfg, "127.0.0.1:8787").expect("status");
        let audit = fs::read_to_string(PairPaths::new(&core_cfg).audit).expect("audit");

        assert!(!report.accepted);
        assert_eq!(report.health, HeartbeatHealth::Revoked);
        assert_eq!(status.healthy_child_count, 0);
        assert_eq!(status.revoked_child_count, 1);
        assert!(audit.contains("heartbeat_rejected_revoked_child"));
    }

    #[test]
    fn stale_heartbeat_classified_stale() {
        let (tmp, core_cfg) = test_cfg();
        let child_cfg = child_cfg(tmp.path());
        let (_join, child) = approved_child_fixture(&core_cfg, &child_cfg);
        let report = submit_heartbeat(
            &core_cfg,
            heartbeat_payload(
                &child.node_id,
                Some(&child.request_id),
                "child-fp",
                now_secs().saturating_sub(heartbeat_fresh_seconds() + 1),
            ),
        )
        .expect("heartbeat");
        let status = status(&core_cfg, "127.0.0.1:8787").expect("status");

        assert!(!report.accepted);
        assert_eq!(report.health, HeartbeatHealth::Stale);
        assert_eq!(status.healthy_child_count, 0);
        assert_eq!(status.stale_child_count, 1);
    }

    #[test]
    fn heartbeat_payload_contains_no_secrets() {
        let payload = heartbeat_payload("child-1", Some("req-1"), "child-fp", now_secs());
        let raw = serde_json::to_string(&payload)
            .expect("json")
            .to_lowercase();

        for forbidden in [
            "openrouter",
            "openai",
            "codex",
            "claude",
            "gemini",
            "broker",
            "exchange",
            "sportsbook",
            "api_key",
            "token",
            "secret",
        ] {
            assert!(!raw.contains(forbidden), "{forbidden}");
        }
    }

    #[test]
    fn manual_approval_grants_observe_only() {
        let (_tmp, cfg) = test_cfg();
        let invite = create_invite(&cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let request =
            submit_pair_request(&cfg, request_input(&invite, "execute")).expect("request");
        let child = approve_request(&cfg, &request.request_id).expect("approve");
        assert_eq!(child.authority.authority, "observe-only");
        assert!(!child.authority.execution_allowed);
        assert!(!child.authority.provider_calls_allowed);
        assert!(!child.authority.canonical_write_allowed);
        assert!(!child.authority.approval_allowed);
    }

    #[test]
    fn deny_blocks_later_approval() {
        let (_tmp, cfg) = test_cfg();
        let invite = create_invite(&cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let request =
            submit_pair_request(&cfg, request_input(&invite, "observe")).expect("request");
        deny_request(&cfg, &request.request_id).expect("deny");
        assert!(approve_request(&cfg, &request.request_id).is_err());
    }

    #[test]
    fn revoke_blocks_child() {
        let (_tmp, cfg) = test_cfg();
        let invite = create_invite(&cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let request =
            submit_pair_request(&cfg, request_input(&invite, "observe")).expect("request");
        let child = approve_request(&cfg, &request.request_id).expect("approve");
        let revoked = revoke_child(&cfg, &child.node_id).expect("revoke");
        assert_eq!(revoked.status, ChildStatus::Revoked);
        assert!(!child_can_receive_work(&revoked));
    }

    #[test]
    fn expired_invite_rejects_request() {
        let (_tmp, cfg) = test_cfg();
        let mut invite = create_invite(&cfg, "127.0.0.1:8787", 1, false, false)
            .expect("invite")
            .invite;
        invite.expires_at = now_secs().saturating_sub(1);
        let paths = PairPaths::new(&cfg);
        write_json(&paths.invite_path(&invite.invite_id), &invite).expect("write expired");
        let request =
            submit_pair_request(&cfg, request_input(&invite, "observe")).expect("request");
        assert_eq!(request.status, PairRequestStatus::Expired);
    }

    #[test]
    fn public_bind_requires_warning_or_flag() {
        assert!(trusted_lan_warning().contains("same trusted local network"));
        assert!(trusted_lan_warning().contains("Wi-Fi is supported"));
        assert!(trusted_lan_warning().contains("Ethernet is optional"));
        let report = cockpit(&test_cfg().1, "0.0.0.0:8787", false, true).expect("cockpit");
        assert_eq!(report.bind, "0.0.0.0:8787");
    }

    #[test]
    fn pairing_preflight_blocks_read_only_workspace() {
        let (tmp, mut cfg) = test_cfg();
        cfg.workspace_dir = tmp.path().join("blocked-workspace");
        fs::create_dir_all(&cfg.workspace_dir).expect("workspace dir");
        fs::write(cfg.workspace_dir.join("state"), b"not a directory").expect("state blocker");

        let err = preflight_pairing_writes(&cfg).expect_err("preflight should fail");
        let message = format!("{err:#}");

        assert!(message.contains("write pairing invite"));
        assert!(message.contains("state/pairing/invites"));
    }

    fn request_input(invite: &PairInvite, authority: &str) -> PairRequestInput {
        PairRequestInput {
            invite_id: invite.invite_id.clone(),
            claimed_device_name: "android-tablet-01".to_string(),
            claimed_role: "agent-cluster-child-worker".to_string(),
            claimed_surface: "termux".to_string(),
            runtime_kind: "quant-m-child-local".to_string(),
            device_class: "android-tablet".to_string(),
            os: Some("android".to_string()),
            architecture: Some("arm64".to_string()),
            requested_authority: authority.to_string(),
            core_url: invite.local_url.clone(),
            child_fingerprint: Some("child-fp".to_string()),
        }
    }

    fn approved_child_fixture(
        core_cfg: &Config,
        child_cfg: &Config,
    ) -> (ChildJoinReport, ChildRecord) {
        let invite = create_invite(core_cfg, "127.0.0.1:8787", 5, false, false)
            .expect("invite")
            .invite;
        let join =
            child_join_by_url(child_cfg, Some(core_cfg), &invite.local_url, None).expect("join");
        let child = approve_request(core_cfg, &join.request.request_id).expect("approve");
        (join, child)
    }

    fn heartbeat_payload(
        node_id: &str,
        request_id: Option<&str>,
        child_fingerprint: &str,
        timestamp: u64,
    ) -> ChildHeartbeatPayload {
        ChildHeartbeatPayload {
            node_id: node_id.to_string(),
            request_id: request_id.map(ToOwned::to_owned),
            child_fingerprint: child_fingerprint.to_string(),
            device_name: "android-tablet-01".to_string(),
            claimed_role: "agent-cluster-child-worker".to_string(),
            authority: ChildAuthority::observe_only(),
            timestamp,
            os: "android".to_string(),
            architecture: "arm64".to_string(),
            runtime_surface: "termux".to_string(),
            child_binary_version: env!("CARGO_PKG_VERSION").to_string(),
            core_url: "http://127.0.0.1:8787".to_string(),
            active_pack_hash: None,
            battery_status: None,
            storage_status: None,
            network_status: None,
        }
    }
}
