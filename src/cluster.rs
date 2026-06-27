use crate::cluster_boundary::ClusterSurfaceKind;
use crate::compute::{self, ComputeBackend};
use crate::config::Config;
use crate::desk_registry::DeskId;
use crate::device_telemetry::{
    self, DeviceTelemetry, DeviceTelemetryWarningPolicy, telemetry_warnings,
};
use crate::pairing::{self, PairingAuthority};
use crate::timing;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ClusterNodeId(String);

impl ClusterNodeId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_id("cluster node id", &value)?;
        Ok(Self(value))
    }

    #[allow(dead_code)]
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClusterNodeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ClusterNodeId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value.trim())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct ClusterRoleId(String);

impl ClusterRoleId {
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = value.into();
        validate_id("cluster role id", &value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ClusterRoleId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for ClusterRoleId {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        Self::new(value.trim())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ClusterCapability {
    Echo,
    Sleep,
    HttpGet,
    Heartbeat,
    QueueReadWrite,
    EvidenceWrite,
    RoleDisplay,
    CapabilityReport,
    Termux,
    BatteryStatus,
    NetworkStatus,
    PythonAvailable,
    OllamaAvailable,
    ComputeScalar,
}

impl fmt::Display for ClusterCapability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::Echo => "echo",
            Self::Sleep => "sleep",
            Self::HttpGet => "http_get",
            Self::Heartbeat => "heartbeat",
            Self::QueueReadWrite => "queue_read_write",
            Self::EvidenceWrite => "evidence_write",
            Self::RoleDisplay => "role_display",
            Self::CapabilityReport => "capability_report",
            Self::Termux => "termux",
            Self::BatteryStatus => "battery_status",
            Self::NetworkStatus => "network_status",
            Self::PythonAvailable => "python_available",
            Self::OllamaAvailable => "ollama_available",
            Self::ComputeScalar => "compute_scalar",
        };
        f.write_str(value)
    }
}

impl FromStr for ClusterCapability {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "echo" => Ok(Self::Echo),
            "sleep" => Ok(Self::Sleep),
            "http_get" | "http-get" | "http" => Ok(Self::HttpGet),
            "heartbeat" => Ok(Self::Heartbeat),
            "queue_read_write" | "queue-read-write" | "queue" => Ok(Self::QueueReadWrite),
            "evidence_write" | "evidence-write" | "evidence" => Ok(Self::EvidenceWrite),
            "role_display" | "role-display" => Ok(Self::RoleDisplay),
            "capability_report" | "capability-report" => Ok(Self::CapabilityReport),
            "termux" => Ok(Self::Termux),
            "battery_status" | "battery-status" => Ok(Self::BatteryStatus),
            "network_status" | "network-status" => Ok(Self::NetworkStatus),
            "python_available" | "python-available" => Ok(Self::PythonAvailable),
            "ollama_available" | "ollama-available" => Ok(Self::OllamaAvailable),
            "compute_scalar" | "compute-scalar" | "compute" => Ok(Self::ComputeScalar),
            other => Err(anyhow!("unsupported cluster capability '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ClusterAuthorityLevel {
    Observe,
    Analyze,
    Propose,
}

impl fmt::Display for ClusterAuthorityLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observe => f.write_str("observe"),
            Self::Analyze => f.write_str("analyze"),
            Self::Propose => f.write_str("propose"),
        }
    }
}

impl FromStr for ClusterAuthorityLevel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "observe" => Ok(Self::Observe),
            "analyze" => Ok(Self::Analyze),
            "propose" => Ok(Self::Propose),
            other => Err(anyhow!("unsupported cluster authority level '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterJobKind {
    Echo,
    Sleep,
    HttpGet,
    ComputeFreshnessScan,
    ComputePegDeviation,
    DeskObserveEvidenceFreshness,
    DeskObservePegDeviation,
}

impl ClusterJobKind {
    fn required_capability(self) -> ClusterCapability {
        match self {
            Self::Echo => ClusterCapability::Echo,
            Self::Sleep => ClusterCapability::Sleep,
            Self::HttpGet => ClusterCapability::HttpGet,
            Self::ComputeFreshnessScan
            | Self::ComputePegDeviation
            | Self::DeskObserveEvidenceFreshness
            | Self::DeskObservePegDeviation => ClusterCapability::ComputeScalar,
        }
    }
}

impl fmt::Display for ClusterJobKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Echo => f.write_str("echo"),
            Self::Sleep => f.write_str("sleep"),
            Self::HttpGet => f.write_str("http_get"),
            Self::ComputeFreshnessScan => f.write_str("compute_freshness_scan"),
            Self::ComputePegDeviation => f.write_str("compute_peg_deviation"),
            Self::DeskObserveEvidenceFreshness => f.write_str("desk_observe_evidence_freshness"),
            Self::DeskObservePegDeviation => f.write_str("desk_observe_peg_deviation"),
        }
    }
}

impl FromStr for ClusterJobKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "echo" => Ok(Self::Echo),
            "sleep" => Ok(Self::Sleep),
            "http_get" | "http-get" | "http" => Ok(Self::HttpGet),
            "compute_freshness_scan" | "compute-freshness-scan" | "freshness" => {
                Ok(Self::ComputeFreshnessScan)
            }
            "compute_peg_deviation" | "compute-peg-deviation" | "peg-deviation" => {
                Ok(Self::ComputePegDeviation)
            }
            "desk_observe_evidence_freshness"
            | "desk-observe-evidence-freshness"
            | "desk_observe_freshness" => Ok(Self::DeskObserveEvidenceFreshness),
            "desk_observe_peg_deviation" | "desk-observe-peg-deviation" | "desk_observe_peg" => {
                Ok(Self::DeskObservePegDeviation)
            }
            other => Err(anyhow!("unsupported cluster job kind '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterNodeState {
    Unregistered,
    Registered,
    Active,
    Stale,
    Suspended,
    Retired,
}

impl fmt::Display for ClusterNodeState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unregistered => f.write_str("unregistered"),
            Self::Registered => f.write_str("registered"),
            Self::Active => f.write_str("active"),
            Self::Stale => f.write_str("stale"),
            Self::Suspended => f.write_str("suspended"),
            Self::Retired => f.write_str("retired"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterNodeEvent {
    Register,
    Heartbeat,
    MarkStale,
    Suspend,
    Retire,
}

impl fmt::Display for ClusterNodeEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Register => f.write_str("register"),
            Self::Heartbeat => f.write_str("heartbeat"),
            Self::MarkStale => f.write_str("mark_stale"),
            Self::Suspend => f.write_str("suspend"),
            Self::Retire => f.write_str("retire"),
        }
    }
}

pub struct ClusterNodeFsm;

