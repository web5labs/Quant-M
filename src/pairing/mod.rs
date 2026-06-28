pub mod approval;
pub mod fingerprint;
pub mod invite;
pub mod qr;
pub mod request;
pub mod server;
pub mod storage;
pub mod token;

use crate::cluster::{self, ClusterCapability};
use crate::cluster_boundary::ClusterSurfaceKind;
use crate::config::Config;
use crate::desk_registry::DeskId;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::Duration as StdDuration;

const DEFAULT_TTL_MINUTES: i64 = 10;
const MAX_TTL_MINUTES: i64 = 30;
const DEV_AUTO_ACCEPT_MAX_TTL_MINUTES: i64 = 5;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum PairingAuthority {
    Observe,
    Analyze,
}

impl Default for PairingAuthority {
    fn default() -> Self {
        Self::Observe
    }
}

impl fmt::Display for PairingAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observe => f.write_str("observe"),
            Self::Analyze => f.write_str("analyze"),
        }
    }
}

impl FromStr for PairingAuthority {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "observe" => Ok(Self::Observe),
            "analyze" => Ok(Self::Analyze),
            "propose" | "approve" | "execute" | "canonical_write" | "canonical-write" => {
                Err(anyhow!("pairing authority '{}' is not allowed", value))
            }
            other => Err(anyhow!("unsupported pairing authority '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingInvite {
    pub invite_id: String,
    pub invite_token_hash: String,
    pub created_at: String,
    pub expires_at: String,
    pub desk_id: Option<DeskId>,
    pub requested_role: Option<String>,
    pub requested_node_name: Option<String>,
    pub max_authority: PairingAuthority,
    pub core_url: String,
    pub core_fingerprint: String,
    pub one_time: bool,
    pub used: bool,
    pub revoked: bool,
    pub dev_auto_accept: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingInviteView {
    pub invite: PairingInvite,
    pub invite_token: String,
    pub local_link: String,
    pub child_command: String,
    pub qr_payload: String,
    pub execution_enabled: bool,
    pub canonical_write_enabled: bool,
    pub approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PairingRequestStatus {
    Pending,
    Approved,
    Rejected,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingRequest {
    pub request_id: String,
    pub invite_id: String,
    pub node_display_name: String,
    pub node_public_key: String,
    pub surface: String,
    pub claimed_capabilities: Vec<String>,
    pub compute_claims_present: bool,
    pub requested_role: Option<String>,
    pub requested_authority: PairingAuthority,
    pub requested_at: String,
    pub source_addr: Option<String>,
    pub status: PairingRequestStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AcceptedPairedNode {
    pub node_id: String,
    pub node_display_name: String,
    pub node_public_key: String,
    #[serde(default)]
    pub node_auth_token_hash: String,
    pub accepted_at: String,
    pub accepted_by: String,
    pub initial_desk_id: Option<DeskId>,
    pub initial_role: Option<String>,
    pub initial_lease_expires_at: Option<String>,
    pub authority_level: PairingAuthority,
    pub execution_enabled: bool,
    pub canonical_write_enabled: bool,
    pub approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingEvent {
    pub event_id: String,
    pub timestamp: String,
    pub kind: String,
    pub invite_id: Option<String>,
    pub request_id: Option<String>,
    pub node_id: Option<String>,
    pub reason: String,
    pub replay_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildIdentity {
    pub node_display_name: String,
    pub node_public_key: String,
    pub created_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildCorePairing {
    pub core_url: String,
    pub core_fingerprint: String,
    pub request_id: String,
    pub node_id: Option<String>,
    pub status: PairingRequestStatus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PairingInviteOptions<'a> {
    pub name: Option<&'a str>,
    pub desk: Option<&'a str>,
    pub role: Option<&'a str>,
    pub ttl: Duration,
    pub core_url: &'a str,
    pub dev_auto_accept: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChildPairRequestInput<'a> {
    pub core_url: &'a str,
    pub invite_token: &'a str,
    pub node_name: Option<&'a str>,
    pub surface: &'a str,
    pub capabilities: &'a [String],
    pub requested_role: Option<&'a str>,
    pub requested_authority: PairingAuthority,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingServerRequestPayload {
    pub invite_token: String,
    pub node_display_name: String,
    pub node_public_key: String,
    pub surface: String,
    #[serde(default)]
    pub claimed_capabilities: Vec<String>,
    #[serde(default)]
    pub requested_role: Option<String>,
    #[serde(default)]
    pub requested_authority: PairingAuthority,
    #[serde(default)]
    pub execution_enabled: bool,
    #[serde(default)]
    pub canonical_write_enabled: bool,
    #[serde(default)]
    pub approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterHeartbeatServerPayload {
    pub node_id: String,
    #[serde(default)]
    pub node_auth_token: String,
    #[serde(default)]
    pub surface: Option<String>,
    #[serde(default)]
    pub claimed_capabilities: Vec<String>,
    #[serde(default)]
    pub execution_enabled: bool,
    #[serde(default)]
    pub canonical_write_enabled: bool,
    #[serde(default)]
    pub approval_enabled: bool,
    #[serde(default)]
    pub device_telemetry: Option<crate::device_telemetry::DeviceTelemetry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairingStatusResponse {
    pub request_id: String,
    pub status: PairingRequestStatus,
    pub node_id: Option<String>,
    #[serde(default)]
    pub node_auth_token: Option<String>,
    pub execution_enabled: bool,
    pub canonical_write_enabled: bool,
    pub approval_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct PairingScanPayload {
    pub core_url: String,
    pub invite_token: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PairDoctorReport {
    pub pairing_feature_enabled: bool,
    pub core_fingerprint_exists: bool,
    pub core_fingerprint: Option<String>,
    pub pairing_state_dir_exists: bool,
    pub active_invites: usize,
    pub pending_requests: usize,
    pub accepted_nodes: usize,
    pub server_bind_warning: String,
    pub lan_url_hint: String,
    pub authority_boundary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChildDoctorReport {
    pub child_identity_exists: bool,
    pub node_display_name: Option<String>,
    pub node_public_key_present: bool,
    pub paired: bool,
    pub approved: bool,
    pub paired_core_url: Option<String>,
    pub core_fingerprint_stored: Option<String>,
    pub last_pairing_status: Option<PairingRequestStatus>,
    pub paired_node_id: Option<String>,
    pub last_heartbeat_status: String,
    pub active_lease_id: Option<String>,
    pub active_desk_id: Option<String>,
    pub active_role_id: Option<String>,
    pub jobs_enabled: bool,
    pub execution_enabled: bool,
    pub approval_enabled: bool,
    pub canonical_write_enabled: bool,
}

struct PairingPaths {
    dir: PathBuf,
    invites: PathBuf,
    requests: PathBuf,
    accepted_nodes: PathBuf,
    revoked_invites: PathBuf,
    events: PathBuf,
    core_fingerprint: PathBuf,
    child_identity: PathBuf,
    child_pairing: PathBuf,
    child_core: PathBuf,
}

impl PairingPaths {
    fn new(cfg: &Config) -> Self {
        let dir = cfg.workspace_dir.join("state/pairing");
        let child = cfg.workspace_dir.join("child");
        Self {
            invites: dir.join("invites.jsonl"),
            requests: dir.join("requests.jsonl"),
            accepted_nodes: dir.join("accepted-nodes.jsonl"),
            revoked_invites: dir.join("revoked-invites.jsonl"),
            events: dir.join("events.jsonl"),
            core_fingerprint: dir.join("core-fingerprint.json"),
            child_identity: child.join("identity.toml"),
            child_pairing: child.join("pairing.toml"),
            child_core: child.join("core.toml"),
            dir,
        }
    }

    fn ensure(&self) -> Result<()> {
        fs::create_dir_all(&self.dir)?;
        if let Some(parent) = self.child_identity.parent() {
            fs::create_dir_all(parent)?;
        }
        Ok(())
    }
}

pub fn core_fingerprint(cfg: &Config) -> Result<String> {
    let paths = PairingPaths::new(cfg);
    paths.ensure()?;
    if paths.core_fingerprint.exists() {
        let value: Value = serde_json::from_str(&fs::read_to_string(&paths.core_fingerprint)?)?;
        if let Some(fp) = value.get("core_fingerprint").and_then(Value::as_str) {
            return Ok(fp.to_string());
        }
    }
    let fp = format!("core-fp-{}", token::short_hash(&token::generate_token()));
    fs::write(
        &paths.core_fingerprint,
        serde_json::to_string_pretty(&serde_json::json!({
            "core_fingerprint": fp,
            "created_at": now(),
        }))?,
    )?;
    Ok(fp)
}

pub fn create_invite(cfg: &Config, options: PairingInviteOptions<'_>) -> Result<PairingInviteView> {
    let paths = PairingPaths::new(cfg);
    paths.ensure()?;
    let ttl = bounded_ttl(options.ttl, options.dev_auto_accept)?;
    let token = token::generate_token();
    let invite = PairingInvite {
        invite_id: format!("pair_{}", token::short_hash(&token)),
        invite_token_hash: token::hash_token(&token),
        created_at: now(),
        expires_at: (Utc::now() + ttl).to_rfc3339(),
        desk_id: options.desk.map(DeskId::new),
        requested_role: options.role.map(str::to_string),
        requested_node_name: options.name.map(str::to_string),
        max_authority: PairingAuthority::Observe,
        core_url: options.core_url.trim_end_matches('/').to_string(),
        core_fingerprint: core_fingerprint(cfg)?,
        one_time: true,
        used: false,
        revoked: false,
        dev_auto_accept: options.dev_auto_accept,
    };
    append_json_line(&paths.invites, &invite)?;
    append_event(
        cfg,
        "pairing_invite_created",
        Some(&invite.invite_id),
        None,
        None,
        "short-lived observe-only invite created",
    )?;
    let local_link = local_link(&invite.core_url, &token);
    Ok(PairingInviteView {
        qr_payload: local_link.clone(),
        child_command: format!(
            "quant-m child pair --core {} --invite {}",
            invite.core_url, token
        ),
        local_link,
        invite,
        invite_token: token,
        execution_enabled: false,
        canonical_write_enabled: false,
        approval_enabled: false,
    })
}

pub fn list_invites(cfg: &Config) -> Result<Vec<PairingInvite>> {
    read_jsonl(PairingPaths::new(cfg).invites)
}

pub fn list_requests(cfg: &Config) -> Result<Vec<PairingRequest>> {
    read_jsonl(PairingPaths::new(cfg).requests)
}

pub fn list_events(cfg: &Config) -> Result<Vec<PairingEvent>> {
    read_jsonl(PairingPaths::new(cfg).events)
}

pub fn list_accepted_nodes(cfg: &Config) -> Result<Vec<AcceptedPairedNode>> {
    read_jsonl(PairingPaths::new(cfg).accepted_nodes)
}

pub fn pair_doctor(cfg: &Config, bind: &str) -> Result<PairDoctorReport> {
    let paths = PairingPaths::new(cfg);
    let invites = list_invites(cfg)?;
    let requests = list_requests(cfg)?;
    let accepted = list_accepted_nodes(cfg)?;
    let active_invites = invites
        .iter()
        .filter(|invite| !invite.revoked && !invite.used && !invite_expired(invite))
        .count();
    let pending_requests = requests
        .iter()
        .filter(|request| request.status == PairingRequestStatus::Pending)
        .count();
    Ok(PairDoctorReport {
        pairing_feature_enabled: true,
        core_fingerprint_exists: paths.core_fingerprint.exists(),
        core_fingerprint: read_core_fingerprint(&paths)?,
        pairing_state_dir_exists: paths.dir.exists(),
        active_invites,
        pending_requests,
        accepted_nodes: accepted.len(),
        server_bind_warning: pairing_bind_warning(bind),
        lan_url_hint: lan_url_hint(&invites),
        authority_boundary: "pairing enrolls observe-only children; it does not grant leases, jobs, compute trust, approval, canonical writes, provider calls, trades, or bets".to_string(),
    })
}

pub fn child_doctor(cfg: &Config) -> Result<ChildDoctorReport> {
    let identity = child_identity(cfg)?;
    let pairing = child_pairing(cfg)?;
    let paired_node_id = pairing.as_ref().and_then(|pairing| pairing.node_id.clone());
    let status = paired_node_id
        .as_deref()
        .and_then(|node_id| node_id.parse().ok())
        .and_then(|node_id| cluster::node_status(cfg, &node_id).ok());
    Ok(ChildDoctorReport {
        child_identity_exists: identity.is_some(),
        node_display_name: identity
            .as_ref()
            .map(|identity| identity.node_display_name.clone()),
        node_public_key_present: identity
            .as_ref()
            .is_some_and(|identity| !identity.node_public_key.trim().is_empty()),
        paired: status.as_ref().is_some_and(|status| status.paired),
        approved: status.as_ref().is_some_and(|status| status.approved),
        paired_core_url: pairing.as_ref().map(|pairing| pairing.core_url.clone()),
        core_fingerprint_stored: pairing
            .as_ref()
            .map(|pairing| pairing.core_fingerprint.clone()),
        last_pairing_status: pairing.as_ref().map(|pairing| pairing.status.clone()),
        last_heartbeat_status: child_heartbeat_status(cfg, paired_node_id.as_deref())?,
        active_lease_id: status
            .as_ref()
            .and_then(|status| status.active_lease_id.clone()),
        active_desk_id: status
            .as_ref()
            .and_then(|status| status.active_desk_id.clone()),
        active_role_id: status
            .as_ref()
            .and_then(|status| status.active_role_id.as_ref().map(ToString::to_string)),
        jobs_enabled: false,
        paired_node_id,
        execution_enabled: false,
        approval_enabled: false,
        canonical_write_enabled: false,
    })
}

pub fn revoke_invite(cfg: &Config, invite_id: &str) -> Result<PairingInvite> {
    let paths = PairingPaths::new(cfg);
    let mut invites = list_invites(cfg)?;
    let invite = invites
        .iter_mut()
        .find(|invite| invite.invite_id == invite_id)
        .ok_or_else(|| anyhow!("pairing invite '{}' not found", invite_id))?;
    invite.revoked = true;
    let revoked = invite.clone();
    rewrite_jsonl(&paths.invites, &invites)?;
    append_json_line(&paths.revoked_invites, &revoked)?;
    append_event(
        cfg,
        "pairing_invite_revoked",
        Some(invite_id),
        None,
        None,
        "operator revoked pairing invite",
    )?;
    Ok(revoked)
}

pub fn submit_child_pair_request(
    cfg: &Config,
    input: ChildPairRequestInput<'_>,
) -> Result<PairingRequest> {
    let identity = load_or_create_child_identity(cfg, input.node_name.unwrap_or("child-node"))?;
    let (request, invite) = submit_pairing_request_with_identity(
        cfg,
        PairingRequestIdentityInput {
            core_url: Some(input.core_url),
            invite_token: input.invite_token,
            node_display_name: &identity.node_display_name,
            node_public_key: &identity.node_public_key,
            surface: input.surface,
            capabilities: input.capabilities,
            requested_role: input.requested_role,
            requested_authority: input.requested_authority,
            source_addr: Some("local_cli"),
        },
    )?;
    if invite.dev_auto_accept {
        let accepted = approve_request(cfg, &request.request_id, "dev_auto_accept")?;
        store_child_pairing(
            cfg,
            &ChildCorePairing {
                core_url: input.core_url.trim_end_matches('/').to_string(),
                core_fingerprint: invite.core_fingerprint,
                request_id: request.request_id.clone(),
                node_id: Some(accepted.node_id),
                status: PairingRequestStatus::Approved,
            },
        )?;
    } else {
        store_child_pairing(
            cfg,
            &ChildCorePairing {
                core_url: input.core_url.trim_end_matches('/').to_string(),
                core_fingerprint: invite.core_fingerprint,
                request_id: request.request_id.clone(),
                node_id: None,
                status: PairingRequestStatus::Pending,
            },
        )?;
    }
    Ok(request)
}

pub fn submit_server_pair_request(
    cfg: &Config,
    payload: PairingServerRequestPayload,
    source_addr: Option<&str>,
) -> Result<PairingRequest> {
    if payload.execution_enabled || payload.canonical_write_enabled || payload.approval_enabled {
        return Err(anyhow!(
            "pairing request cannot claim execution, approval, or canonical write authority"
        ));
    }
    let (request, _invite) = submit_pairing_request_with_identity(
        cfg,
        PairingRequestIdentityInput {
            core_url: None,
            invite_token: &payload.invite_token,
            node_display_name: &payload.node_display_name,
            node_public_key: &payload.node_public_key,
            surface: &payload.surface,
            capabilities: &payload.claimed_capabilities,
            requested_role: payload.requested_role.as_deref(),
            requested_authority: payload.requested_authority,
            source_addr,
        },
    )?;
    Ok(request)
}

struct PairingRequestIdentityInput<'a> {
    core_url: Option<&'a str>,
    invite_token: &'a str,
    node_display_name: &'a str,
    node_public_key: &'a str,
    surface: &'a str,
    capabilities: &'a [String],
    requested_role: Option<&'a str>,
    requested_authority: PairingAuthority,
    source_addr: Option<&'a str>,
}

fn submit_pairing_request_with_identity(
    cfg: &Config,
    input: PairingRequestIdentityInput<'_>,
) -> Result<(PairingRequest, PairingInvite)> {
    if input.node_public_key.trim().is_empty() {
        return Err(anyhow!("pairing request missing node public key"));
    }
    if input.node_display_name.trim().is_empty() {
        return Err(anyhow!("pairing request missing node display name"));
    }
    let paths = PairingPaths::new(cfg);
    paths.ensure()?;
    let invites = list_invites(cfg)?;
    let invite_index = find_valid_invite_index(&invites, input.invite_token)?;
    let invite = invites[invite_index].clone();
    if let Some(core_url) = input.core_url
        && invite.core_url != core_url.trim_end_matches('/')
    {
        return Err(anyhow!("core URL does not match invite"));
    }
    if input.requested_authority > invite.max_authority {
        return Err(anyhow!("pairing request exceeds invite authority"));
    }
    if let Some(invite_role) = invite.requested_role.as_deref()
        && input.requested_role.is_some_and(|role| role != invite_role)
    {
        return Err(anyhow!("requested role differs from invite role"));
    }
    reject_forbidden_claims(input.capabilities)?;
    let request = PairingRequest {
        request_id: format!("pair_req_{}", token::short_hash(&token::generate_token())),
        invite_id: invite.invite_id.clone(),
        node_display_name: input.node_display_name.to_string(),
        node_public_key: input.node_public_key.to_string(),
        surface: input.surface.to_string(),
        claimed_capabilities: input.capabilities.to_vec(),
        compute_claims_present: input.capabilities.iter().any(|cap| cap.contains("compute")),
        requested_role: input
            .requested_role
            .map(str::to_string)
            .or(invite.requested_role.clone()),
        requested_authority: input.requested_authority,
        requested_at: now(),
        source_addr: input.source_addr.map(str::to_string),
        status: PairingRequestStatus::Pending,
    };
    append_json_line(&paths.requests, &request)?;
    append_event(
        cfg,
        "pairing_request_received",
        Some(&invite.invite_id),
        Some(&request.request_id),
        None,
        "child submitted observe-only pairing request",
    )?;
    Ok((request, invite))
}

pub fn approve_request(
    cfg: &Config,
    request_id: &str,
    accepted_by: &str,
) -> Result<AcceptedPairedNode> {
    let paths = PairingPaths::new(cfg);
    let mut requests = list_requests(cfg)?;
    let request = requests
        .iter_mut()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| anyhow!("pairing request '{}' not found", request_id))?;
    if request.status != PairingRequestStatus::Pending {
        return Err(anyhow!("pairing request is not pending"));
    }
    let invite = list_invites(cfg)?
        .into_iter()
        .find(|invite| invite.invite_id == request.invite_id)
        .ok_or_else(|| anyhow!("pairing invite for request not found"))?;
    if invite_expired(&invite) {
        request.status = PairingRequestStatus::Expired;
        rewrite_jsonl(&paths.requests, &requests)?;
        return Err(anyhow!("pairing invite expired"));
    }
    let surface = request
        .surface
        .parse::<ClusterSurfaceKind>()
        .context("invalid paired node surface")?;
    let capabilities = parse_claimed_capabilities(&request.claimed_capabilities)?;
    let node = cluster::register_node(cfg, &request.node_display_name, surface, capabilities)?;
    let node_auth_token = make_node_auth_token(&request.request_id, &request.node_public_key);
    let accepted = AcceptedPairedNode {
        node_id: node.node_id.to_string(),
        node_display_name: request.node_display_name.clone(),
        node_public_key: request.node_public_key.clone(),
        node_auth_token_hash: token::hash_token(&node_auth_token),
        accepted_at: now(),
        accepted_by: accepted_by.to_string(),
        initial_desk_id: invite.desk_id,
        initial_role: request.requested_role.clone(),
        initial_lease_expires_at: None,
        authority_level: PairingAuthority::Observe,
        execution_enabled: false,
        canonical_write_enabled: false,
        approval_enabled: false,
    };
    request.status = PairingRequestStatus::Approved;
    rewrite_jsonl(&paths.requests, &requests)?;
    mark_invite_used(cfg, &invite.invite_id)?;
    append_json_line(&paths.accepted_nodes, &accepted)?;
    if let Some(mut pairing) = child_pairing(cfg)?
        && pairing.request_id == request_id
    {
        pairing.node_id = Some(accepted.node_id.clone());
        pairing.status = PairingRequestStatus::Approved;
        store_child_pairing(cfg, &pairing)?;
    }
    append_event(
        cfg,
        "pairing_request_approved",
        Some(&invite.invite_id),
        Some(request_id),
        Some(&accepted.node_id),
        "approved as observe-only paired child",
    )?;
    Ok(accepted)
}

pub fn reject_request(cfg: &Config, request_id: &str, reason: &str) -> Result<PairingRequest> {
    let paths = PairingPaths::new(cfg);
    let mut requests = list_requests(cfg)?;
    let request = requests
        .iter_mut()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| anyhow!("pairing request '{}' not found", request_id))?;
    request.status = PairingRequestStatus::Rejected;
    let rejected = request.clone();
    rewrite_jsonl(&paths.requests, &requests)?;
    append_event(
        cfg,
        "pairing_request_rejected",
        Some(&rejected.invite_id),
        Some(request_id),
        None,
        reason,
    )?;
    Ok(rejected)
}

pub fn load_or_create_child_identity(cfg: &Config, node_name: &str) -> Result<ChildIdentity> {
    let paths = PairingPaths::new(cfg);
    paths.ensure()?;
    if paths.child_identity.exists() {
        let raw = fs::read_to_string(&paths.child_identity)?;
        return toml::from_str(&raw).context("failed to parse child identity");
    }
    let identity = ChildIdentity {
        node_display_name: node_name.to_string(),
        node_public_key: format!("child-pub-{}", token::short_hash(&token::generate_token())),
        created_at: now(),
    };
    fs::write(&paths.child_identity, toml::to_string_pretty(&identity)?)?;
    append_event(
        cfg,
        "child_identity_created",
        None,
        None,
        None,
        "local child identity created",
    )?;
    Ok(identity)
}

pub fn child_identity(cfg: &Config) -> Result<Option<ChildIdentity>> {
    let path = PairingPaths::new(cfg).child_identity;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(toml::from_str(&fs::read_to_string(path)?)?))
}

pub fn child_pairing(cfg: &Config) -> Result<Option<ChildCorePairing>> {
    let path = PairingPaths::new(cfg).child_pairing;
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(toml::from_str(&fs::read_to_string(path)?)?))
}

pub fn unpair_child(cfg: &Config) -> Result<()> {
    let paths = PairingPaths::new(cfg);
    for path in [&paths.child_pairing, &paths.child_core] {
        if path.exists() {
            fs::remove_file(path)?;
        }
    }
    Ok(())
}

#[allow(dead_code)]
pub fn invite_payload(core_url: &str, invite_token: &str) -> String {
    local_link(core_url.trim_end_matches('/'), invite_token)
}

pub fn render_qr_to_terminal(payload: &str) -> Result<String> {
    #[cfg(feature = "pairing-qr")]
    {
        let code = qrcode::QrCode::new(payload.as_bytes())?;
        Ok(code
            .render::<qrcode::render::unicode::Dense1x2>()
            .module_dimensions(1, 1)
            .build())
    }
    #[cfg(not(feature = "pairing-qr"))]
    {
        Ok(format!(
            "terminal QR unavailable; rebuild with --features pairing-qr\npayload: {payload}"
        ))
    }
}

pub fn save_qr_png(payload: &str, path: &Path) -> Result<()> {
    #[cfg(feature = "pairing-qr")]
    {
        let code = qrcode::QrCode::new(payload.as_bytes())?;
        let image = code.render::<image::Luma<u8>>().build();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        image.save(path)?;
        Ok(())
    }
    #[cfg(not(feature = "pairing-qr"))]
    {
        let _ = payload;
        let _ = path;
        Err(anyhow!("PNG QR output requires --features pairing-qr"))
    }
}

pub fn pair_scan_image(cfg: &Config, image: &Path) -> Result<PairingRequest> {
    #[cfg(feature = "pairing-scan-image")]
    {
        let decoded = decode_pairing_qr_image(image)?;
        submit_scanned_pairing_payload(cfg, &decoded)
    }
    #[cfg(not(feature = "pairing-scan-image"))]
    {
        let _ = cfg;
        let _ = image;
        Err(anyhow!(
            "QR image decode requires --features pairing-scan-image"
        ))
    }
}

#[allow(dead_code)]
pub fn submit_scanned_pairing_payload(
    cfg: &Config,
    payload: &PairingScanPayload,
) -> Result<PairingRequest> {
    let capabilities = vec!["echo".to_string(), "sleep".to_string()];
    submit_child_pair_request(
        cfg,
        ChildPairRequestInput {
            core_url: &payload.core_url,
            invite_token: &payload.invite_token,
            node_name: Some("tablet-01"),
            surface: "termux_worker",
            capabilities: &capabilities,
            requested_role: None,
            requested_authority: PairingAuthority::Observe,
        },
    )
}

#[cfg(feature = "pairing-scan-image")]
fn decode_pairing_qr_image(image_path: &Path) -> Result<PairingScanPayload> {
    let image = image::open(image_path)
        .with_context(|| format!("failed to open QR image {}", image_path.display()))?
        .to_luma8();
    let mut prepared = rqrr::PreparedImage::prepare(image);
    let grids = prepared.detect_grids();
    if grids.is_empty() {
        return Err(anyhow!("image contains no QR payload"));
    }
    let mut payloads = Vec::new();
    for grid in grids {
        let (_meta, content) = grid.decode().context("failed to decode QR payload")?;
        let payload = parse_pairing_scan_payload(&content)?;
        if !payloads.contains(&payload) {
            payloads.push(payload);
        }
    }
    if payloads.len() != 1 {
        return Err(anyhow!(
            "image contains multiple conflicting pairing payloads"
        ));
    }
    Ok(payloads.remove(0))
}

#[allow(dead_code)]
pub fn parse_pairing_scan_payload(raw: &str) -> Result<PairingScanPayload> {
    reject_secret_bearing_payload(raw)?;
    if raw.starts_with("quantm://pair") {
        parse_quantm_pair_uri(raw)
    } else if raw.starts_with("http://") || raw.starts_with("https://") {
        parse_pairing_http_url(raw)
    } else {
        Err(anyhow!("QR is not a Quant-M pairing payload"))
    }
}

#[allow(dead_code)]
fn parse_pairing_http_url(raw: &str) -> Result<PairingScanPayload> {
    let (core_url, token) = raw
        .split_once("/pair/i/")
        .ok_or_else(|| anyhow!("malformed pairing URL"))?;
    validate_local_core_url(core_url)?;
    validate_invite_token(token)?;
    Ok(PairingScanPayload {
        core_url: core_url.trim_end_matches('/').to_string(),
        invite_token: token.to_string(),
    })
}

#[allow(dead_code)]
fn parse_quantm_pair_uri(raw: &str) -> Result<PairingScanPayload> {
    let query = raw
        .split_once('?')
        .map(|(_prefix, query)| query)
        .ok_or_else(|| anyhow!("malformed quantm pairing URI"))?;
    let mut core_url = None;
    let mut invite = None;
    for part in query.split('&') {
        let Some((key, value)) = part.split_once('=') else {
            continue;
        };
        match key {
            "core" => core_url = Some(percent_decode(value)),
            "invite" => invite = Some(percent_decode(value)),
            _ => {}
        }
    }
    let core_url = core_url.ok_or_else(|| anyhow!("quantm pairing URI missing core"))?;
    let invite_token = invite.ok_or_else(|| anyhow!("quantm pairing URI missing invite"))?;
    validate_local_core_url(&core_url)?;
    validate_invite_token(&invite_token)?;
    Ok(PairingScanPayload {
        core_url: core_url.trim_end_matches('/').to_string(),
        invite_token,
    })
}

#[allow(dead_code)]
fn validate_invite_token(token: &str) -> Result<()> {
    if token.len() < 24
        || token.len() > 128
        || !token
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '-' || ch == '_')
    {
        return Err(anyhow!("malformed invite token"));
    }
    Ok(())
}

#[allow(dead_code)]
fn validate_local_core_url(core_url: &str) -> Result<()> {
    let Some(rest) = core_url.strip_prefix("http://") else {
        return Err(anyhow!("pairing QR must use local http URL by default"));
    };
    let host_port = rest.split('/').next().unwrap_or(rest);
    let host = host_port.split(':').next().unwrap_or(host_port);
    if host == "localhost"
        || host == "127.0.0.1"
        || host.starts_with("127.")
        || host.starts_with("10.")
        || host.starts_with("192.168.")
        || is_private_172(host)
        || host.ends_with(".local")
    {
        Ok(())
    } else {
        Err(anyhow!(
            "pairing QR public/non-local URL rejected by default"
        ))
    }
}

#[allow(dead_code)]
fn is_private_172(host: &str) -> bool {
    let mut parts = host.split('.');
    matches!(parts.next(), Some("172"))
        && parts
            .next()
            .and_then(|part| part.parse::<u8>().ok())
            .is_some_and(|octet| (16..=31).contains(&octet))
}

#[allow(dead_code)]
fn reject_secret_bearing_payload(raw: &str) -> Result<()> {
    let lower = raw.to_ascii_lowercase();
    for forbidden in [
        "private_key",
        "secret",
        "api_key",
        "broker",
        "exchange_key",
        "sportsbook",
        "execute",
        "approval",
        "canonical_write",
        "canonical-write",
    ] {
        if lower.contains(forbidden) {
            return Err(anyhow!(
                "pairing QR contains forbidden secret or authority field"
            ));
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = u8::from_str_radix(&value[i + 1..i + 3], 16)
        {
            out.push(hex);
            i += 3;
            continue;
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

pub fn serve_pairing_server(cfg: &Config, bind: &str) -> Result<()> {
    let warning = pairing_bind_warning(bind);
    append_event(cfg, "pairing_server_started", None, None, None, &warning)?;
    println!("{warning}");
    let listener = TcpListener::bind(bind)
        .with_context(|| format!("failed to bind pairing server to {bind}"))?;
    println!("Quant-M pairing server listening on http://{bind}");
    for stream in listener.incoming() {
        match stream {
            Ok(mut stream) => {
                if let Err(err) = handle_pairing_stream(cfg, &mut stream) {
                    let _ = write_http_response(
                        &mut stream,
                        500,
                        "Internal Server Error",
                        "text/plain; charset=utf-8",
                        &format!("pairing server error: {err}"),
                    );
                }
            }
            Err(err) => eprintln!("pairing server connection error: {err}"),
        }
    }
    Ok(())
}

pub fn pairing_bind_warning(bind: &str) -> String {
    if bind.starts_with("0.0.0.0") {
        "Pairing server is visible on all local interfaces. Use only on a trusted LAN.".to_string()
    } else {
        "Pairing server is local. Pairing remains enrollment-only and grants no execution authority."
            .to_string()
    }
}

fn handle_pairing_stream(cfg: &Config, stream: &mut TcpStream) -> Result<()> {
    stream
        .set_read_timeout(Some(StdDuration::from_secs(5)))
        .ok();
    stream
        .set_write_timeout(Some(StdDuration::from_secs(5)))
        .ok();
    let mut buffer = vec![0_u8; 64 * 1024];
    let read = stream.read(&mut buffer)?;
    let peer = stream.peer_addr().ok().map(|addr| addr.ip().to_string());
    let response = handle_pairing_http_request(
        cfg,
        &String::from_utf8_lossy(&buffer[..read]),
        peer.as_deref(),
    );
    write_http_response(
        stream,
        response.status_code,
        response.status_text,
        response.content_type,
        &response.body,
    )
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PairingHttpResponse {
    pub status_code: u16,
    pub status_text: &'static str,
    pub content_type: &'static str,
    pub body: String,
}

pub fn handle_pairing_http_request(
    cfg: &Config,
    raw: &str,
    source_addr: Option<&str>,
) -> PairingHttpResponse {
    match route_pairing_http_request(cfg, raw, source_addr) {
        Ok(response) => response,
        Err(err) => http_response(
            400,
            "Bad Request",
            "application/json",
            serde_json::json!({
                "error": err.to_string(),
                "execution_enabled": false,
                "canonical_write_enabled": false,
                "approval_enabled": false
            })
            .to_string(),
        ),
    }
}

fn route_pairing_http_request(
    cfg: &Config,
    raw: &str,
    source_addr: Option<&str>,
) -> Result<PairingHttpResponse> {
    let request = ParsedHttpRequest::parse(raw)?;
    match (request.method.as_str(), request.path.as_str()) {
        ("GET", path) if path.starts_with("/pair/i/") => {
            let token = path.trim_start_matches("/pair/i/");
            let invite = invite_for_token(cfg, token)?;
            Ok(http_response(
                200,
                "OK",
                "text/html; charset=utf-8",
                render_pairing_invite_page(&invite, token),
            ))
        }
        ("POST", "/pair/request") => {
            let payload: PairingServerRequestPayload =
                serde_json::from_str(&request.body).context("failed to parse pairing request")?;
            let pairing_request = submit_server_pair_request(cfg, payload, source_addr)?;
            append_event(
                cfg,
                "pairing_server_request_received",
                Some(&pairing_request.invite_id),
                Some(&pairing_request.request_id),
                None,
                "server accepted pending pairing request",
            )?;
            Ok(http_response(
                200,
                "OK",
                "application/json",
                serde_json::to_string_pretty(&serde_json::json!({
                    "request_id": pairing_request.request_id,
                    "status": pairing_request.status,
                    "execution_enabled": false,
                    "canonical_write_enabled": false,
                    "approval_enabled": false
                }))?,
            ))
        }
        ("GET", path) if path.starts_with("/pair/status/") => {
            let request_id = path.trim_start_matches("/pair/status/");
            let status = pairing_status(cfg, request_id)?;
            Ok(http_response(
                200,
                "OK",
                "application/json",
                serde_json::to_string_pretty(&status)?,
            ))
        }
        ("POST", "/cluster/heartbeat") => {
            let payload: ClusterHeartbeatServerPayload =
                serde_json::from_str(&request.body).context("failed to parse cluster heartbeat")?;
            let heartbeat = submit_server_cluster_heartbeat(cfg, payload)?;
            Ok(http_response(
                200,
                "OK",
                "application/json",
                serde_json::to_string_pretty(&serde_json::json!({
                    "heartbeat_id": heartbeat.heartbeat_id,
                    "node_id": heartbeat.node_id,
                    "paired": heartbeat.paired,
                    "approved": heartbeat.approved,
                    "execution_enabled": false,
                    "canonical_write_enabled": false,
                    "approval_enabled": false
                }))?,
            ))
        }
        _ => Ok(http_response(
            404,
            "Not Found",
            "text/plain; charset=utf-8",
            "pairing route not found".to_string(),
        )),
    }
}

fn submit_server_cluster_heartbeat(
    cfg: &Config,
    payload: ClusterHeartbeatServerPayload,
) -> Result<cluster::ClusterHeartbeat> {
    if payload.execution_enabled || payload.canonical_write_enabled || payload.approval_enabled {
        return Err(anyhow!(
            "heartbeat cannot claim execution, approval, or canonical write authority"
        ));
    }
    let node_id = payload.node_id.parse()?;
    let surface = payload.surface.as_deref().map(str::parse).transpose()?;
    let claimed_capabilities = payload
        .claimed_capabilities
        .iter()
        .map(|capability| capability.parse())
        .collect::<Result<Vec<_>>>()?;
    verify_node_auth_token(cfg, &payload.node_id, &payload.node_auth_token)?;
    cluster::heartbeat_with_input(
        cfg,
        cluster::ClusterHeartbeatInput {
            node_id,
            surface,
            claimed_capabilities,
            execution_enabled: false,
            approval_enabled: false,
            canonical_write_enabled: false,
            source: cluster::HeartbeatSource::ChildCli,
            device_telemetry: payload.device_telemetry,
        },
    )
}

fn verify_node_auth_token(cfg: &Config, node_id: &str, node_auth_token: &str) -> Result<()> {
    if node_auth_token.trim().is_empty() {
        return Err(anyhow!("heartbeat missing paired node auth token"));
    }
    let accepted = read_jsonl::<AcceptedPairedNode>(PairingPaths::new(cfg).accepted_nodes)?
        .into_iter()
        .find(|node| node.node_id == node_id)
        .ok_or_else(|| anyhow!("heartbeat node is not an accepted paired node"))?;
    if accepted.node_auth_token_hash.trim().is_empty() {
        return Err(anyhow!("paired node has no heartbeat auth token"));
    }
    if token::hash_token(node_auth_token) != accepted.node_auth_token_hash {
        return Err(anyhow!("heartbeat node auth token rejected"));
    }
    Ok(())
}

fn write_http_response(
    stream: &mut TcpStream,
    status_code: u16,
    status_text: &str,
    content_type: &str,
    body: &str,
) -> Result<()> {
    write!(
        stream,
        "HTTP/1.1 {status_code} {status_text}\r\nContent-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )?;
    stream.flush()?;
    Ok(())
}

fn http_response(
    status_code: u16,
    status_text: &'static str,
    content_type: &'static str,
    body: String,
) -> PairingHttpResponse {
    PairingHttpResponse {
        status_code,
        status_text,
        content_type,
        body,
    }
}

pub fn render_invite(
    view: &PairingInviteView,
    qr_text: Option<&str>,
    png: Option<&Path>,
) -> String {
    let mut out = format!(
        "Quant-M pairing invite created.\nInvite: {}\nDesk: {}\nRole: {}\nMax authority: {}\nExpires at: {}\nExecution: disabled\nCanonical write: disabled\nLocal link:\n{}\nChild command:\n{}\n",
        view.invite.invite_id,
        view.invite
            .desk_id
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_else(|| "none".to_string()),
        view.invite.requested_role.as_deref().unwrap_or("none"),
        view.invite.max_authority,
        view.invite.expires_at,
        view.local_link,
        view.child_command
    );
    if view.invite.dev_auto_accept {
        out.push_str(
            "DEV AUTO-ACCEPT ENABLED\nAuthority is capped at observe.\nExecution remains disabled.\n",
        );
    }
    if let Some(qr_text) = qr_text {
        out.push_str("Scan from tablet:\n");
        out.push_str(qr_text);
        out.push('\n');
    }
    if let Some(png) = png {
        out.push_str(&format!("PNG QR: {}\n", png.display()));
    }
    out
}

pub fn render_pair_doctor(report: &PairDoctorReport) -> String {
    format!(
        "pairing doctor\npairing_feature_enabled: {}\ncore_fingerprint_exists: {}\ncore_fingerprint: {}\npairing_state_dir_exists: {}\nactive_invites: {}\npending_requests: {}\naccepted_nodes: {}\nserver_bind_warning: {}\nlan_url_hint: {}\nauthority_boundary: {}\n",
        report.pairing_feature_enabled,
        report.core_fingerprint_exists,
        report.core_fingerprint.as_deref().unwrap_or("missing"),
        report.pairing_state_dir_exists,
        report.active_invites,
        report.pending_requests,
        report.accepted_nodes,
        report.server_bind_warning,
        report.lan_url_hint,
        report.authority_boundary
    )
}

pub fn render_child_doctor(report: &ChildDoctorReport) -> String {
    format!(
        "child doctor\nchild_identity_exists: {}\nnode_display_name: {}\nnode_public_key_present: {}\npaired: {}\napproved: {}\npaired_core_url: {}\ncore_fingerprint_stored: {}\nlast_pairing_status: {}\npaired_node_id: {}\nlast_heartbeat_status: {}\nactive_lease: {}\ndesk: {}\nrole: {}\njobs_enabled: {}\nexecution_enabled: {}\napproval_enabled: {}\ncanonical_write_enabled: {}\n",
        report.child_identity_exists,
        report.node_display_name.as_deref().unwrap_or("none"),
        report.node_public_key_present,
        report.paired,
        report.approved,
        report.paired_core_url.as_deref().unwrap_or("none"),
        report.core_fingerprint_stored.as_deref().unwrap_or("none"),
        report
            .last_pairing_status
            .as_ref()
            .map(|status| format!("{status:?}"))
            .unwrap_or_else(|| "none".to_string()),
        report.paired_node_id.as_deref().unwrap_or("none"),
        report.last_heartbeat_status,
        report.active_lease_id.as_deref().unwrap_or("none"),
        report.active_desk_id.as_deref().unwrap_or("none"),
        report.active_role_id.as_deref().unwrap_or("none"),
        report.jobs_enabled,
        report.execution_enabled,
        report.approval_enabled,
        report.canonical_write_enabled
    )
}

fn read_core_fingerprint(paths: &PairingPaths) -> Result<Option<String>> {
    if !paths.core_fingerprint.exists() {
        return Ok(None);
    }
    let value: Value = serde_json::from_str(&fs::read_to_string(&paths.core_fingerprint)?)?;
    Ok(value
        .get("core_fingerprint")
        .and_then(Value::as_str)
        .map(str::to_string))
}

fn lan_url_hint(invites: &[PairingInvite]) -> String {
    invites
        .iter()
        .rev()
        .find(|invite| !invite.revoked && !invite_expired(invite))
        .map(|invite| invite.core_url.clone())
        .unwrap_or_else(|| {
            "create an invite with --core http://<lan-ip>:8787 for tablet pairing".to_string()
        })
}

fn child_heartbeat_status(cfg: &Config, node_id: Option<&str>) -> Result<String> {
    let Some(node_id) = node_id else {
        return Ok("no paired node id".to_string());
    };
    let heartbeats_path = cfg.workspace_dir.join("state/cluster/heartbeats.jsonl");
    let heartbeats = read_jsonl::<cluster::ClusterHeartbeat>(heartbeats_path)?;
    let latest = heartbeats
        .into_iter()
        .filter(|heartbeat| heartbeat.node_id.as_str() == node_id)
        .filter_map(|heartbeat| {
            DateTime::parse_from_rfc3339(&heartbeat.timestamp)
                .ok()
                .map(|timestamp| timestamp.with_timezone(&Utc))
        })
        .max();
    match latest {
        Some(timestamp) if Utc::now() - timestamp < Duration::minutes(5) => {
            Ok("online".to_string())
        }
        Some(timestamp) => Ok(format!("stale since {}", timestamp.to_rfc3339())),
        None => Ok("no heartbeat recorded".to_string()),
    }
}

fn bounded_ttl(ttl: Duration, dev_auto_accept: bool) -> Result<Duration> {
    let ttl = if ttl <= Duration::zero() {
        Duration::minutes(DEFAULT_TTL_MINUTES)
    } else {
        ttl
    };
    let max = if dev_auto_accept {
        Duration::minutes(DEV_AUTO_ACCEPT_MAX_TTL_MINUTES)
    } else {
        Duration::minutes(MAX_TTL_MINUTES)
    };
    if ttl > max {
        return Err(anyhow!(
            "pairing invite ttl exceeds max {} minutes",
            max.num_minutes()
        ));
    }
    Ok(ttl)
}

fn find_valid_invite_index(invites: &[PairingInvite], invite_token: &str) -> Result<usize> {
    let hash = token::hash_token(invite_token);
    let index = invites
        .iter()
        .position(|invite| invite.invite_token_hash == hash)
        .ok_or_else(|| anyhow!("pairing invite token rejected"))?;
    let invite = &invites[index];
    if invite.revoked {
        return Err(anyhow!("pairing invite revoked"));
    }
    if invite.used && invite.one_time {
        return Err(anyhow!("pairing invite already used"));
    }
    if invite_expired(invite) {
        return Err(anyhow!("pairing invite expired"));
    }
    Ok(index)
}

fn invite_expired(invite: &PairingInvite) -> bool {
    chrono::DateTime::parse_from_rfc3339(&invite.expires_at)
        .map(|ts| ts.with_timezone(&Utc) < Utc::now())
        .unwrap_or(true)
}

fn invite_for_token(cfg: &Config, invite_token: &str) -> Result<PairingInvite> {
    let invites = list_invites(cfg)?;
    let index = find_valid_invite_index(&invites, invite_token)?;
    Ok(invites[index].clone())
}

fn pairing_status(cfg: &Config, request_id: &str) -> Result<PairingStatusResponse> {
    let request = list_requests(cfg)?
        .into_iter()
        .find(|request| request.request_id == request_id)
        .ok_or_else(|| anyhow!("pairing request '{}' not found", request_id))?;
    let accepted_node = if request.status == PairingRequestStatus::Approved {
        read_jsonl::<AcceptedPairedNode>(PairingPaths::new(cfg).accepted_nodes)?
            .into_iter()
            .find(|node| node.node_public_key == request.node_public_key)
    } else {
        None
    };
    let node_id = accepted_node.as_ref().map(|node| node.node_id.clone());
    let node_auth_token = if let Some(node) = accepted_node {
        if node.node_auth_token_hash.trim().is_empty() {
            None
        } else {
            node_auth_token_for_status(cfg, &request.request_id, &node.node_id)?
        }
    } else {
        None
    };
    Ok(PairingStatusResponse {
        request_id: request.request_id,
        status: request.status,
        node_id,
        node_auth_token,
        execution_enabled: false,
        canonical_write_enabled: false,
        approval_enabled: false,
    })
}

fn make_node_auth_token(request_id: &str, node_public_key: &str) -> String {
    format!(
        "node_auth_{}",
        token::hash_token(&format!("{request_id}:{node_public_key}:heartbeat-auth"))
    )
}

fn node_auth_token_for_status(
    cfg: &Config,
    request_id: &str,
    node_id: &str,
) -> Result<Option<String>> {
    let Some(request) = list_requests(cfg)?
        .into_iter()
        .find(|request| request.request_id == request_id)
    else {
        return Ok(None);
    };
    let candidate = make_node_auth_token(request_id, &request.node_public_key);
    let accepted = read_jsonl::<AcceptedPairedNode>(PairingPaths::new(cfg).accepted_nodes)?
        .into_iter()
        .find(|node| node.node_id == node_id);
    if accepted
        .as_ref()
        .is_some_and(|node| token::hash_token(&candidate) == node.node_auth_token_hash)
    {
        Ok(Some(candidate))
    } else {
        Ok(None)
    }
}

fn render_pairing_invite_page(invite: &PairingInvite, invite_token: &str) -> String {
    let command = format!(
        "quant-m child pair --core {} --invite {}",
        invite.core_url, invite_token
    );
    let link = local_link(&invite.core_url, invite_token);
    let qr_html = inline_pairing_qr_html(&link);
    format!(
        "<!doctype html><html><head><meta charset=\"utf-8\"><title>Quant-M Local Pairing Invite</title><style>body{{font-family:-apple-system,BlinkMacSystemFont,Segoe UI,sans-serif;max-width:720px;margin:32px auto;padding:0 16px;line-height:1.45}}pre{{white-space:pre-wrap;overflow-wrap:anywhere;background:#f5f5f5;padding:12px;border-radius:6px}}.qr svg{{width:280px;height:280px;max-width:100%;border:1px solid #ddd;padding:12px;background:white}}</style></head><body><h1>Quant-M Local Pairing Invite</h1><p><strong>Core:</strong> {}</p><p><strong>Core fingerprint:</strong> {}</p><p><strong>Requested node name:</strong> {}</p><p><strong>Requested desk:</strong> {}</p><p><strong>Requested role:</strong> {}</p><p><strong>Authority:</strong> observe only</p><p><strong>Execution:</strong> disabled</p><p><strong>Canonical write:</strong> disabled</p><p><strong>Approval:</strong> disabled</p><p><strong>Expires at:</strong> {}</p><h2>Scan QR</h2>{}<h2>Copy this Termux command</h2><pre>{}</pre><p><strong>Invite link:</strong> <a href=\"{}\">{}</a></p></body></html>",
        html_escape(&invite.core_url),
        html_escape(&invite.core_fingerprint),
        html_escape(invite.requested_node_name.as_deref().unwrap_or("none")),
        html_escape(
            &invite
                .desk_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string())
        ),
        html_escape(invite.requested_role.as_deref().unwrap_or("none")),
        html_escape(&invite.expires_at),
        qr_html,
        html_escape(&command),
        html_escape(&link),
        html_escape(&link)
    )
}

fn inline_pairing_qr_html(payload: &str) -> String {
    #[cfg(feature = "pairing-qr")]
    {
        match qrcode::QrCode::new(payload.as_bytes()) {
            Ok(code) => {
                let svg = code
                    .render::<qrcode::render::svg::Color>()
                    .min_dimensions(280, 280)
                    .build();
                format!("<div class=\"qr\" aria-label=\"Quant-M pairing QR\">{svg}</div>")
            }
            Err(err) => format!("<p>QR unavailable: {}</p>", html_escape(&err.to_string())),
        }
    }
    #[cfg(not(feature = "pairing-qr"))]
    {
        let _ = payload;
        "<p>QR unavailable; rebuild with --features pairing-qr.</p>".to_string()
    }
}

fn html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

struct ParsedHttpRequest {
    method: String,
    path: String,
    body: String,
}

impl ParsedHttpRequest {
    fn parse(raw: &str) -> Result<Self> {
        let (head, body) = raw.split_once("\r\n\r\n").unwrap_or((raw, ""));
        let request_line = head
            .lines()
            .next()
            .ok_or_else(|| anyhow!("empty HTTP request"))?;
        let mut parts = request_line.split_whitespace();
        let method = parts
            .next()
            .ok_or_else(|| anyhow!("missing HTTP method"))?
            .to_string();
        let path = parts
            .next()
            .ok_or_else(|| anyhow!("missing HTTP path"))?
            .split('?')
            .next()
            .unwrap_or("")
            .to_string();
        Ok(Self {
            method,
            path,
            body: body.to_string(),
        })
    }
}

fn mark_invite_used(cfg: &Config, invite_id: &str) -> Result<()> {
    let paths = PairingPaths::new(cfg);
    let mut invites = list_invites(cfg)?;
    if let Some(invite) = invites
        .iter_mut()
        .find(|invite| invite.invite_id == invite_id)
    {
        invite.used = true;
    }
    rewrite_jsonl(&paths.invites, &invites)
}

fn reject_forbidden_claims(capabilities: &[String]) -> Result<()> {
    for capability in capabilities {
        let normalized = capability.to_ascii_lowercase();
        if normalized.contains("execute")
            || normalized.contains("approval")
            || normalized.contains("approve")
            || normalized.contains("canonical")
            || normalized.contains("provider_credential")
            || normalized.contains("broker")
            || normalized.contains("exchange_key")
        {
            return Err(anyhow!(
                "pairing request includes forbidden capability claim"
            ));
        }
    }
    Ok(())
}

fn parse_claimed_capabilities(values: &[String]) -> Result<Vec<ClusterCapability>> {
    values
        .iter()
        .map(|value| value.parse::<ClusterCapability>())
        .collect()
}

fn store_child_pairing(cfg: &Config, pairing: &ChildCorePairing) -> Result<()> {
    let paths = PairingPaths::new(cfg);
    paths.ensure()?;
    fs::write(&paths.child_pairing, toml::to_string_pretty(pairing)?)?;
    fs::write(&paths.child_core, toml::to_string_pretty(pairing)?)?;
    append_event(
        cfg,
        "child_pairing_stored",
        None,
        Some(&pairing.request_id),
        pairing.node_id.as_deref(),
        "child stored core pairing metadata",
    )?;
    Ok(())
}

fn append_event(
    cfg: &Config,
    kind: &str,
    invite_id: Option<&str>,
    request_id: Option<&str>,
    node_id: Option<&str>,
    reason: &str,
) -> Result<()> {
    let event = PairingEvent {
        event_id: format!("pair_evt_{}", token::short_hash(&token::generate_token())),
        timestamp: now(),
        kind: kind.to_string(),
        invite_id: invite_id.map(str::to_string),
        request_id: request_id.map(str::to_string),
        node_id: node_id.map(str::to_string),
        reason: reason.to_string(),
        replay_safe: true,
    };
    append_json_line(&PairingPaths::new(cfg).events, &event)
}

fn local_link(core_url: &str, invite_token: &str) -> String {
    format!("{}/pair/i/{}", core_url.trim_end_matches('/'), invite_token)
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn rewrite_jsonl<T: Serialize>(path: &Path, values: &[T]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut out = String::new();
    for value in values {
        out.push_str(&serde_json::to_string(value)?);
        out.push('\n');
    }
    fs::write(path, out)?;
    Ok(())
}

fn read_jsonl<T>(path: PathBuf) -> Result<Vec<T>>
where
    T: for<'de> Deserialize<'de>,
{
    if !path.exists() {
        return Ok(vec![]);
    }
    fs::read_to_string(&path)?
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| {
            serde_json::from_str(line)
                .with_context(|| format!("failed to parse {}", path.display()))
        })
        .collect()
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().join("workspace");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.worker.inbox_path = cfg.workspace_dir.join("queue/inbox.ndjson");
        cfg.worker.outbox_path = cfg.workspace_dir.join("queue/outbox.ndjson");
        cfg.worker.inflight_path = cfg.workspace_dir.join("queue/inflight.json");
        cfg.worker.state_path = cfg.workspace_dir.join("state/worker_state.json");
        cfg.worker.dead_letter_path = cfg.workspace_dir.join("queue/dead-letter.ndjson");
        (tmp, cfg)
    }

    fn invite(cfg: &Config) -> PairingInviteView {
        create_invite(
            cfg,
            PairingInviteOptions {
                name: Some("tablet-01"),
                desk: Some("crypto"),
                role: Some("stablecoin_peg_watcher"),
                ttl: Duration::minutes(10),
                core_url: "http://127.0.0.1:8787",
                dev_auto_accept: false,
            },
        )
        .expect("invite")
    }

    fn child_input<'a>(
        view: &'a PairingInviteView,
        capabilities: &'a [String],
        authority: PairingAuthority,
    ) -> ChildPairRequestInput<'a> {
        ChildPairRequestInput {
            core_url: &view.invite.core_url,
            invite_token: &view.invite_token,
            node_name: Some("tablet-01"),
            surface: "termux_worker",
            capabilities,
            requested_role: Some("stablecoin_peg_watcher"),
            requested_authority: authority,
        }
    }

    fn server_payload(view: &PairingInviteView) -> PairingServerRequestPayload {
        PairingServerRequestPayload {
            invite_token: view.invite_token.clone(),
            node_display_name: "tablet-01".to_string(),
            node_public_key: format!("child-pub-{}", token::short_hash("server-test")),
            surface: "termux_worker".to_string(),
            claimed_capabilities: vec![
                "echo".to_string(),
                "sleep".to_string(),
                "compute_scalar".to_string(),
            ],
            requested_role: Some("stablecoin_peg_watcher".to_string()),
            requested_authority: PairingAuthority::Observe,
            execution_enabled: false,
            canonical_write_enabled: false,
            approval_enabled: false,
        }
    }

    fn post_pair_request(payload: &PairingServerRequestPayload) -> String {
        let body = serde_json::to_string(payload).expect("payload json");
        format!(
            "POST /pair/request HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    }

    fn post_cluster_heartbeat(node_id: &str, node_auth_token: &str) -> String {
        let body = serde_json::to_string(&serde_json::json!({
            "node_id": node_id,
            "node_auth_token": node_auth_token,
            "surface": "termux_worker",
            "claimed_capabilities": ["echo", "sleep", "heartbeat", "compute_scalar"],
            "execution_enabled": false,
            "canonical_write_enabled": false,
            "approval_enabled": false,
            "device_telemetry": null
        }))
        .expect("heartbeat json");
        format!(
            "POST /cluster/heartbeat HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
            body.len(),
            body
        )
    }

    #[test]
    fn pairing_invite_generates_short_lived_token() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        assert!(view.invite_token.len() >= 32);
        assert_eq!(view.invite.max_authority, PairingAuthority::Observe);
        assert!(!view.execution_enabled);
    }

    #[test]
    fn pairing_invite_stores_token_hash_not_plaintext() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let raw = fs::read_to_string(cfg.workspace_dir.join("state/pairing/invites.jsonl"))
            .expect("invites");
        assert!(raw.contains(&view.invite.invite_token_hash));
        assert!(!raw.contains(&view.invite_token));
    }

    #[test]
    fn pairing_invite_qr_payload_does_not_include_long_lived_secret() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        assert!(view.qr_payload.contains("/pair/i/"));
        assert!(!view.qr_payload.contains("private"));
        assert!(!view.qr_payload.contains("broker"));
        assert!(!view.qr_payload.contains("execute"));
    }

    #[test]
    fn pairing_invite_local_link_contains_invite_token() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        assert!(view.local_link.contains(&view.invite_token));
    }

    #[cfg(feature = "pairing-qr")]
    #[test]
    fn terminal_qr_render_is_plain_unicode_without_ansi() {
        let qr =
            render_qr_to_terminal("http://10.0.0.184:8787/pair/i/testtoken12345678901234567890")
                .expect("terminal qr");
        assert!(qr.contains('█') || qr.contains('▄') || qr.contains('▀'));
        assert!(!qr.contains("\u{1b}["));
        assert!(!qr.contains("terminal QR rendered"));
    }

    #[test]
    fn pairing_request_defaults_to_pending_and_records_key() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &[
                    "echo".to_string(),
                    "sleep".to_string(),
                    "compute_scalar".to_string(),
                ],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        assert_eq!(req.status, PairingRequestStatus::Pending);
        assert!(req.node_public_key.starts_with("child-pub-"));
        assert!(req.compute_claims_present);
    }

    #[test]
    fn pairing_request_rejects_authority_escalation() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let err = submit_child_pair_request(
            &cfg,
            child_input(&view, &["echo".to_string()], PairingAuthority::Analyze),
        )
        .expect_err("authority rejected");
        assert!(err.to_string().contains("exceeds"));
    }

    #[test]
    fn pairing_request_rejects_execution_claim() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let err = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &["execute_trade".to_string()],
                PairingAuthority::Observe,
            ),
        )
        .expect_err("execution rejected");
        assert!(err.to_string().contains("forbidden"));
    }

    #[test]
    fn pairing_approval_creates_node_id_and_observe_defaults() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &[
                    "echo".to_string(),
                    "sleep".to_string(),
                    "compute_scalar".to_string(),
                ],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        let accepted = approve_request(&cfg, &req.request_id, "operator").expect("approve");
        assert_eq!(accepted.node_id, "node:tablet-01");
        assert_eq!(accepted.authority_level, PairingAuthority::Observe);
        assert!(!accepted.execution_enabled);
        assert!(!accepted.approval_enabled);
        assert!(!accepted.canonical_write_enabled);
        let pairing = child_pairing(&cfg)
            .expect("child pairing")
            .expect("child pairing present");
        assert_eq!(pairing.status, PairingRequestStatus::Approved);
        assert_eq!(pairing.node_id.as_deref(), Some("node:tablet-01"));
    }

    #[test]
    fn pair_doctor_reports_core_pairing_setup() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &["echo".to_string(), "sleep".to_string()],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        let report = pair_doctor(&cfg, "0.0.0.0:8787").expect("doctor");
        assert!(report.pairing_feature_enabled);
        assert!(report.core_fingerprint_exists);
        assert_eq!(
            report.core_fingerprint.as_deref(),
            Some(view.invite.core_fingerprint.as_str())
        );
        assert!(report.pairing_state_dir_exists);
        assert_eq!(report.active_invites, 1);
        assert_eq!(report.pending_requests, 1);
        assert_eq!(report.accepted_nodes, 0);
        assert!(report.server_bind_warning.contains("trusted LAN"));
        assert_eq!(report.lan_url_hint, "http://127.0.0.1:8787");
        assert!(report.authority_boundary.contains("does not grant leases"));
        assert_eq!(req.status, PairingRequestStatus::Pending);
    }

    #[test]
    fn child_doctor_reports_pairing_without_authority() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &["echo".to_string(), "sleep".to_string()],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        approve_request(&cfg, &req.request_id, "operator").expect("approve");
        let report = child_doctor(&cfg).expect("child doctor");
        assert!(report.child_identity_exists);
        assert_eq!(report.node_display_name.as_deref(), Some("tablet-01"));
        assert!(report.node_public_key_present);
        assert_eq!(
            report.paired_core_url.as_deref(),
            Some("http://127.0.0.1:8787")
        );
        assert_eq!(
            report.last_pairing_status,
            Some(PairingRequestStatus::Approved)
        );
        assert_eq!(report.paired_node_id.as_deref(), Some("node:tablet-01"));
        assert_eq!(report.last_heartbeat_status, "no heartbeat recorded");
        assert!(!report.execution_enabled);
        assert!(!report.approval_enabled);
        assert!(!report.canonical_write_enabled);
    }

    #[test]
    fn tablet_pairing_e2e_final_state_is_observe_only_without_lease() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &[
                    "echo".to_string(),
                    "sleep".to_string(),
                    "compute_scalar".to_string(),
                ],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        let accepted = approve_request(&cfg, &req.request_id, "operator").expect("approve");
        cluster::heartbeat(&cfg, &accepted.node_id.parse().expect("node id")).expect("heartbeat");
        let accepted_nodes = list_accepted_nodes(&cfg).expect("accepted nodes");
        assert_eq!(accepted_nodes.len(), 1);
        assert_eq!(accepted_nodes[0].initial_lease_expires_at, None);
        assert_eq!(accepted_nodes[0].authority_level, PairingAuthority::Observe);
        assert!(!accepted_nodes[0].execution_enabled);
        let child = child_doctor(&cfg).expect("child doctor");
        assert_eq!(child.last_heartbeat_status, "online");
        let report = cluster::report(&cfg).expect("cluster report");
        assert!(report.active_leases.is_empty());
    }

