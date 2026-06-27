use anyhow::{Context, Result, anyhow};
use chrono::Duration;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::net::{SocketAddr, TcpStream};
use std::path::PathBuf;
use std::thread;
use std::time::Duration as StdDuration;

use crate::cluster::{self, ClusterLease, ClusterNodeId, ClusterNodeStatus, ClusterRoleId};
use crate::config::Config;
use crate::pairing::{
    self, AcceptedPairedNode, PairingAuthority, PairingInviteView, PairingRequest,
    PairingRequestStatus,
};

#[derive(Debug, Clone)]
pub struct DeviceAddOptions {
    pub name: String,
    pub desk: String,
    pub role: String,
    pub ttl: Duration,
    pub core_url: String,
    pub qr: bool,
    pub png: Option<PathBuf>,
    pub watch: bool,
    pub auto_approve_observe: bool,
    pub grant_observe_lease: bool,
    pub lease_ttl: Duration,
    pub no_server: bool,
    pub serve: bool,
    pub bind: String,
    pub watch_timeout_seconds: u64,
    pub watch_poll_seconds: u64,
    pub approval_mode: DeviceApprovalMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DeviceApprovalMode {
    NoPrompt,
    Prompt,
    #[cfg(test)]
    Input(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceAddResult {
    pub device_name: String,
    pub desk: String,
    pub role: String,
    pub invite: PairingInviteView,
    pub qr_text: Option<String>,
    pub request: Option<PairingRequest>,
    pub rejected: Option<PairingRequest>,
    pub accepted: Option<AcceptedPairedNode>,
    pub lease: Option<ClusterLease>,
    pub final_state: DeviceFinalState,
    pub server_started: bool,
    pub server_reachable: bool,
    pub server_warning: String,
    pub execution_enabled: bool,
    pub approval_enabled: bool,
    pub canonical_write_enabled: bool,
    pub provider_calls_enabled: bool,
    pub proposal_creation_enabled: bool,
    pub compute_validation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeviceFinalState {
    pub node: Option<String>,
    pub paired: bool,
    pub approved: bool,
    pub online: bool,
    pub stale: bool,
    pub lease: Option<String>,
    pub authority: String,
    pub desk: Option<String>,
    pub role: Option<String>,
    pub execution_enabled: bool,
    pub approval_enabled: bool,
    pub canonical_write_enabled: bool,
    pub provider_calls_enabled: bool,
    pub compute_validation: String,
    pub proposal_creation_enabled: bool,
}

pub fn add_device(cfg: &Config, options: DeviceAddOptions) -> Result<DeviceAddResult> {
    validate_options(&options)?;
    let doctor = pairing::pair_doctor(cfg, &options.bind)?;
    let server_reachable_before = pairing_server_reachable(&options.bind);
    let server_started = if options.serve {
        let cfg = cfg.clone();
        let bind = options.bind.clone();
        thread::spawn(move || {
            if let Err(err) = pairing::serve_pairing_server(&cfg, &bind) {
                eprintln!("pairing server stopped: {err}");
            }
        });
        thread::sleep(StdDuration::from_millis(100));
        true
    } else {
        false
    };
    let server_reachable = server_reachable_before || pairing_server_reachable(&options.bind);
    if !options.no_server && !server_reachable && !server_started {
        // The wizard can still create an invite; render_add_device shows the operator how to start the server.
    }
    let invite = pairing::create_invite(
        cfg,
        pairing::PairingInviteOptions {
            name: Some(&options.name),
            desk: Some(&options.desk),
            role: Some(&options.role),
            ttl: options.ttl,
            core_url: &options.core_url,
            dev_auto_accept: options.auto_approve_observe,
        },
    )?;
    if let Some(path) = options.png.as_deref() {
        pairing::save_qr_png(&invite.qr_payload, path)?;
    }
    let qr_text = if options.qr {
        Some(pairing::render_qr_to_terminal(&invite.qr_payload)?)
    } else {
        None
    };
    let request = if options.watch {
        let request = watch_pending_request_for_invite(
            cfg,
            &invite.invite.invite_id,
            options.watch_timeout_seconds,
            options.watch_poll_seconds,
        )?;
        if request.is_some() {
            request
        } else {
            pending_request_for_name(cfg, &options.name)?
        }
    } else {
        None
    };
    let mut rejected = None;
    let accepted = if let Some(request) = request.as_ref() {
        if request.requested_authority != PairingAuthority::Observe {
            return Err(anyhow!("device add only approves observe authority"));
        }
        if options.auto_approve_observe {
            Some(pairing::approve_request(
                cfg,
                &request.request_id,
                "device_add_auto_approve_observe",
            )?)
        } else if approval_allows(&options.approval_mode, request)? {
            Some(pairing::approve_request(
                cfg,
                &request.request_id,
                "device_add_interactive",
            )?)
        } else {
            rejected = Some(pairing::reject_request(
                cfg,
                &request.request_id,
                "device add interactive approval declined",
            )?);
            None
        }
    } else {
        None
    };
    let accepted = accepted
        .or_else(|| {
            accepted_node_for_invite(cfg, &invite.invite.invite_id)
                .ok()
                .flatten()
        })
        .or_else(|| accepted_node_for_name(cfg, &options.name).ok().flatten());
    let lease = if options.grant_observe_lease {
        let accepted = accepted
            .as_ref()
            .ok_or_else(|| anyhow!("grant-observe-lease requires approved paired node"))?;
        let node_id: ClusterNodeId = accepted.node_id.parse()?;
        let role_id: ClusterRoleId = options.role.parse()?;
        Some(cluster::grant_observe_lease(
            cfg,
            &node_id,
            &role_id,
            options.lease_ttl,
            "device_add",
        )?)
    } else {
        None
    };
    let final_state = final_state(cfg, accepted.as_ref(), lease.as_ref())?;
    Ok(DeviceAddResult {
        device_name: options.name,
        desk: options.desk,
        role: options.role,
        invite,
        qr_text,
        request,
        rejected,
        accepted,
        lease,
        final_state,
        server_started,
        server_reachable,
        server_warning: doctor.server_bind_warning,
        execution_enabled: false,
        approval_enabled: false,
        canonical_write_enabled: false,
        provider_calls_enabled: false,
        proposal_creation_enabled: false,
        compute_validation: "none".to_string(),
    })
}

pub fn render_add_device(result: &DeviceAddResult) -> String {
    let mut out = String::new();
    out.push_str("Quant-M Add Device\n");
    out.push_str(&format!("Device name: {}\n", result.device_name));
    out.push_str(&format!("Desk: {}\n", result.desk));
    out.push_str(&format!("Role: {}\n", result.role));
    out.push_str("Pairing authority: observe\n");
    out.push_str("Auto approve: ");
    out.push_str(if result.accepted.is_some() {
        "yes\n"
    } else {
        "no\n"
    });
    out.push_str("Auto lease: ");
    out.push_str(if result.lease.is_some() {
        "yes\n"
    } else {
        "no\n"
    });
    out.push_str("Execution: disabled\nApproval: disabled\nCanonical write: disabled\n");
    if !result.server_reachable && !result.server_started {
        out.push_str("Pairing server is not running.\n");
        out.push_str("Start it in another terminal:\n");
        out.push_str("quant-m pair serve --bind 0.0.0.0:8787\n");
        out.push_str("Or rerun with:\n--serve --bind 0.0.0.0:8787\n");
    } else if result.server_started {
        out.push_str("Pairing server: started by device add\n");
    } else {
        out.push_str("Pairing server: reachable\n");
    }
    if !result.server_warning.is_empty() {
        out.push_str(&format!("Warning: {}\n", result.server_warning));
    }
    out.push_str(&format!("Local link:\n{}\n", result.invite.local_link));
    out.push_str(&format!(
        "Child command:\n{}\n",
        result
            .invite
            .child_command
            .replace("quant-m child", "quant-m-child")
    ));
    out.push_str("Child heartbeat:\nquant-m-child heartbeat\n");
    if let Some(qr) = result.qr_text.as_deref() {
        out.push_str("Scan QR:\n");
        out.push_str(qr);
        out.push('\n');
    }
    if let Some(request) = result.request.as_ref() {
        out.push_str("Pairing request received.\n");
        out.push_str(&format!(
            "Node display name: {}\n",
            request.node_display_name
        ));
        out.push_str(&format!("Surface: {}\n", request.surface));
        out.push_str("OS: unknown until heartbeat\n");
        out.push_str("Arch: unknown until heartbeat\n");
        out.push_str("Storage available: unknown until heartbeat\n");
        out.push_str("Battery: unknown until heartbeat\n");
        out.push_str(&format!(
            "Claimed capabilities: {}\n",
            request.claimed_capabilities.join(",")
        ));
        out.push_str(if request.compute_claims_present {
            "Compute claims: unvalidated\n"
        } else {
            "Compute claims: none\n"
        });
        out.push_str(&format!(
            "Requested authority: {}\nExecution requested: false\nApproval requested: false\nCanonical write requested: false\n",
            request.requested_authority
        ));
        if result.accepted.is_some() {
            out.push_str("Approved.\n");
            out.push_str("Next on child:\nquant-m-child heartbeat\n");
        } else if result.rejected.is_some() {
            out.push_str("Rejected.\nNo node created.\nExecution remains disabled.\n");
        }
    } else {
        out.push_str("Waiting for pairing request: no pending request observed yet\n");
    }
    out.push_str("\nFinal device state\n");
    out.push_str(&format!(
        "Node: {}\nPaired: {}\nApproved: {}\nOnline: {}\nStale: {}\nLease: {}\nAuthority: {}\nDesk: {}\nRole: {}\nExecution: disabled\nApproval: disabled\nCanonical write: disabled\nProvider calls: disabled\nCompute validation: {}\nProposal creation: disabled\n",
        result.final_state.node.as_deref().unwrap_or("none"),
        result.final_state.paired,
        result.final_state.approved,
        result.final_state.online,
        result.final_state.stale,
        result.final_state.lease.as_deref().unwrap_or("none"),
        result.final_state.authority,
        result.final_state.desk.as_deref().unwrap_or("none"),
        result.final_state.role.as_deref().unwrap_or("none"),
        result.final_state.compute_validation,
    ));
    out
}

fn validate_options(options: &DeviceAddOptions) -> Result<()> {
    if options.name.trim().is_empty() {
        return Err(anyhow!("device name is required"));
    }
    if options.desk.trim().is_empty() {
        return Err(anyhow!("desk is required"));
    }
    if options.role.trim().is_empty() {
        return Err(anyhow!("role is required"));
    }
    if options.ttl <= Duration::zero() {
        return Err(anyhow!("invite ttl must be positive"));
    }
    if options.auto_approve_observe && options.ttl > Duration::minutes(5) {
        return Err(anyhow!("auto-approve observe ttl cannot exceed 5m"));
    }
    if options.grant_observe_lease && options.lease_ttl <= Duration::zero() {
        return Err(anyhow!("lease ttl must be positive"));
    }
    Ok(())
}

fn watch_pending_request_for_invite(
    cfg: &Config,
    invite_id: &str,
    timeout_seconds: u64,
    poll_seconds: u64,
) -> Result<Option<PairingRequest>> {
    let poll_seconds = poll_seconds.max(1);
    let mut waited = 0;
    loop {
        if let Some(request) = pending_request_for_invite(cfg, invite_id)? {
            return Ok(Some(request));
        }
        if waited >= timeout_seconds {
            return Ok(None);
        }
        let step = poll_seconds.min(timeout_seconds - waited);
        thread::sleep(StdDuration::from_secs(step));
        waited += step;
    }
}

fn pending_request_for_invite(cfg: &Config, invite_id: &str) -> Result<Option<PairingRequest>> {
    Ok(pairing::list_requests(cfg)?.into_iter().find(|request| {
        request.invite_id == invite_id && request.status == PairingRequestStatus::Pending
    }))
}

fn pending_request_for_name(cfg: &Config, name: &str) -> Result<Option<PairingRequest>> {
    Ok(pairing::list_requests(cfg)?.into_iter().find(|request| {
        request.node_display_name == name && request.status == PairingRequestStatus::Pending
    }))
}

fn accepted_node_for_invite(cfg: &Config, invite_id: &str) -> Result<Option<AcceptedPairedNode>> {
    let approved_request = pairing::list_requests(cfg)?.into_iter().find(|request| {
        request.invite_id == invite_id && request.status == PairingRequestStatus::Approved
    });
    let Some(request) = approved_request else {
        return Ok(None);
    };
    Ok(pairing::list_accepted_nodes(cfg)?
        .into_iter()
        .find(|node| node.node_display_name == request.node_display_name))
}

fn accepted_node_for_name(cfg: &Config, name: &str) -> Result<Option<AcceptedPairedNode>> {
    Ok(pairing::list_accepted_nodes(cfg)?
        .into_iter()
        .find(|node| node.node_display_name == name))
}

fn approval_allows(mode: &DeviceApprovalMode, request: &PairingRequest) -> Result<bool> {
    match mode {
        DeviceApprovalMode::NoPrompt => Ok(false),
        #[cfg(test)]
        DeviceApprovalMode::Input(input) => {
            Ok(input.trim().eq_ignore_ascii_case("y") || input.trim().eq_ignore_ascii_case("yes"))
        }
        DeviceApprovalMode::Prompt => {
            println!("Pairing request received");
            println!("Request: {}", request.request_id);
            println!("Device name: {}", request.node_display_name);
            println!("Surface: {}", request.surface);
            println!(
                "Claimed capabilities: {}",
                request.claimed_capabilities.join(", ")
            );
            println!(
                "Compute claims: {}",
                if request.compute_claims_present {
                    "unvalidated"
                } else {
                    "none"
                }
            );
            println!("Requested authority: {}", request.requested_authority);
            println!("Execution requested: false");
            println!("Approval requested: false");
            println!("Canonical write requested: false");
            print!("Approve observe-only child? [y/N] ");
            io::stdout().flush()?;
            let mut line = String::new();
            io::stdin().read_line(&mut line)?;
            Ok(line.trim().eq_ignore_ascii_case("y") || line.trim().eq_ignore_ascii_case("yes"))
        }
    }
}

fn pairing_server_reachable(bind: &str) -> bool {
    let Ok(addr) = connect_addr_for_bind(bind) else {
        return false;
    };
    TcpStream::connect_timeout(&addr, StdDuration::from_millis(200)).is_ok()
}

fn connect_addr_for_bind(bind: &str) -> Result<SocketAddr> {
    let mut parts = bind.rsplitn(2, ':');
    let port = parts
        .next()
        .ok_or_else(|| anyhow!("missing pairing server port"))?;
    let host = parts.next().unwrap_or("127.0.0.1");
    let host = if host == "0.0.0.0" || host == "::" {
        "127.0.0.1"
    } else {
        host
    };
    format!("{host}:{port}")
        .parse()
        .with_context(|| format!("invalid pairing server bind '{bind}'"))
}

fn final_state(
    cfg: &Config,
    accepted: Option<&AcceptedPairedNode>,
    lease: Option<&ClusterLease>,
) -> Result<DeviceFinalState> {
    if let Some(accepted) = accepted {
        let node_id: ClusterNodeId = accepted.node_id.parse()?;
        let status = cluster::node_status(cfg, &node_id).unwrap_or_else(|_| ClusterNodeStatus {
            node_id,
            paired: true,
            approved: true,
            online: false,
            stale: true,
            last_heartbeat_at: None,
            active_lease_id: lease.map(|lease| lease.lease_id.clone()),
            active_role_id: lease.map(|lease| lease.role_id.clone()),
            active_desk_id: lease.map(|lease| lease.desk_id.clone()),
            authority: cluster::LeaseAuthority::Observe,
            execution_enabled: false,
            approval_enabled: false,
            canonical_write_enabled: false,
            jobs_enabled: false,
            device_telemetry: None,
            telemetry_warnings: vec![],
        });
        Ok(DeviceFinalState {
            node: Some(status.node_id.to_string()),
            paired: status.paired,
            approved: status.approved,
            online: status.online,
            stale: status.stale,
            lease: status.active_lease_id,
            authority: status.authority.to_string(),
            desk: status.active_desk_id.map(|desk| desk.to_string()),
            role: status.active_role_id.map(|role| role.to_string()),
            execution_enabled: false,
            approval_enabled: false,
            canonical_write_enabled: false,
            provider_calls_enabled: false,
            compute_validation: "none".to_string(),
            proposal_creation_enabled: false,
        })
    } else {
        Ok(DeviceFinalState {
            node: None,
            paired: false,
            approved: false,
            online: false,
            stale: true,
            lease: None,
            authority: "observe".to_string(),
            desk: None,
            role: None,
            execution_enabled: false,
            approval_enabled: false,
            canonical_write_enabled: false,
            provider_calls_enabled: false,
            compute_validation: "none".to_string(),
            proposal_creation_enabled: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::pairing::{ChildPairRequestInput, PairingAuthority};
    use tempfile::TempDir;

    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = Config {
            workspace_dir: tmp.path().join("workspace"),
            ..Config::default()
        };
        (tmp, cfg)
    }

    fn options(name: &str) -> DeviceAddOptions {
        DeviceAddOptions {
            name: name.to_string(),
            desk: "crypto".to_string(),
            role: "stablecoin_peg_watcher".to_string(),
            ttl: Duration::minutes(5),
            core_url: "http://127.0.0.1:8787".to_string(),
            qr: false,
            png: None,
            watch: true,
            auto_approve_observe: false,
            grant_observe_lease: false,
            lease_ttl: Duration::minutes(30),
            no_server: true,
            serve: false,
            bind: "127.0.0.1:8787".to_string(),
            watch_timeout_seconds: 0,
            watch_poll_seconds: 1,
            approval_mode: DeviceApprovalMode::NoPrompt,
        }
    }

    fn submit_request(cfg: &Config, result: &DeviceAddResult, name: &str) -> PairingRequest {
        pairing::submit_child_pair_request(
            cfg,
            ChildPairRequestInput {
                core_url: &result.invite.invite.core_url,
                invite_token: &result.invite.invite_token,
                node_name: Some(name),
                surface: "termux_worker",
                capabilities: &["echo".to_string(), "compute_scalar".to_string()],
                requested_role: Some("stablecoin_peg_watcher"),
                requested_authority: PairingAuthority::Observe,
            },
        )
        .expect("request")
    }

    #[test]
    fn device_add_creates_pairing_invite() {
        let (_tmp, cfg) = test_config();
        let result = add_device(&cfg, options("tablet-01")).expect("device add");
        assert_eq!(pairing::list_invites(&cfg).expect("invites").len(), 1);
        assert_eq!(
            result.invite.invite.max_authority,
            PairingAuthority::Observe
        );
    }

    #[test]
    fn device_add_defaults_to_manual_approval_and_no_lease() {
        let (_tmp, cfg) = test_config();
        let result = add_device(&cfg, options("tablet-01")).expect("device add");
        assert!(result.accepted.is_none());
        assert!(result.lease.is_none());
        assert!(!result.final_state.execution_enabled);
        assert!(!result.final_state.proposal_creation_enabled);
    }

    #[test]
    fn device_add_auto_approve_observe_only_after_request() {
        let (_tmp, cfg) = test_config();
        let first = add_device(&cfg, options("tablet-01")).expect("invite");
        submit_request(&cfg, &first, "tablet-01");
        let mut approve = options("tablet-01");
        approve.auto_approve_observe = true;
        let result = add_device(&cfg, approve).expect("auto approve");
        assert!(result.accepted.is_some());
        assert_eq!(
            pairing::list_accepted_nodes(&cfg).expect("accepted").len(),
            1
        );
    }

    #[test]
    fn device_add_watch_detects_pending_request() {
        let (_tmp, cfg) = test_config();
        let first = add_device(&cfg, options("tablet-01")).expect("invite");
        submit_request(&cfg, &first, "tablet-01");
        let result = add_device(&cfg, options("tablet-01")).expect("watch");
        assert!(result.request.is_some());
        assert!(result.accepted.is_none());
        assert!(result.rejected.is_some());
    }

    #[test]
    fn device_add_watch_prompts_manual_approval() {
        let (_tmp, cfg) = test_config();
        let first = add_device(&cfg, options("tablet-01")).expect("invite");
        submit_request(&cfg, &first, "tablet-01");
        let mut opts = options("tablet-01");
        opts.approval_mode = DeviceApprovalMode::Input("y".to_string());
        let result = add_device(&cfg, opts).expect("manual approval");
        assert!(result.accepted.is_some());
        assert!(result.rejected.is_none());
    }

    #[test]
    fn device_add_watch_rejects_by_default_on_empty_input() {
        let (_tmp, cfg) = test_config();
        let first = add_device(&cfg, options("tablet-01")).expect("invite");
        submit_request(&cfg, &first, "tablet-01");
        let mut opts = options("tablet-01");
        opts.approval_mode = DeviceApprovalMode::Input(String::new());
        let result = add_device(&cfg, opts).expect("manual reject");
        assert!(result.accepted.is_none());
        assert!(result.rejected.is_some());
        assert_eq!(
            pairing::list_requests(&cfg).expect("requests")[0].status,
            PairingRequestStatus::Rejected
        );
    }

    #[test]
    fn device_add_rejects_auto_approve_ttl_above_cap() {
        let (_tmp, cfg) = test_config();
        let mut opts = options("tablet-01");
        opts.auto_approve_observe = true;
        opts.ttl = Duration::minutes(10);
        let err = add_device(&cfg, opts).expect_err("ttl rejected");
        assert!(err.to_string().contains("5m"));
    }

    #[test]
    fn device_add_grants_observe_lease_only_when_explicit() {
        let (_tmp, cfg) = test_config();
        let first = add_device(&cfg, options("tablet-01")).expect("invite");
        submit_request(&cfg, &first, "tablet-01");
        let request_id = pairing::list_requests(&cfg).expect("requests")[0]
            .request_id
            .clone();
        pairing::approve_request(&cfg, &request_id, "operator").expect("approve");
        let mut opts = options("tablet-01");
        opts.grant_observe_lease = true;
        let result = add_device(&cfg, opts).expect("lease");
        let lease = result.lease.expect("observe lease");
        assert_eq!(lease.authority, cluster::LeaseAuthority::Observe);
    }

    #[test]
    fn device_add_final_state_has_zero_execution() {
        let (_tmp, cfg) = test_config();
        let result = add_device(&cfg, options("tablet-01")).expect("device add");
        assert!(!result.final_state.execution_enabled);
        assert!(!result.final_state.approval_enabled);
        assert!(!result.final_state.canonical_write_enabled);
        assert!(!result.final_state.provider_calls_enabled);
        assert!(!result.final_state.proposal_creation_enabled);
    }
}