impl ClusterNodeFsm {
    pub fn transition(
        &self,
        current: ClusterNodeState,
        event: ClusterNodeEvent,
    ) -> Result<ClusterNodeState> {
        use ClusterNodeEvent::*;
        use ClusterNodeState::*;
        match (current, event) {
            (Unregistered, Register) => Ok(Registered),
            (Registered, Heartbeat) | (Stale, Heartbeat) => Ok(Active),
            (Registered, MarkStale) | (Active, MarkStale) => Ok(Stale),
            (Registered, Suspend) | (Active, Suspend) | (Stale, Suspend) => Ok(Suspended),
            (Registered, Retire) | (Active, Retire) | (Stale, Retire) | (Suspended, Retire) => {
                Ok(Retired)
            }
            (Retired, _) => Err(anyhow!(
                "cluster_node terminal state rejects event '{event}'"
            )),
            _ => Err(anyhow!(
                "cluster_node event '{}' is not allowed from state '{}'",
                event,
                current
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterLeaseState {
    Requested,
    Granted,
    Renewed,
    Expired,
    Revoked,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterLeaseEvent {
    Grant,
    Renew,
    Expire,
    Revoke,
}

pub struct ClusterLeaseFsm;

impl ClusterLeaseFsm {
    pub fn transition(
        &self,
        current: ClusterLeaseState,
        event: ClusterLeaseEvent,
    ) -> Result<ClusterLeaseState> {
        use ClusterLeaseEvent::*;
        use ClusterLeaseState::*;
        match (current, event) {
            (Requested, Grant) => Ok(Granted),
            (Granted, Renew) | (Renewed, Renew) => Ok(Renewed),
            (Granted, Expire) | (Renewed, Expire) | (Requested, Expire) => Ok(Expired),
            (Granted, Revoke) | (Renewed, Revoke) | (Requested, Revoke) => Ok(Revoked),
            (Expired, _) | (Revoked, _) => {
                Err(anyhow!("cluster_lease terminal state rejects event"))
            }
            _ => Err(anyhow!(
                "cluster_lease event is not allowed from current state"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ClusterJobState {
    Created,
    Assigned,
    AcceptedByChild,
    Running,
    EvidenceReturned,
    RecordedByCore,
    ReplayVerified,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum ClusterJobEvent {
    Assign,
    Accept,
    Start,
    ReturnEvidence,
    Record,
    VerifyReplay,
    Reject,
    Expire,
}

#[allow(dead_code)]
pub struct ClusterJobFsm;

#[allow(dead_code)]
impl ClusterJobFsm {
    pub fn transition(
        &self,
        current: ClusterJobState,
        event: ClusterJobEvent,
    ) -> Result<ClusterJobState> {
        use ClusterJobEvent::*;
        use ClusterJobState::*;
        match (current, event) {
            (Created, Assign) => Ok(Assigned),
            (Assigned, Accept) => Ok(AcceptedByChild),
            (AcceptedByChild, Start) => Ok(Running),
            (Running, ReturnEvidence) => Ok(EvidenceReturned),
            (EvidenceReturned, Record) => Ok(RecordedByCore),
            (RecordedByCore, VerifyReplay) => Ok(ReplayVerified),
            (Created, Reject) | (Assigned, Reject) | (AcceptedByChild, Reject) => Ok(Rejected),
            (Created, Expire) | (Assigned, Expire) | (AcceptedByChild, Expire) => Ok(Expired),
            (ReplayVerified, _) | (Rejected, _) | (Expired, _) => {
                Err(anyhow!("cluster_job terminal state rejects event"))
            }
            _ => Err(anyhow!(
                "cluster_job event is not allowed from current state"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterNode {
    pub node_id: ClusterNodeId,
    pub display_name: String,
    pub surface: ClusterSurfaceKind,
    pub capabilities: Vec<ClusterCapability>,
    pub registered_at: String,
    pub state: ClusterNodeState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterRole {
    pub role_id: ClusterRoleId,
    pub desk_id: String,
    pub display_name: String,
    pub max_authority: ClusterAuthorityLevel,
    pub allowed_capabilities: Vec<ClusterCapability>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterLease {
    pub lease_id: String,
    pub node_id: ClusterNodeId,
    pub role_id: ClusterRoleId,
    #[serde(default)]
    pub desk_id: String,
    #[serde(default)]
    pub authority: LeaseAuthority,
    pub granted_at: String,
    #[serde(default = "default_operator")]
    pub created_by: String,
    pub expires_at: String,
    pub state: ClusterLeaseState,
    #[serde(default)]
    pub revoked: bool,
    #[serde(default)]
    pub revoked_at: Option<String>,
    #[serde(default)]
    pub revoked_reason: Option<String>,
    #[serde(default)]
    pub policy_hash: Option<String>,
    #[serde(default)]
    pub playbook_id: Option<String>,
    #[serde(default)]
    pub playbook_hash: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum LeaseAuthority {
    Observe,
}

impl Default for LeaseAuthority {
    fn default() -> Self {
        Self::Observe
    }
}

impl fmt::Display for LeaseAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Observe => f.write_str("observe"),
        }
    }
}

impl FromStr for LeaseAuthority {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "observe" => Ok(Self::Observe),
            "analyze" | "propose" | "approve" | "execute" | "canonical_write"
            | "canonical-write" => Err(anyhow!(
                "lease authority '{}' exceeds this checkpoint boundary",
                value
            )),
            other => Err(anyhow!("unsupported lease authority '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum HeartbeatSource {
    ChildCli,
    CoreCliSmoke,
    TestFixture,
}

impl Default for HeartbeatSource {
    fn default() -> Self {
        Self::ChildCli
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterHeartbeat {
    pub heartbeat_id: String,
    pub node_id: ClusterNodeId,
    pub role_id: Option<ClusterRoleId>,
    pub lease_id: Option<String>,
    pub timestamp: String,
    pub queue_depth: usize,
    pub current_job_id: Option<String>,
    pub capability_hash: String,
    pub software_version: String,
    #[serde(default)]
    pub paired: bool,
    #[serde(default)]
    pub approved: bool,
    #[serde(default)]
    pub source: HeartbeatSource,
    #[serde(default)]
    pub device_telemetry: Option<DeviceTelemetry>,
    #[serde(default = "default_replay_safe")]
    pub replay_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterHeartbeatInput {
    pub node_id: ClusterNodeId,
    pub surface: Option<ClusterSurfaceKind>,
    pub claimed_capabilities: Vec<ClusterCapability>,
    pub execution_enabled: bool,
    pub approval_enabled: bool,
    pub canonical_write_enabled: bool,
    pub source: HeartbeatSource,
    pub device_telemetry: Option<DeviceTelemetry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterNodeStatus {
    pub node_id: ClusterNodeId,
    pub paired: bool,
    pub approved: bool,
    pub online: bool,
    pub stale: bool,
    pub last_heartbeat_at: Option<String>,
    pub active_lease_id: Option<String>,
    pub active_role_id: Option<ClusterRoleId>,
    pub active_desk_id: Option<String>,
    pub authority: LeaseAuthority,
    pub execution_enabled: bool,
    pub approval_enabled: bool,
    pub canonical_write_enabled: bool,
    pub jobs_enabled: bool,
    pub device_telemetry: Option<DeviceTelemetry>,
    pub telemetry_warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LeaseValidationResult {
    Valid,
    Missing,
    Expired,
    Revoked,
    NodeNotPaired,
    NodeNotApproved,
    AuthorityExceedsPairing,
    NodeMismatch,
    DeskMismatch,
    RoleMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterComputeRequirement {
    pub fixture: String,
    pub backend_requested: ComputeBackend,
    pub timing_policy_hash: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(clippy::enum_variant_names)]
pub enum DeskEvidenceKind {
    EvidenceFreshnessObservation,
    StablecoinPegDeviationObservation,
    BitcoinDcaScheduleObservation,
    ForexCalendarTimingObservation,
    SportsEventSlateObservation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeskObservationEvidence {
    pub evidence_id: String,
    pub node_id: String,
    pub lease_id: String,
    pub desk_id: DeskId,
    pub role_id: String,
    pub knowledge_pack_id: Option<String>,
    pub playbook_id: Option<String>,
    pub playbook_hash: Option<String>,
    pub evidence_kind: DeskEvidenceKind,
    pub authority: LeaseAuthority,
    pub timing_decision_id: String,
    pub input_hash: String,
    pub output_hash: String,
    pub compute_meta: Option<compute::validation::ComputeEvidenceMeta>,
    pub numeric_confidence: Option<compute::boundary::NumericConfidence>,
    pub created_at: String,
    pub replay_safe: bool,
    pub proposal_created: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterJobEnvelope {
    pub job_id: String,
    pub node_id: ClusterNodeId,
    pub role_id: ClusterRoleId,
    pub lease_id: String,
    pub desk_id: String,
    pub job_kind: ClusterJobKind,
    pub authority_level: ClusterAuthorityLevel,
    pub payload: Value,
    pub created_at: String,
    pub expires_at: String,
    pub required_capabilities: Vec<ClusterCapability>,
    pub evidence_required: bool,
    pub policy_hash: String,
    #[serde(default)]
    pub compute_requirement: Option<ClusterComputeRequirement>,
    #[serde(default)]
    pub playbook_id: Option<String>,
    #[serde(default)]
    pub playbook_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterEvidenceReceipt {
    pub receipt_id: String,
    pub job_id: String,
    pub node_id: ClusterNodeId,
    pub role_id: ClusterRoleId,
    pub desk_id: String,
    pub timestamp: String,
    pub result_status: String,
    pub output_hash: String,
    pub artifact_paths: Vec<PathBuf>,
    pub stderr_summary: Option<String>,
    pub policy_decision: String,
    pub replay_safe: bool,
    pub promoted_to_proposal: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ClusterReport {
    pub online_nodes: Vec<ClusterNodeId>,
    pub stale_nodes: Vec<ClusterNodeId>,
    pub active_leases: Vec<String>,
    pub failed_jobs: usize,
    pub recent_evidence: Vec<String>,
    pub recent_compute_evidence: Vec<String>,
    pub pending_proposals: usize,
    pub device_health: Vec<DeviceHealthSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeviceHealthSummary {
    pub node_id: ClusterNodeId,
    pub device_display_name: Option<String>,
    pub os: Option<String>,
    pub arch: Option<String>,
    pub storage_available_bytes: Option<u64>,
    pub battery_percent: Option<f64>,
    pub battery_charging: Option<bool>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterDeviceAuthority {
    CoreAuthority,
    ChildEvidenceWorker,
}

impl fmt::Display for ClusterDeviceAuthority {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CoreAuthority => f.write_str("core_authority"),
            Self::ChildEvidenceWorker => f.write_str("child_evidence_worker"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterTransportMode {
    LocalFile,
    ManualSync,
    SshLan,
    TermuxSsh,
    FutureHttpPull,
}

impl fmt::Display for ClusterTransportMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::LocalFile => f.write_str("local_file"),
            Self::ManualSync => f.write_str("manual_sync"),
            Self::SshLan => f.write_str("ssh_lan"),
            Self::TermuxSsh => f.write_str("termux_ssh"),
            Self::FutureHttpPull => f.write_str("future_http_pull"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterTransportStatus {
    AvailableNow,
    Planned,
}

impl fmt::Display for ClusterTransportStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AvailableNow => f.write_str("available_now"),
            Self::Planned => f.write_str("planned"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterTransportOption {
    pub mode: ClusterTransportMode,
    pub status: ClusterTransportStatus,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterDeviceOption {
    pub profile_id: String,
    pub display_name: String,
    pub authority: ClusterDeviceAuthority,
    pub can_be_execution_leader: bool,
    pub surface: ClusterSurfaceKind,
    pub runtime_profile: String,
    pub transports: Vec<ClusterTransportOption>,
    pub default_capabilities: Vec<ClusterCapability>,
    pub recommended_roles: Vec<ClusterRoleId>,
    pub notes: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClusterDeskFsmState {
    KnowledgePackLoaded,
    CandidateObserved,
    LanguageMatched,
    CriteriaValidated,
    RiskBoxChecked,
    AsymmetryScored,
    PaperPlanReady,
    Rejected,
}

impl fmt::Display for ClusterDeskFsmState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::KnowledgePackLoaded => f.write_str("knowledge_pack_loaded"),
            Self::CandidateObserved => f.write_str("candidate_observed"),
            Self::LanguageMatched => f.write_str("language_matched"),
            Self::CriteriaValidated => f.write_str("criteria_validated"),
            Self::RiskBoxChecked => f.write_str("risk_box_checked"),
            Self::AsymmetryScored => f.write_str("asymmetry_scored"),
            Self::PaperPlanReady => f.write_str("paper_plan_ready"),
            Self::Rejected => f.write_str("rejected"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterRailCriterion {
    pub criterion_id: String,
    pub language: String,
    pub required: bool,
    pub evidence_signal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterDeskRiskLimits {
    pub starting_unit: String,
    pub max_open_positions_or_slips: u8,
    pub max_parallel_instruments_or_events: u8,
    pub safe_margin_required: bool,
    pub live_execution_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterDeskRail {
    pub rail_id: String,
    pub desk_id: String,
    pub display_name: String,
    pub knowledge_packs: Vec<String>,
    pub fsm_states: Vec<ClusterDeskFsmState>,
    pub technical_language: Vec<String>,
    pub fundamental_language: Vec<String>,
    pub non_negotiable_rules: Vec<String>,
    pub criteria: Vec<ClusterRailCriterion>,
    pub risk_limits: ClusterDeskRiskLimits,
    pub forbidden_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ClusterEvent {
    event_id: String,
    event_kind: String,
    node_id: Option<ClusterNodeId>,
    detail: String,
    occurred_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
struct ClusterLeaseEventRecord {
    event_id: String,
    timestamp: String,
    kind: String,
    node_id: Option<ClusterNodeId>,
    lease_id: Option<String>,
    desk_id: Option<String>,
    role_id: Option<String>,
    reason: String,
    replay_safe: bool,
}

pub fn init_cluster(cfg: &Config) -> Result<()> {
    let paths = ClusterPaths::new(cfg);
    paths.ensure_dirs()?;
    write_json_pretty(&paths.roles, &default_roles()?)?;
    if !paths.leases.exists() {
        write_json_pretty(&paths.leases, &Vec::<ClusterLease>::new())?;
    }
    append_event(
        &paths,
        "cluster_initialized",
        None,
        "cluster state initialized",
    )?;
    Ok(())
}

pub fn register_node(
    cfg: &Config,
    display_name: &str,
    surface: ClusterSurfaceKind,
    capabilities: Vec<ClusterCapability>,
) -> Result<ClusterNode> {
    init_cluster(cfg)?;
    if display_name.trim().is_empty() {
        return Err(anyhow!("cluster node display name is empty"));
    }
    let node_id = ClusterNodeId::new(slug_id("node", display_name))?;
    let state =
        ClusterNodeFsm.transition(ClusterNodeState::Unregistered, ClusterNodeEvent::Register)?;
    let node = ClusterNode {
        node_id: node_id.clone(),
        display_name: display_name.trim().to_string(),
        surface,
        capabilities: normalize_capabilities(capabilities),
        registered_at: now(),
        state,
    };
    let paths = ClusterPaths::new(cfg);
    append_json_line(&paths.nodes, &node)?;
    append_event(
        &paths,
        "cluster_node_registered",
        Some(node_id),
        "child node registered as non-authoritative worker",
    )?;
    Ok(node)
}

pub fn list_nodes(cfg: &Config) -> Result<Vec<ClusterNode>> {
    let paths = ClusterPaths::new(cfg);
    read_jsonl(&paths.nodes)
}

pub fn assign_role(
    cfg: &Config,
    node_id: &ClusterNodeId,
    role_id: &ClusterRoleId,
    ttl: Duration,
) -> Result<ClusterLease> {
    grant_observe_lease(cfg, node_id, role_id, ttl, "operator")
}

pub fn grant_observe_lease(
    cfg: &Config,
    node_id: &ClusterNodeId,
    role_id: &ClusterRoleId,
    ttl: Duration,
    created_by: &str,
) -> Result<ClusterLease> {
    grant_observe_lease_internal(cfg, node_id, role_id, ttl, created_by, None)
}

pub fn grant_observe_lease_with_playbook(
    cfg: &Config,
    node_id: &ClusterNodeId,
    role_id: &ClusterRoleId,
    ttl: Duration,
    created_by: &str,
    playbook_id: &str,
) -> Result<ClusterLease> {
    let playbook = crate::playbook::load_playbook_by_id(cfg, playbook_id)?;
    crate::playbook::validate_playbook(&playbook)?;
    grant_observe_lease_internal(
        cfg,
        node_id,
        role_id,
        ttl,
        created_by,
        Some((playbook.playbook_id, playbook.hash)),
    )
}

fn grant_observe_lease_internal(
    cfg: &Config,
    node_id: &ClusterNodeId,
    role_id: &ClusterRoleId,
    ttl: Duration,
    created_by: &str,
    playbook: Option<(String, String)>,
) -> Result<ClusterLease> {
    init_cluster(cfg)?;
    ensure_approved_paired_node(cfg, node_id)?;
    let node = find_node(cfg, node_id)?;
    let role = find_role(cfg, role_id)?;
    if ttl <= Duration::zero() {
        return Err(anyhow!("lease ttl must be positive"));
    }
    if ttl > Duration::hours(2) {
        return Err(anyhow!("lease ttl exceeds max 2h"));
    }
    let node_caps: BTreeSet<_> = node.capabilities.iter().copied().collect();
    for required in &role.allowed_capabilities {
        if matches!(
            required,
            ClusterCapability::Heartbeat
                | ClusterCapability::QueueReadWrite
                | ClusterCapability::EvidenceWrite
                | ClusterCapability::RoleDisplay
                | ClusterCapability::CapabilityReport
        ) {
            continue;
        }
        if !node_caps.contains(required) {
            return Err(anyhow!(
                "node '{}' lacks role capability '{}'",
                node_id,
                required
            ));
        }
    }
    let granted =
        ClusterLeaseFsm.transition(ClusterLeaseState::Requested, ClusterLeaseEvent::Grant)?;
    let (playbook_id, playbook_hash) = playbook
        .map(|(id, hash)| (Some(id), Some(hash)))
        .unwrap_or((None, None));
    let lease = ClusterLease {
        lease_id: format!("lease-{}-{}", node_id.as_str(), Utc::now().timestamp()),
        node_id: node_id.clone(),
        role_id: role_id.clone(),
        desk_id: role.desk_id.clone(),
        authority: LeaseAuthority::Observe,
        granted_at: now(),
        created_by: created_by.to_string(),
        expires_at: (Utc::now() + ttl).to_rfc3339(),
        state: granted,
        revoked: false,
        revoked_at: None,
        revoked_reason: None,
        policy_hash: Some("paired-child-observe-lease-v1".to_string()),
        playbook_id,
        playbook_hash,
    };
    let paths = ClusterPaths::new(cfg);
    let mut leases = read_json_file::<Vec<ClusterLease>>(&paths.leases)?.unwrap_or_default();
    leases.retain(|existing| existing.node_id != *node_id);
    leases.push(lease.clone());
    write_json_pretty(&paths.leases, &leases)?;
    append_event(
        &paths,
        "cluster_role_assigned",
        Some(node_id.clone()),
        &format!("role={} lease={}", role_id, lease.lease_id),
    )?;
    append_lease_event(
        &paths,
        "cluster_lease_granted",
        Some(node_id),
        Some(&lease.lease_id),
        Some(&role.desk_id),
        Some(role_id.as_str()),
        "observe-only lease granted; jobs remain disabled in this checkpoint",
    )?;
    Ok(lease)
}

pub fn heartbeat(cfg: &Config, node_id: &ClusterNodeId) -> Result<ClusterHeartbeat> {
    heartbeat_with_input(
        cfg,
        ClusterHeartbeatInput {
            node_id: node_id.clone(),
            surface: None,
            claimed_capabilities: vec![],
            execution_enabled: false,
            approval_enabled: false,
            canonical_write_enabled: false,
            source: HeartbeatSource::ChildCli,
            device_telemetry: None,
        },
    )
}

pub fn heartbeat_with_input(
    cfg: &Config,
    input: ClusterHeartbeatInput,
) -> Result<ClusterHeartbeat> {
    init_cluster(cfg)?;
    if input.execution_enabled || input.approval_enabled || input.canonical_write_enabled {
        return Err(anyhow!(
            "heartbeat cannot claim execution, approval, or canonical write authority"
        ));
    }
    ensure_approved_paired_node(cfg, &input.node_id)?;
    let node = find_node(cfg, &input.node_id)?;
    if let Some(surface) = input.surface
        && surface != node.surface
    {
        return Err(anyhow!("heartbeat surface does not match registered node"));
    }
    let lease = active_lease(cfg, &input.node_id).ok();
    let paths = ClusterPaths::new(cfg);
    let hb = ClusterHeartbeat {
        heartbeat_id: format!(
            "heartbeat-{}-{}",
            input.node_id.as_str(),
            Utc::now().timestamp()
        ),
        node_id: input.node_id.clone(),
        role_id: lease.as_ref().map(|lease| lease.role_id.clone()),
        lease_id: lease.as_ref().map(|lease| lease.lease_id.clone()),
        timestamp: now(),
        queue_depth: queued_jobs_for_node(&paths, &input.node_id)?.len(),
        current_job_id: None,
        capability_hash: capability_hash(if input.claimed_capabilities.is_empty() {
            &node.capabilities
        } else {
            &input.claimed_capabilities
        }),
        software_version: env!("CARGO_PKG_VERSION").to_string(),
        paired: true,
        approved: true,
        source: input.source,
        device_telemetry: sanitize_device_telemetry(input.device_telemetry)?,
        replay_safe: true,
    };
    append_json_line(&paths.heartbeats, &hb)?;
    let _ = ClusterNodeFsm.transition(node.state, ClusterNodeEvent::Heartbeat)?;
    append_event(
        &paths,
        "cluster_heartbeat_received",
        Some(input.node_id.clone()),
        "heartbeat recorded",
    )?;
    append_lease_event(
        &paths,
        "cluster_heartbeat_received",
        Some(&input.node_id),
        lease.as_ref().map(|lease| lease.lease_id.as_str()),
        lease.as_ref().map(|lease| lease.desk_id.as_str()),
        lease.as_ref().map(|lease| lease.role_id.as_str()),
        "approved paired child heartbeat recorded",
    )?;
    Ok(hb)
}

pub fn submit_job(
    cfg: &Config,
    node_id: &ClusterNodeId,
    desk_id: &str,
    job_kind: ClusterJobKind,
    payload: Value,
    ttl: Duration,
) -> Result<ClusterJobEnvelope> {
    init_cluster(cfg)?;
    let lease = active_lease(cfg, node_id)?;
    let role = find_role(cfg, &lease.role_id)?;
    let required = job_kind.required_capability();
    let compute_requirement = build_compute_requirement(job_kind, &payload)?;
    let job = ClusterJobEnvelope {
        job_id: format!(
            "cluster-job-{}-{}",
            node_id.as_str(),
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        node_id: node_id.clone(),
        role_id: lease.role_id.clone(),
        lease_id: lease.lease_id.clone(),
        desk_id: desk_id.trim().to_string(),
        job_kind,
        authority_level: ClusterAuthorityLevel::Observe,
        payload,
        created_at: now(),
        expires_at: (Utc::now() + ttl).to_rfc3339(),
        required_capabilities: vec![required],
        evidence_required: true,
        policy_hash: "local-file-cluster-policy-v1".to_string(),
        compute_requirement,
        playbook_id: lease.playbook_id.clone(),
        playbook_hash: lease.playbook_hash.clone(),
    };
    validate_job(cfg, &job, &role)?;
    let paths = ClusterPaths::new(cfg);
    append_json_line(&paths.jobs_for_node(node_id), &job)?;
    append_event(
        &paths,
        "cluster_job_submitted",
        Some(node_id.clone()),
        &format!("job={} desk={} kind={}", job.job_id, desk_id, job_kind),
    )?;
    Ok(job)
}

pub fn child_run_once(
    cfg: &Config,
    node_id: &ClusterNodeId,
) -> Result<Option<ClusterEvidenceReceipt>> {
    init_cluster(cfg)?;
    let paths = ClusterPaths::new(cfg);
    let mut jobs = queued_jobs_for_node(&paths, node_id)?;
    if jobs.is_empty() {
        return Ok(None);
    }
    let job = jobs.remove(0);
    let role = find_role(cfg, &job.role_id)?;
    validate_job(cfg, &job, &role)?;
    write_jsonl(&paths.jobs_for_node(node_id), &jobs)?;
    append_event(
        &paths,
        "cluster_job_started",
        Some(node_id.clone()),
        &format!("job={}", job.job_id),
    )?;
    let output = execute_cluster_job(&job)?;
    let receipt_id = format!("receipt-{}", job.job_id);
    let artifact_path = paths.evidence.join(format!("{receipt_id}.json"));
    let receipt = ClusterEvidenceReceipt {
        receipt_id,
        job_id: job.job_id.clone(),
        node_id: node_id.clone(),
        role_id: job.role_id.clone(),
        desk_id: job.desk_id.clone(),
        timestamp: now(),
        result_status: "ok".to_string(),
        output_hash: stable_hash(&output),
        artifact_paths: vec![artifact_path.clone()],
        stderr_summary: None,
        policy_decision: "allowed_local_observe_job".to_string(),
        replay_safe: true,
        promoted_to_proposal: false,
    };
    write_json_pretty(
        &artifact_path,
        &serde_json::json!({
            "receipt": receipt,
            "output": output,
            "non_authoritative": true
        }),
    )?;
    append_event(
        &paths,
        "cluster_evidence_recorded",
        Some(node_id.clone()),
        &format!("job={} receipt={}", job.job_id, receipt.receipt_id),
    )?;
    Ok(Some(receipt))
}

pub fn report(cfg: &Config) -> Result<ClusterReport> {
    init_cluster(cfg)?;
    let paths = ClusterPaths::new(cfg);
    let nodes = list_nodes(cfg)?;
    let heartbeats = read_jsonl::<ClusterHeartbeat>(&paths.heartbeats)?;
    let latest_heartbeat = heartbeats.into_iter().fold(
        BTreeMap::<ClusterNodeId, ClusterHeartbeat>::new(),
        |mut acc, heartbeat| {
            acc.insert(heartbeat.node_id.clone(), heartbeat);
            acc
        },
    );
    let now = Utc::now();
    let mut online_nodes = Vec::new();
    let mut stale_nodes = Vec::new();
    for node in nodes {
        match latest_heartbeat.get(&node.node_id) {
            Some(hb) if parse_ts(&hb.timestamp).is_ok_and(|ts| now - ts < Duration::minutes(5)) => {
                online_nodes.push(node.node_id);
            }
            _ => stale_nodes.push(node.node_id),
        }
    }
    let active_leases = read_json_file::<Vec<ClusterLease>>(&paths.leases)?
        .unwrap_or_default()
        .into_iter()
        .filter(|lease| !is_expired(&lease.expires_at))
        .map(|lease| lease.lease_id)
        .collect();
    let recent_evidence = fs::read_dir(&paths.evidence)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .filter_map(|entry| entry.file_name().into_string().ok())
        .take(10)
        .collect();
    let recent_compute_evidence = recent_compute_evidence(&paths.evidence)?;
    let device_health = latest_heartbeat
        .values()
        .filter_map(|heartbeat| {
            let telemetry = heartbeat.device_telemetry.as_ref()?;
            let warnings = telemetry_warnings(
                heartbeat.node_id.as_str(),
                telemetry,
                &DeviceTelemetryWarningPolicy::default(),
            );
            Some(DeviceHealthSummary {
                node_id: heartbeat.node_id.clone(),
                device_display_name: telemetry.device_display_name.clone(),
                os: Some(telemetry.os.clone()),
                arch: Some(telemetry.arch.clone()),
                storage_available_bytes: telemetry
                    .storage
                    .as_ref()
                    .and_then(|storage| storage.available_bytes),
                battery_percent: telemetry
                    .battery
                    .as_ref()
                    .and_then(|battery| battery.percent),
                battery_charging: telemetry
                    .battery
                    .as_ref()
                    .and_then(|battery| battery.charging),
                warnings,
            })
        })
        .collect();
    Ok(ClusterReport {
        online_nodes,
        stale_nodes,
        active_leases,
        failed_jobs: 0,
        recent_evidence,
        recent_compute_evidence,
        pending_proposals: 0,
        device_health,
    })
}

pub fn node_statuses(cfg: &Config) -> Result<Vec<ClusterNodeStatus>> {
    init_cluster(cfg)?;
    let nodes = list_nodes(cfg)?;
    nodes
        .into_iter()
        .map(|node| node_status(cfg, &node.node_id))
        .collect()
}

pub fn node_status(cfg: &Config, node_id: &ClusterNodeId) -> Result<ClusterNodeStatus> {
    let paired = paired_node_record(cfg, node_id)?.is_some();
    let approved = paired;
    let heartbeat = latest_heartbeat_for_node(cfg, node_id)?;
    let online = heartbeat
        .as_ref()
        .and_then(|heartbeat| parse_ts(&heartbeat.timestamp).ok())
        .is_some_and(|timestamp| Utc::now() - timestamp <= Duration::seconds(120));
    let stale = !online;
    let lease = active_lease(cfg, node_id).ok();
    let device_telemetry = heartbeat
        .as_ref()
        .and_then(|heartbeat| heartbeat.device_telemetry.clone());
    let telemetry_warnings = device_telemetry
        .as_ref()
        .map(|telemetry| {
            telemetry_warnings(
                node_id.as_str(),
                telemetry,
                &DeviceTelemetryWarningPolicy::default(),
            )
        })
        .unwrap_or_default();
    Ok(ClusterNodeStatus {
        node_id: node_id.clone(),
        paired,
        approved,
        online,
        stale,
        last_heartbeat_at: heartbeat.map(|heartbeat| heartbeat.timestamp),
        active_lease_id: lease.as_ref().map(|lease| lease.lease_id.clone()),
        active_role_id: lease.as_ref().map(|lease| lease.role_id.clone()),
        active_desk_id: lease.as_ref().map(|lease| lease.desk_id.clone()),
        authority: lease
            .as_ref()
            .map(|lease| lease.authority)
            .unwrap_or(LeaseAuthority::Observe),
        execution_enabled: false,
        approval_enabled: false,
        canonical_write_enabled: false,
        jobs_enabled: false,
        device_telemetry,
        telemetry_warnings,
    })
}

pub fn list_leases(cfg: &Config) -> Result<Vec<ClusterLease>> {
    let paths = ClusterPaths::new(cfg);
    Ok(read_json_file::<Vec<ClusterLease>>(&paths.leases)?.unwrap_or_default())
}

pub fn inspect_lease(cfg: &Config, lease_id: &str) -> Result<ClusterLease> {
    list_leases(cfg)?
        .into_iter()
        .find(|lease| lease.lease_id == lease_id)
        .ok_or_else(|| anyhow!("cluster lease '{}' not found", lease_id))
}

pub fn check_lease(cfg: &Config, node_id: &ClusterNodeId) -> Result<LeaseValidationResult> {
    if paired_node_record(cfg, node_id)?.is_none() {
        return Ok(LeaseValidationResult::NodeNotPaired);
    }
    let leases = list_leases(cfg)?;
    let Some(lease) = leases.into_iter().find(|lease| &lease.node_id == node_id) else {
        return Ok(LeaseValidationResult::Missing);
    };
    if lease.revoked || lease.state == ClusterLeaseState::Revoked {
        return Ok(LeaseValidationResult::Revoked);
    }
    if is_expired(&lease.expires_at) || lease.state == ClusterLeaseState::Expired {
        return Ok(LeaseValidationResult::Expired);
    }
    if lease.authority > LeaseAuthority::Observe {
        return Ok(LeaseValidationResult::AuthorityExceedsPairing);
    }
    Ok(LeaseValidationResult::Valid)
}

pub fn revoke_lease(
    cfg: &Config,
    lease_id: Option<&str>,
    node_id: Option<&ClusterNodeId>,
    reason: &str,
) -> Result<ClusterLease> {
    init_cluster(cfg)?;
    let paths = ClusterPaths::new(cfg);
    let mut leases = list_leases(cfg)?;
    let lease = leases
        .iter_mut()
        .find(|lease| {
            lease_id.is_some_and(|id| lease.lease_id == id)
                || node_id.is_some_and(|node_id| &lease.node_id == node_id)
        })
        .ok_or_else(|| anyhow!("cluster lease not found"))?;
    lease.revoked = true;
    lease.revoked_at = Some(now());
    lease.revoked_reason = Some(reason.to_string());
    lease.state = ClusterLeaseState::Revoked;
    let revoked = lease.clone();
    write_json_pretty(&paths.leases, &leases)?;
    append_lease_event(
        &paths,
        "cluster_lease_revoked",
        Some(&revoked.node_id),
        Some(&revoked.lease_id),
        Some(&revoked.desk_id),
        Some(revoked.role_id.as_str()),
        reason,
    )?;
    Ok(revoked)
}

pub fn render_report(report: &ClusterReport) -> String {
    let mut out = format!(
        "cluster report\nonline_nodes: {}\nstale_nodes: {}\nactive_leases: {}\nfailed_jobs: {}\nrecent_evidence: {}\nrecent_compute_evidence: {}\npending_proposals: {}\n",
        report.online_nodes.len(),
        report.stale_nodes.len(),
        report.active_leases.len(),
        report.failed_jobs,
        report.recent_evidence.len(),
        report.recent_compute_evidence.len(),
        report.pending_proposals
    );
    if !report.device_health.is_empty() {
        out.push_str("Device health:\n");
        for health in &report.device_health {
            let storage = device_telemetry::format_bytes(health.storage_available_bytes);
            let battery = health
                .battery_percent
                .map(|percent| {
                    format!(
                        "{percent:.0}%{}",
                        if health.battery_charging.unwrap_or(false) {
                            " charging"
                        } else {
                            ""
                        }
                    )
                })
                .unwrap_or_else(|| "unknown".to_string());
            out.push_str(&format!(
                "- {} device={} os={} arch={} battery={} storage_available={} status=ok\n",
                health.node_id,
                health.device_display_name.as_deref().unwrap_or("unknown"),
                health.os.as_deref().unwrap_or("unknown"),
                health.arch.as_deref().unwrap_or("unknown"),
                battery,
                storage,
            ));
            for warning in &health.warnings {
                out.push_str(warning);
                out.push('\n');
            }
        }
    }
    out
}

fn recent_compute_evidence(evidence_dir: &Path) -> Result<Vec<String>> {
    let mut summaries = Vec::new();
    let Some(entries) = fs::read_dir(evidence_dir).ok() else {
        return Ok(summaries);
    };
    for entry in entries.flatten().take(25) {
        let raw = match fs::read_to_string(entry.path()) {
            Ok(raw) => raw,
            Err(_) => continue,
        };
        let artifact: Value = match serde_json::from_str(&raw) {
            Ok(value) => value,
            Err(_) => continue,
        };
        let Some(output_raw) = artifact.get("output").and_then(Value::as_str) else {
            continue;
        };
        let Ok(output) = serde_json::from_str::<Value>(output_raw) else {
            continue;
        };
        let Some(meta) = output.get("compute_meta") else {
            continue;
        };
        let workload = output
            .get("workload")
            .and_then(Value::as_str)
            .unwrap_or("compute");
        let backend_used = meta
            .get("backend_used")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        let validation = meta
            .get("validation_outcome")
            .and_then(Value::as_str)
            .unwrap_or("unknown");
        summaries.push(format!(
            "{workload}: backend_used={backend_used} validation={validation} authority=none"
        ));
        if summaries.len() >= 10 {
            break;
        }
    }
    Ok(summaries)
}

fn sanitize_device_telemetry(
    telemetry: Option<DeviceTelemetry>,
) -> Result<Option<DeviceTelemetry>> {
    let Some(telemetry) = telemetry else {
        return Ok(None);
    };
    let raw = serde_json::to_string(&telemetry)?;
    if raw.len() > 16 * 1024 {
        return Err(anyhow!("device telemetry payload exceeds size limit"));
    }
    let lowered = raw.to_ascii_lowercase();
    for forbidden in [
        "api_key",
        "apikey",
        "secret",
        "broker",
        "exchange_key",
        "sportsbook",
        "execution_enabled",
        "approval_enabled",
        "canonical_write_enabled",
        "lease_id",
        "role_override",
    ] {
        if lowered.contains(forbidden) {
            return Err(anyhow!(
                "device telemetry contains forbidden authority or credential claim"
            ));
        }
    }
    Ok(Some(telemetry))
}

pub fn edge_device_options() -> Vec<ClusterDeviceOption> {
    vec![
        ClusterDeviceOption {
            profile_id: "raspberry_pi3_dietpi_core".to_string(),
            display_name: "Raspberry Pi 3 DietPi core authority".to_string(),
            authority: ClusterDeviceAuthority::CoreAuthority,
            can_be_execution_leader: true,
            surface: ClusterSurfaceKind::StaffOsWorkspace,
            runtime_profile: "armv7-dietpi-core".to_string(),
            transports: vec![
                transport(
                    ClusterTransportMode::LocalFile,
                    ClusterTransportStatus::AvailableNow,
                    "authoritative workspace/state/cluster store on the core node",
                ),
                transport(
                    ClusterTransportMode::SshLan,
                    ClusterTransportStatus::Planned,
                    "LAN SSH control plane for polling child queues and receipts",
                ),
            ],
            default_capabilities: normalize_capabilities(vec![
                ClusterCapability::Heartbeat,
                ClusterCapability::QueueReadWrite,
                ClusterCapability::EvidenceWrite,
                ClusterCapability::RoleDisplay,
                ClusterCapability::CapabilityReport,
                ClusterCapability::PythonAvailable,
            ]),
            recommended_roles: vec![role_id("generic_evidence_collector")],
            notes: "single execution leader; owns leases, queues, evidence receipts, and policy gates"
                .to_string(),
        },
        ClusterDeviceOption {
            profile_id: "android_termux_phone".to_string(),
            display_name: "Android phone via Termux".to_string(),
            authority: ClusterDeviceAuthority::ChildEvidenceWorker,
            can_be_execution_leader: false,
            surface: ClusterSurfaceKind::TermuxWorker,
            runtime_profile: "termux-edge-worker".to_string(),
            transports: vec![
                transport(
                    ClusterTransportMode::ManualSync,
                    ClusterTransportStatus::AvailableNow,
                    "copy local-file jobs and receipts during early demos",
                ),
                transport(
                    ClusterTransportMode::TermuxSsh,
                    ClusterTransportStatus::Planned,
                    "Termux sshd worker endpoint on trusted LAN",
                ),
            ],
            default_capabilities: normalize_capabilities(vec![
                ClusterCapability::Echo,
                ClusterCapability::Sleep,
                ClusterCapability::Heartbeat,
                ClusterCapability::EvidenceWrite,
                ClusterCapability::Termux,
                ClusterCapability::BatteryStatus,
                ClusterCapability::NetworkStatus,
            ]),
            recommended_roles: vec![role_id("forex_calendar_watcher"), role_id("generic_evidence_collector")],
            notes: "battery-aware child node; reports observations and never promotes execution"
                .to_string(),
        },
        ClusterDeviceOption {
            profile_id: "android_termux_tablet".to_string(),
            display_name: "Android tablet via Termux".to_string(),
            authority: ClusterDeviceAuthority::ChildEvidenceWorker,
            can_be_execution_leader: false,
            surface: ClusterSurfaceKind::TermuxWorker,
            runtime_profile: "termux-tablet-worker".to_string(),
            transports: vec![
                transport(
                    ClusterTransportMode::ManualSync,
                    ClusterTransportStatus::AvailableNow,
                    "local-file queue and evidence exchange for demo use",
                ),
                transport(
                    ClusterTransportMode::TermuxSsh,
                    ClusterTransportStatus::Planned,
                    "LAN SSH worker endpoint once remote transport is implemented",
                ),
            ],
            default_capabilities: normalize_capabilities(vec![
                ClusterCapability::Echo,
                ClusterCapability::Sleep,
                ClusterCapability::Heartbeat,
                ClusterCapability::EvidenceWrite,
                ClusterCapability::RoleDisplay,
                ClusterCapability::Termux,
                ClusterCapability::BatteryStatus,
                ClusterCapability::NetworkStatus,
            ]),
            recommended_roles: vec![role_id("forex_calendar_watcher"), role_id("sports_scout")],
            notes: "larger-screen child worker for watchlists, screenshots, and manual review"
                .to_string(),
        },
        ClusterDeviceOption {
            profile_id: "raspberry_pi_edge_worker".to_string(),
            display_name: "Raspberry Pi LAN edge worker".to_string(),
            authority: ClusterDeviceAuthority::ChildEvidenceWorker,
            can_be_execution_leader: false,
            surface: ClusterSurfaceKind::LocalWorker,
            runtime_profile: "armv7-dietpi-edge-worker".to_string(),
            transports: vec![
                transport(
                    ClusterTransportMode::ManualSync,
                    ClusterTransportStatus::AvailableNow,
                    "local-file worker queue for first bench tests",
                ),
                transport(
                    ClusterTransportMode::SshLan,
                    ClusterTransportStatus::Planned,
                    "trusted LAN SSH worker connection from the core authority",
                ),
            ],
            default_capabilities: normalize_capabilities(vec![
                ClusterCapability::Echo,
                ClusterCapability::Sleep,
                ClusterCapability::Heartbeat,
                ClusterCapability::EvidenceWrite,
                ClusterCapability::NetworkStatus,
                ClusterCapability::PythonAvailable,
            ]),
            recommended_roles: vec![role_id("forex_calendar_watcher"), role_id("bitcoin_dca_monitor")],
            notes: "always-on child node suited for polling and paper evidence capture".to_string(),
        },
        ClusterDeviceOption {
            profile_id: "linux_laptop_edge_worker".to_string(),
            display_name: "Linux laptop or mini PC edge worker".to_string(),
            authority: ClusterDeviceAuthority::ChildEvidenceWorker,
            can_be_execution_leader: false,
            surface: ClusterSurfaceKind::LocalWorker,
            runtime_profile: "linux-edge-worker".to_string(),
            transports: vec![
                transport(
                    ClusterTransportMode::LocalFile,
                    ClusterTransportStatus::AvailableNow,
                    "same-machine or mounted workspace worker mode",
                ),
                transport(
                    ClusterTransportMode::SshLan,
                    ClusterTransportStatus::Planned,
                    "trusted LAN SSH worker connection",
                ),
                transport(
                    ClusterTransportMode::FutureHttpPull,
                    ClusterTransportStatus::Planned,
                    "future pull-only API transport with signed envelopes",
                ),
            ],
            default_capabilities: normalize_capabilities(vec![
                ClusterCapability::Echo,
                ClusterCapability::Sleep,
                ClusterCapability::Heartbeat,
                ClusterCapability::QueueReadWrite,
                ClusterCapability::EvidenceWrite,
                ClusterCapability::CapabilityReport,
                ClusterCapability::PythonAvailable,
                ClusterCapability::OllamaAvailable,
            ]),
            recommended_roles: vec![role_id("browser_research_worker"), role_id("generic_evidence_collector")],
            notes: "heavier child worker for local models or browser-assisted evidence; still non-authoritative"
                .to_string(),
        },
    ]
}

pub fn render_edge_device_options(options: &[ClusterDeviceOption]) -> String {
    let mut out = String::from("cluster edge device options\n");
    for option in options {
        let transports = option
            .transports
            .iter()
            .map(|transport| format!("{}:{}", transport.mode, transport.status))
            .collect::<Vec<_>>()
            .join(",");
        let roles = option
            .recommended_roles
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        let capabilities = option
            .default_capabilities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",");
        out.push_str(&format!(
            "{} authority={} execution_leader={} surface={} transports={} roles={} capabilities={}\n",
            option.profile_id,
            option.authority,
            option.can_be_execution_leader,
            option.surface,
            transports,
            roles,
            capabilities
        ));
    }
    out
}

pub fn desk_rails() -> Vec<ClusterDeskRail> {
    vec![
        ClusterDeskRail {
            rail_id: "forex_carry_rollover_positive_swap".to_string(),
            desk_id: "forex".to_string(),
            display_name: "Forex carry rollover positive-swap rail".to_string(),
            knowledge_packs: vec![
                "lead_coach/market_structure".to_string(),
                "lead_coach/asymmetric_risk_language".to_string(),
                "forex/carry_rollover".to_string(),
                "forex/broker_swap_terms".to_string(),
            ],
            fsm_states: default_desk_fsm_states(),
            technical_language: vec![
                "positive_swap_direction".to_string(),
                "discount_entry_zone".to_string(),
                "support_retest_or_mean_reversion_area".to_string(),
                "safe_margin_distance".to_string(),
                "rollover_carry_evidence".to_string(),
            ],
            fundamental_language: vec![
                "central_bank_rate_differential".to_string(),
                "macro_event_blackout".to_string(),
                "broker_swap_schedule_verified".to_string(),
                "spread_and_rollover_cost_checked".to_string(),
            ],
            non_negotiable_rules: vec![
                "paper/research only until core policy and operator approval explicitly allow otherwise"
                    .to_string(),
                "trade only the broker-verified positive swap direction".to_string(),
                "USDJPY long-only is an example only when current broker swap data verifies long is positive"
                    .to_string(),
                "entry candidate must be at a defined discount zone, not at premium/chase pricing"
                    .to_string(),
                "starting entry size is 0.01 until accountant scale approval changes the rail"
                    .to_string(),
                "maximum two open paper trades per currency pair".to_string(),
                "maximum three currency pairs under this carry rail at one time".to_string(),
            ],
            criteria: vec![
                criterion(
                    "swap_direction_verified",
                    "positive swap direction is verified from configured broker terms",
                    true,
                    "broker swap table or captured platform evidence",
                ),
                criterion(
                    "discount_entry_verified",
                    "candidate entry sits inside the desk-defined discount zone",
                    true,
                    "price relative to prior range, support, mean, or policy-defined value area",
                ),
                criterion(
                    "macro_blackout_clear",
                    "no high-impact macro event blocks entry review",
                    true,
                    "calendar evidence from an approved knowledge pack/source",
                ),
                criterion(
                    "safe_margin_box",
                    "margin use remains inside safe paper-risk limits",
                    true,
                    "accountant rail calculation and open-position count",
                ),
            ],
            risk_limits: ClusterDeskRiskLimits {
                starting_unit: "0.01 paper lot".to_string(),
                max_open_positions_or_slips: 2,
                max_parallel_instruments_or_events: 3,
                safe_margin_required: true,
                live_execution_allowed: false,
            },
            forbidden_actions: default_forbidden_actions(),
        },
        ClusterDeskRail {
            rail_id: "sports_major_event_probability_scout".to_string(),
            desk_id: "sports".to_string(),
            display_name: "Sports major-event probability scout rail".to_string(),
            knowledge_packs: vec![
                "lead_coach/event_attention".to_string(),
                "sports/market_baselines".to_string(),
                "sports/injury_and_line_movement".to_string(),
            ],
            fsm_states: default_desk_fsm_states(),
            technical_language: vec![
                "implied_probability".to_string(),
                "book_price_vs_model_price".to_string(),
                "closing_line_value_hypothesis".to_string(),
                "liquidity_and_attention_score".to_string(),
                "variance_bucket".to_string(),
            ],
            fundamental_language: vec![
                "major_event_calendar".to_string(),
                "NBA playoffs".to_string(),
                "FIFA World Cup".to_string(),
                "Super Bowl".to_string(),
                "injury_report".to_string(),
                "participant_motivation".to_string(),
                "public_trend_pressure".to_string(),
            ],
            non_negotiable_rules: vec![
                "paper/research only; no wager placement".to_string(),
                "scout only events with enough market attention to audit price movement".to_string(),
                "no candidate advances without participant/news baseline evidence".to_string(),
                "no candidate advances on popularity alone; price edge evidence is required"
                    .to_string(),
            ],
            criteria: vec![
                criterion(
                    "event_attention_verified",
                    "event is high-attention or strategically relevant to the study queue",
                    true,
                    "calendar, search/trend, schedule, or liquidity evidence",
                ),
                criterion(
                    "baseline_available",
                    "teams/participants have enough historical and current-context evidence",
                    true,
                    "records, injuries, roster/news, travel/rest, matchup notes",
                ),
                criterion(
                    "price_edge_hypothesis",
                    "book price and model baseline disagree enough to justify paper tracking",
                    true,
                    "implied probability comparison and line movement note",
                ),
            ],
            risk_limits: ClusterDeskRiskLimits {
                starting_unit: "paper slip".to_string(),
                max_open_positions_or_slips: 2,
                max_parallel_instruments_or_events: 3,
                safe_margin_required: true,
                live_execution_allowed: false,
            },
            forbidden_actions: default_forbidden_actions(),
        },
        ClusterDeskRail {
            rail_id: "crypto_dca_and_peg_monitor".to_string(),
            desk_id: "crypto".to_string(),
            display_name: "Crypto DCA and peg-monitor rail".to_string(),
            knowledge_packs: vec![
                "lead_coach/regime_filter".to_string(),
                "crypto/bitcoin_dca".to_string(),
                "crypto/stablecoin_peg".to_string(),
            ],
            fsm_states: default_desk_fsm_states(),
            technical_language: vec![
                "regime_filter".to_string(),
                "drawdown_band".to_string(),
                "peg_deviation".to_string(),
                "liquidity_depth".to_string(),
            ],
            fundamental_language: vec![
                "exchange_risk".to_string(),
                "custody_risk".to_string(),
                "issuer_news".to_string(),
                "macro_liquidity_context".to_string(),
            ],
            non_negotiable_rules: vec![
                "paper/research only; no exchange order placement".to_string(),
                "stablecoin peg alerts are evidence, not automatic buy/sell instructions".to_string(),
            ],
            criteria: vec![
                criterion(
                    "regime_context_present",
                    "market regime label exists before a paper DCA note advances",
                    true,
                    "drawdown, volatility, and liquidity evidence",
                ),
                criterion(
                    "peg_deviation_explained",
                    "stablecoin peg movement has source/context evidence",
                    true,
                    "price deviation, venue, and issuer/news note",
                ),
            ],
            risk_limits: ClusterDeskRiskLimits {
                starting_unit: "paper unit".to_string(),
                max_open_positions_or_slips: 2,
                max_parallel_instruments_or_events: 3,
                safe_margin_required: true,
                live_execution_allowed: false,
            },
            forbidden_actions: default_forbidden_actions(),
        },
        ClusterDeskRail {
            rail_id: "stocks_options_index_session".to_string(),
            desk_id: "stocks_options".to_string(),
            display_name: "Stock index session watcher rail".to_string(),
            knowledge_packs: vec![
                "lead_coach/session_context".to_string(),
                "stocks/index_market_structure".to_string(),
                "options/volatility_language".to_string(),
            ],
            fsm_states: default_desk_fsm_states(),
            technical_language: vec![
                "session_open_range".to_string(),
                "VWAP_context".to_string(),
                "expected_move".to_string(),
                "implied_volatility".to_string(),
                "liquidity_window".to_string(),
            ],
            fundamental_language: vec![
                "earnings_calendar".to_string(),
                "FOMC_or_CPI_blackout".to_string(),
                "sector_breadth".to_string(),
                "risk_on_risk_off_context".to_string(),
            ],
            non_negotiable_rules: vec![
                "paper/research only; no stock or option order placement".to_string(),
                "no candidate advances during undefined event-risk blackout".to_string(),
            ],
            criteria: vec![
                criterion(
                    "session_structure_defined",
                    "session context and liquidity window are defined",
                    true,
                    "open range, VWAP, breadth, and volatility notes",
                ),
                criterion(
                    "event_risk_checked",
                    "earnings/macro event risk is checked before paper plan",
                    true,
                    "calendar and news evidence",
                ),
            ],
            risk_limits: ClusterDeskRiskLimits {
                starting_unit: "paper contract/share unit".to_string(),
                max_open_positions_or_slips: 2,
                max_parallel_instruments_or_events: 3,
                safe_margin_required: true,
                live_execution_allowed: false,
            },
            forbidden_actions: default_forbidden_actions(),
        },
        ClusterDeskRail {
            rail_id: "prediction_market_event_research".to_string(),
            desk_id: "prediction_markets".to_string(),
            display_name: "Prediction-market event research rail".to_string(),
            knowledge_packs: vec![
                "lead_coach/probability_language".to_string(),
                "prediction_markets/event_resolution".to_string(),
                "prediction_markets/liquidity_and_rules".to_string(),
            ],
            fsm_states: default_desk_fsm_states(),
            technical_language: vec![
                "market_probability".to_string(),
                "resolution_criteria".to_string(),
                "liquidity_depth".to_string(),
                "spread_width".to_string(),
            ],
            fundamental_language: vec![
                "source_reliability".to_string(),
                "event_timeline".to_string(),
                "rule_interpretation".to_string(),
                "information_asymmetry_note".to_string(),
            ],
            non_negotiable_rules: vec![
                "paper/research only; no contract purchase or sale".to_string(),
                "no candidate advances without clear resolution criteria".to_string(),
            ],
            criteria: vec![
                criterion(
                    "resolution_rules_clear",
                    "market resolution rules are captured and unambiguous",
                    true,
                    "market rule text and source notes",
                ),
                criterion(
                    "probability_gap_explained",
                    "probability gap has evidence beyond opinion",
                    true,
                    "source timeline and market price comparison",
                ),
            ],
            risk_limits: ClusterDeskRiskLimits {
                starting_unit: "paper contract".to_string(),
                max_open_positions_or_slips: 2,
                max_parallel_instruments_or_events: 3,
                safe_margin_required: true,
                live_execution_allowed: false,
            },
            forbidden_actions: default_forbidden_actions(),
        },
    ]
}

pub fn render_desk_rails(rails: &[ClusterDeskRail]) -> String {
    let mut out = String::from("cluster desk rails\n");
    for rail in rails {
        let states = rail
            .fsm_states
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(">");
        out.push_str(&format!(
            "{} desk={} states={} live_execution_allowed={} criteria={} knowledge_packs={}\n",
            rail.rail_id,
            rail.desk_id,
            states,
            rail.risk_limits.live_execution_allowed,
            rail.criteria.len(),
            rail.knowledge_packs.join(",")
        ));
    }
    out
}

pub fn render_node_statuses(statuses: &[ClusterNodeStatus]) -> String {
    let mut out = String::new();
    for status in statuses {
        let (device, os, arch, storage, battery) =
            if let Some(telemetry) = status.device_telemetry.as_ref() {
                (
                    telemetry
                        .device_display_name
                        .as_deref()
                        .unwrap_or("unknown")
                        .to_string(),
                    telemetry.os.clone(),
                    telemetry.arch.clone(),
                    device_telemetry::format_bytes(
                        telemetry
                            .storage
                            .as_ref()
                            .and_then(|storage| storage.available_bytes),
                    ),
                    device_telemetry::format_battery(telemetry.battery.as_ref()),
                )
            } else {
                (
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                    "unknown".to_string(),
                )
            };
        out.push_str(&format!(
            "{} paired={} approved={} online={} stale={} device={} os={} arch={} battery={} storage_available={} role={} lease={} authority={} execution={} approval={} canonical_write={} jobs_enabled={}\n",
            status.node_id,
            status.paired,
            status.approved,
            status.online,
            status.stale,
            device,
            os,
            arch,
            battery,
            storage,
            status
                .active_role_id
                .as_ref()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
            status.active_lease_id.as_deref().unwrap_or("none"),
            status.authority,
            status.execution_enabled,
            status.approval_enabled,
            status.canonical_write_enabled,
            status.jobs_enabled,
        ));
        for warning in &status.telemetry_warnings {
            out.push_str(warning);
            out.push('\n');
        }
    }
    out
}

pub fn render_leases(leases: &[ClusterLease]) -> String {
    let mut out = String::new();
    for lease in leases {
        let state = if lease.revoked {
            "revoked"
        } else if is_expired(&lease.expires_at) {
            "expired"
        } else {
            "active"
        };
        out.push_str(&format!(
            "{} node={} desk={} role={} authority={} {} expires_at={} execution=false approval=false canonical_write=false jobs_enabled=false\n",
            lease.lease_id,
            lease.node_id,
            lease.desk_id,
            lease.role_id,
            lease.authority,
            state,
            lease.expires_at
        ));
    }
    out
}

pub fn parse_capabilities(value: &str) -> Result<Vec<ClusterCapability>> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::parse)
        .collect()
}

pub fn parse_ttl(value: &str) -> Result<Duration> {
    let trimmed = value.trim();
    let (number, unit) = trimmed.split_at(trimmed.len().saturating_sub(1));
    let amount = number
        .parse::<i64>()
        .with_context(|| format!("invalid ttl '{}'", value))?;
    match unit {
        "s" => Ok(Duration::seconds(amount)),
        "m" => Ok(Duration::minutes(amount)),
        "h" => Ok(Duration::hours(amount)),
        _ => Err(anyhow!(
            "invalid ttl '{}'; expected suffix s, m, or h",
            value
        )),
    }
}

fn validate_job(cfg: &Config, job: &ClusterJobEnvelope, role: &ClusterRole) -> Result<()> {
    if !is_leased_child_job_enabled(job.job_kind) {
        return Err(anyhow!(
            "only echo and scalar compute evidence jobs are enabled for leased children in this checkpoint"
        ));
    }
    if is_expired(&job.expires_at) {
        return Err(anyhow!("cluster job '{}' expired", job.job_id));
    }
    ensure_approved_paired_node(cfg, &job.node_id)?;
    let lease = active_lease(cfg, &job.node_id)?;
    if lease.authority != LeaseAuthority::Observe {
        return Err(anyhow!("cluster job requires observe-only lease"));
    }
    if lease.lease_id != job.lease_id || lease.role_id != job.role_id {
        return Err(anyhow!(
            "cluster job lease does not match active node lease"
        ));
    }
    if lease.playbook_hash != job.playbook_hash {
        return Err(anyhow!(
            "cluster job playbook hash does not match active lease"
        ));
    }
    let status = node_status(cfg, &job.node_id)?;
    if !status.online || status.stale {
        return Err(anyhow!(
            "cluster leased child job requires fresh child heartbeat"
        ));
    }
    if role.role_id != job.role_id || role.desk_id != job.desk_id {
        return Err(anyhow!("cluster job role/desk mismatch"));
    }
    if job.authority_level > role.max_authority {
        return Err(anyhow!("cluster job authority exceeds role authority"));
    }
    let node = find_node(cfg, &job.node_id)?;
    let node_caps: BTreeSet<_> = node.capabilities.iter().copied().collect();
    let role_caps: BTreeSet<_> = role.allowed_capabilities.iter().copied().collect();
    for required in &job.required_capabilities {
        if !node_caps.contains(required) || !role_caps.contains(required) {
            return Err(anyhow!(
                "cluster job requires unavailable capability '{}'",
                required
            ));
        }
    }
    if matches!(job.job_kind, ClusterJobKind::HttpGet) && !cfg.worker.allow_http_get {
        return Err(anyhow!("cluster http_get jobs are disabled by config"));
    }
    let timing = timing::check_timing(
        cfg,
        timing::TimingCheckRequest {
            desk_id: DeskId::new(role.desk_id.clone()),
            role_id: Some(role.role_id.to_string()),
            child_node_id: Some(job.node_id.to_string()),
            trigger: None,
            evidence_timestamp: Some(Utc::now()),
            proposal_requested: false,
        },
    )?;
    if timing.decision != timing::TimingFsmDecision::AllowEvaluation {
        return Err(anyhow!("cluster leased child job blocked by timing gate"));
    }
    if is_compute_job(job.job_kind) {
        let compute = job
            .compute_requirement
            .as_ref()
            .ok_or_else(|| anyhow!("cluster compute job is missing compute requirement"))?;
        ensure_allowed_compute_fixture(job.job_kind, &compute.fixture)?;
        if is_desk_observe_job(job.job_kind) {
            ensure_desk_observe_payload_is_evidence_only(&job.payload)?;
        }
        if compute.backend_requested != ComputeBackend::Scalar {
            if compute::backend_is_quarantined(
                cfg,
                job.node_id.as_str(),
                compute.backend_requested,
            )? {
                return Err(anyhow!("cluster compute backend is quarantined for node"));
            }
            return Err(anyhow!(
                "cluster compute jobs allow scalar backend only in this checkpoint"
            ));
        }
        if latest_heartbeat_for_node(cfg, &job.node_id)?
            .filter(|heartbeat| {
                parse_ts(&heartbeat.timestamp)
                    .is_ok_and(|ts| Utc::now() - ts < Duration::minutes(5))
            })
            .is_none()
        {
            return Err(anyhow!(
                "cluster compute job requires fresh child heartbeat"
            ));
        }
        let timing = timing::check_timing(
            cfg,
            timing::TimingCheckRequest {
                desk_id: DeskId::new(role.desk_id.clone()),
                role_id: Some(role.role_id.to_string()),
                child_node_id: Some(job.node_id.to_string()),
                trigger: None,
                evidence_timestamp: Some(Utc::now()),
                proposal_requested: false,
            },
        )?;
        if timing.decision != timing::TimingFsmDecision::AllowEvaluation {
            return Err(anyhow!("cluster compute job blocked by timing gate"));
        }
    }
    Ok(())
}

fn is_leased_child_job_enabled(job_kind: ClusterJobKind) -> bool {
    matches!(
        job_kind,
        ClusterJobKind::Echo
            | ClusterJobKind::ComputeFreshnessScan
            | ClusterJobKind::ComputePegDeviation
            | ClusterJobKind::DeskObserveEvidenceFreshness
            | ClusterJobKind::DeskObservePegDeviation
    )
}

fn ensure_allowed_compute_fixture(job_kind: ClusterJobKind, fixture: &str) -> Result<()> {
    let fixture = fixture.trim();
    let allowed = match job_kind {
        ClusterJobKind::ComputeFreshnessScan | ClusterJobKind::DeskObserveEvidenceFreshness => {
            matches!(fixture, "evidence_freshness" | "evidence_freshness_scan")
        }
        ClusterJobKind::ComputePegDeviation | ClusterJobKind::DeskObservePegDeviation => matches!(
            fixture,
            "stablecoin_peg" | "stablecoin_peg_deviation" | "boundary_ambiguous_peg_scan"
        ),
        _ => true,
    };
    if allowed {
        Ok(())
    } else {
        Err(anyhow!(
            "cluster compute fixture '{}' is not enabled for leased child scalar evidence jobs",
            fixture
        ))
    }
}

fn ensure_desk_observe_payload_is_evidence_only(payload: &Value) -> Result<()> {
    let raw = serde_json::to_string(payload)?;
    let lower = raw.to_ascii_lowercase();
    for forbidden in [
        "provider",
        "http_get",
        "shell",
        "net_edge",
        "net edge",
        "arbitrage",
        "arb",
        "proposal",
        "approve",
        "canonical",
        "execution",
        "trade",
        "bet",
        "buy",
        "sell",
    ] {
        if lower.contains(forbidden) {
            return Err(anyhow!(
                "desk observe evidence payload contains forbidden intent '{}'",
                forbidden
            ));
        }
    }
    Ok(())
}

fn execute_cluster_job(job: &ClusterJobEnvelope) -> Result<String> {
    match job.job_kind {
        ClusterJobKind::Echo => Ok(job
            .payload
            .get("text")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()),
        ClusterJobKind::Sleep => {
            let millis = job
                .payload
                .get("millis")
                .and_then(Value::as_u64)
                .unwrap_or(1)
                .min(1_000);
            std::thread::sleep(std::time::Duration::from_millis(millis));
            Ok(format!("slept for {millis}ms"))
        }
        ClusterJobKind::HttpGet => Err(anyhow!(
            "http_get execution is not implemented in cluster child mode"
        )),
        ClusterJobKind::ComputeFreshnessScan => execute_compute_freshness(job),
        ClusterJobKind::ComputePegDeviation => execute_compute_peg_deviation(job),
        ClusterJobKind::DeskObserveEvidenceFreshness => execute_desk_observe_freshness(job),
        ClusterJobKind::DeskObservePegDeviation => execute_desk_observe_peg_deviation(job),
    }
}

fn build_compute_requirement(
    job_kind: ClusterJobKind,
    payload: &Value,
) -> Result<Option<ClusterComputeRequirement>> {
    if !is_compute_job(job_kind) {
        return Ok(None);
    }
    let fixture = payload
        .get("fixture")
        .and_then(Value::as_str)
        .unwrap_or(match job_kind {
            ClusterJobKind::ComputeFreshnessScan | ClusterJobKind::DeskObserveEvidenceFreshness => {
                "evidence_freshness"
            }
            ClusterJobKind::ComputePegDeviation | ClusterJobKind::DeskObservePegDeviation => {
                "stablecoin_peg_deviation"
            }
            _ => unreachable!("guarded by is_compute_job"),
        })
        .to_string();
    let backend = payload
        .get("backend")
        .and_then(Value::as_str)
        .unwrap_or("scalar")
        .parse::<ComputeBackend>()?;
    Ok(Some(ClusterComputeRequirement {
        fixture,
        backend_requested: backend,
        timing_policy_hash: "timing-policy-scalar-compute-v1".to_string(),
    }))
}

fn is_compute_job(job_kind: ClusterJobKind) -> bool {
    matches!(
        job_kind,
        ClusterJobKind::ComputeFreshnessScan
            | ClusterJobKind::ComputePegDeviation
            | ClusterJobKind::DeskObserveEvidenceFreshness
            | ClusterJobKind::DeskObservePegDeviation
    )
}

fn is_desk_observe_job(job_kind: ClusterJobKind) -> bool {
    matches!(
        job_kind,
        ClusterJobKind::DeskObserveEvidenceFreshness | ClusterJobKind::DeskObservePegDeviation
    )
}

fn execute_compute_freshness(job: &ClusterJobEnvelope) -> Result<String> {
    let compute = job
        .compute_requirement
        .as_ref()
        .ok_or_else(|| anyhow!("missing compute requirement"))?;
    let input = compute::fixtures::evidence_freshness_fixture(&compute.fixture)?;
    let output = compute::evidence_freshness_scan(&input)?;
    let input_hash = stable_hash(&serde_json::to_string(&input)?);
    let output_raw = serde_json::to_string(&output)?;
    let output_hash = stable_hash(&output_raw);
    Ok(serde_json::to_string(&serde_json::json!({
        "workload": "evidence_freshness",
        "output": output,
        "compute_meta": compute_meta(job, compute, input_hash, output_hash, compute::boundary::NumericConfidence::Exact),
        "non_authoritative": true,
        "proposal_created": false
    }))?)
}

fn execute_compute_peg_deviation(job: &ClusterJobEnvelope) -> Result<String> {
    let compute = job
        .compute_requirement
        .as_ref()
        .ok_or_else(|| anyhow!("missing compute requirement"))?;
    let input = compute::fixtures::peg_deviation_fixture(&compute.fixture)?;
    let output = compute::peg_deviation_scan(&input, 10.0, 0.01)?;
    let input_hash = stable_hash(&serde_json::to_string(&input)?);
    let output_raw = serde_json::to_string(&output)?;
    let output_hash = stable_hash(&output_raw);
    let confidence = output.numeric_confidence;
    Ok(serde_json::to_string(&serde_json::json!({
        "workload": "peg_deviation",
        "output": output,
        "compute_meta": compute_meta(job, compute, input_hash, output_hash, confidence),
        "non_authoritative": true,
        "proposal_created": false
    }))?)
}

fn execute_desk_observe_freshness(job: &ClusterJobEnvelope) -> Result<String> {
    let compute = job
        .compute_requirement
        .as_ref()
        .ok_or_else(|| anyhow!("missing compute requirement"))?;
    let input = compute::fixtures::evidence_freshness_fixture(&compute.fixture)?;
    let output = compute::evidence_freshness_scan(&input)?;
    let input_hash = stable_hash(&serde_json::to_string(&input)?);
    let output_raw = serde_json::to_string(&output)?;
    let output_hash = stable_hash(&output_raw);
    let meta = compute_meta(
        job,
        compute,
        input_hash.clone(),
        output_hash.clone(),
        compute::boundary::NumericConfidence::Exact,
    );
    let observation = desk_observation_evidence(
        job,
        DeskEvidenceKind::EvidenceFreshnessObservation,
        Some(meta.clone()),
        Some(compute::boundary::NumericConfidence::Exact),
        input_hash,
        output_hash,
    );
    Ok(serde_json::to_string(&serde_json::json!({
        "workload": "desk_observe_evidence_freshness",
        "output": output,
        "compute_meta": meta,
        "desk_observation": observation,
        "non_authoritative": true,
        "proposal_created": false
    }))?)
}

fn execute_desk_observe_peg_deviation(job: &ClusterJobEnvelope) -> Result<String> {
    let compute = job
        .compute_requirement
        .as_ref()
        .ok_or_else(|| anyhow!("missing compute requirement"))?;
    let input = compute::fixtures::peg_deviation_fixture(&compute.fixture)?;
    let output = compute::peg_deviation_scan(&input, 10.0, 0.01)?;
    let input_hash = stable_hash(&serde_json::to_string(&input)?);
    let output_raw = serde_json::to_string(&output)?;
    let output_hash = stable_hash(&output_raw);
    let confidence = output.numeric_confidence;
    let meta = compute_meta(
        job,
        compute,
        input_hash.clone(),
        output_hash.clone(),
        confidence,
    );
    let observation = desk_observation_evidence(
        job,
        DeskEvidenceKind::StablecoinPegDeviationObservation,
        Some(meta.clone()),
        Some(confidence),
        input_hash,
        output_hash,
    );
    Ok(serde_json::to_string(&serde_json::json!({
        "workload": "desk_observe_peg_deviation",
        "output": output,
        "compute_meta": meta,
        "desk_observation": observation,
        "non_authoritative": true,
        "proposal_created": false
    }))?)
}

fn desk_observation_evidence(
    job: &ClusterJobEnvelope,
    evidence_kind: DeskEvidenceKind,
    compute_meta: Option<compute::validation::ComputeEvidenceMeta>,
    numeric_confidence: Option<compute::boundary::NumericConfidence>,
    input_hash: String,
    output_hash: String,
) -> DeskObservationEvidence {
    let timing_decision_id = compute_meta
        .as_ref()
        .and_then(|meta| meta.timing_decision_id.clone())
        .unwrap_or_else(|| format!("cluster-timing-{}-{}", job.node_id, job.job_id));
    DeskObservationEvidence {
        evidence_id: format!("desk-evidence-{}", job.job_id),
        node_id: job.node_id.to_string(),
        lease_id: job.lease_id.clone(),
        desk_id: DeskId::new(job.desk_id.clone()),
        role_id: job.role_id.to_string(),
        knowledge_pack_id: job
            .payload
            .get("knowledge_pack_id")
            .and_then(Value::as_str)
            .map(str::to_string),
        playbook_id: job.playbook_id.clone(),
        playbook_hash: job.playbook_hash.clone(),
        evidence_kind,
        authority: LeaseAuthority::Observe,
        timing_decision_id,
        input_hash,
        output_hash,
        compute_meta,
        numeric_confidence,
        created_at: now(),
        replay_safe: true,
        proposal_created: false,
    }
}

fn compute_meta(
    job: &ClusterJobEnvelope,
    compute: &ClusterComputeRequirement,
    input_hash: String,
    output_hash: String,
    confidence: compute::boundary::NumericConfidence,
) -> compute::validation::ComputeEvidenceMeta {
    compute::validation::ComputeEvidenceMeta {
        backend_requested: compute.backend_requested,
        backend_used: ComputeBackend::Scalar,
        scalar_fallback_used: compute.backend_requested != ComputeBackend::Scalar,
        verified_against_scalar: true,
        validation_outcome: compute::validation::ComputeValidationOutcome::AcceptedScalarOnly,
        numeric_confidence: confidence,
        tolerance: Some(0.000_001),
        threshold_epsilon: Some(0.01),
        runtime_ms: None,
        input_hash,
        output_hash,
        timing_decision_id: Some(format!("cluster-timing-{}-{}", job.node_id, job.job_id)),
    }
}

fn default_roles() -> Result<Vec<ClusterRole>> {
    Ok(vec![
        role(
            "forex_calendar_watcher",
            "forex",
            "Forex calendar watcher",
            ClusterAuthorityLevel::Analyze,
            &[ClusterCapability::Echo, ClusterCapability::Sleep],
        ),
        role(
            "stablecoin_peg_watcher",
            "crypto",
            "Stablecoin peg watcher",
            ClusterAuthorityLevel::Observe,
            &[ClusterCapability::Echo, ClusterCapability::ComputeScalar],
        ),
        role(
            "bitcoin_dca_monitor",
            "crypto",
            "Bitcoin DCA monitor",
            ClusterAuthorityLevel::Analyze,
            &[ClusterCapability::Echo, ClusterCapability::Sleep],
        ),
        role(
            "sports_scout",
            "sports",
            "Sports scout",
            ClusterAuthorityLevel::Analyze,
            &[ClusterCapability::Echo],
        ),
        role(
            "stock_index_session_watcher",
            "stocks_options",
            "Stock index session watcher",
            ClusterAuthorityLevel::Analyze,
            &[ClusterCapability::Echo],
        ),
        role(
            "prediction_market_watcher",
            "prediction_markets",
            "Prediction market watcher",
            ClusterAuthorityLevel::Observe,
            &[ClusterCapability::Echo],
        ),
        role(
            "generic_evidence_collector",
            "research",
            "Generic evidence collector",
            ClusterAuthorityLevel::Observe,
            &[
                ClusterCapability::Echo,
                ClusterCapability::Sleep,
                ClusterCapability::ComputeScalar,
            ],
        ),
        role(
            "browser_research_worker",
            "research",
            "Browser research worker",
            ClusterAuthorityLevel::Observe,
            &[ClusterCapability::Echo, ClusterCapability::HttpGet],
        ),
    ])
}

fn role(
    role_id: &str,
    desk_id: &str,
    display_name: &str,
    max_authority: ClusterAuthorityLevel,
    allowed: &[ClusterCapability],
) -> ClusterRole {
    let mut allowed_capabilities = vec![
        ClusterCapability::Heartbeat,
        ClusterCapability::QueueReadWrite,
        ClusterCapability::EvidenceWrite,
        ClusterCapability::RoleDisplay,
        ClusterCapability::CapabilityReport,
    ];
    allowed_capabilities.extend_from_slice(allowed);
    ClusterRole {
        role_id: ClusterRoleId::new(role_id).expect("static role id"),
        desk_id: desk_id.to_string(),
        display_name: display_name.to_string(),
        max_authority,
        allowed_capabilities: normalize_capabilities(allowed_capabilities),
        description: "paper/evidence-only child role; core remains authority".to_string(),
    }
}

fn role_id(value: &str) -> ClusterRoleId {
    ClusterRoleId::new(value).expect("static role id")
}

fn transport(
    mode: ClusterTransportMode,
    status: ClusterTransportStatus,
    notes: &str,
) -> ClusterTransportOption {
    ClusterTransportOption {
        mode,
        status,
        notes: notes.to_string(),
    }
}

fn criterion(
    criterion_id: &str,
    language: &str,
    required: bool,
    evidence_signal: &str,
) -> ClusterRailCriterion {
    ClusterRailCriterion {
        criterion_id: criterion_id.to_string(),
        language: language.to_string(),
        required,
        evidence_signal: evidence_signal.to_string(),
    }
}

fn default_desk_fsm_states() -> Vec<ClusterDeskFsmState> {
    vec![
        ClusterDeskFsmState::KnowledgePackLoaded,
        ClusterDeskFsmState::CandidateObserved,
        ClusterDeskFsmState::LanguageMatched,
        ClusterDeskFsmState::CriteriaValidated,
        ClusterDeskFsmState::RiskBoxChecked,
        ClusterDeskFsmState::AsymmetryScored,
        ClusterDeskFsmState::PaperPlanReady,
    ]
}

fn default_forbidden_actions() -> Vec<String> {
    vec![
        "live_trade_execution".to_string(),
        "live_bet_placement".to_string(),
        "automatic_order_sizing".to_string(),
        "bypass_core_policy".to_string(),
        "bypass_operator_approval".to_string(),
        "profit_guarantee_claims".to_string(),
    ]
}

fn find_node(cfg: &Config, node_id: &ClusterNodeId) -> Result<ClusterNode> {
    list_nodes(cfg)?
        .into_iter()
        .rev()
        .find(|node| &node.node_id == node_id)
        .ok_or_else(|| anyhow!("cluster node '{}' not found", node_id))
}

fn find_role(cfg: &Config, role_id: &ClusterRoleId) -> Result<ClusterRole> {
    let paths = ClusterPaths::new(cfg);
    read_json_file::<Vec<ClusterRole>>(&paths.roles)?
        .unwrap_or_else(|| default_roles().unwrap_or_default())
        .into_iter()
        .find(|role| &role.role_id == role_id)
        .ok_or_else(|| anyhow!("cluster role '{}' not found", role_id))
}

fn active_lease(cfg: &Config, node_id: &ClusterNodeId) -> Result<ClusterLease> {
    let paths = ClusterPaths::new(cfg);
    read_json_file::<Vec<ClusterLease>>(&paths.leases)?
        .unwrap_or_default()
        .into_iter()
        .find(|lease| {
            &lease.node_id == node_id
                && !lease.revoked
                && lease.state == ClusterLeaseState::Granted
                && !is_expired(&lease.expires_at)
        })
        .ok_or_else(|| anyhow!("no active cluster lease for node '{}'", node_id))
}

fn paired_node_record(
    cfg: &Config,
    node_id: &ClusterNodeId,
) -> Result<Option<pairing::AcceptedPairedNode>> {
    Ok(pairing::list_accepted_nodes(cfg)?
        .into_iter()
        .find(|node| node.node_id == node_id.as_str()))
}

fn ensure_approved_paired_node(cfg: &Config, node_id: &ClusterNodeId) -> Result<()> {
    let Some(node) = paired_node_record(cfg, node_id)? else {
        return Err(anyhow!(
            "cluster node '{}' is not an approved paired child",
            node_id
        ));
    };
    if node.authority_level != PairingAuthority::Observe {
        return Err(anyhow!("paired child authority exceeds observe boundary"));
    }
    if node.execution_enabled || node.approval_enabled || node.canonical_write_enabled {
        return Err(anyhow!("paired child authority flags are not allowed"));
    }
    Ok(())
}

fn latest_heartbeat_for_node(
    cfg: &Config,
    node_id: &ClusterNodeId,
) -> Result<Option<ClusterHeartbeat>> {
    let paths = ClusterPaths::new(cfg);
    Ok(read_jsonl::<ClusterHeartbeat>(&paths.heartbeats)?
        .into_iter()
        .rev()
        .find(|heartbeat| &heartbeat.node_id == node_id))
}

fn queued_jobs_for_node(
    paths: &ClusterPaths,
    node_id: &ClusterNodeId,
) -> Result<Vec<ClusterJobEnvelope>> {
    read_jsonl(&paths.jobs_for_node(node_id))
}

fn is_expired(ts: &str) -> bool {
    parse_ts(ts).map_or(true, |value| value <= Utc::now())
}

fn parse_ts(ts: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(ts)?.with_timezone(&Utc))
}

fn normalize_capabilities(mut capabilities: Vec<ClusterCapability>) -> Vec<ClusterCapability> {
    capabilities.sort();
    capabilities.dedup();
    capabilities
}

fn capability_hash(capabilities: &[ClusterCapability]) -> String {
    stable_hash(
        &capabilities
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

fn slug_id(prefix: &str, value: &str) -> String {
    let slug = value
        .trim()
        .to_ascii_lowercase()
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    format!("{prefix}:{slug}")
}

fn validate_id(label: &str, value: &str) -> Result<()> {
    if value.trim().is_empty() {
        return Err(anyhow!("{label} is empty"));
    }
    if value.contains('/') || value.contains('\\') || value.contains("..") {
        return Err(anyhow!("{label} contains unsafe path characters"));
    }
    Ok(())
}

fn now() -> String {
    Utc::now().to_rfc3339()
}

fn default_operator() -> String {
    "operator".to_string()
}

fn default_replay_safe() -> bool {
    true
}

fn append_event(
    paths: &ClusterPaths,
    event_kind: &str,
    node_id: Option<ClusterNodeId>,
    detail: &str,
) -> Result<()> {
    let event = ClusterEvent {
        event_id: format!(
            "cluster-event-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        event_kind: event_kind.to_string(),
        node_id,
        detail: detail.to_string(),
        occurred_at: now(),
    };
    append_json_line(&paths.events, &event)
}

fn append_lease_event(
    paths: &ClusterPaths,
    kind: &str,
    node_id: Option<&ClusterNodeId>,
    lease_id: Option<&str>,
    desk_id: Option<&str>,
    role_id: Option<&str>,
    reason: &str,
) -> Result<()> {
    let event = ClusterLeaseEventRecord {
        event_id: format!(
            "cluster-lease-event-{}",
            Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ),
        timestamp: now(),
        kind: kind.to_string(),
        node_id: node_id.cloned(),
        lease_id: lease_id.map(str::to_string),
        desk_id: desk_id.map(str::to_string),
        role_id: role_id.map(str::to_string),
        reason: reason.to_string(),
        replay_safe: true,
    };
    append_json_line(&paths.lease_events, &event)
}

fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn write_jsonl<T: Serialize>(path: &Path, values: &[T]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let mut content = String::new();
    for value in values {
        content.push_str(&serde_json::to_string(value)?);
        content.push('\n');
    }
    fs::write(path, content)?;
    Ok(())
}

fn read_jsonl<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Vec<T>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw = fs::read_to_string(path)?;
    raw.lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).context("failed to parse cluster jsonl record"))
        .collect()
}

fn write_json_pretty<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

fn read_json_file<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<Option<T>> {
    if !path.exists() {
        return Ok(None);
    }
    Ok(Some(serde_json::from_str(&fs::read_to_string(path)?)?))
}

struct ClusterPaths {
    dir: PathBuf,
    nodes: PathBuf,
    heartbeats: PathBuf,
    roles: PathBuf,
    leases: PathBuf,
    lease_events: PathBuf,
    events: PathBuf,
    evidence: PathBuf,
    jobs: PathBuf,
}

impl ClusterPaths {
    fn new(cfg: &Config) -> Self {
        let dir = cfg.workspace_dir.join("state/cluster");
        Self {
            nodes: dir.join("nodes.jsonl"),
            heartbeats: dir.join("heartbeats.jsonl"),
            roles: dir.join("roles.json"),
            leases: dir.join("leases.json"),
            lease_events: dir.join("lease-events.jsonl"),
            events: dir.join("events.jsonl"),
            evidence: dir.join("evidence"),
            jobs: dir.join("jobs"),
            dir,
        }
    }

    fn ensure_dirs(&self) -> Result<()> {
        fs::create_dir_all(&self.dir)?;
        fs::create_dir_all(&self.evidence)?;
        fs::create_dir_all(&self.jobs)?;
        Ok(())
    }

    fn jobs_for_node(&self, node_id: &ClusterNodeId) -> PathBuf {
        self.jobs
            .join(format!("{}.jsonl", node_id.as_str().replace(':', "_")))
    }
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

    fn approved_paired_child(
        cfg: &Config,
        name: &str,
        role: &str,
        capabilities: &[&str],
    ) -> ClusterNode {
        let invite = pairing::create_invite(
            cfg,
            pairing::PairingInviteOptions {
                name: Some(name),
                desk: Some("crypto"),
                role: Some(role),
                ttl: Duration::minutes(10),
                core_url: "http://127.0.0.1:8787",
                dev_auto_accept: false,
            },
        )
        .expect("invite");
        let capabilities = capabilities
            .iter()
            .map(|capability| (*capability).to_string())
            .collect::<Vec<_>>();
        let request = pairing::submit_child_pair_request(
            cfg,
            pairing::ChildPairRequestInput {
                core_url: &invite.invite.core_url,
                invite_token: &invite.invite_token,
                node_name: Some(name),
                surface: "termux_worker",
                capabilities: &capabilities,
                requested_role: Some(role),
                requested_authority: PairingAuthority::Observe,
            },
        )
        .expect("request");
        let accepted =
            pairing::approve_request(cfg, &request.request_id, "operator").expect("approve");
        find_node(cfg, &accepted.node_id.parse().expect("accepted node id")).expect("node")
    }

    fn compute_output_for_receipt(receipt: &ClusterEvidenceReceipt) -> Value {
        let artifact = fs::read_to_string(&receipt.artifact_paths[0]).expect("artifact");
        let artifact_json: Value = serde_json::from_str(&artifact).expect("artifact json");
        assert_eq!(artifact_json["non_authoritative"], true);
        let output = artifact_json
            .get("output")
            .and_then(Value::as_str)
            .expect("output string");
        serde_json::from_str(output).expect("compute output json")
    }

    fn sample_telemetry() -> DeviceTelemetry {
        DeviceTelemetry {
            device_display_name: Some("tablet-01".to_string()),
            hostname: Some("termux-host".to_string()),
            model_hint: Some("android-tablet".to_string()),
            os: "android".to_string(),
            arch: "aarch64".to_string(),
            storage: Some(device_telemetry::StorageTelemetry {
                path: "workspace".to_string(),
                total_bytes: Some(64 * 1024 * 1024 * 1024),
                available_bytes: Some(512 * 1024 * 1024),
                used_percent: Some(99.2),
                source: device_telemetry::StorageTelemetrySource::Unknown,
            }),
            battery: Some(device_telemetry::BatteryTelemetry {
                percent: Some(10.0),
                charging: Some(false),
                status: Some("Discharging".to_string()),
                temperature_c: None,
                health: Some("Good".to_string()),
                source: device_telemetry::BatteryTelemetrySource::Unknown,
            }),
            collection_errors: vec![],
            collected_at: now(),
        }
    }

    #[test]
    fn cluster_node_registers_and_heartbeats() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        let heartbeat = heartbeat(&cfg, &node.node_id).expect("heartbeat");
        assert_eq!(heartbeat.node_id, node.node_id);
        assert!(heartbeat.paired);
        assert!(heartbeat.approved);
    }

    #[test]
    fn unpaired_child_heartbeat_rejected() {
        let (_tmp, cfg) = test_config();
        let node = register_node(
            &cfg,
            "raw-tablet",
            ClusterSurfaceKind::TermuxWorker,
            vec![ClusterCapability::Echo],
        )
        .expect("register");
        let err = heartbeat(&cfg, &node.node_id).expect_err("unpaired rejected");
        assert!(err.to_string().contains("approved paired child"));
    }

    #[test]
    fn heartbeat_does_not_create_lease_or_assign_role() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let status = node_status(&cfg, &node.node_id).expect("status");
        assert!(status.online);
        assert_eq!(status.active_lease_id, None);
        assert_eq!(status.active_role_id, None);
        assert!(!status.execution_enabled);
        assert!(!status.jobs_enabled);
    }

    #[test]
    fn heartbeat_rejects_authority_claims() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        let err = heartbeat_with_input(
            &cfg,
            ClusterHeartbeatInput {
                node_id: node.node_id,
                surface: None,
                claimed_capabilities: vec![],
                execution_enabled: true,
                approval_enabled: false,
                canonical_write_enabled: false,
                source: HeartbeatSource::TestFixture,
                device_telemetry: None,
            },
        )
        .expect_err("authority rejected");
        assert!(err.to_string().contains("cannot claim execution"));
    }

    #[test]
    fn heartbeat_records_device_telemetry() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        let heartbeat = heartbeat_with_input(
            &cfg,
            ClusterHeartbeatInput {
                node_id: node.node_id,
                surface: None,
                claimed_capabilities: vec![],
                execution_enabled: false,
                approval_enabled: false,
                canonical_write_enabled: false,
                source: HeartbeatSource::TestFixture,
                device_telemetry: Some(sample_telemetry()),
            },
        )
        .expect("heartbeat");
        assert_eq!(
            heartbeat
                .device_telemetry
                .as_ref()
                .map(|telemetry| telemetry.os.as_str()),
            Some("android")
        );
    }

    #[test]
    fn cluster_nodes_displays_device_telemetry() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        heartbeat_with_input(
            &cfg,
            ClusterHeartbeatInput {
                node_id: node.node_id,
                surface: None,
                claimed_capabilities: vec![],
                execution_enabled: false,
                approval_enabled: false,
                canonical_write_enabled: false,
                source: HeartbeatSource::TestFixture,
                device_telemetry: Some(sample_telemetry()),
            },
        )
        .expect("heartbeat");
        let rendered = render_node_statuses(&node_statuses(&cfg).expect("statuses"));
        assert!(rendered.contains("os=android"));
        assert!(rendered.contains("arch=aarch64"));
        assert!(rendered.contains("battery=10%"));
    }

    #[test]
    fn device_telemetry_does_not_create_lease_or_enable_jobs_or_authority() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        heartbeat_with_input(
            &cfg,
            ClusterHeartbeatInput {
                node_id: node.node_id.clone(),
                surface: None,
                claimed_capabilities: vec![],
                execution_enabled: false,
                approval_enabled: false,
                canonical_write_enabled: false,
                source: HeartbeatSource::TestFixture,
                device_telemetry: Some(sample_telemetry()),
            },
        )
        .expect("heartbeat");
        let status = node_status(&cfg, &node.node_id).expect("status");
        assert!(status.active_lease_id.is_none());
        assert!(!status.jobs_enabled);
        assert!(!status.execution_enabled);
        assert!(!status.approval_enabled);
        assert!(!status.canonical_write_enabled);
    }

    #[test]
    fn device_telemetry_rejects_authority_claims() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        let mut telemetry = sample_telemetry();
        telemetry.collection_errors = vec!["execution_enabled=true".to_string()];
        let err = heartbeat_with_input(
            &cfg,
            ClusterHeartbeatInput {
                node_id: node.node_id,
                surface: None,
                claimed_capabilities: vec![],
                execution_enabled: false,
                approval_enabled: false,
                canonical_write_enabled: false,
                source: HeartbeatSource::TestFixture,
                device_telemetry: Some(telemetry),
            },
        )
        .expect_err("telemetry claim rejected");
        assert!(err.to_string().contains("forbidden authority"));
    }

    #[test]
    fn low_storage_warning_is_advisory_only() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        heartbeat_with_input(
            &cfg,
            ClusterHeartbeatInput {
                node_id: node.node_id.clone(),
                surface: None,
                claimed_capabilities: vec![],
                execution_enabled: false,
                approval_enabled: false,
                canonical_write_enabled: false,
                source: HeartbeatSource::TestFixture,
                device_telemetry: Some(sample_telemetry()),
            },
        )
        .expect("heartbeat");
        let status = node_status(&cfg, &node.node_id).expect("status");
        assert!(!status.telemetry_warnings.is_empty());
        assert!(!status.jobs_enabled);
        assert!(status.active_lease_id.is_none());
    }

    #[test]
    fn observe_lease_can_be_checked_and_revoked() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("stablecoin_peg_watcher").expect("role");
        let lease = grant_observe_lease(&cfg, &node.node_id, &role_id, Duration::minutes(30), "op")
            .expect("lease");
        assert_eq!(lease.authority, LeaseAuthority::Observe);
        assert_eq!(
            check_lease(&cfg, &node.node_id).expect("check"),
            LeaseValidationResult::Valid
        );
        let status = node_status(&cfg, &node.node_id).expect("status");
        assert_eq!(
            status.active_lease_id.as_deref(),
            Some(lease.lease_id.as_str())
        );
        assert_eq!(
            status
                .active_role_id
                .as_ref()
                .map(ToString::to_string)
                .as_deref(),
            Some("stablecoin_peg_watcher")
        );
        assert!(!status.execution_enabled);
        assert!(!status.jobs_enabled);
        revoke_lease(&cfg, Some(&lease.lease_id), None, "test complete").expect("revoke");
        assert_eq!(
            check_lease(&cfg, &node.node_id).expect("check revoked"),
            LeaseValidationResult::Revoked
        );
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "crypto",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "revoked lease should block"}),
            Duration::minutes(5),
        )
        .expect_err("revoked lease blocks echo");
        assert!(err.to_string().contains("no active cluster lease"));
        let events = read_jsonl::<ClusterLeaseEventRecord>(&ClusterPaths::new(&cfg).lease_events)
            .expect("lease events");
        assert!(
            events
                .iter()
                .any(|event| event.kind == "cluster_lease_granted")
        );
        assert!(
            events
                .iter()
                .any(|event| event.kind == "cluster_lease_revoked")
        );
    }

    #[test]
    fn lease_records_playbook_hash() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-peg-01",
            "stablecoin_peg_watcher",
            &["echo", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("stablecoin_peg_watcher").expect("role");
        let lease = grant_observe_lease_with_playbook(
            &cfg,
            &node.node_id,
            &role_id,
            Duration::minutes(30),
            "operator",
            "stablecoin_peg_watcher",
        )
        .expect("lease");
        assert_eq!(lease.playbook_id.as_deref(), Some("stablecoin_peg_watcher"));
        assert!(
            lease
                .playbook_hash
                .as_deref()
                .is_some_and(|hash| !hash.is_empty())
        );
    }

    #[test]
    fn role_assignment_and_echo_roundtrip_records_evidence() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "pi-forex-01",
            "forex_calendar_watcher",
            &["echo", "sleep"],
        );
        let role_id = ClusterRoleId::new("forex_calendar_watcher").expect("role id");
        let lease =
            assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let job = submit_job(
            &cfg,
            &node.node_id,
            "forex",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "macro calendar clear"}),
            Duration::minutes(5),
        )
        .expect("job");
        assert_eq!(job.lease_id, lease.lease_id);
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        assert_eq!(receipt.desk_id, "forex");
        assert!(receipt.replay_safe);
        let report = report(&cfg).expect("report");
        assert_eq!(report.recent_evidence.len(), 1);
    }

    #[test]
    fn leased_child_can_run_echo_job_as_evidence_only() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-echo-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        grant_observe_lease(
            &cfg,
            &node.node_id,
            &role_id,
            Duration::minutes(30),
            "operator",
        )
        .expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let job = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "tablet online"}),
            Duration::minutes(5),
        )
        .expect("echo job");
        assert_eq!(job.authority_level, ClusterAuthorityLevel::Observe);
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        assert_eq!(receipt.result_status, "ok");
        assert!(receipt.replay_safe);
        assert!(!receipt.promoted_to_proposal);
        let artifact = fs::read_to_string(&receipt.artifact_paths[0]).expect("artifact");
        assert!(artifact.contains("tablet online"));
        assert!(artifact.contains("\"non_authoritative\": true"));
    }

    #[test]
    fn expired_lease_rejects_jobs() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        let err = assign_role(&cfg, &node.node_id, &role_id, Duration::seconds(-1))
            .expect_err("negative ttl rejected");
        assert!(err.to_string().contains("ttl"));
        let lease = grant_observe_lease(
            &cfg,
            &node.node_id,
            &role_id,
            Duration::seconds(1),
            "operator",
        )
        .expect("lease");
        let mut leases = list_leases(&cfg).expect("leases");
        leases[0].expires_at = (Utc::now() - Duration::seconds(1)).to_rfc3339();
        write_json_pretty(&ClusterPaths::new(&cfg).leases, &leases).expect("write leases");
        assert_eq!(lease.lease_id, leases[0].lease_id);
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "hello"}),
            Duration::minutes(5),
        )
        .expect_err("expired lease rejected");
        assert!(err.to_string().contains("no active cluster lease"));
    }

    #[test]
    fn http_get_is_denied_by_default() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "browser_research_worker",
            &["echo", "http_get"],
        );
        let role_id = ClusterRoleId::new("browser_research_worker").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::HttpGet,
            serde_json::json!({"url": "https://example.com"}),
            Duration::minutes(5),
        )
        .expect_err("http denied");
        assert!(err.to_string().contains("only echo and scalar compute"));
    }

    #[test]
    fn cluster_compute_job_requires_valid_lease() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "hello"}),
            Duration::minutes(5),
        )
        .expect_err("lease required");
        assert!(err.to_string().contains("no active cluster lease"));
    }

    #[test]
    fn offline_child_cannot_run_echo_job() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "hello"}),
            Duration::minutes(5),
        )
        .expect_err("heartbeat required");
        assert!(err.to_string().contains("fresh child heartbeat"));
    }

    #[test]
    fn echo_job_respects_timing_gate() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let cooldowns_path = cfg.workspace_dir.join("state/timing/cooldowns.json");
        fs::create_dir_all(cooldowns_path.parent().expect("cooldowns parent"))
            .expect("cooldowns dir");
        fs::write(
            cooldowns_path,
            serde_json::to_string_pretty(&vec![timing::TimingCooldown {
                desk_id: DeskId::new("research"),
                role_id: "generic_evidence_collector".to_string(),
                until: (Utc::now() + Duration::minutes(10)).to_rfc3339(),
                reason: "test timing gate".to_string(),
            }])
            .expect("cooldowns json"),
        )
        .expect("write cooldowns");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::Echo,
            serde_json::json!({"text": "timing blocked"}),
            Duration::minutes(5),
        )
        .expect_err("timing gate blocks");
        assert!(err.to_string().contains("timing gate"));
    }

    #[test]
    fn leased_child_can_run_scalar_freshness_scan() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let job = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::ComputeFreshnessScan,
            serde_json::json!({"fixture": "evidence_freshness", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect("scalar freshness job");
        assert_eq!(job.authority_level, ClusterAuthorityLevel::Observe);
        assert_eq!(
            job.compute_requirement
                .as_ref()
                .expect("compute requirement")
                .backend_requested,
            ComputeBackend::Scalar
        );
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        assert!(receipt.replay_safe);
        assert!(!receipt.promoted_to_proposal);
        let output = compute_output_for_receipt(&receipt);
        assert_eq!(output["workload"], "evidence_freshness");
        assert_eq!(output["compute_meta"]["backend_used"], "scalar");
        assert_eq!(output["compute_meta"]["backend_requested"], "scalar");
        assert_eq!(
            output["compute_meta"]["validation_outcome"],
            "accepted_scalar_only"
        );
        assert!(
            output["compute_meta"]["input_hash"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            output["compute_meta"]["output_hash"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(output["proposal_created"], false);
        let report = report(&cfg).expect("report");
        assert_eq!(report.recent_compute_evidence.len(), 1);
        assert_eq!(report.pending_proposals, 0);
    }

    #[test]
    fn leased_child_can_run_scalar_peg_deviation_scan() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "edge-peg-01",
            "stablecoin_peg_watcher",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("stablecoin_peg_watcher").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let job = submit_job(
            &cfg,
            &node.node_id,
            "crypto",
            ClusterJobKind::ComputePegDeviation,
            serde_json::json!({"fixture": "boundary_ambiguous_peg_scan", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect("scalar peg job");
        assert_eq!(job.authority_level, ClusterAuthorityLevel::Observe);
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        assert!(receipt.replay_safe);
        assert!(!receipt.promoted_to_proposal);
        let output = compute_output_for_receipt(&receipt);
        assert_eq!(output["workload"], "peg_deviation");
        assert_eq!(output["compute_meta"]["backend_used"], "scalar");
        assert_eq!(
            output["compute_meta"]["numeric_confidence"],
            "boundary_ambiguous"
        );
        assert!(output["output"]["max_deviation_bps"].as_f64().is_some());
        let artifact = fs::read_to_string(&receipt.artifact_paths[0]).expect("artifact");
        for forbidden in [
            "buy",
            "sell",
            "arb",
            "edge approved",
            "proposal accepted",
            "trade",
            "bet",
        ] {
            assert!(!artifact.to_ascii_lowercase().contains(forbidden));
        }
    }

    #[test]
    fn compute_evidence_does_not_create_proposal_or_authority() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let _job = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::ComputeFreshnessScan,
            serde_json::json!({"fixture": "evidence_freshness", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect("compute job");
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        assert!(!receipt.promoted_to_proposal);
        let status = node_status(&cfg, &node.node_id).expect("status");
        assert!(!status.execution_enabled);
        assert!(!status.approval_enabled);
        assert!(!status.canonical_write_enabled);
        assert!(!status.jobs_enabled);
        assert_eq!(report(&cfg).expect("report").pending_proposals, 0);
    }

    #[test]
    fn simd_backend_rejected_in_this_checkpoint() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let _ = compute::validation::quarantine_backend(
            &cfg,
            node.node_id.as_str(),
            ComputeBackend::ArmNeon,
            "test quarantine",
        )
        .expect("quarantine");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::ComputeFreshnessScan,
            serde_json::json!({"fixture": "evidence_freshness", "backend": "arm_neon"}),
            Duration::minutes(5),
        )
        .expect_err("quarantined backend rejected");
        assert!(err.to_string().contains("quarantined"));
    }

    #[test]
    fn net_edge_workload_rejected_in_this_checkpoint() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::ComputeFreshnessScan,
            serde_json::json!({"fixture": "net_edge_arbitrage", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect_err("future workload rejected");
        assert!(err.to_string().contains("not enabled"));
    }

    #[test]
    fn leased_child_can_record_stablecoin_peg_observation() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-peg-01",
            "stablecoin_peg_watcher",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("stablecoin_peg_watcher").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let job = submit_job(
            &cfg,
            &node.node_id,
            "crypto",
            ClusterJobKind::DeskObservePegDeviation,
            serde_json::json!({
                "fixture": "stablecoin_peg_deviation",
                "backend": "scalar",
                "knowledge_pack_id": "stablecoin/peg-monitor"
            }),
            Duration::minutes(5),
        )
        .expect("desk observe job");
        assert_eq!(job.authority_level, ClusterAuthorityLevel::Observe);
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        assert!(receipt.replay_safe);
        assert!(!receipt.promoted_to_proposal);
        let output = compute_output_for_receipt(&receipt);
        assert_eq!(output["workload"], "desk_observe_peg_deviation");
        assert_eq!(
            output["desk_observation"]["evidence_kind"],
            "stablecoin_peg_deviation_observation"
        );
        assert_eq!(output["desk_observation"]["desk_id"], "crypto");
        assert_eq!(
            output["desk_observation"]["role_id"],
            "stablecoin_peg_watcher"
        );
        assert_eq!(
            output["desk_observation"]["knowledge_pack_id"],
            "stablecoin/peg-monitor"
        );
        assert_eq!(output["desk_observation"]["proposal_created"], false);
        assert_eq!(output["desk_observation"]["replay_safe"], true);
        assert_eq!(
            output["desk_observation"]["compute_meta"]["backend_used"],
            "scalar"
        );
        assert!(
            output["desk_observation"]["input_hash"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert!(
            output["desk_observation"]["output_hash"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
        assert_eq!(report(&cfg).expect("report").pending_proposals, 0);
    }

    #[test]
    fn desk_observe_records_freshness_observation_without_proposal() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-fresh-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let _job = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::DeskObserveEvidenceFreshness,
            serde_json::json!({"fixture": "evidence_freshness", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect("freshness observation job");
        let receipt = child_run_once(&cfg, &node.node_id)
            .expect("run")
            .expect("receipt");
        let output = compute_output_for_receipt(&receipt);
        assert_eq!(
            output["desk_observation"]["evidence_kind"],
            "evidence_freshness_observation"
        );
        assert_eq!(output["desk_observation"]["proposal_created"], false);
        let status = node_status(&cfg, &node.node_id).expect("status");
        assert!(!status.execution_enabled);
        assert!(!status.approval_enabled);
        assert!(!status.canonical_write_enabled);
    }

    #[test]
    fn desk_observe_rejects_desk_mismatch() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-peg-01",
            "stablecoin_peg_watcher",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("stablecoin_peg_watcher").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::DeskObservePegDeviation,
            serde_json::json!({"fixture": "stablecoin_peg_deviation", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect_err("desk mismatch rejected");
        assert!(err.to_string().contains("role/desk mismatch"));
    }

    #[test]
    fn desk_observe_rejects_timing_block() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-fresh-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        let cooldowns_path = cfg.workspace_dir.join("state/timing/cooldowns.json");
        fs::create_dir_all(cooldowns_path.parent().expect("cooldowns parent"))
            .expect("cooldowns dir");
        fs::write(
            cooldowns_path,
            serde_json::to_string_pretty(&vec![timing::TimingCooldown {
                desk_id: DeskId::new("research"),
                role_id: "generic_evidence_collector".to_string(),
                until: (Utc::now() + Duration::minutes(10)).to_rfc3339(),
                reason: "test timing gate".to_string(),
            }])
            .expect("cooldowns json"),
        )
        .expect("write cooldowns");
        let err = submit_job(
            &cfg,
            &node.node_id,
            "research",
            ClusterJobKind::DeskObserveEvidenceFreshness,
            serde_json::json!({"fixture": "evidence_freshness", "backend": "scalar"}),
            Duration::minutes(5),
        )
        .expect_err("timing gate blocks");
        assert!(err.to_string().contains("timing gate"));
    }

    #[test]
    fn desk_observe_rejects_provider_net_edge_and_arbitrage_language() {
        let (_tmp, cfg) = test_config();
        let node = approved_paired_child(
            &cfg,
            "tablet-fresh-01",
            "generic_evidence_collector",
            &["echo", "sleep", "compute_scalar"],
        );
        let role_id = ClusterRoleId::new("generic_evidence_collector").expect("role id");
        assign_role(&cfg, &node.node_id, &role_id, Duration::minutes(30)).expect("lease");
        heartbeat(&cfg, &node.node_id).expect("heartbeat");
        for forbidden in ["provider_call", "net_edge", "arbitrage"] {
            let err = submit_job(
                &cfg,
                &node.node_id,
                "research",
                ClusterJobKind::DeskObserveEvidenceFreshness,
                serde_json::json!({
                    "fixture": "evidence_freshness",
                    "backend": "scalar",
                    "note": forbidden
                }),
                Duration::minutes(5),
            )
            .expect_err("forbidden intent rejected");
            assert!(err.to_string().contains("forbidden intent"));
        }
    }

    #[test]
    fn edge_device_options_keep_single_authority_boundary() {
        let options = edge_device_options();
        assert!(
            options
                .iter()
                .any(|option| option.profile_id == "raspberry_pi3_dietpi_core")
        );
        assert!(
            options
                .iter()
                .any(|option| option.profile_id == "android_termux_phone")
        );
        assert!(
            options
                .iter()
                .any(|option| option.profile_id == "android_termux_tablet")
        );
        assert_eq!(
            options
                .iter()
                .filter(|option| option.can_be_execution_leader)
                .count(),
            1
        );
        assert!(
            options
                .iter()
                .filter(|option| !option.can_be_execution_leader)
                .all(|option| option.authority == ClusterDeviceAuthority::ChildEvidenceWorker)
        );
    }

    #[test]
    fn edge_device_options_render_transport_status() {
        let rendered = render_edge_device_options(&edge_device_options());
        assert!(rendered.contains("raspberry_pi3_dietpi_core"));
        assert!(rendered.contains("ssh_lan:planned"));
        assert!(rendered.contains("termux_ssh:planned"));
        assert!(rendered.contains("execution_leader=false"));
    }

    #[test]
    fn desk_rails_encode_forex_carry_risk_box() {
        let rails = desk_rails();
        let forex = rails
            .iter()
            .find(|rail| rail.rail_id == "forex_carry_rollover_positive_swap")
            .expect("forex rail");
        assert!(!forex.risk_limits.live_execution_allowed);
        assert_eq!(forex.risk_limits.starting_unit, "0.01 paper lot");
        assert_eq!(forex.risk_limits.max_open_positions_or_slips, 2);
        assert_eq!(forex.risk_limits.max_parallel_instruments_or_events, 3);
        assert!(
            forex
                .non_negotiable_rules
                .iter()
                .any(|rule| rule.contains("positive swap direction"))
        );
        assert!(
            forex
                .criteria
                .iter()
                .any(|criterion| criterion.criterion_id == "discount_entry_verified")
        );
    }

    #[test]
    fn desk_rails_encode_sports_event_scouting_language() {
        let rails = desk_rails();
        let sports = rails
            .iter()
            .find(|rail| rail.rail_id == "sports_major_event_probability_scout")
            .expect("sports rail");
        assert!(!sports.risk_limits.live_execution_allowed);
        assert!(
            sports
                .fundamental_language
                .iter()
                .any(|term| term == "NBA playoffs")
        );
        assert!(
            sports
                .fundamental_language
                .iter()
                .any(|term| term == "FIFA World Cup")
        );
        assert!(
            sports
                .criteria
                .iter()
                .any(|criterion| criterion.criterion_id == "price_edge_hypothesis")
        );
    }

    #[test]
    fn cluster_fsms_reject_invalid_terminal_transitions() {
        let node_fsm = ClusterNodeFsm;
        assert!(
            node_fsm
                .transition(ClusterNodeState::Retired, ClusterNodeEvent::Heartbeat)
                .is_err()
        );
        let job_fsm = ClusterJobFsm;
        assert!(
            job_fsm
                .transition(ClusterJobState::ReplayVerified, ClusterJobEvent::Start)
                .is_err()
        );
    }
}
