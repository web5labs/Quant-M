use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
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
    pub last_audit_event: Option<PairAuditEvent>,
}

#[derive(Debug, Clone, Serialize)]
pub struct PairCockpitReport {
    pub role: String,
    pub workspace: PathBuf,
    pub bind: String,
    pub port: u16,
    pub local_url: String,
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
    pub qr_rendered: bool,
    pub qr_warning: Option<String>,
    pub manual_fallback_command: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ChildListReport {
    pub pending: Vec<PairRequest>,
    pub approved: Vec<ChildRecord>,
    pub revoked: Vec<ChildRecord>,
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
}

#[allow(dead_code)]
pub fn default_bind() -> &'static str {
    DEFAULT_BIND
}

pub fn create_invite(
    cfg: &Config,
    bind: &str,
    ttl_minutes: u64,
    qr: bool,
    dry_run: bool,
) -> Result<DeviceAddReport> {
    let paths = PairPaths::new(cfg);
    let invite = build_invite(cfg, bind, ttl_minutes.max(1));
    let (qr_rendered, qr_warning) = render_qr_status(&invite.local_url, qr);
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
        invite,
        qr_rendered,
        qr_warning,
    })
}

pub fn cockpit(cfg: &Config, bind: &str, qr: bool, dry_run: bool) -> Result<PairCockpitReport> {
    if !dry_run {
        preflight_pairing_writes(cfg)?;
    }
    let paths = PairPaths::new(cfg);
    let pending = list_pending_requests(&paths)?;
    let children = list_child_records(&paths)?;
    let local_url = base_url(bind);
    let (qr_rendered, qr_warning) = render_qr_status(&local_url, qr);
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
        local_url,
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

pub fn status(cfg: &Config, bind: &str) -> Result<PairStatusReport> {
    let paths = PairPaths::new(cfg);
    let pending = list_pending_requests(&paths)?;
    let children = list_child_records(&paths)?;
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
    let node_id = format!("child-{}", safe_id(request_id));
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
                (
                    "200 OK",
                    "text/plain; charset=utf-8",
                    render_join_page(cfg, bind, invite_id),
                )
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

pub fn render_status(report: &PairStatusReport) -> String {
    format!(
        "pairing_status: {}\nbind: {}\nport: {}\npending_requests: {}\napproved_children: {}\nrevoked_children: {}\nlast_audit_event: {}\n",
        report.server_status,
        report.bind,
        report.port,
        report.pending_request_count,
        report.approved_child_count,
        report.revoked_child_count,
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
    "Camera QR scanning is not implemented in this runtime yet. Open the QR URL manually or paste it with quant-m child join --url <url>.\n\nTermux/manual fallback:\n  pkg update\n  pkg install curl openssh termux-api\n  quant-m child join --url http://<core-lan-ip>:8787/join/<invite_id>\n\nThe child requests observe-only authority, stores no provider keys, and waits for manual core approval.\n".to_string()
}

pub fn join_metadata(cfg: &Config, bind: &str, invite_id: &str) -> Result<JoinMetadata> {
    let paths = PairPaths::new(cfg);
    let invite: PairInvite = read_json(&paths.invite_path(invite_id))?;
    if invite.expires_at <= now_secs() {
        anyhow::bail!(
            "invite {} is expired; expires_at={}",
            invite.invite_id,
            invite.expires_at
        );
    }
    let core_url = base_url(bind);
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

fn build_invite(cfg: &Config, bind: &str, ttl_minutes: u64) -> PairInvite {
    let invite_id = make_id("inv");
    let local_url = format!("{}/join/{}", base_url(bind), invite_id);
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

fn render_join_page(cfg: &Config, bind: &str, invite_id: &str) -> String {
    format!(
        "Quant-M child join\ncore_name: {}\ncore_fingerprint: {}\ninvite_id: {}\nmanual_command: quant-m child join --url {}/join/{}\npair_command: quant-m child join --url {}/join/{}\n{}\n",
        cfg.node_id,
        core_fingerprint(cfg),
        invite_id,
        base_url(bind),
        invite_id,
        base_url(bind),
        invite_id,
        render_safety_status()
    )
}

fn render_qr_status(url: &str, qr: bool) -> (bool, Option<String>) {
    render_qr_status_for(url, qr, false)
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
    "warning: trusted LAN only; do not expose pairing to the public internet; no secrets should be placed on children; children remain observe-only\n".to_string()
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
        assert!(trusted_lan_warning().contains("trusted LAN only"));
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
}