    #[test]
    fn used_pairing_invite_rejected() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &["echo".to_string(), "sleep".to_string()],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        approve_request(&cfg, &req.request_id, "operator").expect("approve");
        let err = submit_child_pair_request(
            &cfg,
            ChildPairRequestInput {
                node_name: Some("tablet-02"),
                ..child_input(
                    &view,
                    &["echo".to_string(), "sleep".to_string()],
                    PairingAuthority::Observe,
                )
            },
        )
        .expect_err("used rejected");
        assert!(err.to_string().contains("already used"));
    }

    #[test]
    fn revoked_pairing_invite_rejected() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        revoke_invite(&cfg, &view.invite.invite_id).expect("revoke");
        let err = submit_child_pair_request(
            &cfg,
            child_input(&view, &["echo".to_string()], PairingAuthority::Observe),
        )
        .expect_err("revoked rejected");
        assert!(err.to_string().contains("revoked"));
    }

    #[test]
    fn dev_auto_accept_observe_only() {
        let (_tmp, cfg) = test_config();
        let view = create_invite(
            &cfg,
            PairingInviteOptions {
                name: Some("tablet-01"),
                desk: Some("crypto"),
                role: Some("stablecoin_peg_watcher"),
                ttl: Duration::minutes(5),
                core_url: "http://127.0.0.1:8787",
                dev_auto_accept: true,
            },
        )
        .expect("invite");
        let req = submit_child_pair_request(
            &cfg,
            child_input(
                &view,
                &["echo".to_string(), "sleep".to_string()],
                PairingAuthority::Observe,
            ),
        )
        .expect("request");
        let requests = list_requests(&cfg).expect("requests");
        assert_eq!(req.status, PairingRequestStatus::Pending);
        assert_eq!(requests[0].status, PairingRequestStatus::Approved);
    }

    #[test]
    fn pairing_event_log_written() {
        let (_tmp, cfg) = test_config();
        let _ = invite(&cfg);
        let events = list_events(&cfg).expect("events");
        assert!(
            events
                .iter()
                .any(|event| event.kind == "pairing_invite_created")
        );
        assert!(events.iter().all(|event| event.replay_safe));
    }

    #[test]
    fn pair_server_request_defaults_pending() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"status\": \"pending\""));
        let requests = list_requests(&cfg).expect("requests");
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].status, PairingRequestStatus::Pending);
    }

    #[test]
    fn pair_server_cluster_heartbeat_marks_node_online_without_authority() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 200);
        let request = list_requests(&cfg).expect("requests").remove(0);
        let accepted = approve_request(&cfg, &request.request_id, "operator").expect("approve");
        let node_auth_token = pairing_status(&cfg, &request.request_id)
            .expect("pairing status")
            .node_auth_token
            .expect("node auth token");
        let response = handle_pairing_http_request(
            &cfg,
            &post_cluster_heartbeat(&accepted.node_id.to_string(), &node_auth_token),
            None,
        );
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("\"paired\": true"));
        assert!(response.body.contains("\"approved\": true"));
        assert!(response.body.contains("\"execution_enabled\": false"));
        let node_id = accepted.node_id.parse().expect("node id");
        let status = cluster::node_status(&cfg, &node_id).expect("status");
        assert!(status.online);
        assert!(!status.stale);
        assert!(!status.execution_enabled);
        assert!(!status.approval_enabled);
        assert!(!status.canonical_write_enabled);
    }

    #[test]
    fn pair_server_rejects_spoofed_heartbeat_without_node_auth_token() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 200);
        let request = list_requests(&cfg).expect("requests").remove(0);
        let accepted = approve_request(&cfg, &request.request_id, "operator").expect("approve");
        let response = handle_pairing_http_request(
            &cfg,
            &post_cluster_heartbeat(&accepted.node_id.to_string(), "wrong-token"),
            None,
        );
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("auth token rejected"));
        let node_id = accepted.node_id.parse().expect("node id");
        let status = cluster::node_status(&cfg, &node_id).expect("status");
        assert!(!status.online);
    }

    #[test]
    fn pair_server_rejects_authority_escalation() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let mut payload = server_payload(&view);
        payload.requested_authority = PairingAuthority::Analyze;
        let response = handle_pairing_http_request(&cfg, &post_pair_request(&payload), None);
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("exceeds"));
    }

    #[test]
    fn pair_server_rejects_revoked_invite() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        revoke_invite(&cfg, &view.invite.invite_id).expect("revoke");
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("revoked"));
    }

    #[test]
    fn pair_server_rejects_expired_invite() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let paths = PairingPaths::new(&cfg);
        let mut invites = list_invites(&cfg).expect("invites");
        invites[0].expires_at = (Utc::now() - Duration::minutes(1)).to_rfc3339();
        rewrite_jsonl(&paths.invites, &invites).expect("rewrite invites");
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("expired"));
    }

    #[test]
    fn pair_server_rejects_used_invite() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 200);
        let request_id = list_requests(&cfg).expect("requests")[0].request_id.clone();
        approve_request(&cfg, &request_id, "operator").expect("approve");
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("already used"));
    }

    #[test]
    fn pair_server_does_not_assign_lease_or_enable_execution() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let response =
            handle_pairing_http_request(&cfg, &post_pair_request(&server_payload(&view)), None);
        assert_eq!(response.status_code, 200);
        let request_id = list_requests(&cfg).expect("requests")[0].request_id.clone();
        let accepted = approve_request(&cfg, &request_id, "operator").expect("approve");
        assert_eq!(accepted.initial_lease_expires_at, None);
        assert!(!accepted.execution_enabled);
        let status_response = handle_pairing_http_request(
            &cfg,
            &format!("GET /pair/status/{request_id} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n"),
            None,
        );
        assert_eq!(status_response.status_code, 200);
        assert!(
            status_response
                .body
                .contains("\"execution_enabled\": false")
        );
    }

    #[test]
    fn pair_server_invite_page_shows_boundary() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let response = handle_pairing_http_request(
            &cfg,
            &format!(
                "GET /pair/i/{} HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n",
                view.invite_token
            ),
            None,
        );
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("Quant-M Local Pairing Invite"));
        assert!(response.body.contains("observe only"));
        assert!(response.body.contains("Execution:</strong> disabled"));
    }

    #[test]
    fn pair_server_logs_event() {
        let (_tmp, cfg) = test_config();
        let message = pairing_bind_warning("0.0.0.0:8787");
        append_event(&cfg, "pairing_server_started", None, None, None, &message).expect("event");
        let events = list_events(&cfg).expect("events");
        assert!(
            events
                .iter()
                .any(|event| event.kind == "pairing_server_started")
        );
        assert!(message.contains("trusted LAN"));
    }

    #[test]
    fn child_pair_scan_rejects_non_quantm_qr() {
        let err = parse_pairing_scan_payload("mailto:not-a-pairing-payload")
            .expect_err("non pairing payload rejected");
        assert!(err.to_string().contains("not a Quant-M"));
    }

    #[test]
    fn child_pair_scan_rejects_malformed_payload() {
        let err = parse_pairing_scan_payload("http://127.0.0.1:8787/pair/i/short")
            .expect_err("malformed rejected");
        assert!(err.to_string().contains("malformed invite token"));
    }

    #[test]
    fn child_pair_scan_rejects_secret_bearing_payload() {
        let err = parse_pairing_scan_payload(
            "http://127.0.0.1:8787/pair/i/abcdefghijklmnopqrstuvwxyz123456?api_key=secret",
        )
        .expect_err("secret rejected");
        assert!(err.to_string().contains("forbidden"));
    }

    #[test]
    fn child_pair_scan_rejects_public_url_by_default() {
        let err = parse_pairing_scan_payload(
            "http://203.0.113.10:8787/pair/i/abcdefghijklmnopqrstuvwxyz123456",
        )
        .expect_err("public rejected");
        assert!(err.to_string().contains("public"));
    }

    #[test]
    fn child_pair_scan_parses_valid_local_pairing_url() {
        let payload = parse_pairing_scan_payload(
            "http://192.168.1.42:8787/pair/i/abcdefghijklmnopqrstuvwxyz123456",
        )
        .expect("payload");
        assert_eq!(payload.core_url, "http://192.168.1.42:8787");
        assert_eq!(payload.invite_token, "abcdefghijklmnopqrstuvwxyz123456");
    }

    #[test]
    fn child_pair_scan_parses_quantm_pair_uri() {
        let payload = parse_pairing_scan_payload(
            "quantm://pair?v=1&core=http%3A%2F%2F127.0.0.1%3A8787&invite=abcdefghijklmnopqrstuvwxyz123456",
        )
        .expect("payload");
        assert_eq!(payload.core_url, "http://127.0.0.1:8787");
        assert_eq!(payload.invite_token, "abcdefghijklmnopqrstuvwxyz123456");
    }

    #[test]
    fn child_pair_scan_uses_same_request_path_as_child_pair() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let payload = parse_pairing_scan_payload(&format!(
            "{}/pair/i/{}",
            view.invite.core_url, view.invite_token
        ))
        .expect("payload");
        let request = submit_scanned_pairing_payload(&cfg, &payload).expect("request");
        assert_eq!(request.status, PairingRequestStatus::Pending);
        assert_eq!(request.requested_authority, PairingAuthority::Observe);
        assert_eq!(
            request.requested_role.as_deref(),
            Some("stablecoin_peg_watcher")
        );
        let stored = list_requests(&cfg).expect("requests");
        assert_eq!(stored.len(), 1);
        assert_eq!(stored[0].request_id, request.request_id);
    }

    #[test]
    fn child_pair_scan_does_not_assign_lease_or_enable_execution() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let payload = PairingScanPayload {
            core_url: view.invite.core_url.clone(),
            invite_token: view.invite_token.clone(),
        };
        let request = submit_scanned_pairing_payload(&cfg, &payload).expect("request");
        let accepted = approve_request(&cfg, &request.request_id, "operator").expect("approve");
        assert_eq!(accepted.initial_lease_expires_at, None);
        assert_eq!(accepted.authority_level, PairingAuthority::Observe);
        assert!(!accepted.execution_enabled);
        assert!(!accepted.approval_enabled);
        assert!(!accepted.canonical_write_enabled);
    }

    #[cfg(feature = "pairing-scan-image")]
    #[test]
    fn child_pair_scan_decodes_valid_pairing_qr() {
        let (_tmp, cfg) = test_config();
        let view = invite(&cfg);
        let image_path = cfg.workspace_dir.join("pairing-qr.png");
        let code = qrcode::QrCode::new(view.qr_payload.as_bytes()).expect("qr");
        let image = code.render::<image::Luma<u8>>().build();
        fs::create_dir_all(image_path.parent().expect("image parent")).expect("image dir");
        image.save(&image_path).expect("save qr");
        let request = pair_scan_image(&cfg, &image_path).expect("scan");
        assert_eq!(request.status, PairingRequestStatus::Pending);
        assert_eq!(request.requested_authority, PairingAuthority::Observe);
    }
}
