mod adapters;
mod agent_shell;
mod boil;
mod bootstrap;
mod capabilities;
mod channels;
mod child_bootstrap;
mod child_pack_sync;
mod cluster_boundary;
mod compaction;
mod config;
mod consensus;
mod context_decay;
mod context_firewall;
mod context_guardian;
mod context_status;
mod cost_ledger;
mod council_router;
mod daemon;
mod demo_flow;
mod desk_registry;
mod domain;
mod execution_runtime;
mod forex;
mod fsm_authority;
mod fsm_core;
mod fsm_registry;
mod heartbeat;
mod llm;
mod logutil;
mod loop_dry_run;
mod memory;
mod onboarding_router;
mod pairing;
mod policy_registry;
mod question;
mod scheduler_registry;
mod sessions;
mod shared_state;
mod shutdown;
mod side_effect_gate;
mod skill_registry;
mod skills;
mod state_review;
mod state_sql;
mod strategist;
mod telegram;
mod terminal_cockpit;
mod truth_files;
mod tui_shell;
mod worker;
mod worker_proposals;
mod workflow_registry;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use serde::{Serialize, de::DeserializeOwned};
use std::io::{self, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::{env, fs};

use adapters::AdapterHub;
use config::Config;
use memory::MemoryStore;
use worker::job_from_json;

const CLI_BANNER: &str = "\
\x1b[38;2;25;95;255mQ\
\x1b[38;2;35;110;255mu\
\x1b[38;2;45;125;255ma\
\x1b[38;2;55;140;255mn\
\x1b[38;2;65;155;255mt\
\x1b[38;2;75;170;255m-\
\x1b[38;2;85;185;255mM\x1b[0m\n";
const ANSI_RESET: &str = "\x1b[0m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_DIM: &str = "\x1b[2m";
const ANSI_BLUE: &str = "\x1b[38;2;80;160;255m";
const ANSI_CYAN: &str = "\x1b[38;2;70;220;230m";
const ANSI_GREEN: &str = "\x1b[38;2;80;220;140m";
const ANSI_YELLOW: &str = "\x1b[38;2;245;200;95m";
const ANSI_MAGENTA: &str = "\x1b[38;2;210;140;255m";
const ANSI_RED: &str = "\x1b[38;2;255;95;95m";

#[derive(Parser, Debug)]
#[command(name = "quant-m")]
#[command(
    about = "Quant-M: local-first governed agent work",
    long_about = "Quant-M is a local-first Rust runtime for governed agent work. It preserves memory, shared state, replayable session evidence, worker proposals, and human approval boundaries. A configured device runs as a Quant-M Agent Node."
)]
#[command(before_help = CLI_BANNER)]
struct Cli {
    #[arg(long)]
    config: Option<PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
#[allow(clippy::large_enum_variant)]
enum Commands {
    /// Prepare local state if needed, then open the Quant-M shell.
    Start,
    /// Guided first-run setup for humans.
    Onboard {
        #[arg(long)]
        advanced: bool,
        #[arg(long)]
        json: bool,
    },
    /// Create config and workspace files with safe defaults.
    Init {
        #[arg(long)]
        non_interactive: bool,
        #[arg(long)]
        json: bool,
    },
    /// Configure models, API keys, channels, paths, and runtime profile.
    Setup {
        #[arg(long)]
        non_interactive: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        local_model_provider: Option<String>,
        #[arg(long)]
        local_model: Option<String>,
        #[arg(long)]
        remote_model_provider: Option<String>,
        #[arg(long)]
        remote_model: Option<String>,
        #[arg(long = "openrouter-model")]
        openrouter_models: Vec<String>,
        #[arg(long)]
        openrouter_api_key: Option<String>,
        #[arg(long)]
        channel: Option<String>,
        #[arg(long)]
        channel_value: Option<String>,
        #[arg(long)]
        runtime_profile: Option<String>,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        workspace_path: Option<PathBuf>,
        #[arg(long)]
        state_path: Option<PathBuf>,
        #[arg(long)]
        session_path: Option<PathBuf>,
        #[arg(long)]
        external_network: Option<String>,
        #[arg(long)]
        context_guardian: Option<String>,
    },
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
    Onboarding {
        #[command(subcommand)]
        command: OnboardingCommand,
    },
    Doctor {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        providers: bool,
        #[arg(long)]
        live: bool,
    },
    Provider {
        #[command(subcommand)]
        command: ProviderCommand,
    },
    Tool {
        #[command(subcommand)]
        command: ToolCommand,
    },
    /// Show opt-in runtime features and integration status.
    Settings {
        #[arg(long)]
        json: bool,
    },
    Capabilities {
        #[command(subcommand)]
        command: Option<CapabilitiesCommand>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        status: Option<String>,
    },
    #[command(visible_alias = "shell")]
    Agent,
    Tui {
        #[command(subcommand)]
        command: Option<TuiCommand>,
    },
    /// Run the local proof workflow and print the next inspection commands.
    Demo,
    Status,
    Daemon {
        #[command(subcommand)]
        command: Option<DaemonCommand>,
    },
    Worker {
        #[command(subcommand)]
        command: WorkerCommand,
    },
    Bootstrap {
        #[command(subcommand)]
        command: BootstrapCommand,
    },
    Pack {
        #[command(subcommand)]
        command: PackCommand,
    },
    Pair {
        #[command(subcommand)]
        command: PairCommand,
    },
    Device {
        #[command(subcommand)]
        command: DeviceCommand,
    },
    Child {
        #[command(subcommand)]
        command: ChildCommand,
    },
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
    Heartbeat {
        #[command(subcommand)]
        command: HeartbeatCommand,
    },
    Skills {
        #[command(subcommand)]
        command: SkillsCommand,
    },
    Adapter {
        #[command(subcommand)]
        command: AdapterCommand,
    },
    Session {
        #[command(subcommand)]
        command: SessionCommand,
    },
    Skill {
        #[command(subcommand)]
        command: SkillCommand,
    },
    Policy {
        #[command(subcommand)]
        command: PolicyCommand,
    },
    Workflow {
        #[command(subcommand)]
        command: WorkflowCommand,
    },
    Fsm {
        #[command(subcommand)]
        command: FsmCommand,
    },
    Scheduler {
        #[command(subcommand)]
        command: SchedulerCommand,
    },
    Run {
        #[command(subcommand)]
        command: RunCommand,
    },
    Desk {
        #[command(subcommand)]
        command: DeskCommand,
    },
    Domain {
        #[command(subcommand)]
        command: DomainCommand,
    },
    Llm {
        #[command(subcommand)]
        command: LlmCommand,
    },
    Loop {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
        #[arg(long, default_value = "all")]
        scope: String,
        #[arg(long, default_value_t = 10)]
        max_candidates: usize,
    },
    Telegram {
        #[command(subcommand)]
        command: TelegramCommand,
    },
    Channel {
        #[command(subcommand)]
        command: ChannelCommand,
    },
    Cockpit {
        #[command(subcommand)]
        command: CockpitCommand,
    },
    Compact {
        session_id: String,
    },
    ContextStatus {
        #[arg(long)]
        json: bool,
    },
    Context {
        #[command(subcommand)]
        command: ContextCommand,
    },
    Replay {
        session_id: String,
        #[arg(long)]
        json: bool,
    },
    Consensus {
        #[arg(long)]
        dry_run: bool,
        question: String,
    },
    /// Evaluate adaptive Council policy without calling models or providers.
    Council {
        #[command(subcommand)]
        command: CouncilCommand,
    },
    Strategist {
        #[arg(long)]
        dry_run: bool,
        #[arg(long)]
        json: bool,
    },
    Question {
        #[command(subcommand)]
        command: QuestionCommand,
    },
    Cost {
        #[command(subcommand)]
        command: CostCommand,
    },
    /// Measure raw-vs-boiled continuation context cost for a session.
    Boil {
        #[arg(required = true, num_args = 1..)]
        args: Vec<String>,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = "rough-default")]
        pricing_profile: String,
    },
    InitTruth {
        #[arg(long)]
        force: bool,
        #[arg(long)]
        json: bool,
    },
    State {
        #[command(subcommand)]
        command: StateCommand,
    },
}

#[derive(Subcommand, Debug)]
enum DaemonCommand {
    Start,
}

#[derive(Subcommand, Debug)]
enum OnboardingCommand {
    Status {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CouncilCommand {
    /// Print the versioned default adaptive Council policy.
    Policy {
        #[arg(long)]
        json: bool,
    },
    /// Evaluate a prepared candidate/audit packet in provider-free shadow mode.
    Shadow {
        #[arg(long)]
        input: PathBuf,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        record: bool,
    },
}

#[derive(Subcommand, Debug)]
enum WorkerCommand {
    Submit {
        job_json: String,
    },
    Once {
        job_json: String,
    },
    Run,
    Proposal {
        #[command(subcommand)]
        command: WorkerProposalCommand,
    },
}

#[derive(Subcommand, Debug)]
enum WorkerProposalCommand {
    Submit {
        #[arg(long)]
        surface: String,
        #[arg(long)]
        kind: String,
        #[arg(long)]
        summary: String,
        #[arg(long)]
        worker_id: Option<String>,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long)]
        workflow_id: Option<String>,
        #[arg(long)]
        decision_scope: Option<String>,
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        surface: Option<String>,
        #[arg(long)]
        status: Option<String>,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum BootstrapCommand {
    /// List valid prebuilt child binary bundles without starting the server.
    List {
        #[arg(long, default_value = "./release-bundles")]
        bundle_dir: PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:8788")]
        bootstrap_url: String,
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        core_url: String,
        #[arg(long)]
        json: bool,
    },
    /// Serve child binary bootstrap page, metadata API, and approved downloads.
    Serve {
        #[arg(long, default_value = "0.0.0.0:8788")]
        bind: String,
        #[arg(long, default_value = "./release-bundles")]
        bundle_dir: PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:8787")]
        core_url: String,
    },
}

#[derive(Subcommand, Debug)]
enum PackCommand {
    /// List valid child knowledge packs, optionally filtered by child role.
    List {
        #[arg(long, default_value = "./release-packs")]
        pack_dir: PathBuf,
        #[arg(long, default_value = "http://127.0.0.1:8789")]
        pack_url: String,
        #[arg(long)]
        role: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Serve approved child knowledge packs for checksum-verified download.
    Serve {
        #[arg(long, default_value = "0.0.0.0:8789")]
        bind: String,
        #[arg(long, default_value = "./release-packs")]
        pack_dir: PathBuf,
    },
}

#[derive(Subcommand, Debug)]
enum PairCommand {
    /// Show the Agent Cluster pairing cockpit without granting child authority.
    Cockpit {
        #[arg(long, default_value = "0.0.0.0:8787")]
        bind: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        interface: Option<String>,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value_t = true)]
        qr: bool,
    },
    /// Diagnose same-Wi-Fi/local-network pairing URL selection.
    Doctor {
        #[arg(long, default_value = "0.0.0.0:8787")]
        bind: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        interface: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Show pairing server, pending request, and child status.
    Status {
        #[arg(long, default_value = "0.0.0.0:8787")]
        bind: String,
        #[arg(long)]
        json: bool,
    },
    /// Serve the minimal LAN pairing page and join URL text.
    Serve {
        #[arg(long, default_value = "0.0.0.0:8787")]
        bind: String,
        #[arg(long)]
        allow_public_bind: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DeviceCommand {
    /// Create a short-lived child pairing invite.
    Add {
        #[arg(long)]
        qr: bool,
        #[arg(long)]
        watch: bool,
        #[arg(long)]
        dry_run: bool,
        #[arg(long, default_value = "0.0.0.0:8787")]
        bind: String,
        #[arg(long)]
        host: Option<String>,
        #[arg(long)]
        port: Option<u16>,
        #[arg(long)]
        interface: Option<String>,
        #[arg(long, default_value_t = 30)]
        ttl_minutes: u64,
    },
}

#[derive(Subcommand, Debug)]
enum ChildCommand {
    /// Join a core invite as an observe-only child request.
    Join {
        #[arg(long)]
        url: Option<String>,
        #[arg(long)]
        manual: bool,
        #[arg(long, hide = true)]
        requested_authority: Option<String>,
    },
    /// Create or print the local child identity.
    Identity {
        #[arg(long)]
        json: bool,
    },
    /// Send an observe-only child heartbeat to the approved core.
    Heartbeat {
        #[arg(long)]
        core: Option<String>,
        #[arg(long)]
        once: bool,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        interval: Option<u64>,
    },
    /// List pending child requests and approved children.
    List {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        include_revoked: bool,
    },
    /// Manually approve a pending child request as observe-only.
    Approve {
        request_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Deny a pending child request.
    Deny {
        request_id: String,
        #[arg(long)]
        json: bool,
    },
    /// Revoke an approved child.
    Revoke {
        node_id: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum MemoryCommand {
    Add {
        key: String,
        content: String,
        #[arg(long, default_value = "daily")]
        category: String,
    },
    Search {
        query: String,
        #[arg(long, default_value_t = 10)]
        limit: usize,
    },
    List {
        #[arg(long, default_value_t = 20)]
        limit: usize,
        #[arg(long)]
        category: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum HeartbeatCommand {
    Tick,
    Run,
}

#[derive(Subcommand, Debug)]
enum SkillsCommand {
    List,
    Show { name: String },
    Run { name: String, input: String },
}

#[derive(Subcommand, Debug)]
enum AdapterCommand {
    Send {
        message: String,
        #[arg(long, default_value = "manual")]
        kind: String,
    },
}

#[derive(Subcommand, Debug)]
enum ConfigCommand {
    Show {
        #[arg(long)]
        json: bool,
    },
    SetModel {
        provider: String,
        model: String,
    },
    ClearModel {
        provider: Option<String>,
    },
    SetChannel {
        channel: String,
        value: String,
    },
    Validate,
}

#[derive(Subcommand, Debug)]
enum ProviderCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Validate {
        provider: String,
        #[arg(long)]
        live: bool,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ToolCommand {
    List {
        #[arg(long)]
        json: bool,
    },
    Setup {
        tool: String,
    },
    Scan {
        #[arg(long)]
        json: bool,
    },
    Validate {
        tool: String,
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CapabilitiesCommand {
    Show {
        capability_id: String,
        #[arg(long)]
        json: bool,
    },
    AuditDocs {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum SessionCommand {
    List,
    Show {
        id: String,
    },
    Replay {
        id: String,
    },
    ResumePlan {
        id: String,
    },
    Approve {
        id: String,
        #[arg(long)]
        reason: String,
    },
    Deny {
        id: String,
        #[arg(long)]
        reason: String,
    },
    NeedsInfo {
        id: String,
        #[arg(long)]
        reason: String,
    },
}

#[derive(Subcommand, Debug)]
enum DomainCommand {
    List,
    Show { domain_id: String },
}

#[derive(Subcommand, Debug)]
enum SkillCommand {
    List {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long = "side-effect")]
        side_effect: Option<String>,
    },
    Show {
        skill_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum PolicyCommand {
    List {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long = "side-effect")]
        side_effect: Option<String>,
    },
    Show {
        policy_id: String,
    },
    EvaluateSkill {
        skill_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum WorkflowCommand {
    List {
        #[arg(long)]
        domain: Option<String>,
    },
    Show {
        workflow_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum FsmCommand {
    Authority {
        #[arg(long)]
        json: bool,
    },
    List {
        #[arg(long)]
        domain: Option<String>,
    },
    Show {
        fsm_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum SchedulerCommand {
    List {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        trigger: Option<String>,
    },
    Show {
        scheduler_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum DeskCommand {
    List {
        #[arg(long)]
        category: Option<String>,
        #[arg(long)]
        domain: Option<String>,
    },
    Show {
        desk_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum RunCommand {
    Workflow { workflow_id: String },
}

#[derive(Subcommand, Debug)]
enum CostCommand {
    Summary {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        workflow: Option<String>,
        #[arg(long)]
        session: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum LlmCommand {
    Ask { prompt: String },
}

#[derive(Subcommand, Debug)]
enum TuiCommand {
    /// Open the governed chat-shaped evidence cockpit.
    Chat {
        /// Keep chat mode inspect-only. This prevents Codex CLI calls from /ask or plain text.
        #[arg(long)]
        inspect: bool,
    },
}

#[derive(Subcommand, Debug)]
enum QuestionCommand {
    Ask {
        #[arg(long)]
        mode: String,
        question: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        write_proposals: bool,
    },
}

#[derive(Subcommand, Debug)]
enum ContextCommand {
    Packet {
        #[arg(long)]
        state: String,
        #[arg(long, default_value = "small")]
        size: String,
        #[arg(long)]
        task: Option<String>,
        #[arg(long)]
        json: bool,
    },
    Guard {
        #[arg(long)]
        json: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        watch: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TelegramCommand {
    Run,
}

#[derive(Subcommand, Debug)]
enum ChannelCommand {
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum CockpitCommand {
    Plan {
        #[arg(long, default_value = "auto")]
        host: String,
        #[arg(long = "repo")]
        repo_paths: Vec<PathBuf>,
        #[arg(long = "model")]
        models: Vec<String>,
    },
}

#[derive(Subcommand, Debug)]
enum StateCommand {
    Init,
    Summary,
    List {
        #[arg(long)]
        domain: Option<String>,
    },
    Show {
        key: String,
    },
    Snapshot {
        #[arg(long)]
        domain: Option<String>,
    },
    Review {
        #[arg(long)]
        domain: Option<String>,
        #[arg(long)]
        json: bool,
    },
    ExpireStale,
    SignalUpsert {
        json: String,
    },
    HandoffAdd {
        json: String,
    },
    HandoffList {
        #[arg(long)]
        desk: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: usize,
    },
    RiskAdd {
        json: String,
    },
    OrderAdd {
        json: String,
    },
    ForexIngest {
        json: String,
    },
    ForexGetSignal {
        symbol: String,
    },
    ForexGetHandoff {
        symbol: String,
    },
    SwapHealth {
        json: String,
    },
    SwapHealthGet {
        symbol: String,
    },
    MacroRefreshMql5 {
        #[arg(long, default_value_t = 48)]
        hours_ahead: i64,
    },
    MacroGetPair {
        pair: String,
    },
}

#[derive(Debug, Clone, Serialize)]
struct InitReport {
    status: String,
    config: PathBuf,
    workspace: PathBuf,
    state_sqlite: PathBuf,
    session_dir: PathBuf,
    role: config::OnboardingRole,
    runtime_profile: config::RuntimeProfile,
}

#[derive(Debug, Clone, Serialize)]
struct SetupReport {
    status: String,
    config: PathBuf,
    workspace: PathBuf,
    state_sqlite: PathBuf,
    session_dir: PathBuf,
    role: config::OnboardingRole,
    runtime_profile: config::RuntimeProfile,
    external_network_enabled: bool,
    multi_model_enabled: bool,
    search_enabled: bool,
    browser_harness_enabled: bool,
    context_guardian_enabled: bool,
    preferred_channel: config::ChannelPreference,
    preferred_local_model: Option<config::ModelPreference>,
    preferred_remote_model: Option<config::ModelPreference>,
    preferred_openrouter_model: Option<String>,
    openrouter_key_present: bool,
    provider_count: usize,
    tool_count: usize,
    enabled_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct DoctorReport {
    role: config::OnboardingRole,
    runtime_profile: config::RuntimeProfile,
    config_exists: bool,
    workspace_exists: bool,
    state_path_exists: bool,
    session_path_exists: bool,
    workflow_run_ok: bool,
    shared_state_list_ok: bool,
    session_list_ok: bool,
    checked_binary: PathBuf,
    generated_session_id: Option<String>,
    provider_diagnostics: Vec<ProviderValidationReport>,
}

#[derive(Debug, Clone, Serialize)]
struct OnboardingStatusReport {
    config: PathBuf,
    workspace: PathBuf,
    onboarding_completed: bool,
    role: config::OnboardingRole,
    runtime_profile: config::RuntimeProfile,
    next_action: onboarding_router::OnboardingNextAction,
    provider_route_status: onboarding_router::ProviderRouteStatus,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderListItem {
    id: String,
    enabled: bool,
    kind: config::ProviderKind,
    api_base: String,
    api_key_env: String,
    key_present: bool,
    preferred_models: Vec<String>,
    live_validation_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
struct ToolListItem {
    id: String,
    enabled: bool,
    kind: config::ToolKind,
    command: String,
    validation_args: Vec<String>,
    command_present: bool,
}

#[derive(Debug, Clone, Serialize)]
struct SettingsReport {
    config: PathBuf,
    workspace: PathBuf,
    session_dir: PathBuf,
    multi_model_enabled: bool,
    search_enabled: bool,
    browser_harness_enabled: bool,
    external_network_enabled: bool,
    context_guardian_enabled: bool,
    preferred_local_model: Option<config::ModelPreference>,
    preferred_remote_model: Option<config::ModelPreference>,
    preferred_openrouter_model: Option<String>,
    providers: Vec<ProviderListItem>,
    enabled_tools: Vec<String>,
    detected_tools: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ProviderValidationReport {
    id: String,
    enabled: bool,
    kind: config::ProviderKind,
    api_base: String,
    api_key_env: String,
    key_present: bool,
    live_requested: bool,
    live_ok: Option<bool>,
    message: String,
}

#[derive(Debug, Clone, Serialize)]
struct ToolValidationReport {
    id: String,
    enabled: bool,
    kind: config::ToolKind,
    command: String,
    command_present: bool,
    validation_ok: bool,
    message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StorageMode {
    Inspect,
    SessionWrite,
    RuntimePreflight,
    WorkerRun,
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let config_path = resolve_config_path(cli.config.clone())?;
    let command = cli.command.unwrap_or(Commands::Start);

    if is_onboarding_command(&command) {
        return handle_onboarding_command(command, &config_path).await;
    }

    if matches!(command, Commands::Start) {
        return run_start_flow(&config_path);
    }

    let cfg = Config::load_or_create(&config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.validate()?;
    bootstrap::ensure_workspace(&cfg)?;
    prepare_storage_for_command(&cfg, &command)?;

    match command {
        Commands::Start
        | Commands::Init { .. }
        | Commands::Onboard { .. }
        | Commands::Onboarding { .. }
        | Commands::Setup { .. }
        | Commands::Config { .. }
        | Commands::Doctor { .. }
        | Commands::Provider { .. }
        | Commands::Tool { .. }
        | Commands::Settings { .. } => unreachable!("onboarding commands are handled earlier"),
        Commands::Capabilities {
            command,
            json,
            category,
            status,
        } => handle_capabilities_command(&cfg, command, json, category, status)?,
        Commands::Agent => {
            agent_shell::run(&cfg, &config_path)?;
        }
        Commands::Tui { command } => match command {
            None => tui_shell::run(&cfg, &config_path)?,
            Some(TuiCommand::Chat { inspect }) => {
                tui_shell::run_chat(&cfg, &config_path, inspect)?;
            }
        },
        Commands::Demo => {
            let result = demo_flow::run(&cfg)?;
            print!("{}", demo_flow::render(&result));
        }
        Commands::Status => {
            print_status(&cfg)?;
        }
        Commands::Daemon { command } => match command {
            None | Some(DaemonCommand::Start) => {
                daemon::run(cfg.clone()).await?;
            }
        },
        Commands::Worker { command } => match command {
            WorkerCommand::Submit { job_json } => {
                let job = job_from_json(&job_json)?;
                worker::submit_job(&cfg, &job)?;
                println!("{}", serde_json::to_string_pretty(&job)?);
            }
            WorkerCommand::Once { job_json } => {
                let job = job_from_json(&job_json)?;
                let adapters = AdapterHub::new(&cfg)?;
                let result = worker::run_once(&cfg, job, &adapters).await?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            WorkerCommand::Run => {
                let adapters = AdapterHub::new(&cfg)?;
                worker::run_loop(cfg.clone(), adapters).await?;
            }
            WorkerCommand::Proposal { command } => match command {
                WorkerProposalCommand::Submit {
                    surface,
                    kind,
                    summary,
                    worker_id,
                    session_id,
                    workflow_id,
                    decision_scope,
                    json,
                } => {
                    let input = worker_proposals::SubmitWorkerProposalInput {
                        source_surface: surface.parse()?,
                        source_worker_id: worker_id.unwrap_or_else(|| cfg.node_id.clone()),
                        proposal_kind: kind.parse()?,
                        summary,
                        session_id,
                        workflow_id,
                        decision_scope,
                    };
                    let (_record, submitted) =
                        worker_proposals::submit_worker_proposal(&cfg, input)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&submitted)?);
                    } else {
                        print!("{}", worker_proposals::render_submit_summary(&submitted));
                    }
                }
                WorkerProposalCommand::List {
                    surface,
                    status,
                    json,
                } => {
                    let surface = surface
                        .as_deref()
                        .map(str::parse)
                        .transpose()
                        .context("invalid worker proposal surface filter")?;
                    let status = status
                        .as_deref()
                        .map(str::parse)
                        .transpose()
                        .context("invalid worker proposal status filter")?;
                    let listed = worker_proposals::list_worker_proposals(&cfg, surface, status)?;
                    if json {
                        println!("{}", serde_json::to_string_pretty(&listed)?);
                    } else {
                        print!("{}", worker_proposals::render_list_summary(&listed));
                    }
                }
            },
        },
        Commands::Bootstrap { command } => match command {
            BootstrapCommand::List {
                bundle_dir,
                bootstrap_url,
                core_url,
                json,
            } => {
                let listing =
                    child_bootstrap::list_bundles(&bundle_dir, &bootstrap_url, &core_url)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&listing)?);
                } else {
                    print!("{}", child_bootstrap::render_listing_text(&listing));
                }
            }
            BootstrapCommand::Serve {
                bind,
                bundle_dir,
                core_url,
            } => {
                child_bootstrap::serve(bundle_dir, &bind, &core_url)?;
            }
        },
        Commands::Pack { command } => match command {
            PackCommand::List {
                pack_dir,
                pack_url,
                role,
                json,
            } => {
                let listing = child_pack_sync::list_packs(&pack_dir, &pack_url, role.as_deref())?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&listing)?);
                } else {
                    print!("{}", child_pack_sync::render_listing_text(&listing));
                }
            }
            PackCommand::Serve { bind, pack_dir } => {
                child_pack_sync::serve(pack_dir, &bind)?;
            }
        },
        Commands::Pair { command } => match command {
            PairCommand::Cockpit {
                bind,
                host,
                port,
                interface,
                dry_run,
                qr,
            } => {
                let bind = bind_with_optional_port(&bind, port);
                let options = pairing::AdvertiseOptions { host, interface };
                let report = pairing::cockpit_with_options(&cfg, &bind, qr, dry_run, &options)?;
                print!("{}", pairing::render_cockpit(&report));
            }
            PairCommand::Doctor {
                bind,
                host,
                port,
                interface,
                json,
            } => {
                let bind = bind_with_optional_port(&bind, port);
                let options = pairing::AdvertiseOptions { host, interface };
                let report = pairing::doctor(&cfg, &bind, &options)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", pairing::render_doctor(&report));
                }
            }
            PairCommand::Status { bind, json } => {
                let report = pairing::status(&cfg, &bind)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", pairing::render_status(&report));
                }
            }
            PairCommand::Serve {
                bind,
                allow_public_bind,
            } => {
                pairing::serve(&cfg, &bind, allow_public_bind)?;
            }
        },
        Commands::Device { command } => match command {
            DeviceCommand::Add {
                qr,
                watch,
                dry_run,
                bind,
                host,
                port,
                interface,
                ttl_minutes,
            } => {
                if watch {
                    print!("{}", pairing::render_pending_watch(&cfg)?);
                } else {
                    let bind = bind_with_optional_port(&bind, port);
                    let options = pairing::AdvertiseOptions { host, interface };
                    let report = pairing::create_invite_with_options(
                        &cfg,
                        &bind,
                        ttl_minutes,
                        qr,
                        dry_run,
                        &options,
                    )?;
                    print!("{}", pairing::render_device_add(&report));
                }
            }
        },
        Commands::Child { command } => match command {
            ChildCommand::Join {
                url,
                manual,
                requested_authority,
            } => {
                if manual {
                    print!("{}", pairing::render_child_join_manual());
                } else {
                    let url = url.with_context(|| {
                        "child join requires --url <join-url> or --manual for fallback instructions"
                    })?;
                    let report = pairing::child_join_by_url(
                        &cfg,
                        None,
                        &url,
                        requested_authority.as_deref(),
                    )?;
                    print!("{}", pairing::render_child_join(&report));
                }
            }
            ChildCommand::Identity { json } => {
                let report = pairing::child_identity(&cfg)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", pairing::render_child_identity(&report));
                }
            }
            ChildCommand::Heartbeat {
                core,
                once: _,
                json,
                interval,
            } => {
                if let Some(interval) = interval
                    && interval < 15
                {
                    anyhow::bail!("child heartbeat --interval must be at least 15 seconds");
                }
                let report = pairing::child_heartbeat(&cfg, None, core.as_deref(), None)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", pairing::render_child_heartbeat(&report));
                }
            }
            ChildCommand::List {
                json,
                include_revoked,
            } => {
                let report = pairing::list_children(&cfg, true, include_revoked)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", pairing::render_child_list(&report));
                }
            }
            ChildCommand::Approve { request_id, json } => {
                let child = pairing::approve_request(&cfg, &request_id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&child)?);
                } else {
                    println!(
                        "approved node_id={} request_id={} authority={} provider_calls={} execution={} approval={} canonical_writes={}",
                        child.node_id,
                        child.request_id,
                        child.authority.authority,
                        child.authority.provider_calls_allowed,
                        child.authority.execution_allowed,
                        child.authority.approval_allowed,
                        child.authority.canonical_write_allowed,
                    );
                }
            }
            ChildCommand::Deny { request_id, json } => {
                let request = pairing::deny_request(&cfg, &request_id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&request)?);
                } else {
                    println!("denied request_id={} status=denied", request.request_id);
                }
            }
            ChildCommand::Revoke { node_id, json } => {
                let child = pairing::revoke_child(&cfg, &node_id)?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&child)?);
                } else {
                    println!("revoked node_id={} status=revoked", child.node_id);
                }
            }
        },
        Commands::Memory { command } => {
            let store = MemoryStore::open(&cfg)?;
            match command {
                MemoryCommand::Add {
                    key,
                    content,
                    category,
                } => {
                    let inserted = store.add_entry(&key, &content, &category)?;
                    println!("{}", serde_json::to_string_pretty(&inserted)?);
                }
                MemoryCommand::Search { query, limit } => {
                    let found = store.search(&query, limit)?;
                    println!("{}", serde_json::to_string_pretty(&found)?);
                }
                MemoryCommand::List { limit, category } => {
                    let listed = store.list(limit, category.as_deref())?;
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
            }
        }
        Commands::Heartbeat { command } => {
            let adapters = AdapterHub::new(&cfg)?;
            match command {
                HeartbeatCommand::Tick => {
                    let results = heartbeat::tick(&cfg, &adapters).await?;
                    println!("{}", serde_json::to_string_pretty(&results)?);
                }
                HeartbeatCommand::Run => {
                    heartbeat::run_loop(cfg.clone(), adapters).await?;
                }
            }
        }
        Commands::Skills { command } => match command {
            SkillsCommand::List => {
                let listed = skills::list_skills(&cfg)?;
                println!("{}", serde_json::to_string_pretty(&listed)?);
            }
            SkillsCommand::Show { name } => {
                let detail = skills::show_skill(&cfg, &name)?;
                println!("{}", serde_json::to_string_pretty(&detail)?);
            }
            SkillsCommand::Run { name, input } => {
                let output = skills::run_skill(&cfg, &name, &input).await?;
                println!("{output}");
            }
        },
        Commands::Adapter { command } => match command {
            AdapterCommand::Send { message, kind } => {
                let adapters = AdapterHub::new(&cfg)?;
                adapters.send_simple(&kind, &message).await?;
            }
        },
        Commands::Session { command } => match command {
            SessionCommand::List => {
                let listed = sessions::list_sessions(&cfg)?;
                println!("{}", serde_json::to_string_pretty(&listed)?);
            }
            SessionCommand::Show { id } => {
                let session_id = id.parse::<sessions::SessionId>()?;
                let detail = sessions::show_session(&cfg, &session_id)?;
                println!("{}", serde_json::to_string_pretty(&detail)?);
            }
            SessionCommand::Replay { id } => {
                let session_id = id.parse::<sessions::SessionId>()?;
                let replay = sessions::replay_session(&cfg, &session_id)?;
                println!("{}", serde_json::to_string_pretty(&replay)?);
            }
            SessionCommand::ResumePlan { id } => {
                let session_id = id.parse::<sessions::SessionId>()?;
                let plan = sessions::resume_plan_session(&cfg, &session_id)?;
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
            SessionCommand::Approve { id, reason } => {
                let session_id = id.parse::<sessions::SessionId>()?;
                let record = sessions::record_operator_decision(
                    &cfg,
                    &session_id,
                    sessions::OperatorDecision::Approved,
                    &reason,
                    &operator_identity(&cfg),
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            SessionCommand::Deny { id, reason } => {
                let session_id = id.parse::<sessions::SessionId>()?;
                let record = sessions::record_operator_decision(
                    &cfg,
                    &session_id,
                    sessions::OperatorDecision::Denied,
                    &reason,
                    &operator_identity(&cfg),
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            SessionCommand::NeedsInfo { id, reason } => {
                let session_id = id.parse::<sessions::SessionId>()?;
                let record = sessions::record_operator_decision(
                    &cfg,
                    &session_id,
                    sessions::OperatorDecision::NeedsMoreInfo,
                    &reason,
                    &operator_identity(&cfg),
                )?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
        },
        Commands::Skill { command } => {
            let registry = skill_registry::builtin_registry()?;
            match command {
                SkillCommand::List {
                    domain,
                    side_effect,
                } => {
                    let domain = domain
                        .as_deref()
                        .map(str::parse::<sessions::DomainId>)
                        .transpose()?;
                    let side_effect = side_effect
                        .as_deref()
                        .map(str::parse::<skill_registry::SideEffectLevel>)
                        .transpose()?;
                    let listed = registry.list(domain.as_ref(), side_effect.as_ref());
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
                SkillCommand::Show { skill_id } => {
                    let detail = registry.show(&skill_id)?;
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                }
            }
        }
        Commands::Policy { command } => {
            let policies = policy_registry::builtin_registry()?;
            match command {
                PolicyCommand::List {
                    domain,
                    side_effect,
                } => {
                    let domain = domain
                        .as_deref()
                        .map(str::parse::<sessions::DomainId>)
                        .transpose()?;
                    let side_effect = side_effect
                        .as_deref()
                        .map(str::parse::<skill_registry::SideEffectLevel>)
                        .transpose()?;
                    let listed = policies.list(domain.as_ref(), side_effect.as_ref());
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
                PolicyCommand::Show { policy_id } => {
                    let detail = policies.show(&policy_id)?;
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                }
                PolicyCommand::EvaluateSkill { skill_id } => {
                    let skills = skill_registry::builtin_registry()?;
                    let skill = skills.show(&skill_id)?;
                    let evaluation = policies.evaluate_skill(&skill);
                    println!("{}", serde_json::to_string_pretty(&evaluation)?);
                }
            }
        }
        Commands::Workflow { command } => {
            let registry = workflow_registry::builtin_registry()?;
            match command {
                WorkflowCommand::List { domain } => {
                    let domain = domain
                        .as_deref()
                        .map(str::parse::<sessions::DomainId>)
                        .transpose()?;
                    let listed = registry.list(domain.as_ref());
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
                WorkflowCommand::Show { workflow_id } => {
                    let workflow_id = workflow_id.parse::<workflow_registry::WorkflowId>()?;
                    let detail = registry.show(&workflow_id)?;
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                }
            }
        }
        Commands::Fsm { command } => match command {
            FsmCommand::Authority { json } => {
                let records = fsm_authority::authority_records();
                if json {
                    println!("{}", serde_json::to_string_pretty(&records)?);
                } else {
                    print!("{}", fsm_authority::render_authority_records(&records));
                }
            }
            FsmCommand::List { domain } => {
                let domain = domain
                    .as_deref()
                    .map(str::parse::<sessions::DomainId>)
                    .transpose()?;
                let registry = fsm_registry::builtin_registry()?;
                let listed = registry.list(domain.as_ref());
                println!("{}", serde_json::to_string_pretty(&listed)?);
            }
            FsmCommand::Show { fsm_id } => {
                let fsm_id = fsm_id.parse::<fsm_registry::FsmId>()?;
                let registry = fsm_registry::builtin_registry()?;
                let detail = registry.show(&fsm_id)?;
                println!("{}", serde_json::to_string_pretty(&detail)?);
            }
        },
        Commands::Scheduler { command } => {
            let registry = scheduler_registry::builtin_registry()?;
            match command {
                SchedulerCommand::List { domain, trigger } => {
                    let domain = domain
                        .as_deref()
                        .map(str::parse::<sessions::DomainId>)
                        .transpose()?;
                    let trigger = trigger
                        .as_deref()
                        .map(str::parse::<scheduler_registry::ScheduleTriggerKind>)
                        .transpose()?;
                    let listed = registry.list(domain.as_ref(), trigger.as_ref());
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
                SchedulerCommand::Show { scheduler_id } => {
                    let scheduler_id = scheduler_id.parse::<scheduler_registry::SchedulerId>()?;
                    let detail = registry.show(&scheduler_id)?;
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                }
            }
        }
        Commands::Run { command } => match command {
            RunCommand::Workflow { workflow_id } => {
                let workflow_id = workflow_id.parse::<workflow_registry::WorkflowId>()?;
                let result = execution_runtime::run_workflow(&cfg, &workflow_id)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
        },
        Commands::Desk { command } => {
            let registry = desk_registry::builtin_registry()?;
            match command {
                DeskCommand::List { category, domain } => {
                    let category = category
                        .as_deref()
                        .map(str::parse::<desk_registry::DeskCategory>)
                        .transpose()?;
                    let domain = domain
                        .as_deref()
                        .map(str::parse::<sessions::DomainId>)
                        .transpose()?;
                    let listed = registry.list(category.as_ref(), domain.as_ref());
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
                DeskCommand::Show { desk_id } => {
                    let desk_id = desk_id.parse::<desk_registry::DeskId>()?;
                    let detail = registry.show(&desk_id)?;
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                }
            }
        }
        Commands::Domain { command } => {
            let registry = domain::builtin_registry()?;
            match command {
                DomainCommand::List => {
                    let listed = registry.list();
                    println!("{}", serde_json::to_string_pretty(&listed)?);
                }
                DomainCommand::Show { domain_id } => {
                    let domain_id = domain_id.parse::<sessions::DomainId>()?;
                    let detail = registry.show(&domain_id)?;
                    println!("{}", serde_json::to_string_pretty(&detail)?);
                }
            }
        }
        Commands::Llm { command } => match command {
            LlmCommand::Ask { prompt } => {
                let response = llm::ask(&cfg, &prompt).await?;
                println!("{response}");
            }
        },
        Commands::Loop {
            dry_run,
            json,
            scope,
            max_candidates,
        } => {
            if !dry_run {
                anyhow::bail!("loop currently supports only --dry-run");
            }
            let scope = scope.parse::<loop_dry_run::LoopScope>()?;
            let report = loop_dry_run::run_loop_dry_run(
                &cfg,
                loop_dry_run::LoopDryRunRequest {
                    scope,
                    max_candidates,
                },
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", loop_dry_run::render_loop_report(&report));
            }
        }
        Commands::Telegram { command } => match command {
            TelegramCommand::Run => {
                let adapters = AdapterHub::new(&cfg)?;
                telegram::run_loop(cfg.clone(), adapters).await?;
            }
        },
        Commands::Channel { command } => match command {
            ChannelCommand::List { json } => {
                let items = channels::configured_channels(&cfg);
                if json {
                    println!("{}", serde_json::to_string_pretty(&items)?);
                } else {
                    for item in items {
                        println!(
                            "{} configured={} live_adapter={} notes={}",
                            item.label, item.configured, item.live_adapter, item.notes
                        );
                    }
                }
            }
        },
        Commands::Cockpit { command } => match command {
            CockpitCommand::Plan {
                host,
                repo_paths,
                models,
            } => {
                let host_platform = parse_cockpit_host(&host)?;
                let lane_inputs = build_cockpit_lane_inputs(repo_paths, models);
                let plan =
                    terminal_cockpit::plan_terminal_cockpit(&cfg, host_platform, lane_inputs);
                println!("{}", serde_json::to_string_pretty(&plan)?);
            }
        },
        Commands::Compact { session_id } => {
            let session_id = session_id.parse::<sessions::SessionId>()?;
            let result = compaction::compact_session(&cfg, &session_id)?;
            println!("{}", serde_json::to_string_pretty(&result)?);
        }
        Commands::ContextStatus { json } => {
            let report = context_status::context_status(&cfg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", context_status::render_context_status(&report));
            }
        }
        Commands::Context { command } => match command {
            ContextCommand::Packet {
                state,
                size,
                task,
                json,
            } => {
                let size = size.parse::<context_firewall::PacketSize>()?;
                let result = context_firewall::generate_context_packet(
                    &cfg,
                    context_firewall::ContextPacketRequest {
                        fsm_state: state,
                        size,
                        task,
                    },
                )?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&result)?);
                } else {
                    println!("packet_id: {}", result.packet_id);
                    println!("packet: {}", result.packet_path.display());
                    println!("receipt: {}", result.receipt_path.display());
                    println!(
                        "estimated_token_size: {}",
                        result.receipt.estimated_token_size
                    );
                }
            }
            ContextCommand::Guard { json, force, watch } => {
                if watch {
                    if json {
                        eprintln!(
                            "--json is ignored with --watch; guardian activity is written to the Quant-M log"
                        );
                    }
                    if force {
                        let report = context_guardian::tick_with_options(
                            &cfg,
                            context_guardian::GuardianTickOptions::force(),
                        )?;
                        print!("{}", context_guardian::render_guardian_report(&report));
                    }
                    context_guardian::run_loop_with_shutdown(cfg.clone(), None).await?;
                    return Ok(());
                }
                let report = context_guardian::tick_with_options(
                    &cfg,
                    context_guardian::GuardianTickOptions { force },
                )?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&report)?);
                } else {
                    print!("{}", context_guardian::render_guardian_report(&report));
                }
            }
        },
        Commands::Replay { session_id, json } => {
            let session_id = session_id.parse::<sessions::SessionId>()?;
            let summary = consensus::replay_consensus_session(&cfg, &session_id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&summary)?);
            } else {
                print!("{}", consensus::render_replay_summary(&summary));
            }
        }
        Commands::Consensus { dry_run, question } => {
            if !dry_run {
                anyhow::bail!("consensus currently supports --dry-run only");
            }
            let report = consensus::run_consensus_dry_run(&cfg, &question)?;
            print!("{}", consensus::render_terminal_summary(&report));
        }
        Commands::Council { command } => match command {
            CouncilCommand::Policy { json } => {
                let policy = council_router::default_policy();
                policy.validate()?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&policy)?);
                } else {
                    println!(
                        "Quant-M adaptive Council policy\npolicy_id: {}\nschema: {}\nroutes: {}\nmode: deterministic shadow only\nprovider_calls: none",
                        policy.policy_id,
                        policy.schema_version,
                        policy.routes.len()
                    );
                }
            }
            CouncilCommand::Shadow {
                input,
                json,
                record,
            } => {
                let packet = council_router::read_shadow_input(&input)?;
                let decision =
                    council_router::evaluate_shadow(packet, &council_router::default_policy())?;
                let record_path = if record {
                    Some(council_router::persist_decision_record(
                        &cfg.workspace_dir,
                        &decision,
                    )?)
                } else {
                    None
                };
                if json {
                    println!("{}", serde_json::to_string_pretty(&decision)?);
                } else {
                    print!(
                        "{}",
                        council_router::render_decision(&decision, record_path.as_deref())
                    );
                }
            }
        },
        Commands::Strategist { dry_run, json } => {
            if !dry_run {
                anyhow::bail!("strategist currently supports --dry-run only");
            }
            let report = strategist::run_strategist_dry_run(&cfg)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report.json_output())?);
            } else {
                print!("{}", strategist::render_terminal_summary(&report));
            }
        }
        Commands::Question { command } => match command {
            QuestionCommand::Ask {
                mode,
                question: text,
                json,
                write_proposals,
            } => {
                let mode = mode.parse::<question::QuantMQuestionMode>()?;
                match (mode, write_proposals) {
                    (question::QuantMQuestionMode::AgentCluster, true) => {
                        let plan = question::build_agent_cluster_proposal_plan(&text)?;
                        let result = question::write_agent_cluster_proposal_plan(&cfg, plan)?;
                        if json {
                            println!("{}", serde_json::to_string_pretty(&result)?);
                        } else {
                            print!("{}", question::render_agent_cluster_write_result(&result));
                        }
                    }
                    (question::QuantMQuestionMode::AgentCluster, false) => {
                        let plan = question::build_agent_cluster_proposal_plan(&text)?;
                        if json {
                            println!("{}", serde_json::to_string_pretty(&plan)?);
                        } else {
                            print!("{}", question::render_agent_cluster_proposal_plan(&plan));
                        }
                    }
                    (_, true) => {
                        anyhow::bail!(
                            "--write-proposals is currently supported only for --mode agent-cluster"
                        );
                    }
                    (_, false) => {
                        let question = question::build_question(mode, &text)?;
                        if json {
                            println!("{}", serde_json::to_string_pretty(&question)?);
                        } else {
                            print!("{}", question::render_question(&question));
                        }
                    }
                }
            }
        },
        Commands::Cost { command } => match command {
            CostCommand::Summary {
                json,
                workflow,
                session,
            } => {
                let summary =
                    cost_ledger::summarize_costs(&cfg, workflow.as_deref(), session.as_deref())?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&summary)?);
                } else {
                    print!("{}", cost_ledger::render_cost_summary(&summary));
                }
            }
        },
        Commands::Boil {
            args,
            json,
            dry_run,
            pricing_profile,
        } => run_boil_cli(&cfg, args, json, dry_run, pricing_profile)?,
        Commands::InitTruth { force, json } => {
            let report = truth_files::init_truth_files(&cfg, force)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", truth_files::render_truth_init_report(&report));
            }
        }
        Commands::State { command } => match command {
            StateCommand::Init => {
                state_sql::init_schema(&cfg)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "db_path": cfg.state_sql.sqlite_path
                    }))?
                );
            }
            StateCommand::Summary => {
                let summary = state_sql::summary(&cfg)?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            StateCommand::List { domain } => {
                let domain = domain
                    .as_deref()
                    .map(str::parse::<sessions::DomainId>)
                    .transpose()?;
                let listed = shared_state::list_state(&cfg, domain.as_ref())?;
                println!("{}", serde_json::to_string_pretty(&listed)?);
            }
            StateCommand::Show { key } => {
                let key = key.parse::<shared_state::SharedStateKey>()?;
                let record = shared_state::show_state(&cfg, &key)?;
                println!("{}", serde_json::to_string_pretty(&record)?);
            }
            StateCommand::Snapshot { domain } => {
                let domain = domain
                    .as_deref()
                    .map(str::parse::<sessions::DomainId>)
                    .transpose()?;
                let snapshot = shared_state::snapshot_state(&cfg, domain.as_ref())?;
                println!("{}", serde_json::to_string_pretty(&snapshot)?);
            }
            StateCommand::Review { domain, json } => {
                let review = state_review::review_state(&cfg, domain.as_deref())?;
                if json {
                    println!("{}", serde_json::to_string_pretty(&review)?);
                } else {
                    print!("{}", state_review::render_state_review(&review));
                }
            }
            StateCommand::ExpireStale => {
                let summary = shared_state::expire_stale_now(&cfg)?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            StateCommand::SignalUpsert { json } => {
                let input: state_sql::SharedSignalInput =
                    parse_json_input(&json, "state signal payload")?;
                state_sql::upsert_shared_signal(&cfg, &input)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "signal_id": input.signal_id
                    }))?
                );
            }
            StateCommand::HandoffAdd { json } => {
                let input: state_sql::DeskHandoffInput =
                    parse_json_input(&json, "state handoff payload")?;
                let inserted = state_sql::insert_handoff(&cfg, &input)?;
                println!("{}", serde_json::to_string_pretty(&inserted)?);
            }
            StateCommand::HandoffList { desk, limit } => {
                let rows = state_sql::list_handoffs(&cfg, desk.as_deref(), limit)?;
                println!("{}", serde_json::to_string_pretty(&rows)?);
            }
            StateCommand::RiskAdd { json } => {
                let input: state_sql::RiskReviewInput =
                    parse_json_input(&json, "risk review payload")?;
                let id = state_sql::insert_risk_review(&cfg, &input)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "id": id
                    }))?
                );
            }
            StateCommand::OrderAdd { json } => {
                let input: state_sql::PaperOrderInput =
                    parse_json_input(&json, "paper order payload")?;
                let id = state_sql::insert_paper_order(&cfg, &input)?;
                println!(
                    "{}",
                    serde_json::to_string_pretty(&serde_json::json!({
                        "status": "ok",
                        "id": id
                    }))?
                );
            }
            StateCommand::ForexIngest { json } => {
                let result = forex::ingest_stonex_payload(&cfg, &json)?;
                println!("{}", serde_json::to_string_pretty(&result)?);
            }
            StateCommand::ForexGetSignal { symbol } => {
                let value = forex::get_latest_signal(&cfg, &symbol)?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
            StateCommand::ForexGetHandoff { symbol } => {
                let value = forex::get_handoff(&cfg, &symbol)?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
            StateCommand::SwapHealth { json } => {
                let input: forex::SwapHealthInput = parse_json_input(&json, "swap health payload")?;
                let summary = forex::apply_swap_health(&cfg, &input)?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            StateCommand::SwapHealthGet { symbol } => {
                let value = forex::get_swap_health(&cfg, &symbol)?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
            StateCommand::MacroRefreshMql5 { hours_ahead } => {
                let summary = forex::refresh_mql5_macro(&cfg, hours_ahead).await?;
                println!("{}", serde_json::to_string_pretty(&summary)?);
            }
            StateCommand::MacroGetPair { pair } => {
                let value = forex::get_pair_macro_state(&cfg, &pair)?;
                println!("{}", serde_json::to_string_pretty(&value)?);
            }
        },
    }

    Ok(())
}

fn is_onboarding_command(command: &Commands) -> bool {
    matches!(
        command,
        Commands::Start
            | Commands::Init { .. }
            | Commands::Onboard { .. }
            | Commands::Setup { .. }
            | Commands::Onboarding { .. }
            | Commands::Config { .. }
            | Commands::Doctor { .. }
            | Commands::Provider { .. }
            | Commands::Tool { .. }
            | Commands::Settings { .. }
    )
}

async fn handle_onboarding_command(command: Commands, config_path: &std::path::Path) -> Result<()> {
    match command {
        Commands::Start => run_start_flow(config_path),
        Commands::Onboard { advanced, json } => {
            let report = run_setup_flow(
                config_path,
                SetupArgs {
                    non_interactive: false,
                    force_interactive: true,
                    advanced,
                    local_model_provider: None,
                    local_model: None,
                    local_models: Vec::new(),
                    remote_model_provider: None,
                    remote_model: None,
                    openrouter_models: Vec::new(),
                    openrouter_api_key: None,
                    enable_openrouter: false,
                    channel: None,
                    channel_value: None,
                    role: None,
                    runtime_profile: None,
                    workspace_path: None,
                    state_path: None,
                    session_path: None,
                    external_network: None,
                    context_guardian: None,
                    selected_tools: Vec::new(),
                    replace_tools: false,
                },
            )?;
            print_setup_report(&report, json)
        }
        Commands::Init {
            non_interactive,
            json,
        } => {
            let report = run_init_flow(config_path, non_interactive)?;
            print_serialized_or_text(
                &report,
                json,
                &format!(
                    "status: {}\nconfig: {}\nworkspace: {}\nstate_sqlite: {}\nsession_dir: {}\nrole: {}\nruntime_profile: {}",
                    report.status,
                    report.config.display(),
                    report.workspace.display(),
                    report.state_sqlite.display(),
                    report.session_dir.display(),
                    onboarding_role_label(report.role),
                    runtime_profile_label(report.runtime_profile),
                ),
            )
        }
        Commands::Setup {
            non_interactive,
            json,
            local_model_provider,
            local_model,
            remote_model_provider,
            remote_model,
            openrouter_models,
            openrouter_api_key,
            channel,
            channel_value,
            role,
            runtime_profile,
            workspace_path,
            state_path,
            session_path,
            external_network,
            context_guardian,
        } => {
            let report = run_setup_flow(
                config_path,
                SetupArgs {
                    non_interactive,
                    force_interactive: false,
                    advanced: false,
                    local_model_provider,
                    local_model,
                    local_models: Vec::new(),
                    remote_model_provider,
                    remote_model,
                    openrouter_models,
                    openrouter_api_key,
                    enable_openrouter: false,
                    channel,
                    channel_value,
                    role,
                    runtime_profile,
                    workspace_path,
                    state_path,
                    session_path,
                    external_network,
                    context_guardian,
                    selected_tools: Vec::new(),
                    replace_tools: false,
                },
            )?;
            print_setup_report(&report, json)
        }
        Commands::Config { command } => handle_config_command(config_path, command),
        Commands::Onboarding { command } => handle_onboarding_status_command(config_path, command),
        Commands::Doctor {
            json,
            providers,
            live,
        } => {
            let report = run_doctor(config_path, providers, live).await?;
            let provider_summary = if providers {
                format!(
                    "\nprovider_diagnostics: {}",
                    report.provider_diagnostics.len()
                )
            } else {
                String::new()
            };
            let summary = format!(
                "role: {}\nruntime_profile: {}\nconfig_exists: {}\nworkspace_exists: {}\nstate_path_exists: {}\nsession_path_exists: {}\nworkflow_run_ok: {}\nshared_state_list_ok: {}\nsession_list_ok: {}\nchecked_binary: {}\ngenerated_session_id: {}{}",
                onboarding_role_label(report.role),
                runtime_profile_label(report.runtime_profile),
                report.config_exists,
                report.workspace_exists,
                report.state_path_exists,
                report.session_path_exists,
                report.workflow_run_ok,
                report.shared_state_list_ok,
                report.session_list_ok,
                report.checked_binary.display(),
                report.generated_session_id.as_deref().unwrap_or("none"),
                provider_summary,
            );
            if !(report.config_exists
                && report.workspace_exists
                && report.state_path_exists
                && report.session_path_exists
                && report.workflow_run_ok
                && report.shared_state_list_ok
                && report.session_list_ok)
            {
                print_serialized_or_text(&report, json, &summary)?;
                anyhow::bail!("doctor checks failed");
            }
            print_serialized_or_text(&report, json, &summary)
        }
        Commands::Provider { command } => handle_provider_command(config_path, command).await,
        Commands::Tool { command } => handle_tool_command(config_path, command),
        Commands::Settings { json } => handle_settings_command(config_path, json),
        _ => unreachable!("not an onboarding command"),
    }
}

fn handle_onboarding_status_command(
    config_path: &std::path::Path,
    command: OnboardingCommand,
) -> Result<()> {
    match command {
        OnboardingCommand::Status { json } => {
            let mut cfg = Config::load_or_create(config_path)
                .with_context(|| format!("failed loading config {}", config_path.display()))?;
            cfg.ensure_onboarding_registries();
            cfg.save(config_path)?;
            let provider_route_status = provider_route_status(&cfg);
            let write_status = onboarding_workspace_write_status(&cfg, config_path);
            let exit = onboarding_router::decide_next_action(
                cfg.runtime.role,
                write_status,
                provider_route_status,
            );
            let report = OnboardingStatusReport {
                config: config_path.to_path_buf(),
                workspace: cfg.workspace_dir.clone(),
                onboarding_completed: cfg.preferences.onboarding_completed,
                role: cfg.runtime.role,
                runtime_profile: cfg.runtime.profile,
                next_action: exit.next_action,
                provider_route_status,
            };
            print_serialized_or_text(
                &report,
                json,
                &format!(
                    "onboarding_completed: {}\nrole: {}\nruntime_profile: {}\nworkspace: {}\nnext_action: {:?}\nprovider_route_status: {:?}",
                    report.onboarding_completed,
                    onboarding_role_label(report.role),
                    runtime_profile_label(report.runtime_profile),
                    report.workspace.display(),
                    report.next_action,
                    report.provider_route_status,
                ),
            )
        }
    }
}

fn run_start_flow(config_path: &std::path::Path) -> Result<()> {
    if start_needs_onboarding(config_path)? {
        if !(io::stdin().is_terminal() && io::stdout().is_terminal()) {
            anyhow::bail!(
                "first-run onboarding has not been completed for this project; run `./quantm onboard` in an interactive terminal, or run `./quantm setup --non-interactive` for safe defaults"
            );
        }
        println!("First-run onboarding has not been completed for this project.");
        let report = run_setup_flow(config_path, default_interactive_setup_args())?;
        print_setup_report(&report, false)?;
    }

    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    cfg.save(config_path)?;
    cfg.validate()?;
    bootstrap::ensure_workspace(&cfg)?;

    let write_status = onboarding_workspace_write_status(&cfg, config_path);
    let provider_status = provider_route_status(&cfg);
    let report =
        onboarding_router::decide_next_action(cfg.runtime.role, write_status, provider_status);

    print_quant_m_brand_banner();
    dispatch_onboarding_next_action(&cfg, config_path, &report)
}

fn dispatch_onboarding_next_action(
    cfg: &Config,
    config_path: &Path,
    report: &onboarding_router::OnboardingExitReport,
) -> Result<()> {
    match &report.next_action {
        onboarding_router::OnboardingNextAction::OpenSoloChat => {
            let _truth_report = truth_files::init_truth_files(cfg, false)?;
            println!("{}", start_chat_message(&cfg.workspace_dir));
            tui_shell::run_chat(cfg, config_path, false)
        }
        onboarding_router::OnboardingNextAction::ShowProviderSetup => {
            println!("{}", provider_setup_next_steps(cfg));
            Ok(())
        }
        onboarding_router::OnboardingNextAction::OpenCorePairing => {
            println!("{}", core_pairing_next_steps(cfg));
            Ok(())
        }
        onboarding_router::OnboardingNextAction::OpenChildJoin => {
            println!("{}", child_join_next_steps(cfg));
            Ok(())
        }
        onboarding_router::OnboardingNextAction::OpenStaffWorkerHandoff => {
            println!("{}", staff_worker_next_steps(cfg));
            Ok(())
        }
        onboarding_router::OnboardingNextAction::OpenServerHeadlessSetup => {
            println!("{}", server_headless_next_steps(cfg));
            Ok(())
        }
        onboarding_router::OnboardingNextAction::BlockedReadOnlyWorkspace => {
            println!("{}", read_only_workspace_next_steps(report));
            Ok(())
        }
        onboarding_router::OnboardingNextAction::ShowDoctor => {
            println!("Quant-M needs a doctor check before opening a stateful surface.");
            println!("next: {} doctor", quant_m_command_hint());
            Ok(())
        }
    }
}

fn default_interactive_setup_args() -> SetupArgs {
    SetupArgs {
        non_interactive: false,
        force_interactive: false,
        advanced: false,
        local_model_provider: None,
        local_model: None,
        local_models: Vec::new(),
        remote_model_provider: None,
        remote_model: None,
        openrouter_models: Vec::new(),
        openrouter_api_key: None,
        enable_openrouter: false,
        channel: None,
        channel_value: None,
        role: None,
        runtime_profile: None,
        workspace_path: None,
        state_path: None,
        session_path: None,
        external_network: None,
        context_guardian: None,
        selected_tools: Vec::new(),
        replace_tools: false,
    }
}

fn start_chat_message(workspace: &Path) -> String {
    format!(
        "Quant-M ready.\nworkspace: {}\n\nOpening the governed Quant-M chat cockpit. Type /help, /state, /cost, /ask, or /quit.\nUse `quant-m shell` for the classic text shell.\n",
        workspace.display()
    )
}

fn start_needs_onboarding(config_path: &std::path::Path) -> Result<bool> {
    if !config_path.exists() {
        return Ok(true);
    }
    let cfg = Config::load_existing(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    Ok(!cfg.preferences.onboarding_completed)
}

fn provider_route_status(cfg: &Config) -> onboarding_router::ProviderRouteStatus {
    if tui_shell::selected_chat_tool(cfg).is_some() {
        onboarding_router::ProviderRouteStatus::Available
    } else {
        onboarding_router::ProviderRouteStatus::Missing
    }
}

fn onboarding_workspace_write_status(
    cfg: &Config,
    config_path: &Path,
) -> onboarding_router::WorkspaceWriteStatus {
    let checks = workspace_write_checks(cfg, config_path);
    for (path, operation) in checks {
        if let Err(err) = probe_write_path(&path) {
            return onboarding_router::WorkspaceWriteStatus::ReadOnly {
                path,
                operation,
                message: err.to_string(),
            };
        }
    }
    onboarding_router::WorkspaceWriteStatus::Writable
}

fn workspace_write_checks(cfg: &Config, config_path: &Path) -> Vec<(PathBuf, &'static str)> {
    let mut checks = vec![
        (config_path.to_path_buf(), "write onboarding config"),
        (cfg.runtime.session_dir.clone(), "write session state"),
        (cfg.workspace_dir.join("evidence"), "write evidence records"),
        (cfg.logging.file.clone(), "write audit log"),
    ];

    match cfg.runtime.role {
        config::OnboardingRole::AgentClusterCore => checks.push((
            cfg.workspace_dir.join("state/pairing/invites"),
            "write pairing invite registry",
        )),
        config::OnboardingRole::AgentClusterChildWorker => checks.push((
            cfg.workspace_dir.join("state/child/identity.toml"),
            "write child identity",
        )),
        _ => {}
    }

    checks
}

fn probe_write_path(path: &Path) -> Result<()> {
    if path.extension().is_some() {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        if path.exists() {
            fs::OpenOptions::new()
                .append(true)
                .open(path)
                .with_context(|| format!("failed to open {}", path.display()))?;
        } else if let Some(parent) = path.parent() {
            probe_write_directory(parent)?;
        }
        return Ok(());
    }

    probe_write_directory(path)
}

fn probe_write_directory(path: &Path) -> Result<()> {
    fs::create_dir_all(path).with_context(|| format!("failed to create {}", path.display()))?;
    let probe = path.join(".quantm-write-check");
    fs::write(&probe, b"ok").with_context(|| format!("failed to write {}", probe.display()))?;
    fs::remove_file(&probe).with_context(|| format!("failed to remove {}", probe.display()))
}

fn provider_setup_next_steps(cfg: &Config) -> String {
    let command = quant_m_command_hint();
    format!(
        "No chat-capable provider or local CLI route is available yet.\nworkspace: {}\nrole: {}\n\nChat was not opened.\nNext:\n  {command} provider list\n  {command} tool scan\n  {command} onboard\n  {command} doctor\n\nDetection is not permission. Provider or CLI calls remain disabled until explicitly configured.",
        cfg.workspace_dir.display(),
        onboarding_role_label(cfg.runtime.role),
    )
}

fn core_pairing_next_steps(cfg: &Config) -> String {
    let command = quant_m_command_hint();
    format!(
        "Agent Cluster core selected.\nworkspace: {}\n\nChat was not opened because this role pairs child devices first.\n\nNext:\n  {command} pair doctor\n  {command} pair cockpit\n  {command} device add --qr\n  {command} device add --watch\n  {command} bootstrap serve --bind 0.0.0.0:8788 --bundle-dir ./release-bundles --core-url http://<core-wifi-or-lan-ip>:8787\n\nNetwork:\n  same trusted local network required\n  Wi-Fi is supported\n  Ethernet is optional\n\nSafety:\n  children remain observe-only\n  child provider calls are blocked\n  child canonical writes are blocked\n  child execution authority is blocked",
        cfg.workspace_dir.display()
    )
}

fn child_join_next_steps(cfg: &Config) -> String {
    let command = quant_m_command_hint();
    format!(
        "Agent Cluster child worker selected.\nworkspace: {}\n\nChat was not opened because this role joins a core first.\n\nNext:\n  {command} child join\n  {command} child join --url <core-local-url>\n\nTermux/manual fallback:\n  open the core bootstrap URL or paste the join URL in Termux\n  download the prebuilt child binary when available\n  verify SHA-256 before pairing\n\nSafety:\n  child stores no provider keys\n  child remains observe-only\n  no approval, execution, provider-call, or canonical-write authority is granted",
        cfg.workspace_dir.display()
    )
}

fn staff_worker_next_steps(cfg: &Config) -> String {
    let command = quant_m_command_hint();
    format!(
        "Staff-OS worker selected.\nworkspace: {}\n\nChat was not opened by default for this worker role.\n\nNext:\n  {command} worker proposal list\n  {command} question staff-os-handoff --help\n  {command} doctor",
        cfg.workspace_dir.display()
    )
}

fn server_headless_next_steps(cfg: &Config) -> String {
    let command = quant_m_command_hint();
    format!(
        "Server/VPS node selected.\nworkspace: {}\n\nChat was not opened because this role should start headless-friendly setup first.\n\nNext:\n  {command} onboarding status\n  {command} provider list\n  {command} tool scan\n  {command} doctor\n\nUse explicit commands for chat or TUI after provider and workspace checks pass.",
        cfg.workspace_dir.display()
    )
}

fn read_only_workspace_next_steps(report: &onboarding_router::OnboardingExitReport) -> String {
    let command = quant_m_command_hint();
    match &report.workspace_write_status {
        onboarding_router::WorkspaceWriteStatus::ReadOnly {
            path,
            operation,
            message,
        } => format!(
            "Quant-M cannot open a stateful surface because the workspace/config is not writable.\nfailed_path: {}\nfailed_operation: {}\nerror: {}\n\nChat was not opened.\nPairing was not started.\n\nSafe next commands:\n  {command} doctor\n  {command} setup --workspace-path ./workspace\n",
            path.display(),
            operation,
            message,
        ),
        onboarding_router::WorkspaceWriteStatus::Writable => {
            format!("Quant-M write preflight passed unexpectedly. Run `{command} doctor`.")
        }
    }
}

const QUANT_M_ASCII_BANNER: &[&str] = &[
    "██████╗ ██╗   ██╗ █████╗ ███╗   ██╗████████╗      ███╗   ███╗",
    "██╔═══██╗██║   ██║██╔══██╗████╗  ██║╚══██╔══╝      ████╗ ████║",
    "██║   ██║██║   ██║███████║██╔██╗ ██║   ██║   █████╗██╔████╔██║",
    "██║▄▄ ██║██║   ██║██╔══██║██║╚██╗██║   ██║   ╚════╝██║╚██╔╝██║",
    "╚██████╔╝╚██████╔╝██║  ██║██║ ╚████║   ██║         ██║ ╚═╝ ██║",
    " ╚══▀▀═╝  ╚═════╝ ╚═╝  ╚═╝╚═╝  ╚═══╝   ╚═╝         ╚═╝     ╚═╝",
];

fn print_quant_m_brand_banner() {
    if !io::stdout().is_terminal() {
        return;
    }
    let color_enabled = env::var_os("NO_COLOR").is_none();
    let width = terminal_width();
    if width < 78 {
        if color_enabled {
            println!("{ANSI_BOLD}{ANSI_BLUE}Quant-M{ANSI_RESET}");
            println!("{ANSI_CYAN}Local-first governed AI work.{ANSI_RESET}");
        } else {
            println!("Quant-M");
            println!("Local-first governed AI work.");
        }
        println!();
        return;
    }

    let banner_width = QUANT_M_ASCII_BANNER
        .iter()
        .map(|line| line.chars().count())
        .max()
        .unwrap_or(1);
    println!();
    for line in QUANT_M_ASCII_BANNER {
        if color_enabled {
            println!("{}", render_gradient_line(line, banner_width));
        } else {
            println!("{line}");
        }
    }
    if color_enabled {
        println!();
        println!("{ANSI_CYAN}Local-first Rust control plane for governed AI work.{ANSI_RESET}");
        println!(
            "{ANSI_BLUE}Evidence | replay | FSM authority | side-effect gates | safe defaults{ANSI_RESET}"
        );
    } else {
        println!();
        println!("Local-first Rust control plane for governed AI work.");
        println!("Evidence | replay | FSM authority | side-effect gates | safe defaults");
    }
    println!();
}

fn terminal_width() -> usize {
    env::var("COLUMNS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|width| *width > 0)
        .unwrap_or(80)
}

fn render_gradient_line(line: &str, width: usize) -> String {
    let mut rendered = String::new();
    for (index, ch) in line.chars().enumerate() {
        if ch == ' ' {
            rendered.push(ch);
            continue;
        }
        let denominator = width.saturating_sub(1).max(1) as f32;
        let t = index as f32 / denominator;
        let (red, green, blue) = gradient_rgb(t);
        rendered.push_str(&format!("\x1b[38;2;{red};{green};{blue}m{ch}{ANSI_RESET}"));
    }
    rendered
}

fn gradient_rgb(t: f32) -> (u8, u8, u8) {
    let start = (95.0, 215.0, 255.0);
    let middle = (0.0, 140.0, 255.0);
    let end = (20.0, 80.0, 255.0);
    let (from, to, local_t) = if t < 0.5 {
        (start, middle, t / 0.5)
    } else {
        (middle, end, (t - 0.5) / 0.5)
    };
    (
        lerp_color(from.0, to.0, local_t),
        lerp_color(from.1, to.1, local_t),
        lerp_color(from.2, to.2, local_t),
    )
}

fn lerp_color(from: f32, to: f32, t: f32) -> u8 {
    (from.mul_add(1.0 - t, to * t).round()).clamp(0.0, 255.0) as u8
}

#[derive(Debug, Clone)]
struct SetupArgs {
    non_interactive: bool,
    force_interactive: bool,
    advanced: bool,
    local_model_provider: Option<String>,
    local_model: Option<String>,
    local_models: Vec<String>,
    remote_model_provider: Option<String>,
    remote_model: Option<String>,
    openrouter_models: Vec<String>,
    openrouter_api_key: Option<String>,
    enable_openrouter: bool,
    channel: Option<String>,
    channel_value: Option<String>,
    role: Option<String>,
    runtime_profile: Option<String>,
    workspace_path: Option<PathBuf>,
    state_path: Option<PathBuf>,
    session_path: Option<PathBuf>,
    external_network: Option<String>,
    context_guardian: Option<String>,
    selected_tools: Vec<String>,
    replace_tools: bool,
}

fn print_setup_report(report: &SetupReport, json: bool) -> Result<()> {
    let local_model = format_model_pref(report.preferred_local_model.as_ref());
    let remote_model = format_model_pref(report.preferred_remote_model.as_ref());
    let channel = format_channel_pref(&report.preferred_channel);
    let command = quant_m_command_hint();
    let openrouter_model = report
        .preferred_openrouter_model
        .as_deref()
        .unwrap_or("unset");
    let tools = if report.enabled_tools.is_empty() {
        "none".to_string()
    } else {
        report.enabled_tools.join(", ")
    };
    let tool_validation_next = if report.enabled_tools.is_empty() {
        String::new()
    } else {
        report
            .enabled_tools
            .iter()
            .map(|tool| format!("  {command} tool validate {tool}\n"))
            .collect::<String>()
    };
    print_serialized_or_text(
        report,
        json,
        &format!(
            "Setup complete.\nconfig: {}\nworkspace: {}\nrole: {}\ndevice_type: {}\nnetwork: {}\noperator_channel: {}\ntools: {}\nmulti_model: {}\nsearch: {}\nbrowser_harness: {}\ncontext_guardian: {}\nlocal_model: {}\nremote_model: {}\nopenrouter_model: {}\nopenrouter_key_present: {}\n\nNext:\n  {command}\n  {command} onboarding status\n  {command} doctor\n{}  {command} shell\n\n`{command}` routes to the next surface for the selected role. Chat is explicit and opens only when a solo node has a writable workspace and a valid chat route.",
            report.config.display(),
            report.workspace.display(),
            onboarding_role_label(report.role),
            runtime_profile_label(report.runtime_profile),
            if report.external_network_enabled {
                "enabled"
            } else {
                "local only / ask before live checks"
            },
            channel,
            tools,
            enabled_label(report.multi_model_enabled),
            enabled_label(report.search_enabled),
            enabled_label(report.browser_harness_enabled),
            enabled_label(report.context_guardian_enabled),
            local_model,
            remote_model,
            openrouter_model,
            report.openrouter_key_present,
            tool_validation_next,
        ),
    )
}

fn run_init_flow(config_path: &std::path::Path, _non_interactive: bool) -> Result<InitReport> {
    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    let cfg = cfg.sanitize();
    cfg.validate()?;
    cfg.save(config_path)?;
    bootstrap::ensure_workspace(&cfg)?;
    Ok(InitReport {
        status: "ok".to_string(),
        config: config_path.to_path_buf(),
        workspace: cfg.workspace_dir.clone(),
        state_sqlite: cfg.state_sql.sqlite_path.clone(),
        session_dir: cfg.runtime.session_dir.clone(),
        role: cfg.runtime.role,
        runtime_profile: cfg.runtime.profile,
    })
}

fn run_setup_flow(config_path: &std::path::Path, args: SetupArgs) -> Result<SetupReport> {
    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    let args = if args.force_interactive && !io::stdin().is_terminal() {
        anyhow::bail!(
            "onboard requires an interactive terminal; use setup --non-interactive for scripts"
        );
    } else if args.non_interactive || !io::stdin().is_terminal() {
        if !args.non_interactive && !io::stdin().is_terminal() {
            eprintln!("setup: stdin is not interactive; using safe defaults");
        }
        args
    } else {
        run_interactive_setup(config_path, args, &cfg)?
    };

    if let Some(workspace_path) = args.workspace_path {
        rebase_workspace_paths(&mut cfg, workspace_path);
    }
    if let Some(state_path) = args.state_path {
        cfg.state_sql.sqlite_path = state_path;
    }
    if let Some(session_path) = args.session_path {
        cfg.runtime.session_dir = session_path;
    }
    if let Some(runtime_profile) = args.runtime_profile {
        cfg.runtime.profile = runtime_profile.parse()?;
    }
    if let Some(role) = args.role {
        cfg.runtime.role = role.parse()?;
    }
    if let Some(external_network) = args.external_network {
        cfg.runtime.external_network_enabled = parse_enabled_disabled(&external_network)?;
    }
    if let Some(context_guardian) = args.context_guardian {
        cfg.context_guardian.enabled = parse_enabled_disabled(&context_guardian)?;
    }
    if args.enable_openrouter {
        enable_provider(&mut cfg, "openrouter");
        cfg.llm.api_base = "https://openrouter.ai/api/v1".to_string();
        cfg.llm.api_key_env = "OPENROUTER_API_KEY".to_string();
    }
    let openrouter_models = normalize_model_list(args.openrouter_models);
    if !openrouter_models.is_empty() {
        enable_provider(&mut cfg, "openrouter");
        cfg.preferences.preferred_openrouter_model = openrouter_models.first().cloned();
        cfg.llm.api_base = "https://openrouter.ai/api/v1".to_string();
        if let Some(first) = openrouter_models.first() {
            cfg.llm.model = first.clone();
        }
        if let Some(provider) = cfg.providers.get_mut("openrouter") {
            provider.preferred_models = openrouter_models;
        }
    }
    if let Some(openrouter_api_key) = args.openrouter_api_key {
        let trimmed = openrouter_api_key.trim();
        if !trimmed.is_empty() {
            enable_provider(&mut cfg, "openrouter");
            cfg.llm.enabled = true;
            cfg.llm.api_base = "https://openrouter.ai/api/v1".to_string();
            cfg.llm.api_key_env = "OPENROUTER_API_KEY".to_string();
            cfg.llm.api_key = Some(trimmed.to_string());
        }
    }

    let local_models = normalize_model_list(args.local_models);
    match (args.local_model_provider, args.local_model) {
        (Some(provider), Some(model)) => {
            let provider_id = normalize_registry_id(provider.trim());
            enable_provider(&mut cfg, &provider_id);
            cfg.preferences.preferred_local_model = Some(config::ModelPreference {
                provider: provider_id.clone(),
                model: model.trim().to_string(),
            });
            if !local_models.is_empty()
                && let Some(provider) = cfg.providers.get_mut(&provider_id)
            {
                provider.preferred_models = local_models;
            }
        }
        (None, None) => {}
        _ => anyhow::bail!("--local-model-provider and --local-model must be provided together"),
    }

    match (args.remote_model_provider, args.remote_model) {
        (Some(provider), Some(model)) => {
            enable_provider(&mut cfg, provider.trim());
            cfg.preferences.preferred_remote_model = Some(config::ModelPreference {
                provider: provider.trim().to_string(),
                model: model.trim().to_string(),
            });
        }
        (None, None) => {}
        _ => anyhow::bail!("--remote-model-provider and --remote-model must be provided together"),
    }

    match (args.channel, args.channel_value) {
        (Some(channel), value) => {
            let channel = channel.parse::<config::ExternalChannel>()?;
            cfg.set_channel_preference(channel, value.as_deref())?;
        }
        (None, Some(_)) => anyhow::bail!("--channel-value requires --channel"),
        (None, None) => {}
    }
    if args.replace_tools {
        disable_all_tools(&mut cfg);
    }
    for tool in &args.selected_tools {
        enable_tool(&mut cfg, tool);
    }
    cfg.preferences.preferred_chat_tool = args
        .selected_tools
        .iter()
        .find(|tool| is_chat_capable_tool(tool))
        .cloned();

    let mut cfg = cfg.sanitize();
    cfg.validate()?;
    bootstrap::ensure_workspace(&cfg).with_context(|| {
        format!(
            "failed to prepare workspace '{}'; choose a writable project folder such as ./workspace or ~/Quant-M/workspace",
            cfg.workspace_dir.display()
        )
    })?;
    cfg.preferences.onboarding_completed = true;
    cfg.save(config_path)?;
    Ok(SetupReport {
        status: if args.non_interactive {
            "ok_non_interactive".to_string()
        } else {
            "ok".to_string()
        },
        config: config_path.to_path_buf(),
        workspace: cfg.workspace_dir.clone(),
        state_sqlite: cfg.state_sql.sqlite_path.clone(),
        session_dir: cfg.runtime.session_dir.clone(),
        role: cfg.runtime.role,
        runtime_profile: cfg.runtime.profile,
        external_network_enabled: cfg.runtime.external_network_enabled,
        multi_model_enabled: cfg.runtime.multi_model_enabled,
        search_enabled: cfg.runtime.search_enabled,
        browser_harness_enabled: cfg.runtime.browser_harness_enabled,
        context_guardian_enabled: cfg.context_guardian.enabled,
        preferred_channel: cfg.preferences.preferred_channel.clone(),
        preferred_local_model: cfg.preferences.preferred_local_model.clone(),
        preferred_remote_model: cfg.preferences.preferred_remote_model.clone(),
        preferred_openrouter_model: cfg.preferences.preferred_openrouter_model.clone(),
        openrouter_key_present: provider_key_present(&cfg, "openrouter")
            || cfg.resolve_llm_api_key().is_some(),
        provider_count: cfg.providers.len(),
        tool_count: cfg.tools.len(),
        enabled_tools: enabled_tool_ids(&cfg),
    })
}

fn run_interactive_setup(
    config_path: &std::path::Path,
    args: SetupArgs,
    cfg: &Config,
) -> Result<SetupArgs> {
    let base_args = args.clone();
    let mut args = args;

    print_quant_m_brand_banner();
    println!(
        "{}{}🧠 Welcome to Quant-M{}",
        ANSI_BOLD, ANSI_BLUE, ANSI_RESET
    );
    println!();
    println!(
        "{}Quant-M is a local-first Rust runtime for governed agent work.{}",
        ANSI_BOLD, ANSI_RESET
    );
    println!("It stores memory, sessions, shared state, and replay evidence on this device.");
    println!(
        "{}This device can run as a Quant-M node; choose its role by capability, not hardware.{}",
        ANSI_DIM, ANSI_RESET
    );
    println!();

    if args.workspace_path.is_none() {
        print_onboarding_section(
            "1",
            "Workspace",
            "Choose where local memory and sessions live.",
        );
        let default = "./workspace";
        let answer = prompt_workspace_path(
            "Where should Quant-M store its local memory, state, sessions, and queues?",
            default,
        )?;
        args.workspace_path = Some(PathBuf::from(answer));
    }

    if args.role.is_none() {
        print_onboarding_section(
            "2",
            "Role",
            "Choose what this node should do before choosing its hardware class.",
        );
        args.role = Some(prompt_numbered_choice(
            "How should this device participate in Quant-M?",
            &[
                ("🧭 Solo local node", "solo-local-node"),
                ("🛰️ Agent Cluster core", "agent-cluster-core"),
                (
                    "📡 Agent Cluster child worker",
                    "agent-cluster-child-worker",
                ),
                ("🏢 Staff-OS worker", "staff-os-worker"),
                ("🖥️ Server/VPS node", "server-vps-node"),
            ],
            1,
        )?);
    }

    if args.runtime_profile.is_none() {
        print_onboarding_section(
            "3",
            "Device",
            "Pick the closest runtime profile after deciding this node's role.",
        );
        args.runtime_profile = Some(prompt_numbered_choice(
            "Choose this device class.",
            &[
                ("💻 Laptop or desktop", "laptop"),
                ("📱 Android/Termux or small edge device", "edge"),
                ("🏢 Staff-OS worker node", "staff-os-worker"),
                ("🖥️ VPS / server", "vps"),
            ],
            1,
        )?);
    }

    if args.external_network.is_none() {
        print_onboarding_section("4", "Network", "Keep first-run safe unless you opt in.");
        let network_mode = prompt_numbered_choice(
            "Should Quant-M use the internet?",
            &[
                ("🔒 No, local only", "disabled"),
                ("✋ Ask me before network use", "explicit"),
                ("🌐 Yes, allow provider checks", "enabled"),
            ],
            1,
        )?;
        args.external_network = Some(if network_mode == "enabled" {
            "enabled".to_string()
        } else {
            "disabled".to_string()
        });
        if network_mode == "explicit" {
            println!("Quant-M will stay local by default and ask before live provider checks.");
        }
    }

    let model_provider = if setup_role_is_child(&args) {
        print_onboarding_section(
            "5",
            "Models",
            "Child workers do not store provider keys or configure provider calls.",
        );
        println!(
            "{}Child role selected: skipping provider keys and chat model setup.{}",
            ANSI_DIM, ANSI_RESET
        );
        "skip".to_string()
    } else {
        print_onboarding_section(
            "5",
            "Models",
            "Quant-M can run with no model selected. Remote and local models are optional.",
        );
        let model_provider = prompt_numbered_choice(
            "Do you have an OpenRouter API key?",
            &[
                ("⏭️ No, none for now", "skip"),
                ("🔑 Yes, use OPENROUTER_API_KEY from my environment", "env"),
                ("💾 Yes, paste and save a key locally", "save"),
                ("📋 Not yet, show me the export command", "export"),
            ],
            1,
        )?;
        match model_provider.as_str() {
            "skip" => {}
            "env" => {
                args.enable_openrouter = true;
                println!("Quant-M will read OPENROUTER_API_KEY from your environment when needed.");
            }
            "save" => {
                println!(
                    "This stores the key in local quant-m.toml. Prefer env vars on shared machines."
                );
                let key = prompt_secret_like("Paste OpenRouter API key to save locally")?;
                if !key.trim().is_empty() {
                    args.openrouter_api_key = Some(key);
                }
            }
            "export" => {
                println!("Run: export OPENROUTER_API_KEY='<your-openrouter-key>'");
            }
            _ => unreachable!("choice is constrained"),
        }
        model_provider
    };

    let detected_local_providers = if setup_role_is_child(&args) {
        Vec::new()
    } else {
        detected_local_provider_options()
    };
    if !detected_local_providers.is_empty() {
        println!();
        println!(
            "{}{}Detected local model(s){}",
            ANSI_BOLD, ANSI_GREEN, ANSI_RESET
        );
        for (provider, note) in &detected_local_providers {
            println!(
                "  {}{}{}   {} {}({}){}",
                ANSI_CYAN,
                provider,
                ANSI_RESET,
                local_provider_display(provider),
                ANSI_DIM,
                note,
                ANSI_RESET
            );
        }
        println!(
            "{}Detection checks common macOS, Windows, and Linux model directories. It does not start a model server or grant execution permission.{}",
            ANSI_DIM, ANSI_RESET
        );
    }

    let has_local_models = if setup_role_is_child(&args) {
        false
    } else if detected_local_providers.is_empty() {
        prompt_yes_no("Do you have local model(s) available?", false)?
    } else {
        prompt_yes_no("Use detected local model(s)?", true)?
    };
    if has_local_models {
        println!(
            "{}Local models stay on this machine. Quant-M records the provider/model names only; it does not start a model server.{}",
            ANSI_DIM, ANSI_RESET
        );
        let local_provider = if detected_local_providers.is_empty() {
            prompt_choice(
                "Local model provider [ollama/lmstudio]",
                "ollama",
                &["ollama", "lmstudio"],
            )?
        } else if detected_local_providers.len() == 1 {
            detected_local_providers[0].0.clone()
        } else {
            prompt_detected_local_provider(&detected_local_providers)?
        };
        let local_models = prompt_local_models(&local_provider)?;
        if let Some(first) = local_models.first() {
            args.local_model_provider = Some(local_provider);
            args.local_model = Some(first.clone());
            args.local_models = local_models;
        }
    } else if !setup_role_is_child(&args) {
        println!(
            "{}No local model selected. You can add one later with `quant-m setup --local-model-provider ollama --local-model <name>` or clear stale choices with `quant-m config clear-model`.{}",
            ANSI_DIM, ANSI_RESET
        );
    }

    if args.advanced {
        println!();
        println!(
            "{}{}⚙ Advanced model routing{}",
            ANSI_BOLD, ANSI_MAGENTA, ANSI_RESET
        );
        if !matches!(model_provider.as_str(), "skip") && args.openrouter_models.is_empty() {
            args.openrouter_models = prompt_openrouter_models()?;
        }
        let validation = prompt_choice(
            "Provider validation posture [local/live/none]",
            "local",
            &["local", "live", "none"],
        )?;
        if validation == "live" {
            println!(
                "Live validation is explicit. Run provider validate <provider> --live after setup."
            );
        }
        let harness = prompt_choice(
            "Optional companion runtime to recognize [hermes/pi-agent/openclaw/skip]",
            "skip",
            &["hermes", "pi-agent", "openclaw", "skip"],
        )?;
        if harness == "openclaw" {
            println!("🦞 OpenClaw recognized.");
        }
        if harness != "skip" {
            enable_tool_arg(&mut args, &harness);
        }
    }

    print_onboarding_section(
        "6",
        "Developer tools",
        "Choose optional CLIs Quant-M should recognize. Detection does not grant execution permission.",
    );
    let selected_tools = prompt_developer_tools(cfg)?;
    args.replace_tools = true;
    if selected_tools.is_empty() {
        let command = quant_m_command_hint();
        println!(
            "{}No CLI tools selected.{} You can add them later with `{command} onboard` or `{command} tool scan`.",
            ANSI_DIM, ANSI_RESET
        );
    } else {
        for tool in &selected_tools {
            enable_tool_arg(&mut args, tool);
        }
        let command = quant_m_command_hint();
        println!(
            "{}{}✓ Tool preferences:{} {}",
            ANSI_BOLD,
            ANSI_GREEN,
            ANSI_RESET,
            selected_tools
                .iter()
                .map(|tool| developer_tool_display(tool))
                .collect::<Vec<_>>()
                .join(", ")
        );
        println!(
            "{}Detection is not permission. Shell-backed use still requires config and policy.{}",
            ANSI_DIM, ANSI_RESET
        );
        println!("Validate any selected tool later with: {command} tool validate <tool>");
    }

    if args.channel.is_none() {
        print_onboarding_section(
            "7",
            "Operator channel",
            "Choose the default way Quant-M talks to you.",
        );
        let channel = prompt_numbered_choice(
            "How should Quant-M talk to you?",
            &[
                ("⌨️ Terminal", "terminal"),
                ("🔗 Webhook later", "webhook-later"),
                ("✈️ Telegram later", "telegram-later"),
            ],
            1,
        )?;
        match channel.as_str() {
            "terminal" => {
                args.channel = Some("none".to_string());
                args.channel_value = None;
            }
            "telegram-later" => {
                args.channel = Some("telegram".to_string());
                args.channel_value = Some("disabled".to_string());
            }
            "webhook-later" => {
                println!("Webhook stays disabled until adapters.webhook_url is explicitly set.");
            }
            _ => unreachable!("choice is constrained"),
        }
    }

    if args.context_guardian.is_none() {
        print_onboarding_section(
            "8",
            "Continuity",
            "Keep long sessions recoverable with local handoff packets.",
        );
        let guardian = prompt_numbered_choice(
            "Enable context guardian?",
            &[
                ("🛡️ Yes, keep continuity handoffs ready", "enabled"),
                ("⏭️ No, I will run context guard manually", "disabled"),
            ],
            1,
        )?;
        args.context_guardian = Some(guardian);
    }

    print_onboarding_review(config_path, &args);
    let finish = prompt_numbered_choice(
        "Ready to save this onboarding profile?",
        &[
            ("✅ Save and continue", "save"),
            ("🔁 Start over", "restart"),
        ],
        1,
    )?;
    if finish == "restart" {
        println!();
        println!(
            "{}{}↻ Restarting onboarding.{}",
            ANSI_BOLD, ANSI_MAGENTA, ANSI_RESET
        );
        return run_interactive_setup(config_path, restart_interactive_args(&base_args), cfg);
    }

    println!();
    println!(
        "{}{}✓ Review accepted.{} Preparing workspace and saving profile...",
        ANSI_BOLD, ANSI_GREEN, ANSI_RESET
    );
    println!("Config will be written to: {}", config_path.display());
    Ok(args)
}

fn prompt_default(label: &str, default: &str) -> Result<String> {
    print!(
        "{}{}{} {}[{}]{}: ",
        ANSI_BOLD, label, ANSI_RESET, ANSI_DIM, default, ANSI_RESET
    );
    io::stdout().flush().context("failed to flush prompt")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("failed to read setup answer")?;
    let raw = input.trim();
    let cleaned = strip_terminal_escape_input(raw);
    if cleaned != raw {
        if cleaned.is_empty() {
            println!("I detected extra terminal input. Press Enter to use the default.");
        } else {
            println!("I detected extra terminal input. Did you mean {cleaned}?");
        }
    }
    let trimmed = cleaned.trim();
    if trimmed.is_empty() {
        Ok(default.to_string())
    } else {
        Ok(trimmed.to_string())
    }
}

fn prompt_workspace_path(label: &str, default: &str) -> Result<String> {
    loop {
        let value = prompt_default(label, default)?;
        if looks_like_pasted_command(&value) {
            println!(
                "{}⚠ That looks like a command, not a folder path.{} Press Enter to use ./workspace or enter a real folder path.",
                ANSI_YELLOW, ANSI_RESET
            );
            continue;
        }
        return Ok(value);
    }
}

fn print_onboarding_section(step: &str, title: &str, hint: &str) {
    println!();
    let border = onboarding_step_border(step);
    println!("{}{}{}", ANSI_DIM, border, ANSI_RESET);
    println!(
        "{}{}Step {step}{}  {}{}{}",
        ANSI_BOLD, ANSI_CYAN, ANSI_RESET, ANSI_BOLD, title, ANSI_RESET
    );
    println!("{}{}{}", ANSI_DIM, hint, ANSI_RESET);
    println!("{}{}{}", ANSI_DIM, border, ANSI_RESET);
}

fn onboarding_step_border(step: &str) -> &'static str {
    match step.parse::<usize>() {
        Ok(number) if number % 2 == 0 => {
            "------------------------------------------------------------"
        }
        _ => "============================================================",
    }
}

fn print_onboarding_review(config_path: &std::path::Path, args: &SetupArgs) {
    let workspace = args
        .workspace_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "./workspace".to_string());
    let profile = args.runtime_profile.as_deref().unwrap_or("laptop");
    let role = args.role.as_deref().unwrap_or("solo-local-node");
    let network = match args.external_network.as_deref() {
        Some("enabled") => "enabled",
        Some("disabled") | None => "local only / ask before live checks",
        Some(other) => other,
    };
    let tools = if args.selected_tools.is_empty() {
        "none".to_string()
    } else {
        normalize_model_list(args.selected_tools.clone()).join(", ")
    };
    let model_provider = if args.enable_openrouter || args.openrouter_api_key.is_some() {
        "OpenRouter"
    } else {
        "none"
    };
    let channel = args.channel.as_deref().unwrap_or("none");
    let context_guardian = match args.context_guardian.as_deref() {
        Some("disabled") => "disabled",
        _ => "enabled",
    };

    println!();
    println!(
        "{}╭─ Onboarding review ─────────────────────╮{}",
        ANSI_CYAN, ANSI_RESET
    );
    println!("{}│{} workspace       {}", ANSI_CYAN, ANSI_RESET, workspace);
    println!("{}│{} role            {}", ANSI_CYAN, ANSI_RESET, role);
    println!("{}│{} device_type     {}", ANSI_CYAN, ANSI_RESET, profile);
    println!("{}│{} network         {}", ANSI_CYAN, ANSI_RESET, network);
    println!(
        "{}│{} model_provider  {}",
        ANSI_CYAN, ANSI_RESET, model_provider
    );
    println!("{}│{} tools           {}", ANSI_CYAN, ANSI_RESET, tools);
    println!("{}│{} channel         {}", ANSI_CYAN, ANSI_RESET, channel);
    println!(
        "{}│{} guardian        {}",
        ANSI_CYAN, ANSI_RESET, context_guardian
    );
    println!(
        "{}│{} config          {}",
        ANSI_CYAN,
        ANSI_RESET,
        config_path.display()
    );
    println!(
        "{}╰─────────────────────────────────────────╯{}",
        ANSI_CYAN, ANSI_RESET
    );
}

fn setup_role_is_child(args: &SetupArgs) -> bool {
    args.role
        .as_deref()
        .and_then(|role| role.parse::<config::OnboardingRole>().ok())
        .is_some_and(|role| role == config::OnboardingRole::AgentClusterChildWorker)
}

fn restart_interactive_args(args: &SetupArgs) -> SetupArgs {
    SetupArgs {
        non_interactive: args.non_interactive,
        force_interactive: args.force_interactive,
        advanced: args.advanced,
        local_model_provider: None,
        local_model: None,
        local_models: Vec::new(),
        remote_model_provider: None,
        remote_model: None,
        openrouter_models: Vec::new(),
        openrouter_api_key: None,
        enable_openrouter: false,
        channel: None,
        channel_value: None,
        role: None,
        runtime_profile: None,
        workspace_path: None,
        state_path: None,
        session_path: None,
        external_network: None,
        context_guardian: None,
        selected_tools: Vec::new(),
        replace_tools: args.replace_tools,
    }
}

fn strip_terminal_escape_input(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        if ch == '^' && chars.peek() == Some(&'[') {
            chars.next();
            if chars.peek() == Some(&'[') {
                chars.next();
            }
            while let Some(next) = chars.peek().copied() {
                if next.is_ascii_alphabetic() || next == '~' {
                    chars.next();
                    break;
                }
                if next.is_ascii_digit() || next == ';' || next == '?' {
                    chars.next();
                    continue;
                }
                break;
            }
            continue;
        }
        output.push(ch);
    }
    output.trim().to_string()
}

fn prompt_yes_no(label: &str, default: bool) -> Result<bool> {
    let default_label = if default { "Y/n" } else { "y/N" };
    loop {
        print!(
            "{}{}{} {}[{}]{}: ",
            ANSI_BOLD, label, ANSI_RESET, ANSI_DIM, default_label, ANSI_RESET
        );
        io::stdout().flush().context("failed to flush prompt")?;
        let mut answer = String::new();
        io::stdin()
            .read_line(&mut answer)
            .context("failed to read setup answer")?;
        let raw = answer.trim();
        let cleaned = strip_terminal_escape_input(raw);
        if cleaned != raw {
            if cleaned.is_empty() {
                println!("I detected extra terminal input. Press Enter to use the default.");
            } else {
                println!("I detected extra terminal input. Did you mean {cleaned}?");
            }
        }
        match cleaned.trim().to_ascii_lowercase().as_str() {
            "y" | "yes" => return Ok(true),
            "n" | "no" => return Ok(false),
            "" => return Ok(default),
            _ => println!("Please answer y or n."),
        }
    }
}

fn prompt_openrouter_models() -> Result<Vec<String>> {
    let options = model_options(&[
        ("qwen/qwen3-coder", "coding default"),
        ("openai/gpt-4.1-mini", "balanced general work"),
        ("openai/gpt-4o-mini", "cheap fallback"),
        ("anthropic/claude-3.5-sonnet", "reasoning/review lane"),
        ("google/gemini-2.5-pro", "long-context reasoning lane"),
    ]);
    prompt_model_menu(
        "OpenRouter models for multiplexing/council routing",
        &options,
        true,
    )
}

fn prompt_local_models(provider: &str) -> Result<Vec<String>> {
    let options = local_model_options(provider);
    if options
        .iter()
        .any(|(_model, note)| note.starts_with("detected"))
    {
        println!(
            "{}Detected local model tags are listed first. Detection does not grant execution permission.{}",
            ANSI_DIM, ANSI_RESET
        );
    } else {
        println!(
            "{}No local model tags were detected in common {provider} locations. Pick a suggested tag or type custom:<model-id>.{}",
            ANSI_DIM, ANSI_RESET
        );
    }
    prompt_model_menu("Local models", &options, false)
}

fn prompt_detected_local_provider(options: &[(String, String)]) -> Result<String> {
    let menu: Vec<(String, String)> = options
        .iter()
        .map(|(provider, note)| (provider.clone(), note.clone()))
        .collect();
    println!();
    println!(
        "{}{}Local model providers{}",
        ANSI_BOLD, ANSI_MAGENTA, ANSI_RESET
    );
    for (index, (provider, note)) in menu.iter().enumerate() {
        println!(
            "  {}{:>2}{}   {} {}({}){}",
            ANSI_CYAN,
            index + 1,
            ANSI_RESET,
            local_provider_display(provider),
            ANSI_DIM,
            note,
            ANSI_RESET
        );
    }
    loop {
        let answer = prompt_default("Select local provider", "1")?;
        if let Ok(index) = answer.trim().parse::<usize>()
            && index > 0
            && let Some((provider, _note)) = menu.get(index - 1)
        {
            return Ok(provider.clone());
        }
        let normalized = normalize_registry_id(&answer);
        if let Some((provider, _note)) = menu.iter().find(|(provider, _note)| {
            normalize_registry_id(provider) == normalized
                || normalize_registry_id(&local_provider_display(provider)) == normalized
        }) {
            return Ok(provider.clone());
        }
        println!("Please choose a detected local provider number or name.");
    }
}

fn detected_local_provider_options() -> Vec<(String, String)> {
    ["ollama", "lmstudio"]
        .into_iter()
        .filter_map(|provider| {
            let models = detect_local_model_tags(provider);
            if models.is_empty() {
                return None;
            }
            let preview = models
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ");
            let suffix = if models.len() > 3 {
                format!(", +{} more", models.len() - 3)
            } else {
                String::new()
            };
            Some((
                provider.to_string(),
                format!("{} detected: {preview}{suffix}", models.len()),
            ))
        })
        .collect()
}

fn local_provider_display(provider: &str) -> String {
    match normalize_registry_id(provider).as_str() {
        "ollama" => "Ollama".to_string(),
        "lmstudio" => "LM Studio".to_string(),
        other => other.to_string(),
    }
}

fn local_model_options(provider: &str) -> Vec<(String, String)> {
    let defaults = default_local_model_options(provider);
    let mut options: Vec<(String, String)> = detect_local_model_tags(provider)
        .into_iter()
        .map(|model| (model, "detected on local disk".to_string()))
        .collect();
    for (model, note) in defaults {
        if !options.iter().any(|(existing, _note)| existing == &model) {
            options.push((model, note));
        }
    }
    options
}

fn default_local_model_options(provider: &str) -> Vec<(String, String)> {
    let ollama = model_options(&[
        ("qwen3-coder:7b", "local coding default"),
        ("llama3.1:8b", "general local fallback"),
        ("deepseek-coder:6.7b", "local code lane"),
    ]);
    let lmstudio = model_options(&[
        ("local-model", "current LM Studio model"),
        ("qwen3-coder", "coding model alias"),
        ("llama-3.1-8b-instruct", "general local alias"),
    ]);
    if provider == "lmstudio" {
        lmstudio
    } else {
        ollama
    }
}

fn model_options(values: &[(&str, &str)]) -> Vec<(String, String)> {
    values
        .iter()
        .map(|(model, note)| ((*model).to_string(), (*note).to_string()))
        .collect()
}

fn prompt_model_menu(
    label: &str,
    options: &[(String, String)],
    allow_none: bool,
) -> Result<Vec<String>> {
    println!();
    println!("{}{}{}{}", ANSI_BOLD, ANSI_MAGENTA, label, ANSI_RESET);
    println!();
    if allow_none {
        println!("  {} 0{}   ⏭️ none for now", ANSI_CYAN, ANSI_RESET);
    }
    for (index, (model, note)) in options.iter().enumerate() {
        println!(
            "  {}{:>2}{}   {} {}({}){}",
            ANSI_CYAN,
            index + 1,
            ANSI_RESET,
            model,
            ANSI_DIM,
            note,
            ANSI_RESET
        );
    }
    println!();
    println!(
        "{}Type numbers separated by commas, a model id, or custom:<model-id>.{}",
        ANSI_DIM, ANSI_RESET
    );
    println!();
    let default = if allow_none { "0" } else { "1" };
    loop {
        let answer = prompt_default("Select model(s)", default)?;
        let selected = parse_model_selection(&answer, options, allow_none);
        match selected {
            Ok(models) => return Ok(models),
            Err(err) => println!("{err}"),
        }
    }
}

fn parse_model_selection(
    answer: &str,
    options: &[(String, String)],
    allow_none: bool,
) -> Result<Vec<String>> {
    let trimmed = answer.trim();
    if trimmed.is_empty() || (allow_none && trimmed == "0") || trimmed.eq_ignore_ascii_case("none")
    {
        return Ok(Vec::new());
    }
    let mut models = Vec::new();
    for raw in trimmed.split(',') {
        let item = raw.trim();
        if item.is_empty() {
            continue;
        }
        if allow_none && (item == "0" || item.eq_ignore_ascii_case("none")) {
            continue;
        }
        if let Some(custom) = item.strip_prefix("custom:") {
            let custom = custom.trim();
            if custom.is_empty() {
                anyhow::bail!("custom model id cannot be empty");
            }
            models.push(custom.to_string());
            continue;
        }
        if let Ok(index) = item.parse::<usize>() {
            if index == 0 && allow_none {
                continue;
            }
            let Some((model, _note)) = options.get(index.saturating_sub(1)) else {
                anyhow::bail!("model number {index} is not in the menu");
            };
            models.push(model.clone());
            continue;
        }
        models.push(item.to_string());
    }
    Ok(normalize_model_list(models))
}

fn detect_local_model_tags(provider: &str) -> Vec<String> {
    match provider {
        "ollama" => detect_ollama_model_tags_from_roots(&local_model_search_roots("ollama")),
        "lmstudio" => detect_lmstudio_model_tags_from_roots(&local_model_search_roots("lmstudio")),
        _ => Vec::new(),
    }
}

fn user_home_dir() -> Option<PathBuf> {
    env::var_os("HOME")
        .or_else(|| env::var_os("USERPROFILE"))
        .map(PathBuf::from)
        .filter(|path| path.is_dir())
}

fn local_model_search_roots(provider: &str) -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = user_home_dir() {
        match provider {
            "ollama" => {
                roots.push(home.join(".ollama/models/manifests"));
            }
            "lmstudio" => {
                roots.push(home.join("Library/Application Support/LM Studio/models"));
                roots.push(home.join(".cache/lm-studio/models"));
                roots.push(home.join(".cache/lmstudio/models"));
                roots.push(home.join(".lmstudio/models"));
                roots.push(home.join("AppData/Local/LM Studio/models"));
                roots.push(home.join("AppData/Roaming/LM Studio/models"));
                roots.push(home.join("Models"));
                roots.push(home.join("models"));
            }
            _ => {}
        }
    }
    if provider == "ollama" {
        push_env_join(&mut roots, "OLLAMA_MODELS", "manifests");
    }
    if provider == "lmstudio" {
        push_env_path(&mut roots, "LMSTUDIO_MODELS_DIR");
        push_env_path(&mut roots, "LM_STUDIO_MODELS_DIR");
        push_env_join(&mut roots, "LOCALAPPDATA", "LM Studio/models");
        push_env_join(&mut roots, "APPDATA", "LM Studio/models");
        push_env_join(&mut roots, "PROGRAMDATA", "LM Studio/models");
        roots.push(PathBuf::from(
            "/Applications/LM Studio.app/Contents/Resources/models",
        ));
        roots.push(PathBuf::from("/opt/LM Studio/models"));
        roots.push(PathBuf::from("/usr/local/share/lmstudio/models"));
        roots.push(PathBuf::from("/usr/share/lmstudio/models"));
    }
    dedupe_paths(roots)
}

fn push_env_path(roots: &mut Vec<PathBuf>, name: &str) {
    if let Some(value) = env::var_os(name) {
        let path = PathBuf::from(value);
        if !path.as_os_str().is_empty() {
            roots.push(path);
        }
    }
}

fn push_env_join(roots: &mut Vec<PathBuf>, name: &str, suffix: &str) {
    if let Some(value) = env::var_os(name) {
        let base = PathBuf::from(value);
        if !base.as_os_str().is_empty() {
            roots.push(base.join(suffix));
        }
    }
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut deduped = Vec::new();
    for path in paths {
        if !deduped.iter().any(|existing: &PathBuf| existing == &path) {
            deduped.push(path);
        }
    }
    deduped
}

#[allow(dead_code)]
fn detect_ollama_model_tags_in(home: &Path) -> Vec<String> {
    detect_ollama_model_tags_from_roots(&[home.join(".ollama/models/manifests")])
}

fn detect_ollama_model_tags_from_roots(roots: &[PathBuf]) -> Vec<String> {
    let mut models = Vec::new();
    for root in roots {
        collect_files_limited(root, 6, 250, &mut |path| {
            if let Some(model) = ollama_model_tag_from_manifest_path(root, path) {
                models.push(model);
            }
        });
    }
    models.sort();
    normalize_model_list(models)
}

fn ollama_model_tag_from_manifest_path(root: &Path, path: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let parts: Vec<String> = rel
        .components()
        .map(|component| component.as_os_str().to_string_lossy().to_string())
        .collect();
    if parts.len() < 4 {
        return None;
    }
    let tag = parts.last()?.trim();
    let model = parts.get(parts.len().saturating_sub(2))?.trim();
    let namespace = parts.get(parts.len().saturating_sub(3))?.trim();
    if tag.is_empty() || model.is_empty() {
        return None;
    }
    if namespace.is_empty() || namespace == "library" {
        Some(format!("{model}:{tag}"))
    } else {
        Some(format!("{namespace}/{model}:{tag}"))
    }
}

#[allow(dead_code)]
fn detect_lmstudio_model_tags_in(home: &Path) -> Vec<String> {
    let roots = [
        home.join("Library/Application Support/LM Studio/models"),
        home.join(".cache/lm-studio/models"),
        home.join(".cache/lmstudio/models"),
        home.join(".lmstudio/models"),
        home.join("AppData/Local/LM Studio/models"),
        home.join("AppData/Roaming/LM Studio/models"),
        home.join("Models"),
        home.join("models"),
    ];
    detect_lmstudio_model_tags_from_roots(&roots)
}

fn detect_lmstudio_model_tags_from_roots(roots: &[PathBuf]) -> Vec<String> {
    let mut models = Vec::new();
    for root in roots {
        collect_files_limited(root, 5, 400, &mut |path| {
            if let Some(model) = lmstudio_model_tag_from_path(path) {
                models.push(model);
            }
        });
    }
    models.sort();
    normalize_model_list(models)
}

fn lmstudio_model_tag_from_path(path: &Path) -> Option<String> {
    let extension = path.extension()?.to_string_lossy().to_ascii_lowercase();
    if !matches!(
        extension.as_str(),
        "gguf" | "bin" | "safetensors" | "onnx" | "mlmodel"
    ) {
        return None;
    }
    path.file_stem()
        .map(|stem| stem.to_string_lossy().trim().to_string())
        .filter(|stem| !stem.is_empty())
}

fn collect_files_limited(
    root: &Path,
    max_depth: usize,
    mut remaining: usize,
    visit: &mut impl FnMut(&Path),
) {
    if !root.is_dir() || remaining == 0 {
        return;
    }
    collect_files_limited_inner(root, max_depth, &mut remaining, visit);
}

fn collect_files_limited_inner(
    root: &Path,
    depth_left: usize,
    remaining: &mut usize,
    visit: &mut impl FnMut(&Path),
) {
    if *remaining == 0 {
        return;
    }
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten() {
        if *remaining == 0 {
            break;
        }
        let path = entry.path();
        if path.is_file() {
            *remaining -= 1;
            visit(&path);
        } else if depth_left > 0 && path.is_dir() {
            collect_files_limited_inner(&path, depth_left - 1, remaining, visit);
        }
    }
}

fn looks_like_pasted_command(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.contains("./target") || lower.contains("target/") || lower.contains(" --config ") {
        return true;
    }
    let command_tokens = [
        "cargo", "quant-m", "run", "clear", "cli", "demo", "doctor", "setup", "onboard", "git",
        "npm", "pnpm", "node", "python", "bash", "zsh", "sh",
    ];
    lower
        .split(|ch: char| ch.is_whitespace() || matches!(ch, ';' | '&' | '|'))
        .filter(|part| !part.is_empty())
        .any(|part| command_tokens.contains(&part))
        || lower.contains(" --release")
}

fn looks_like_api_key(value: &str) -> bool {
    let trimmed = value.trim();
    if trimmed.contains(char::is_whitespace) || trimmed.len() < 24 {
        return false;
    }
    trimmed.starts_with("sk-")
        || trimmed.starts_with("sk-or-")
        || trimmed.starts_with("sk-proj-")
        || trimmed.starts_with("sk-ant-")
        || trimmed.starts_with("AIza")
}

fn normalize_model_list(values: Vec<String>) -> Vec<String> {
    let mut models = Vec::new();
    for value in values {
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("none") {
            continue;
        }
        if !models.iter().any(|model: &String| model == trimmed) {
            models.push(trimmed.to_string());
        }
    }
    models
}

fn quant_m_command_hint() -> String {
    if command_present("quant-m") {
        return "quant-m".to_string();
    }
    if let Ok(cwd) = env::current_dir() {
        let launcher = cwd.join("quantm");
        if launcher.is_file() {
            return "./quantm".to_string();
        }
    }
    let Ok(exe) = env::current_exe() else {
        return "./target/release/quant-m".to_string();
    };
    if let Ok(cwd) = env::current_dir()
        && let Ok(relative) = exe.strip_prefix(&cwd)
    {
        return format!("./{}", relative.display());
    }
    exe.display().to_string()
}

fn print_tool_setup_steps(tool: &str) {
    let tool = normalize_registry_id(tool);
    let command = quant_m_command_hint();
    println!();
    println!(
        "{}{}✨ {} setup{}",
        ANSI_BOLD,
        ANSI_BLUE,
        developer_tool_display(&tool),
        ANSI_RESET
    );
    match tool.as_str() {
        "codex" => {
            println!("  1. Install or open Codex CLI.");
            println!("  2. Run `codex login` and complete browser verification.");
            println!("  3. Verify it responds: codex --version");
        }
        "claude" => {
            println!("  1. Install or open Claude Code CLI.");
            println!("  2. Run `claude login` and complete browser verification.");
            println!("  3. Verify it responds: claude --version");
        }
        "gemini" => {
            println!("  1. Install or open Gemini CLI.");
            println!("  2. Run the Gemini CLI login/auth command shown by Gemini.");
            println!("  3. Verify it responds: gemini --version");
        }
        "antigravity" | "antgravity" => {
            println!("  1. Install or open Antigravity.");
            println!("  2. Complete its browser/account verification flow.");
            println!("  3. Verify any CLI shim responds: antigravity --version");
            println!(
                "  Note: Quant-M will register Antigravity, but will not fake chat responses unless a stable non-interactive CLI prompt command is available."
            );
        }
        "openrouter" => {
            println!("  1. Create or open your OpenRouter account in a browser.");
            println!("  2. Create an API key.");
            println!(
                "  3. Run `{command} onboard` and paste the key when prompted, or export OPENROUTER_API_KEY."
            );
            println!("  4. Validate with `{command} provider validate openrouter --live`.");
            return;
        }
        "openai" => {
            println!("  1. Install or open the OpenAI CLI.");
            println!("  2. Run its login command or set OPENAI_API_KEY.");
            println!("  3. Verify it responds: openai --version");
        }
        other => {
            println!("  1. Install or open `{other}`.");
            println!("  2. Complete its account or browser verification flow.");
            println!("  3. Verify it responds to its configured validation command.");
        }
    }
    println!("  4. Return here and run: {command} onboard");
    println!("  5. After setup, run: {command} tool validate {tool}");
}

fn print_codex_setup_steps() {
    print_tool_setup_steps("codex");
}

fn prompt_numbered_choice(
    label: &str,
    options: &[(&str, &str)],
    default_index: usize,
) -> Result<String> {
    println!();
    println!("{}{}┌─ Question{}", ANSI_DIM, ANSI_CYAN, ANSI_RESET);
    println!(
        "{}{}│{} {}{}{}",
        ANSI_CYAN, ANSI_BOLD, ANSI_RESET, ANSI_BOLD, label, ANSI_RESET
    );
    println!(
        "{}{}└────────────────────────────────────────{}",
        ANSI_DIM, ANSI_CYAN, ANSI_RESET
    );
    println!();
    for (index, (display, _value)) in options.iter().enumerate() {
        println!(
            "  {}{:>2}{}   {}",
            ANSI_CYAN,
            index + 1,
            ANSI_RESET,
            display
        );
        if index + 1 != options.len() {
            println!();
        }
    }
    println!();
    let default = default_index.to_string();
    loop {
        let value = prompt_default("Select", &default)?;
        if let Ok(index) = value.parse::<usize>()
            && index > 0
            && let Some((_display, selected)) = options.get(index - 1)
        {
            return Ok((*selected).to_string());
        }
        if let Some((_display, selected)) = options.iter().find(|(display, selected)| {
            value.eq_ignore_ascii_case(display) || value.eq_ignore_ascii_case(selected)
        }) {
            return Ok((*selected).to_string());
        }
        if looks_like_api_key(&value) {
            println!(
                "{}That looks like an API key.{} For safety, choose option 3 if you want Quant-M to save it locally.",
                ANSI_YELLOW, ANSI_RESET
            );
            continue;
        }
        print_numbered_choice_help(options, default_index);
    }
}

fn print_numbered_choice_help(options: &[(&str, &str)], default_index: usize) {
    println!();
    println!(
        "{}{}╭─ I did not recognize that answer ─────────────╮{}",
        ANSI_RED, ANSI_BOLD, ANSI_RESET
    );
    println!(
        "{}│{} Choose a number, or type the short name shown below.",
        ANSI_RED, ANSI_RESET
    );
    println!("{}│{}", ANSI_RED, ANSI_RESET);
    for (index, (display, value)) in options.iter().enumerate() {
        println!(
            "{}│{}   {}{:>2}{}  {}",
            ANSI_RED,
            ANSI_RESET,
            ANSI_CYAN,
            index + 1,
            ANSI_RESET,
            display
        );
        println!(
            "{}│{}       {}type: {}{}",
            ANSI_RED, ANSI_RESET, ANSI_DIM, value, ANSI_RESET
        );
    }
    println!("{}│{}", ANSI_RED, ANSI_RESET);
    println!(
        "{}│{} Press Enter to use the default: {}{}{}",
        ANSI_RED, ANSI_RESET, ANSI_BOLD, default_index, ANSI_RESET
    );
    println!(
        "{}╰───────────────────────────────────────────────╯{}",
        ANSI_RED, ANSI_RESET
    );
    println!();
}

fn prompt_choice(label: &str, default: &str, allowed: &[&str]) -> Result<String> {
    loop {
        let value = prompt_default(label, default)?;
        if allowed
            .iter()
            .any(|allowed| value.eq_ignore_ascii_case(allowed))
        {
            return Ok(value.to_ascii_lowercase());
        }
        println!("Choose one of: {}", allowed.join(", "));
    }
}

fn prompt_secret_like(label: &str) -> Result<String> {
    print!("{label}: ");
    io::stdout().flush().context("failed to flush prompt")?;
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .context("failed to read setup secret")?;
    Ok(strip_terminal_escape_input(input.trim()))
}

fn prompt_developer_tools(cfg: &Config) -> Result<Vec<String>> {
    let options = developer_tool_menu_options();
    println!();
    println!(
        "{}Pick optional CLI tools to recognize.{}",
        ANSI_BOLD, ANSI_RESET
    );
    println!("  {} 0{}   ⏭️ none for now", ANSI_CYAN, ANSI_RESET);
    println!(
        "  {} 1{}   🔎 Scan PATH and enable detected supported tools",
        ANSI_CYAN, ANSI_RESET
    );
    for (index, (id, label, note)) in options.iter().enumerate() {
        println!(
            "  {}{:>2}{}   {} ({})",
            ANSI_CYAN,
            index + 2,
            ANSI_RESET,
            label,
            note
        );
        if *id == "codex" {
            println!(
                "       {}Codex is the OpenAI Codex CLI; OpenAI CLI is listed separately.{}",
                ANSI_DIM, ANSI_RESET
            );
        }
    }
    println!();
    println!(
        "{}Type numbers separated by commas, ids like codex/claude/antigravity, `scan`, or `none`. Manual choices are allowed before login; validate after browser/account setup.{}",
        ANSI_DIM, ANSI_RESET
    );

    loop {
        let answer = prompt_default("Select CLI tool(s)", "1")?;
        match parse_developer_tool_selection(&answer, cfg) {
            Ok(selected) => return Ok(selected),
            Err(err) => println!("{err}"),
        }
    }
}

fn developer_tool_menu_options() -> &'static [(&'static str, &'static str, &'static str)] {
    &[
        ("codex", "Codex CLI", "OpenAI Codex agent CLI"),
        ("openai", "OpenAI CLI", "OpenAI platform CLI"),
        ("gemini", "Gemini CLI", "Google Gemini CLI"),
        ("claude", "Claude CLI", "Anthropic Claude Code-style CLI"),
        ("anthropic", "Anthropic CLI", "Anthropic platform CLI"),
        ("opencode", "OpenCode CLI", "open coding agent CLI"),
        ("antigravity", "Antigravity CLI", "Antigravity-style CLI"),
        ("ollama", "Ollama", "local model runtime"),
        ("lmstudio", "LM Studio", "local model runtime via lms"),
    ]
}

fn parse_developer_tool_selection(answer: &str, cfg: &Config) -> Result<Vec<String>> {
    let trimmed = answer.trim();
    if trimmed.is_empty() || trimmed == "1" || trimmed.eq_ignore_ascii_case("scan") {
        let detected = scan_supported_developer_tools(cfg);
        if detected.is_empty() {
            let command = quant_m_command_hint();
            println!(
                "{}No supported CLI tools detected on PATH.{}",
                ANSI_YELLOW, ANSI_RESET
            );
            print_codex_setup_steps();
            println!("You can add tools later with: {command} tool scan");
        }
        return Ok(unique_tool_ids(detected.into_iter().map(|tool| tool.id)));
    }
    if trimmed == "0"
        || trimmed.eq_ignore_ascii_case("none")
        || trimmed.eq_ignore_ascii_case("skip")
    {
        return Ok(Vec::new());
    }

    let options = developer_tool_menu_options();
    let mut selected = Vec::new();
    for raw in trimmed.split(',') {
        let item = normalize_registry_id(raw.trim());
        if item.is_empty() {
            continue;
        }
        if item == "scan" {
            selected.extend(
                scan_supported_developer_tools(cfg)
                    .into_iter()
                    .map(|tool| tool.id),
            );
            continue;
        }
        if item == "0" || item == "none" || item == "skip" {
            continue;
        }
        if let Ok(index) = item.parse::<usize>() {
            if index == 1 {
                selected.extend(
                    scan_supported_developer_tools(cfg)
                        .into_iter()
                        .map(|tool| tool.id),
                );
                continue;
            }
            let option_index = index.saturating_sub(2);
            let Some((id, _label, _note)) = options.get(option_index) else {
                anyhow::bail!("tool number {index} is not in the menu");
            };
            selected.push((*id).to_string());
            continue;
        }
        let known = options.iter().any(|(id, _label, _note)| *id == item)
            || supported_developer_tool_ids().contains(&item.as_str());
        if !known {
            anyhow::bail!("unknown CLI tool `{item}`");
        }
        selected.push(item);
    }
    Ok(unique_tool_ids(selected))
}

fn unique_tool_ids(ids: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut selected = Vec::new();
    for id in ids {
        if !selected.contains(&id) {
            selected.push(id);
        }
    }
    selected
}

#[derive(Debug, Clone)]
struct DetectedTool {
    id: String,
    display: String,
}

fn developer_tool_display(id: &str) -> String {
    match id {
        "codex" => "Codex CLI".to_string(),
        "openai" => "OpenAI CLI".to_string(),
        "gemini" => "Gemini CLI".to_string(),
        "anthropic" => "Anthropic CLI".to_string(),
        "claude" => "Claude CLI".to_string(),
        "opencode" => "OpenCode CLI".to_string(),
        "antigravity" => "Antigravity CLI".to_string(),
        "antgravity" => "Antgravity CLI".to_string(),
        "ollama" => "Ollama".to_string(),
        "lmstudio" => "LM Studio".to_string(),
        other => other.to_string(),
    }
}

fn supported_developer_tool_ids() -> &'static [&'static str] {
    &[
        "codex",
        "openai",
        "gemini",
        "anthropic",
        "claude",
        "opencode",
        "antigravity",
        "antgravity",
        "ollama",
        "lmstudio",
    ]
}

fn is_chat_capable_tool(id: &str) -> bool {
    matches!(
        id.trim().to_ascii_lowercase().as_str(),
        "codex"
            | "claude"
            | "anthropic"
            | "gemini"
            | "antigravity"
            | "antgravity"
            | "openai"
            | "opencode"
    )
}

fn scan_supported_developer_tools(cfg: &Config) -> Vec<DetectedTool> {
    supported_developer_tool_ids()
        .iter()
        .filter_map(|id| {
            let tool = cfg.tools.get(*id)?;
            command_present(&tool.command).then(|| DetectedTool {
                id: (*id).to_string(),
                display: developer_tool_display(id),
            })
        })
        .collect()
}

fn enable_provider(cfg: &mut Config, id: &str) {
    cfg.ensure_onboarding_registries();
    let id = normalize_registry_id(id);
    if let Some(provider) = cfg.providers.get_mut(&id) {
        provider.enabled = true;
    }
}

fn enable_tool_arg(args: &mut SetupArgs, id: &str) {
    args.selected_tools.push(id.to_string());
    println!(
        "Tool '{id}' will be visible in `quant-m tool list`; validate it with `quant-m tool validate {id}`."
    );
}

fn enable_tool(cfg: &mut Config, id: &str) {
    cfg.ensure_onboarding_registries();
    let id = normalize_registry_id(id);
    if let Some(tool) = cfg.tools.get_mut(&id) {
        tool.enabled = true;
    }
}

fn disable_all_tools(cfg: &mut Config) {
    cfg.ensure_onboarding_registries();
    for tool in cfg.tools.values_mut() {
        tool.enabled = false;
    }
}

fn handle_config_command(config_path: &std::path::Path, command: ConfigCommand) -> Result<()> {
    match command {
        ConfigCommand::Show { json } => {
            let cfg = Config::load_or_create(config_path)
                .with_context(|| format!("failed loading config {}", config_path.display()))?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&cfg.portable_view(config_path))?
                );
            } else {
                println!("{}", cfg.render_toml(config_path)?);
            }
            Ok(())
        }
        ConfigCommand::SetModel { provider, model } => {
            let mut cfg = Config::load_or_create(config_path)
                .with_context(|| format!("failed loading config {}", config_path.display()))?;
            cfg.set_preferred_model(&provider, &model)?;
            let cfg = cfg.sanitize();
            cfg.validate()?;
            cfg.save(config_path)?;
            println!(
                "updated model preference: provider={} model={}",
                provider.trim(),
                model.trim()
            );
            Ok(())
        }
        ConfigCommand::ClearModel { provider } => {
            let mut cfg = Config::load_or_create(config_path)
                .with_context(|| format!("failed loading config {}", config_path.display()))?;
            let cleared = clear_model_preference(&mut cfg, provider.as_deref())?;
            let cfg = cfg.sanitize();
            cfg.validate()?;
            cfg.save(config_path)?;
            println!("cleared model preference: {cleared}");
            Ok(())
        }
        ConfigCommand::SetChannel { channel, value } => {
            let mut cfg = Config::load_or_create(config_path)
                .with_context(|| format!("failed loading config {}", config_path.display()))?;
            let parsed_channel = channel.parse::<config::ExternalChannel>()?;
            cfg.set_channel_preference(parsed_channel, Some(&value))?;
            let cfg = cfg.sanitize();
            cfg.validate()?;
            cfg.save(config_path)?;
            println!(
                "updated channel preference: channel={} value={}",
                channel.trim(),
                if value.trim().is_empty() {
                    "unset"
                } else {
                    value.trim()
                }
            );
            Ok(())
        }
        ConfigCommand::Validate => {
            let cfg = Config::load_or_create(config_path)
                .with_context(|| format!("failed loading config {}", config_path.display()))?;
            cfg.validate()?;
            println!("config valid: {}", config_path.display());
            Ok(())
        }
    }
}

async fn handle_provider_command(
    config_path: &std::path::Path,
    command: ProviderCommand,
) -> Result<()> {
    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    let cfg = cfg.sanitize();
    cfg.validate()?;
    cfg.save(config_path)?;

    match command {
        ProviderCommand::List { json } => {
            let items = list_providers(&cfg);
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for item in items {
                    println!(
                        "{} enabled={} kind={:?} key_env={} key_present={} api_base={} models={}",
                        item.id,
                        item.enabled,
                        item.kind,
                        if item.api_key_env.is_empty() {
                            "none"
                        } else {
                            item.api_key_env.as_str()
                        },
                        item.key_present,
                        item.api_base,
                        if item.preferred_models.is_empty() {
                            "unset".to_string()
                        } else {
                            item.preferred_models.join(",")
                        }
                    );
                }
            }
            Ok(())
        }
        ProviderCommand::Validate {
            provider,
            live,
            json,
        } => {
            let report = validate_provider(&cfg, &provider, live).await?;
            print_serialized_or_text(&report, json, &format_provider_report(&report))
        }
    }
}

fn handle_tool_command(config_path: &std::path::Path, command: ToolCommand) -> Result<()> {
    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    let mut cfg = cfg.sanitize();
    cfg.validate()?;
    cfg.save(config_path)?;

    match command {
        ToolCommand::List { json } => {
            let items = list_tools(&cfg);
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else {
                for item in items {
                    println!(
                        "{} enabled={} kind={:?} command={} present={} validation_args={}",
                        item.id,
                        item.enabled,
                        item.kind,
                        item.command,
                        item.command_present,
                        if item.validation_args.is_empty() {
                            "none".to_string()
                        } else {
                            item.validation_args.join(" ")
                        }
                    );
                }
            }
            Ok(())
        }
        ToolCommand::Setup { tool } => {
            print_tool_setup_steps(&tool);
            Ok(())
        }
        ToolCommand::Scan { json } => {
            let detected = scan_supported_developer_tools(&cfg);
            for tool in &detected {
                enable_tool(&mut cfg, &tool.id);
            }
            let cfg = cfg.sanitize();
            cfg.validate()?;
            cfg.save(config_path)?;
            let items = list_tools(&cfg);
            if json {
                println!("{}", serde_json::to_string_pretty(&items)?);
            } else if detected.is_empty() {
                println!("No supported developer CLI tools detected.");
            } else {
                println!(
                    "Detected: {}",
                    detected
                        .iter()
                        .map(|tool| tool.display.as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                println!("Enabled integrations:");
                for item in items.into_iter().filter(|item| item.enabled) {
                    println!("  {} ({})", item.id, item.command);
                }
            }
            Ok(())
        }
        ToolCommand::Validate { tool, json } => {
            let report = validate_tool(&cfg, &tool)?;
            print_serialized_or_text(&report, json, &format_tool_report(&report))
        }
    }
}

fn handle_settings_command(config_path: &std::path::Path, json: bool) -> Result<()> {
    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    let cfg = cfg.sanitize();
    cfg.validate()?;
    cfg.save(config_path)?;
    let report = settings_report(&cfg, config_path);
    print_serialized_or_text(&report, json, &format_settings_report(&report))
}

fn handle_capabilities_command(
    cfg: &Config,
    command: Option<CapabilitiesCommand>,
    json: bool,
    category: Option<String>,
    status: Option<String>,
) -> Result<()> {
    match command {
        Some(CapabilitiesCommand::Show {
            capability_id,
            json,
        }) => {
            let record = capabilities::show_capability(cfg, &capability_id)?;
            print_serialized_or_text(&record, json, &format_capability_detail(&record))
        }
        Some(CapabilitiesCommand::AuditDocs { json }) => {
            let report = capabilities::audit_docs(cfg)?;
            print_serialized_or_text(
                &report,
                json,
                &format!(
                    "capability_docs_audit: {}\nchecked_docs: {}\ncapability_count: {}\nmissing_markers:\n{}",
                    report.status,
                    report.checked_docs.join(", "),
                    report.capability_count,
                    if report.missing_markers.is_empty() {
                        "  none".to_string()
                    } else {
                        report
                            .missing_markers
                            .iter()
                            .map(|marker| format!("  - {marker}"))
                            .collect::<Vec<_>>()
                            .join("\n")
                    }
                ),
            )
        }
        None => {
            let category = category
                .as_deref()
                .map(str::parse::<capabilities::CapabilityCategory>)
                .transpose()?;
            let status = status
                .as_deref()
                .map(str::parse::<capabilities::CapabilityStatus>)
                .transpose()?;
            let records = capabilities::filtered_inventory(
                cfg,
                capabilities::CapabilityFilter { category, status },
            )?;
            print_serialized_or_text(&records, json, &format_capability_list(&records))
        }
    }
}

fn format_capability_list(records: &[capabilities::CapabilityRecord]) -> String {
    let mut lines = vec!["Capabilities".to_string()];
    for record in records {
        lines.push(format!(
            "{} [{}] {} - {}",
            record.id, record.status, record.category, record.summary
        ));
    }
    lines.push(
        "\nUse `quant-m capabilities show <capability_id>` for proof paths and gates.".to_string(),
    );
    lines.join("\n")
}

fn format_capability_detail(record: &capabilities::CapabilityRecord) -> String {
    format!(
        "Capability\nid: {}\nname: {}\ncategory: {}\nstatus: {}\nsummary: {}\ncommands:\n{}\nconfig_gates:\n{}\npolicy_gates:\n{}\nartifacts_created:\n{}\nproof_commands:\n{}\nvalidation_commands:\n{}\ndocs:\n{}\nrisks:\n{}\nnotes:\n{}",
        record.id,
        record.name,
        record.category,
        record.status,
        record.summary,
        format_string_list(&record.commands),
        format_string_list(&record.config_gates),
        format_string_list(&record.policy_gates),
        format_string_list(&record.artifacts_created),
        format_string_list(&record.proof_commands),
        format_string_list(&record.validation_commands),
        format_string_list(&record.docs),
        format_string_list(&record.risks),
        format_string_list(&record.notes),
    )
}

fn format_string_list(values: &[String]) -> String {
    if values.is_empty() {
        return "  none".to_string();
    }
    values
        .iter()
        .map(|value| format!("  - {value}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn settings_report(cfg: &Config, config_path: &std::path::Path) -> SettingsReport {
    SettingsReport {
        config: config_path.to_path_buf(),
        workspace: cfg.workspace_dir.clone(),
        session_dir: cfg.runtime.session_dir.clone(),
        multi_model_enabled: cfg.runtime.multi_model_enabled,
        search_enabled: cfg.runtime.search_enabled,
        browser_harness_enabled: cfg.runtime.browser_harness_enabled,
        external_network_enabled: cfg.runtime.external_network_enabled,
        context_guardian_enabled: cfg.context_guardian.enabled,
        preferred_local_model: cfg.preferences.preferred_local_model.clone(),
        preferred_remote_model: cfg.preferences.preferred_remote_model.clone(),
        preferred_openrouter_model: cfg.preferences.preferred_openrouter_model.clone(),
        providers: list_providers(cfg),
        enabled_tools: enabled_tool_ids(cfg),
        detected_tools: scan_supported_developer_tools(cfg)
            .into_iter()
            .map(|tool| tool.display)
            .collect(),
    }
}

fn enabled_label(value: bool) -> &'static str {
    if value { "enabled" } else { "disabled" }
}

fn format_settings_report(report: &SettingsReport) -> String {
    let tools = if report.enabled_tools.is_empty() {
        "none".to_string()
    } else {
        report.enabled_tools.join(", ")
    };
    let detected = if report.detected_tools.is_empty() {
        "none".to_string()
    } else {
        report.detected_tools.join(", ")
    };
    let providers = if report.providers.is_empty() {
        "none".to_string()
    } else {
        report
            .providers
            .iter()
            .map(|provider| {
                format!(
                    "{}:{}:{}",
                    provider.id,
                    enabled_label(provider.enabled),
                    if provider.preferred_models.is_empty() {
                        "models=unset".to_string()
                    } else {
                        format!("models={}", provider.preferred_models.join(","))
                    }
                )
            })
            .collect::<Vec<_>>()
            .join(" | ")
    };
    format!(
        "Settings\nconfig: {}\nworkspace: {}\nsession_dir: {}\nlocal_model: {}\nremote_model: {}\nopenrouter_model: {}\nproviders: {}\nmulti_model_enabled: {}\nsearch_enabled: {}\nbrowser_harness_enabled: {}\nexternal_network_enabled: {}\ncontext_guardian_enabled: {}\nenabled_tools: {}\ndetected_tools: {}\n\nChange model settings:\n  quant-m onboard\n  quant-m setup --local-model-provider ollama --local-model <name>\n  quant-m config set-model openrouter <model-id>\n  quant-m config clear-model [local|remote|openrouter|all]\n\nOther next steps:\n  quant-m provider list\n  quant-m tool scan\n  quant-m context guard",
        report.config.display(),
        report.workspace.display(),
        report.session_dir.display(),
        format_model_pref(report.preferred_local_model.as_ref()),
        format_model_pref(report.preferred_remote_model.as_ref()),
        report
            .preferred_openrouter_model
            .as_deref()
            .unwrap_or("unset"),
        providers,
        enabled_label(report.multi_model_enabled),
        enabled_label(report.search_enabled),
        enabled_label(report.browser_harness_enabled),
        enabled_label(report.external_network_enabled),
        enabled_label(report.context_guardian_enabled),
        tools,
        detected,
    )
}

fn clear_model_preference(cfg: &mut Config, provider: Option<&str>) -> Result<&'static str> {
    match provider.map(str::trim).filter(|value| !value.is_empty()) {
        None | Some("all") => {
            cfg.preferences.preferred_local_model = None;
            cfg.preferences.preferred_remote_model = None;
            cfg.preferences.preferred_openrouter_model = None;
            Ok("all")
        }
        Some("local") => {
            cfg.preferences.preferred_local_model = None;
            Ok("local")
        }
        Some("remote") => {
            cfg.preferences.preferred_remote_model = None;
            cfg.preferences.preferred_openrouter_model = None;
            Ok("remote")
        }
        Some("openrouter") => {
            cfg.preferences.preferred_openrouter_model = None;
            if cfg
                .preferences
                .preferred_remote_model
                .as_ref()
                .is_some_and(|preference| preference.provider.eq_ignore_ascii_case("openrouter"))
            {
                cfg.preferences.preferred_remote_model = None;
            }
            Ok("openrouter")
        }
        Some(other) if is_local_model_provider_id(other) => {
            cfg.preferences.preferred_local_model = None;
            Ok("local")
        }
        Some(other) => anyhow::bail!(
            "unknown model preference scope '{other}'; use local, remote, openrouter, or all"
        ),
    }
}

fn is_local_model_provider_id(provider: &str) -> bool {
    matches!(
        normalize_registry_id(provider).as_str(),
        "ollama" | "lmstudio" | "lm-studio"
    )
}

async fn run_doctor(
    config_path: &std::path::Path,
    include_providers: bool,
    live: bool,
) -> Result<DoctorReport> {
    let mut cfg = Config::load_or_create(config_path)
        .with_context(|| format!("failed loading config {}", config_path.display()))?;
    cfg.ensure_onboarding_registries();
    cfg.save(config_path)?;
    bootstrap::ensure_workspace(&cfg)?;

    let config_exists = config_path.exists();
    let workspace_exists = cfg.workspace_dir.exists();
    let state_path_exists = cfg
        .state_sql
        .sqlite_path
        .parent()
        .map(std::path::Path::exists)
        .unwrap_or(false);
    let session_path_exists = cfg.runtime.session_dir.exists();

    if !(workspace_exists && state_path_exists && session_path_exists) {
        return Ok(DoctorReport {
            role: cfg.runtime.role,
            runtime_profile: cfg.runtime.profile,
            config_exists,
            workspace_exists,
            state_path_exists,
            session_path_exists,
            workflow_run_ok: false,
            shared_state_list_ok: false,
            session_list_ok: false,
            checked_binary: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("quant-m")),
            generated_session_id: None,
            provider_diagnostics: if include_providers {
                validate_all_providers(&cfg, live).await?
            } else {
                Vec::new()
            },
        });
    }

    let workflow = "workflow:mock-research-brief"
        .parse::<workflow_registry::WorkflowId>()
        .expect("static workflow id");
    let run = execution_runtime::run_workflow(&cfg, &workflow)?;
    let sessions_ok = sessions::list_sessions(&cfg).is_ok();
    let state_ok = shared_state::list_state(&cfg, None).is_ok();

    Ok(DoctorReport {
        role: cfg.runtime.role,
        runtime_profile: cfg.runtime.profile,
        config_exists,
        workspace_exists,
        state_path_exists,
        session_path_exists,
        workflow_run_ok: run.status == "ok",
        shared_state_list_ok: state_ok,
        session_list_ok: sessions_ok,
        checked_binary: std::env::current_exe().unwrap_or_else(|_| PathBuf::from("quant-m")),
        generated_session_id: Some(run.session_id.to_string()),
        provider_diagnostics: if include_providers {
            validate_all_providers(&cfg, live).await?
        } else {
            Vec::new()
        },
    })
}

fn list_providers(cfg: &Config) -> Vec<ProviderListItem> {
    cfg.providers
        .iter()
        .map(|(id, provider)| ProviderListItem {
            id: id.clone(),
            enabled: provider.enabled,
            kind: provider.kind,
            api_base: provider.api_base.clone(),
            api_key_env: provider.api_key_env.clone(),
            key_present: provider_key_present(cfg, id),
            preferred_models: provider.preferred_models.clone(),
            live_validation_allowed: provider.live_validation_allowed,
        })
        .collect()
}

fn list_tools(cfg: &Config) -> Vec<ToolListItem> {
    cfg.tools
        .iter()
        .map(|(id, tool)| ToolListItem {
            id: id.clone(),
            enabled: tool.enabled,
            kind: tool.kind,
            command: tool.command.clone(),
            validation_args: tool.validation_args.clone(),
            command_present: command_present(&tool.command),
        })
        .collect()
}

fn enabled_tool_ids(cfg: &Config) -> Vec<String> {
    cfg.tools
        .iter()
        .filter(|(_id, tool)| tool.enabled)
        .map(|(id, _tool)| id.clone())
        .collect()
}

async fn validate_all_providers(cfg: &Config, live: bool) -> Result<Vec<ProviderValidationReport>> {
    let mut reports = Vec::new();
    for id in cfg.providers.keys() {
        reports.push(validate_provider(cfg, id, live).await?);
    }
    Ok(reports)
}

async fn validate_provider(
    cfg: &Config,
    provider_id: &str,
    live: bool,
) -> Result<ProviderValidationReport> {
    let id = normalize_registry_id(provider_id);
    let provider = cfg
        .providers
        .get(&id)
        .with_context(|| format!("unknown provider '{}'", provider_id))?;
    let key_present = provider_key_present(cfg, &id);
    let mut report = ProviderValidationReport {
        id,
        enabled: provider.enabled,
        kind: provider.kind,
        api_base: provider.api_base.clone(),
        api_key_env: provider.api_key_env.clone(),
        key_present,
        live_requested: live,
        live_ok: None,
        message: String::new(),
    };

    if !live {
        report.message = if provider.api_key_env.is_empty() {
            "local config ok; no API key required for this provider kind".to_string()
        } else if key_present {
            format!("local config ok; {} is present", provider.api_key_env)
        } else {
            format!(
                "local config ok; set {} to enable live validation",
                provider.api_key_env
            )
        };
        return Ok(report);
    }

    if matches!(
        provider.kind,
        config::ProviderKind::Ollama | config::ProviderKind::LmStudio
    ) {
        report.live_ok = Some(false);
        report.message =
            "live validation for local HTTP providers is intentionally not run here".to_string();
        return Ok(report);
    }
    let Some(key) = provider_api_key(provider) else {
        report.live_ok = Some(false);
        report.message = format!("missing env var {}", provider.api_key_env);
        return Ok(report);
    };

    let url = format!("{}/models", provider.api_base.trim_end_matches('/'));
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .build()
        .context("failed to build provider validation HTTP client")?;
    let result = client.get(&url).bearer_auth(key).send().await;
    match result {
        Ok(resp) if resp.status().is_success() => {
            report.live_ok = Some(true);
            report.message = format!("live validation ok at {url}");
        }
        Ok(resp) => {
            report.live_ok = Some(false);
            report.message = format!("live validation returned HTTP {}", resp.status());
        }
        Err(err) => {
            report.live_ok = Some(false);
            report.message = format!("live validation failed: {err}");
        }
    }
    Ok(report)
}

fn validate_tool(cfg: &Config, tool_id: &str) -> Result<ToolValidationReport> {
    let id = normalize_registry_id(tool_id);
    let tool = cfg
        .tools
        .get(&id)
        .with_context(|| format!("unknown tool '{}'", tool_id))?;
    let command_present = command_present(&tool.command);
    if !command_present {
        return Ok(ToolValidationReport {
            id,
            enabled: tool.enabled,
            kind: tool.kind,
            command: tool.command.clone(),
            command_present,
            validation_ok: false,
            message: "command not found on PATH".to_string(),
        });
    }

    let output = Command::new(&tool.command)
        .args(&tool.validation_args)
        .output()
        .with_context(|| format!("failed to run {}", tool.command))?;
    let validation_ok = output.status.success();
    let message = if validation_ok {
        "tool responded to safe validation command".to_string()
    } else {
        format!("tool exited with status {}", output.status)
    };
    Ok(ToolValidationReport {
        id,
        enabled: tool.enabled,
        kind: tool.kind,
        command: tool.command.clone(),
        command_present,
        validation_ok,
        message,
    })
}

fn provider_key_present(cfg: &Config, id: &str) -> bool {
    let id = normalize_registry_id(id);
    cfg.providers.get(&id).and_then(provider_api_key).is_some()
}

fn provider_api_key(provider: &config::ProviderConfig) -> Option<String> {
    if provider.api_key_env.trim().is_empty() {
        return None;
    }
    env::var(&provider.api_key_env)
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn command_present(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    if command.contains(std::path::MAIN_SEPARATOR) {
        return std::path::Path::new(command).is_file();
    }
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

fn normalize_registry_id(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace('_', "-")
}

fn format_provider_report(report: &ProviderValidationReport) -> String {
    format!(
        "provider: {}\nenabled: {}\nkey_env: {}\nkey_present: {}\nlive_requested: {}\nlive_ok: {}\nmessage: {}",
        report.id,
        report.enabled,
        if report.api_key_env.is_empty() {
            "none"
        } else {
            report.api_key_env.as_str()
        },
        report.key_present,
        report.live_requested,
        report
            .live_ok
            .map(|value| value.to_string())
            .unwrap_or_else(|| "not_run".to_string()),
        report.message
    )
}

fn format_tool_report(report: &ToolValidationReport) -> String {
    format!(
        "tool: {}\nenabled: {}\ncommand: {}\ncommand_present: {}\nvalidation_ok: {}\nmessage: {}",
        report.id,
        report.enabled,
        report.command,
        report.command_present,
        report.validation_ok,
        report.message
    )
}

fn parse_enabled_disabled(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "enabled" | "true" | "yes" | "on" => Ok(true),
        "disabled" | "false" | "no" | "off" => Ok(false),
        other => anyhow::bail!("expected enabled or disabled, got '{}'", other),
    }
}

fn print_serialized_or_text<T: Serialize>(value: &T, json: bool, text: &str) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!("{text}");
    }
    Ok(())
}

fn runtime_profile_label(profile: config::RuntimeProfile) -> &'static str {
    match profile {
        config::RuntimeProfile::Edge => "edge",
        config::RuntimeProfile::Laptop => "laptop",
        config::RuntimeProfile::Vps => "vps",
        config::RuntimeProfile::StaffOsWorker => "staff-os-worker",
    }
}

fn onboarding_role_label(role: config::OnboardingRole) -> &'static str {
    match role {
        config::OnboardingRole::SoloLocalNode => "solo-local-node",
        config::OnboardingRole::AgentClusterCore => "agent-cluster-core",
        config::OnboardingRole::AgentClusterChildWorker => "agent-cluster-child-worker",
        config::OnboardingRole::StaffOsWorker => "staff-os-worker",
        config::OnboardingRole::ServerVpsNode => "server-vps-node",
    }
}

fn format_model_pref(value: Option<&config::ModelPreference>) -> String {
    value
        .map(|pref| format!("{} {}", pref.provider, pref.model))
        .unwrap_or_else(|| "unset".to_string())
}

fn format_channel_pref(value: &config::ChannelPreference) -> String {
    let label = channels::channel_label(value.channel);
    match value.value.as_deref() {
        Some(extra) => format!("{label}:{extra}"),
        None => label.to_string(),
    }
}

fn rebase_workspace_paths(cfg: &mut Config, new_workspace: PathBuf) {
    let original_workspace = cfg.workspace_dir.clone();
    cfg.workspace_dir = new_workspace;

    rebase_path_if_under_workspace(
        &mut cfg.memory.sqlite_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.memory.core_markdown,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.memory.daily_dir,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.state_sql.sqlite_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.heartbeat.tasks_file,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.worker.inbox_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.worker.outbox_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.worker.inflight_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.worker.state_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.worker.dead_letter_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.logging.file,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(&mut cfg.skills.dir, &original_workspace, &cfg.workspace_dir);
    rebase_path_if_under_workspace(
        &mut cfg.forex.redb_path,
        &original_workspace,
        &cfg.workspace_dir,
    );
    rebase_path_if_under_workspace(
        &mut cfg.runtime.session_dir,
        &original_workspace,
        &cfg.workspace_dir,
    );
}

fn rebase_path_if_under_workspace(
    path: &mut PathBuf,
    original: &std::path::Path,
    new_workspace: &std::path::Path,
) {
    if let Ok(relative) = path.strip_prefix(original) {
        *path = if relative.as_os_str().is_empty() {
            new_workspace.to_path_buf()
        } else {
            new_workspace.join(relative)
        };
    }
}

fn operator_identity(cfg: &Config) -> String {
    env::var("QUANT_M_OPERATOR")
        .or_else(|_| env::var("USER"))
        .or_else(|_| env::var("USERNAME"))
        .map(|value| format!("operator:{}", value.trim()))
        .unwrap_or_else(|_| format!("operator:{}", cfg.node_id))
}

fn run_boil_cli(
    cfg: &Config,
    args: Vec<String>,
    json: bool,
    dry_run: bool,
    pricing_profile: String,
) -> Result<()> {
    match args.as_slice() {
        [command, session_id, evidence_id] if command == "evidence" => {
            if dry_run {
                anyhow::bail!("--dry-run is not supported with boil evidence");
            }
            let lookup = boil::lookup_evidence(cfg, &session_id.parse()?, evidence_id)?;
            if json {
                println!("{}", serde_json::to_string_pretty(&lookup)?);
            } else {
                print!("{}", boil::render_evidence_lookup(&lookup));
            }
            Ok(())
        }
        [session_id] => {
            let report = boil::run_boil(
                cfg,
                boil::BoilRequest {
                    session_id: session_id.parse()?,
                    dry_run,
                    pricing_profile,
                },
            )?;
            if json {
                println!("{}", serde_json::to_string_pretty(&report)?);
            } else {
                print!("{}", boil::render_boil_report(&report));
            }
            Ok(())
        }
        [command, ..] if command == "evidence" => {
            anyhow::bail!("usage: quant-m boil evidence <session_id> <evidence_id>")
        }
        _ => anyhow::bail!("usage: quant-m boil <session_id>"),
    }
}

fn bind_with_optional_port(bind: &str, port: Option<u16>) -> String {
    match port {
        Some(port) => {
            let host = bind.rsplit_once(':').map(|(host, _)| host).unwrap_or(bind);
            format!("{host}:{port}")
        }
        None => bind.to_string(),
    }
}

fn storage_mode_for_command(command: &Commands) -> StorageMode {
    match command {
        Commands::Start
        | Commands::Config { .. }
        | Commands::Onboard { .. }
        | Commands::Onboarding { .. }
        | Commands::Setup { .. }
        | Commands::Provider { .. }
        | Commands::Tool { .. }
        | Commands::Settings { .. } => StorageMode::Inspect,
        Commands::Doctor { .. } => StorageMode::SessionWrite,
        Commands::Agent => StorageMode::Inspect,
        Commands::Tui { .. } => StorageMode::Inspect,
        Commands::Demo => StorageMode::SessionWrite,
        Commands::Skill { .. }
        | Commands::Policy { .. }
        | Commands::Workflow { .. }
        | Commands::Fsm { .. }
        | Commands::Scheduler { .. }
        | Commands::Desk { .. }
        | Commands::Domain { .. }
        | Commands::Channel { .. }
        | Commands::Cockpit { .. }
        | Commands::Compact { .. }
        | Commands::ContextStatus { .. }
        | Commands::Replay { .. }
        | Commands::Cost { .. }
        | Commands::InitTruth { .. } => StorageMode::Inspect,
        Commands::Capabilities { .. } => StorageMode::Inspect,
        Commands::Boil { args, dry_run, .. } => {
            if args.first().is_some_and(|value| value == "evidence") || *dry_run {
                StorageMode::Inspect
            } else {
                StorageMode::SessionWrite
            }
        }
        Commands::Context { .. } => StorageMode::SessionWrite,
        Commands::Question { command } => match command {
            QuestionCommand::Ask {
                write_proposals, ..
            } => {
                if *write_proposals {
                    StorageMode::SessionWrite
                } else {
                    StorageMode::Inspect
                }
            }
        },
        Commands::Run { .. } | Commands::Consensus { .. } | Commands::Strategist { .. } => {
            StorageMode::SessionWrite
        }
        Commands::Council { command } => match command {
            CouncilCommand::Policy { .. } => StorageMode::Inspect,
            CouncilCommand::Shadow { record, .. } => {
                if *record {
                    StorageMode::SessionWrite
                } else {
                    StorageMode::Inspect
                }
            }
        },
        Commands::Session { command } => match command {
            SessionCommand::List
            | SessionCommand::Show { .. }
            | SessionCommand::Replay { .. }
            | SessionCommand::ResumePlan { .. } => StorageMode::Inspect,
            SessionCommand::Approve { .. }
            | SessionCommand::Deny { .. }
            | SessionCommand::NeedsInfo { .. } => StorageMode::SessionWrite,
        },
        Commands::Daemon { .. } | Commands::Heartbeat { .. } | Commands::Telegram { .. } => {
            StorageMode::WorkerRun
        }
        Commands::Bootstrap { command } => match command {
            BootstrapCommand::List { .. } => StorageMode::Inspect,
            BootstrapCommand::Serve { .. } => StorageMode::WorkerRun,
        },
        Commands::Pack { command } => match command {
            PackCommand::List { .. } => StorageMode::Inspect,
            PackCommand::Serve { .. } => StorageMode::WorkerRun,
        },
        Commands::Pair { command } => match command {
            PairCommand::Cockpit { dry_run, .. } => {
                if *dry_run {
                    StorageMode::Inspect
                } else {
                    StorageMode::SessionWrite
                }
            }
            PairCommand::Doctor { .. } => StorageMode::Inspect,
            PairCommand::Status { .. } => StorageMode::Inspect,
            PairCommand::Serve { .. } => StorageMode::WorkerRun,
        },
        Commands::Device { command } => match command {
            DeviceCommand::Add { watch, dry_run, .. } => {
                if *watch || *dry_run {
                    StorageMode::Inspect
                } else {
                    StorageMode::SessionWrite
                }
            }
        },
        Commands::Child { command } => match command {
            ChildCommand::List { .. } => StorageMode::Inspect,
            ChildCommand::Join { .. }
            | ChildCommand::Identity { .. }
            | ChildCommand::Heartbeat { .. } => StorageMode::SessionWrite,
            ChildCommand::Approve { .. }
            | ChildCommand::Deny { .. }
            | ChildCommand::Revoke { .. } => StorageMode::SessionWrite,
        },
        Commands::Worker { command } => match command {
            WorkerCommand::Proposal {
                command: WorkerProposalCommand::List { .. },
            } => StorageMode::Inspect,
            WorkerCommand::Proposal {
                command: WorkerProposalCommand::Submit { .. },
            } => StorageMode::SessionWrite,
            WorkerCommand::Submit { .. } | WorkerCommand::Once { .. } | WorkerCommand::Run => {
                StorageMode::WorkerRun
            }
        },
        Commands::Init { .. }
        | Commands::Status
        | Commands::Memory { .. }
        | Commands::Adapter { .. }
        | Commands::Llm { .. } => StorageMode::RuntimePreflight,
        Commands::Loop { .. } => StorageMode::Inspect,
        Commands::State { command } => match command {
            StateCommand::List { .. }
            | StateCommand::Show { .. }
            | StateCommand::Snapshot { .. }
            | StateCommand::Review { .. }
            | StateCommand::ExpireStale => StorageMode::Inspect,
            StateCommand::Init
            | StateCommand::Summary
            | StateCommand::SignalUpsert { .. }
            | StateCommand::HandoffAdd { .. }
            | StateCommand::HandoffList { .. }
            | StateCommand::RiskAdd { .. }
            | StateCommand::OrderAdd { .. }
            | StateCommand::ForexIngest { .. }
            | StateCommand::ForexGetSignal { .. }
            | StateCommand::ForexGetHandoff { .. }
            | StateCommand::SwapHealth { .. }
            | StateCommand::SwapHealthGet { .. }
            | StateCommand::MacroRefreshMql5 { .. }
            | StateCommand::MacroGetPair { .. } => StorageMode::RuntimePreflight,
        },
        Commands::Skills { command } => match command {
            SkillsCommand::List | SkillsCommand::Show { .. } => StorageMode::Inspect,
            SkillsCommand::Run { .. } => StorageMode::RuntimePreflight,
        },
    }
}

fn parse_cockpit_host(value: &str) -> Result<terminal_cockpit::HostPlatform> {
    match value.trim().to_ascii_lowercase().as_str() {
        "auto" => Ok(terminal_cockpit::HostPlatform::detect()),
        "android" | "android-termux" | "android_termux" | "termux" => {
            Ok(terminal_cockpit::HostPlatform::AndroidTermux)
        }
        "macos" | "mac" | "darwin" | "apple" => Ok(terminal_cockpit::HostPlatform::Macos),
        "linux" => Ok(terminal_cockpit::HostPlatform::Linux),
        "windows" | "win" => Ok(terminal_cockpit::HostPlatform::Windows),
        "unknown" | "plain" => Ok(terminal_cockpit::HostPlatform::Unknown),
        other => Err(anyhow::anyhow!(
            "unsupported cockpit host '{other}'; expected auto, android, macos, linux, windows, or unknown"
        )),
    }
}

fn build_cockpit_lane_inputs(
    repo_paths: Vec<PathBuf>,
    models: Vec<String>,
) -> Vec<terminal_cockpit::CockpitLaneInput> {
    repo_paths
        .into_iter()
        .enumerate()
        .map(|(index, repo_path)| terminal_cockpit::CockpitLaneInput {
            repo_path,
            model: models.get(index).cloned(),
        })
        .collect()
}

fn prepare_storage_for_command(cfg: &Config, command: &Commands) -> Result<()> {
    prepare_storage_for_mode(cfg, storage_mode_for_command(command))
}

fn prepare_storage_for_mode(cfg: &Config, mode: StorageMode) -> Result<()> {
    match mode {
        StorageMode::Inspect | StorageMode::SessionWrite => Ok(()),
        StorageMode::RuntimePreflight | StorageMode::WorkerRun => preflight_runtime(cfg),
    }
}

fn preflight_runtime(cfg: &Config) -> Result<()> {
    let _ = MemoryStore::open(cfg).context("memory preflight failed")?;
    state_sql::sanity_check(cfg).context("shared-state preflight failed")?;
    forex::preflight(cfg).context("forex redb preflight failed")?;
    Ok(())
}

fn resolve_config_path(input: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = input {
        return Ok(path);
    }
    Ok(std::env::current_dir()
        .context("failed to read current directory")?
        .join("quant-m.toml"))
}

fn print_status(cfg: &Config) -> Result<()> {
    let payload = build_status_payload(cfg);
    println!("{}", serde_json::to_string_pretty(&payload)?);
    Ok(())
}

fn build_status_payload(cfg: &Config) -> serde_json::Value {
    let mut status_errors = Vec::new();

    let memory_count = match MemoryStore::open(cfg).and_then(|store| store.count()) {
        Ok(value) => Some(value),
        Err(err) => {
            status_errors.push(format!("memory_count_error={}", err));
            None
        }
    };
    let inbox_depth = match worker::queue_depth(&cfg.worker.inbox_path) {
        Ok(value) => Some(value),
        Err(err) => {
            status_errors.push(format!("inbox_depth_error={}", err));
            None
        }
    };
    let outbox_depth = match worker::queue_depth(&cfg.worker.outbox_path) {
        Ok(value) => Some(value),
        Err(err) => {
            status_errors.push(format!("outbox_depth_error={}", err));
            None
        }
    };
    let dead_letter_depth = match worker::queue_depth(&cfg.worker.dead_letter_path) {
        Ok(value) => Some(value),
        Err(err) => {
            status_errors.push(format!("dead_letter_depth_error={}", err));
            None
        }
    };
    let skills_count = match skills::list_skills(cfg).map(|items| items.len()) {
        Ok(value) => Some(value),
        Err(err) => {
            status_errors.push(format!("skills_count_error={}", err));
            None
        }
    };
    let worker_state = match worker::read_state_checked(&cfg.worker.state_path) {
        Ok(state) => state,
        Err(err) => {
            status_errors.push(format!("worker_state_error={}", err));
            None
        }
    };
    let shared_state = match state_sql::summary(cfg) {
        Ok(summary) => Some(summary),
        Err(err) => {
            status_errors.push(format!("shared_state_error={}", err));
            None
        }
    };
    let forex_redb_ready = match forex::preflight(cfg) {
        Ok(()) => true,
        Err(err) => {
            status_errors.push(format!("forex_redb_error={}", err));
            false
        }
    };

    serde_json::json!({
        "node_id": cfg.node_id,
        "role": onboarding_role_label(cfg.runtime.role),
        "runtime_profile": runtime_profile_label(cfg.runtime.profile),
        "workspace": cfg.workspace_dir,
        "memory_count": memory_count,
        "skills_count": skills_count,
        "queues": {
            "inbox_depth": inbox_depth,
            "outbox_depth": outbox_depth,
            "dead_letter_depth": dead_letter_depth
        },
        "heartbeat": {
            "enabled": cfg.heartbeat.enabled,
            "interval_seconds": cfg.heartbeat.interval_seconds
        },
        "llm": {
            "enabled": cfg.llm.enabled,
            "model": cfg.llm.model
        },
        "http_get_lane": {
            "enabled": cfg.worker.allow_http_get,
            "mode": cfg.worker.http_get_mode
        },
        "telegram": {
            "enabled": cfg.telegram.enabled,
            "allowed_chat_id": cfg.telegram.allowed_chat_id
        },
        "chat_channels": {
            "enabled": cfg.chat_channels.enabled,
            "default_channel": channels::channel_label(cfg.chat_channels.default_channel),
            "configured": channels::configured_channels(cfg)
        },
        "worker_state": worker_state
        ,
        "shared_state": shared_state,
        "forex_state": {
            "redb_path": cfg.forex.redb_path,
            "ready": forex_redb_ready
        },
        "degraded": !status_errors.is_empty(),
        "status_errors": status_errors
    })
}

fn parse_json_input<T: DeserializeOwned>(raw: &str, label: &str) -> Result<T> {
    serde_json::from_str(raw).with_context(|| format!("invalid JSON for {}", label))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shared_state::SharedStateStore;
    use redb::Database;
    use std::thread;
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        let workspace = tmp.path().join("workspace");
        cfg.workspace_dir = workspace.clone();
        cfg.memory.sqlite_path = workspace.join("memory/brain.db");
        cfg.memory.core_markdown = workspace.join("MEMORY.md");
        cfg.memory.daily_dir = workspace.join("daily");
        cfg.worker.inbox_path = workspace.join("queue/inbox.ndjson");
        cfg.worker.outbox_path = workspace.join("queue/outbox.ndjson");
        cfg.worker.inflight_path = workspace.join("queue/inflight.json");
        cfg.worker.state_path = workspace.join("state/worker_state.json");
        cfg.worker.dead_letter_path = workspace.join("queue/dead-letter.ndjson");
        cfg.logging.file = workspace.join("logs/quant-m.log");
        cfg.skills.dir = workspace.join("skills");
        cfg.state_sql.sqlite_path = workspace.join("state/shared-state.db");
        cfg.forex.redb_path = workspace.join("state/forex.redb");
        cfg.runtime.session_dir = workspace.join("state/sessions");
        (tmp, cfg)
    }

    fn sample_shared_state_record() -> shared_state::SharedStateRecord {
        shared_state::SharedStateRecord {
            key: shared_state::SharedStateKey::new("shared.alpha"),
            value: shared_state::SharedStateValue::Status("ready".to_string()),
            domain_id: sessions::DomainId::new("domain:test"),
            source: "test".to_string(),
            confidence: 0.9,
            updated_at: "2026-05-31T00:00:00+00:00".to_string(),
            expires_at: Some("2026-06-01T00:00:00+00:00".to_string()),
            session_id: None,
        }
    }

    fn lock_forex_db(cfg: &Config) -> Database {
        if let Some(parent) = cfg.forex.redb_path.parent() {
            std::fs::create_dir_all(parent).expect("create forex state dir");
        }
        Database::create(&cfg.forex.redb_path).expect("lock forex redb")
    }

    fn config_path_for(tmp: &TempDir) -> PathBuf {
        tmp.path().join("quant-m.toml")
    }

    #[test]
    fn init_creates_config_with_safe_defaults() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);

        let report = run_init_flow(&config_path, true).expect("init flow");
        let cfg = Config::load_existing(&config_path).expect("load config");

        assert_eq!(report.status, "ok");
        assert!(config_path.exists());
        assert!(cfg.workspace_dir.exists());
        assert!(!cfg.preferences.onboarding_completed);
        assert_eq!(
            cfg.runtime.session_dir,
            tmp.path().join("workspace/state/sessions")
        );
        assert!(!cfg.worker.allow_http_get);
        assert!(!cfg.llm.enabled);
        assert!(!cfg.telegram.enabled);
    }

    #[test]
    fn setup_can_run_non_interactively() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");

        let report = run_setup_flow(
            &config_path,
            SetupArgs {
                non_interactive: true,
                force_interactive: false,
                advanced: false,
                local_model_provider: Some("ollama".to_string()),
                local_model: Some("qwen3-coder:7b".to_string()),
                local_models: vec!["qwen3-coder:7b".to_string(), "llama3.1:8b".to_string()],
                remote_model_provider: Some("openrouter".to_string()),
                remote_model: Some("qwen/qwen3-coder".to_string()),
                openrouter_models: vec![
                    "qwen/qwen3-coder".to_string(),
                    "openai/gpt-4o-mini".to_string(),
                ],
                openrouter_api_key: Some("test-key".to_string()),
                enable_openrouter: false,
                channel: Some("telegram".to_string()),
                channel_value: Some("disabled".to_string()),
                role: None,
                runtime_profile: Some("edge".to_string()),
                workspace_path: Some(tmp.path().join("portable-workspace")),
                state_path: Some(tmp.path().join("portable-workspace/state/shared-state.db")),
                session_path: Some(tmp.path().join("portable-workspace/state/sessions")),
                external_network: Some("disabled".to_string()),
                context_guardian: Some("disabled".to_string()),
                selected_tools: Vec::new(),
                replace_tools: false,
            },
        )
        .expect("setup");

        let cfg = Config::load_existing(&config_path).expect("load config");
        assert_eq!(report.status, "ok_non_interactive");
        assert!(cfg.preferences.onboarding_completed);
        assert_eq!(cfg.runtime.profile, config::RuntimeProfile::Edge);
        assert_eq!(cfg.runtime.role, config::OnboardingRole::SoloLocalNode);
        assert_eq!(cfg.workspace_dir, tmp.path().join("portable-workspace"));
        assert_eq!(
            cfg.state_sql.sqlite_path,
            tmp.path().join("portable-workspace/state/shared-state.db")
        );
        assert_eq!(
            cfg.runtime.session_dir,
            tmp.path().join("portable-workspace/state/sessions")
        );
        assert_eq!(
            cfg.preferences.preferred_openrouter_model.as_deref(),
            Some("qwen/qwen3-coder")
        );
        assert_eq!(
            cfg.providers
                .get("openrouter")
                .expect("openrouter")
                .preferred_models,
            vec![
                "qwen/qwen3-coder".to_string(),
                "openai/gpt-4o-mini".to_string(),
            ]
        );
        assert_eq!(
            cfg.providers
                .get("ollama")
                .expect("ollama")
                .preferred_models,
            vec!["qwen3-coder:7b".to_string(), "llama3.1:8b".to_string()]
        );
        assert!(cfg.llm.enabled);
        assert_eq!(cfg.llm.api_key.as_deref(), Some("test-key"));
        assert!(!cfg.context_guardian.enabled);
    }

    #[test]
    fn setup_does_not_complete_when_workspace_cannot_be_created() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");
        let blocked_parent = tmp.path().join("not-a-directory");
        std::fs::write(&blocked_parent, "file blocks directory creation").expect("write blocker");

        let err = run_setup_flow(
            &config_path,
            SetupArgs {
                non_interactive: true,
                force_interactive: false,
                advanced: false,
                local_model_provider: None,
                local_model: None,
                local_models: Vec::new(),
                remote_model_provider: None,
                remote_model: None,
                openrouter_models: Vec::new(),
                openrouter_api_key: None,
                enable_openrouter: false,
                channel: Some("none".to_string()),
                channel_value: None,
                role: Some("solo-local-node".to_string()),
                runtime_profile: Some("laptop".to_string()),
                workspace_path: Some(blocked_parent.join("workspace")),
                state_path: None,
                session_path: None,
                external_network: Some("disabled".to_string()),
                context_guardian: Some("enabled".to_string()),
                selected_tools: vec!["codex".to_string()],
                replace_tools: true,
            },
        )
        .expect_err("setup should fail before saving completed onboarding");

        assert!(err.to_string().contains("failed to prepare workspace"));
        let cfg = Config::load_existing(&config_path).expect("load config");
        assert!(!cfg.preferences.onboarding_completed);
        assert!(!cfg.tools.get("codex").expect("codex").enabled);
    }

    #[test]
    fn local_model_detection_reads_ollama_manifest_tags() {
        let tmp = TempDir::new().expect("tempdir");
        let manifest = tmp
            .path()
            .join(".ollama/models/manifests/registry.ollama.ai/library/qwen3-coder/7b");
        std::fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("mkdir");
        std::fs::write(&manifest, "{}").expect("write manifest");

        let models = detect_ollama_model_tags_in(tmp.path());

        assert_eq!(models, vec!["qwen3-coder:7b"]);
    }

    #[test]
    fn local_model_detection_keeps_non_library_ollama_namespace() {
        let tmp = TempDir::new().expect("tempdir");
        let manifest = tmp
            .path()
            .join(".ollama/models/manifests/registry.ollama.ai/acme/private-coder/latest");
        std::fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("mkdir");
        std::fs::write(&manifest, "{}").expect("write manifest");

        let models = detect_ollama_model_tags_in(tmp.path());

        assert_eq!(models, vec!["acme/private-coder:latest"]);
    }

    #[test]
    fn local_model_detection_reads_lmstudio_model_files() {
        let tmp = TempDir::new().expect("tempdir");
        let model = tmp
            .path()
            .join("Library/Application Support/LM Studio/models/qwen/qwen3-coder-7b.gguf");
        std::fs::create_dir_all(model.parent().expect("model parent")).expect("mkdir");
        std::fs::write(&model, "").expect("write model marker");

        let models = detect_lmstudio_model_tags_in(tmp.path());

        assert_eq!(models, vec!["qwen3-coder-7b"]);
    }

    #[test]
    fn local_model_detection_reads_lmstudio_windows_style_roots() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("LocalAppData/LM Studio/models");
        let model = root.join("publisher/model-family/model-file.safetensors");
        std::fs::create_dir_all(model.parent().expect("model parent")).expect("mkdir");
        std::fs::write(&model, "").expect("write model marker");

        let models = detect_lmstudio_model_tags_from_roots(&[root]);

        assert_eq!(models, vec!["model-file"]);
    }

    #[test]
    fn local_model_detection_reads_ollama_custom_model_roots() {
        let tmp = TempDir::new().expect("tempdir");
        let root = tmp.path().join("ollama-models/manifests");
        let manifest = root.join("registry.ollama.ai/library/llama3.1/8b");
        std::fs::create_dir_all(manifest.parent().expect("manifest parent")).expect("mkdir");
        std::fs::write(&manifest, "{}").expect("write manifest");

        let models = detect_ollama_model_tags_from_roots(&[root]);

        assert_eq!(models, vec!["llama3.1:8b"]);
    }

    #[test]
    fn local_model_options_keep_default_selection_stable() {
        let options = default_local_model_options("ollama");
        let selected = parse_model_selection("1", &options, false).expect("parse");

        assert_eq!(selected, vec!["qwen3-coder:7b"]);
    }

    #[test]
    fn onboard_command_parses() {
        let cli = Cli::try_parse_from(["quant-m", "onboard"]).expect("parse onboard");
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                advanced: false,
                json: false
            })
        ));

        let cli =
            Cli::try_parse_from(["quant-m", "onboard", "--advanced"]).expect("parse advanced");
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                advanced: true,
                json: false
            })
        ));

        let cli = Cli::try_parse_from(["quant-m", "onboard", "--json"]).expect("parse json");
        assert!(matches!(
            cli.command,
            Some(Commands::Onboard {
                advanced: false,
                json: true
            })
        ));
    }

    #[test]
    fn pairing_cockpit_commands_parse_and_use_safe_storage_modes() {
        let cli = Cli::try_parse_from(["quant-m", "pair", "cockpit", "--dry-run"])
            .expect("parse pair cockpit");
        match cli.command {
            Some(Commands::Pair {
                command:
                    PairCommand::Cockpit {
                        bind,
                        host,
                        port,
                        interface,
                        dry_run: true,
                        qr: true,
                    },
            }) => {
                assert_eq!(bind, "0.0.0.0:8787");
                assert_eq!(host, None);
                assert_eq!(port, None);
                assert_eq!(interface, None);
                assert_eq!(
                    storage_mode_for_command(&Commands::Pair {
                        command: PairCommand::Cockpit {
                            bind: "0.0.0.0:8787".to_string(),
                            host: None,
                            port: None,
                            interface: None,
                            dry_run: true,
                            qr: true,
                        },
                    }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from([
            "quant-m",
            "pair",
            "doctor",
            "--host",
            "192.168.1.50",
            "--port",
            "8788",
            "--interface",
            "en0",
            "--json",
        ])
        .expect("parse pair doctor");
        assert!(matches!(
            cli.command,
            Some(Commands::Pair {
                command: PairCommand::Doctor {
                    host: Some(host),
                    port: Some(8788),
                    interface: Some(interface),
                    json: true,
                    ..
                }
            }) if host == "192.168.1.50" && interface == "en0"
        ));

        let cli = Cli::try_parse_from(["quant-m", "pair", "status", "--json"])
            .expect("parse pair status");
        assert!(matches!(
            cli.command,
            Some(Commands::Pair {
                command: PairCommand::Status { json: true, .. }
            })
        ));

        let cli = Cli::try_parse_from(["quant-m", "pair", "serve", "--allow-public-bind"])
            .expect("parse pair serve");
        assert!(matches!(
            cli.command,
            Some(Commands::Pair {
                command: PairCommand::Serve {
                    allow_public_bind: true,
                    ..
                }
            })
        ));
    }

    #[test]
    fn device_and_child_pairing_commands_parse() {
        let cli = Cli::try_parse_from(["quant-m", "device", "add", "--qr", "--dry-run"])
            .expect("parse device add");
        assert!(matches!(
            cli.command,
            Some(Commands::Device {
                command: DeviceCommand::Add {
                    qr: true,
                    dry_run: true,
                    ttl_minutes: 30,
                    host: None,
                    ..
                }
            })
        ));

        let cli = Cli::try_parse_from([
            "quant-m",
            "device",
            "add",
            "--host",
            "192.168.1.42",
            "--port",
            "8789",
            "--interface",
            "wlan0",
            "--dry-run",
        ])
        .expect("parse device add host options");
        assert!(matches!(
            cli.command,
            Some(Commands::Device {
                command: DeviceCommand::Add {
                    host: Some(host),
                    port: Some(8789),
                    interface: Some(interface),
                    dry_run: true,
                    ..
                }
            }) if host == "192.168.1.42" && interface == "wlan0"
        ));

        let cli = Cli::try_parse_from(["quant-m", "device", "add", "--watch"])
            .expect("parse device add watch");
        assert!(matches!(
            cli.command,
            Some(Commands::Device {
                command: DeviceCommand::Add { watch: true, .. }
            })
        ));

        let cli = Cli::try_parse_from(["quant-m", "child", "list", "--include-revoked"])
            .expect("parse child list");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::List {
                    include_revoked: true,
                    ..
                }
            })
        ));

        let cli = Cli::try_parse_from([
            "quant-m",
            "child",
            "join",
            "--url",
            "http://127.0.0.1:8787/join/inv-1",
        ])
        .expect("parse child join");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::Join {
                    url: Some(url),
                    manual: false,
                    ..
                }
            }) if url == "http://127.0.0.1:8787/join/inv-1"
        ));

        let cli = Cli::try_parse_from(["quant-m", "child", "join", "--manual"])
            .expect("parse child join manual");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::Join { manual: true, .. }
            })
        ));

        let cli = Cli::try_parse_from(["quant-m", "child", "identity", "--json"])
            .expect("parse child identity");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::Identity { json: true }
            })
        ));

        let cli = Cli::try_parse_from([
            "quant-m",
            "child",
            "heartbeat",
            "--core",
            "http://127.0.0.1:8787",
            "--once",
            "--json",
        ])
        .expect("parse child heartbeat");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command:
                    ChildCommand::Heartbeat {
                        core: Some(core),
                        once: true,
                        json: true,
                        ..
                    }
            }) if core == "http://127.0.0.1:8787"
        ));

        let cli = Cli::try_parse_from(["quant-m", "child", "approve", "req-1"])
            .expect("parse child approve");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::Approve { request_id, .. }
            }) if request_id == "req-1"
        ));

        let cli =
            Cli::try_parse_from(["quant-m", "child", "deny", "req-1"]).expect("parse child deny");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::Deny { request_id, .. }
            }) if request_id == "req-1"
        ));

        let cli = Cli::try_parse_from(["quant-m", "child", "revoke", "child-1"])
            .expect("parse child revoke");
        assert!(matches!(
            cli.command,
            Some(Commands::Child {
                command: ChildCommand::Revoke { node_id, .. }
            }) if node_id == "child-1"
        ));
    }

    #[test]
    fn start_onboarding_gate_uses_completion_marker() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);

        assert!(start_needs_onboarding(&config_path).expect("missing config needs onboarding"));
        run_init_flow(&config_path, true).expect("init");
        assert!(start_needs_onboarding(&config_path).expect("init is not complete onboarding"));

        let mut cfg = Config::load_existing(&config_path).expect("load");
        cfg.preferences.onboarding_completed = true;
        cfg.save(&config_path).expect("save");

        assert!(!start_needs_onboarding(&config_path).expect("completed onboarding"));
    }

    #[test]
    fn start_requires_onboarding_before_chat_in_non_interactive_context() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);

        let err = run_start_flow(&config_path).expect_err("start should require onboarding");

        assert!(err.to_string().contains("first-run onboarding"));
        assert!(!config_path.exists());
    }

    #[test]
    fn start_flow_chat_message_is_explicit_solo_chat_only() {
        let tmp = TempDir::new().expect("tempdir");
        let message = start_chat_message(tmp.path());

        assert!(message.contains("Opening the governed Quant-M chat cockpit"));
        assert!(message.contains("/ask"));
        assert!(message.contains("quant-m shell"));
    }

    #[test]
    fn legacy_config_without_role_requires_safe_migration() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        std::fs::write(
            &config_path,
            r#"
node_id = "legacy-edge"
workspace_dir = "workspace"

[memory]
sqlite_path = "workspace/memory/brain.db"
core_markdown = "workspace/MEMORY.md"
daily_dir = "workspace/daily"
vector_weight = 0.7
keyword_weight = 0.3
vector_dims = 64

[state_sql]
sqlite_path = "workspace/state/quantm-state.db"

[heartbeat]
enabled = true
interval_seconds = 1800
tasks_file = "workspace/HEARTBEAT.md"

[worker]
inbox_path = "workspace/queue/inbox.ndjson"
outbox_path = "workspace/queue/outbox.ndjson"
inflight_path = "workspace/queue/inflight.json"
state_path = "workspace/state/worker_state.json"
dead_letter_path = "workspace/queue/dead-letter.ndjson"
poll_interval_seconds = 3
command_timeout_seconds = 60
concurrency = 1
max_retries = 1
max_inbox_depth = 1000
allow_shell_commands = false
allow_http_get = false
allow_insecure_https = false
http_get_mode = "deny"
http_get_sandbox_hosts = []

[adapters]
terminal_enabled = true
webhook_url = ""
webhook_timeout_seconds = 10

[logging]
file = "workspace/logs/quant-m.log"
max_bytes = 1048576
keep_files = 3

[skills]
dir = "workspace/skills"
allow_shell_commands = false

[runtime]
profile = "edge"
session_dir = "workspace/state/sessions"
external_network_enabled = false
multi_model_enabled = false
search_enabled = false
browser_harness_enabled = false

[preferences]
onboarding_completed = true
"#,
        )
        .expect("write legacy config");

        let mut cfg = Config::load_or_create(&config_path).expect("load legacy");
        cfg.ensure_onboarding_registries();
        cfg.save(&config_path).expect("persist migration");
        let migrated = Config::load_existing(&config_path).expect("reload migrated");
        let raw = std::fs::read_to_string(&config_path).expect("read migrated");

        assert_eq!(migrated.runtime.profile, config::RuntimeProfile::Edge);
        assert_eq!(migrated.runtime.role, config::OnboardingRole::SoloLocalNode);
        assert!(raw.contains("role = \"solo_local_node\""));
    }

    #[test]
    fn demo_command_parses() {
        let cli = Cli::try_parse_from(["quant-m"]).expect("parse default start");
        assert!(cli.command.is_none());

        let cli = Cli::try_parse_from(["quant-m", "start"]).expect("parse start");
        assert!(matches!(cli.command, Some(Commands::Start)));

        let cli = Cli::try_parse_from(["quant-m", "demo"]).expect("parse demo");
        assert!(matches!(cli.command, Some(Commands::Demo)));

        let cli = Cli::try_parse_from(["quant-m", "shell"]).expect("parse shell alias");
        assert!(matches!(cli.command, Some(Commands::Agent)));

        let cli = Cli::try_parse_from(["quant-m", "tui"]).expect("parse tui");
        assert!(matches!(cli.command, Some(Commands::Tui { command: None })));

        let cli =
            Cli::try_parse_from(["quant-m", "tui", "chat", "--inspect"]).expect("parse tui chat");
        assert!(matches!(
            cli.command,
            Some(Commands::Tui {
                command: Some(TuiCommand::Chat { inspect: true })
            })
        ));
        let cli = Cli::try_parse_from(["quant-m", "tui", "chat"]).expect("parse tui chat");
        assert!(matches!(
            cli.command,
            Some(Commands::Tui {
                command: Some(TuiCommand::Chat { inspect: false })
            })
        ));
        assert_eq!(
            storage_mode_for_command(&Commands::Tui {
                command: Some(TuiCommand::Chat { inspect: true })
            }),
            StorageMode::Inspect
        );

        let cli =
            Cli::try_parse_from(["quant-m", "tool", "setup", "claude"]).expect("parse tool setup");
        assert!(matches!(
            cli.command,
            Some(Commands::Tool {
                command: ToolCommand::Setup { tool }
            }) if tool == "claude"
        ));
    }

    #[test]
    fn onboarding_rejects_command_like_workspace_answers() {
        assert!(looks_like_pasted_command("cargo test setup --release"));
        assert!(looks_like_pasted_command("./target/release/quant-m --help"));
        assert!(looks_like_pasted_command("run demo"));
        assert!(looks_like_pasted_command("clear"));
        assert!(!looks_like_pasted_command("./workspace"));
        assert!(!looks_like_pasted_command("/home/user/quantm/workspace"));
    }

    #[test]
    fn prompt_input_strips_terminal_escape_sequences() {
        assert_eq!(strip_terminal_escape_input("\u{1b}[Cskip"), "skip");
        assert_eq!(strip_terminal_escape_input("^[[Cskip"), "skip");
        assert_eq!(strip_terminal_escape_input("skip"), "skip");
    }

    #[test]
    fn developer_tool_selection_supports_manual_cli_choices() {
        let (_tmp, cfg) = temp_cfg();

        let selected = parse_developer_tool_selection("2,3,antigravity,ollama,lmstudio", &cfg)
            .expect("parse tools");

        assert_eq!(
            selected,
            vec![
                "codex".to_string(),
                "openai".to_string(),
                "antigravity".to_string(),
                "ollama".to_string(),
                "lmstudio".to_string()
            ]
        );
    }

    #[test]
    fn developer_tool_selection_supports_none_and_dedupes() {
        let (_tmp, cfg) = temp_cfg();

        assert!(
            parse_developer_tool_selection("none", &cfg)
                .expect("parse none")
                .is_empty()
        );

        let selected =
            parse_developer_tool_selection("codex, codex, openai", &cfg).expect("parse tools");

        assert_eq!(selected, vec!["codex".to_string(), "openai".to_string()]);
    }

    #[test]
    fn developer_tool_selection_rejects_unknown_tools() {
        let (_tmp, cfg) = temp_cfg();

        assert!(parse_developer_tool_selection("madeup", &cfg).is_err());
    }

    #[test]
    fn setup_can_replace_stale_tool_choices() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");
        let mut cfg = Config::load_existing(&config_path).expect("load");
        enable_tool(&mut cfg, "codex");
        cfg.save(&config_path).expect("save stale tool");

        run_setup_flow(
            &config_path,
            SetupArgs {
                non_interactive: true,
                force_interactive: false,
                advanced: false,
                local_model_provider: None,
                local_model: None,
                local_models: Vec::new(),
                remote_model_provider: None,
                remote_model: None,
                openrouter_models: Vec::new(),
                openrouter_api_key: None,
                enable_openrouter: false,
                channel: None,
                channel_value: None,
                role: None,
                runtime_profile: None,
                workspace_path: None,
                state_path: None,
                session_path: None,
                external_network: None,
                context_guardian: None,
                selected_tools: vec!["openai".to_string()],
                replace_tools: true,
            },
        )
        .expect("replace tools");

        let cfg = Config::load_existing(&config_path).expect("reload");
        assert!(!cfg.tools.get("codex").expect("codex").enabled);
        assert!(cfg.tools.get("openai").expect("openai").enabled);
        assert_eq!(
            cfg.preferences.preferred_chat_tool.as_deref(),
            Some("openai")
        );
    }

    #[test]
    fn config_show_reads_typed_config() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");

        let cfg = Config::load_existing(&config_path).expect("load");
        let rendered = cfg.render_toml(&config_path).expect("render");

        assert!(rendered.contains("workspace_dir = \"workspace\""));
        assert!(rendered.contains("[runtime]"));
    }

    #[test]
    fn config_set_model_updates_typed_config() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");

        handle_config_command(
            &config_path,
            ConfigCommand::SetModel {
                provider: "openrouter".to_string(),
                model: "qwen/qwen3-coder".to_string(),
            },
        )
        .expect("set model");

        let cfg = Config::load_existing(&config_path).expect("load");
        assert_eq!(
            cfg.preferences.preferred_openrouter_model.as_deref(),
            Some("qwen/qwen3-coder")
        );
        assert_eq!(cfg.llm.model, "qwen/qwen3-coder");
    }

    #[test]
    fn config_clear_model_removes_stale_preferences() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");

        handle_config_command(
            &config_path,
            ConfigCommand::SetModel {
                provider: "openrouter".to_string(),
                model: "qwen/qwen3-coder".to_string(),
            },
        )
        .expect("set remote model");
        handle_config_command(
            &config_path,
            ConfigCommand::SetModel {
                provider: "ollama".to_string(),
                model: "qwen3-coder:7b".to_string(),
            },
        )
        .expect("set local model");

        handle_config_command(
            &config_path,
            ConfigCommand::ClearModel {
                provider: Some("openrouter".to_string()),
            },
        )
        .expect("clear openrouter");

        let cfg = Config::load_existing(&config_path).expect("load");
        assert!(cfg.preferences.preferred_openrouter_model.is_none());
        assert!(cfg.preferences.preferred_remote_model.is_none());
        assert_eq!(
            cfg.preferences
                .preferred_local_model
                .as_ref()
                .map(|preference| preference.model.as_str()),
            Some("qwen3-coder:7b")
        );

        handle_config_command(&config_path, ConfigCommand::ClearModel { provider: None })
            .expect("clear all");

        let cfg = Config::load_existing(&config_path).expect("load");
        assert!(cfg.preferences.preferred_openrouter_model.is_none());
        assert!(cfg.preferences.preferred_remote_model.is_none());
        assert!(cfg.preferences.preferred_local_model.is_none());
    }

    #[test]
    fn settings_report_shows_project_paths_and_model_choices() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_setup_flow(
            &config_path,
            SetupArgs {
                non_interactive: true,
                force_interactive: false,
                advanced: false,
                local_model_provider: Some("ollama".to_string()),
                local_model: Some("qwen3-coder:7b".to_string()),
                local_models: vec!["qwen3-coder:7b".to_string()],
                remote_model_provider: None,
                remote_model: None,
                openrouter_models: Vec::new(),
                openrouter_api_key: None,
                enable_openrouter: false,
                channel: None,
                channel_value: None,
                role: None,
                runtime_profile: None,
                workspace_path: Some(tmp.path().join("project-workspace")),
                state_path: None,
                session_path: None,
                external_network: None,
                context_guardian: None,
                selected_tools: Vec::new(),
                replace_tools: false,
            },
        )
        .expect("setup");

        let cfg = Config::load_existing(&config_path).expect("load");
        let rendered = format_settings_report(&settings_report(&cfg, &config_path));

        assert!(rendered.contains(&format!("config: {}", config_path.display())));
        assert!(rendered.contains("local_model: ollama qwen3-coder:7b"));
        assert!(rendered.contains("quant-m config clear-model [local|remote|openrouter|all]"));
    }

    #[test]
    fn config_set_channel_updates_typed_config() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");

        handle_config_command(
            &config_path,
            ConfigCommand::SetChannel {
                channel: "telegram".to_string(),
                value: "disabled".to_string(),
            },
        )
        .expect("set channel");

        let cfg = Config::load_existing(&config_path).expect("load");
        assert_eq!(
            cfg.preferences.preferred_channel.channel,
            config::ExternalChannel::Telegram
        );
        assert!(cfg.preferences.preferred_channel.value.is_none());
    }

    #[test]
    fn doctor_does_not_perform_network_calls() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);
        run_init_flow(&config_path, true).expect("init");

        let mut cfg = Config::load_existing(&config_path).expect("load config");
        cfg.llm.enabled = true;
        cfg.llm.api_key = None;
        cfg.telegram.enabled = true;
        cfg.telegram.bot_token = None;
        cfg.save(&config_path).expect("save config");

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let report = rt
            .block_on(run_doctor(&config_path, false, false))
            .expect("doctor");
        assert!(report.workflow_run_ok);
        assert!(report.shared_state_list_ok);
        assert!(report.session_list_ok);
    }

    #[test]
    fn doctor_bootstraps_missing_local_config() {
        let tmp = TempDir::new().expect("tempdir");
        let config_path = config_path_for(&tmp);

        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime");
        let report = rt
            .block_on(run_doctor(&config_path, false, false))
            .expect("doctor");

        assert!(config_path.exists());
        assert!(report.config_exists);
        assert!(report.workspace_exists);
        assert!(report.state_path_exists);
        assert!(report.session_path_exists);
        assert!(report.workflow_run_ok);
        assert!(report.shared_state_list_ok);
        assert!(report.session_list_ok);
    }

    #[test]
    fn status_payload_contains_core_fields() {
        let (_tmp, cfg) = temp_cfg();

        let payload = build_status_payload(&cfg);
        assert!(payload.get("node_id").is_some());
        assert!(payload.get("queues").is_some());
        assert!(payload.get("shared_state").is_some());
        assert_eq!(
            payload.get("degraded").and_then(|value| value.as_bool()),
            Some(false)
        );
    }

    #[test]
    fn status_payload_reports_degraded_when_reads_fail() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        let workspace = tmp.path().join("workspace");
        std::fs::create_dir_all(&workspace).expect("create workspace");
        cfg.workspace_dir = workspace.clone();
        cfg.memory.sqlite_path = workspace.clone();
        cfg.memory.core_markdown = workspace.join("MEMORY.md");
        cfg.memory.daily_dir = workspace.join("daily");
        cfg.worker.inbox_path = workspace.clone();
        cfg.worker.outbox_path = workspace.clone();
        cfg.worker.inflight_path = workspace.join("queue/inflight.json");
        cfg.worker.state_path = workspace.clone();
        cfg.worker.dead_letter_path = workspace.clone();
        cfg.logging.file = workspace.join("logs/quant-m.log");
        cfg.skills.dir = workspace.join("skills");
        cfg.state_sql.sqlite_path = workspace.clone();
        cfg.forex.redb_path = workspace.clone();

        let payload = build_status_payload(&cfg);
        assert_eq!(
            payload.get("degraded").and_then(|value| value.as_bool()),
            Some(true)
        );
        assert!(
            payload
                .get("status_errors")
                .and_then(|value| value.as_array())
                .map(|items| !items.is_empty())
                .unwrap_or(false)
        );
    }

    #[test]
    fn domain_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Domain {
                command: DomainCommand::List
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Domain {
                command: DomainCommand::Show {
                    domain_id: "domain:mock-trading".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn skill_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Skill {
                command: SkillCommand::List {
                    domain: None,
                    side_effect: None,
                }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Skill {
                command: SkillCommand::Show {
                    skill_id: "mock-research.capture-brief".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn policy_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Policy {
                command: PolicyCommand::List {
                    domain: None,
                    side_effect: None,
                }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Policy {
                command: PolicyCommand::Show {
                    policy_id: "mock-trading.local-write".to_string()
                }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Policy {
                command: PolicyCommand::EvaluateSkill {
                    skill_id: "mock-trading.prepare-paper-review".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn workflow_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Workflow {
                command: WorkflowCommand::List { domain: None }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Workflow {
                command: WorkflowCommand::Show {
                    workflow_id: "workflow:mock-research-brief".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn fsm_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Fsm {
                command: FsmCommand::Authority { json: false }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Fsm {
                command: FsmCommand::List { domain: None }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Fsm {
                command: FsmCommand::Show {
                    fsm_id: "fsm:mock-research-brief".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn fsm_authority_command_parses_and_uses_inspect_mode() {
        let cli =
            Cli::try_parse_from(["quant-m", "fsm", "authority"]).expect("parse fsm authority");
        assert!(matches!(
            cli.command,
            Some(Commands::Fsm {
                command: FsmCommand::Authority { json: false }
            })
        ));

        let cli = Cli::try_parse_from(["quant-m", "fsm", "authority", "--json"])
            .expect("parse fsm authority json");
        assert!(matches!(
            cli.command,
            Some(Commands::Fsm {
                command: FsmCommand::Authority { json: true }
            })
        ));
    }

    #[test]
    fn scheduler_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Scheduler {
                command: SchedulerCommand::List {
                    domain: None,
                    trigger: None,
                }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Scheduler {
                command: SchedulerCommand::Show {
                    scheduler_id: "scheduler:mock-research-brief".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn desk_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Desk {
                command: DeskCommand::List {
                    category: None,
                    domain: None,
                }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::Desk {
                command: DeskCommand::Show {
                    desk_id: "desk:mock-trading-paper".to_string()
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn run_workflow_commands_use_session_write_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::Run {
                command: RunCommand::Workflow {
                    workflow_id: "workflow:mock-research-brief".to_string()
                }
            }),
            StorageMode::SessionWrite
        );
    }

    #[test]
    fn worker_proposal_commands_parse_and_use_safe_storage_modes() {
        let submit = Cli::try_parse_from([
            "quant-m",
            "worker",
            "proposal",
            "submit",
            "--surface",
            "cmux_lane",
            "--kind",
            "evidence",
            "--summary",
            "Architecture lane recommends provider contracts after worker boundary hardening.",
            "--json",
        ])
        .expect("parse worker proposal submit");
        match submit.command {
            Some(Commands::Worker {
                command:
                    WorkerCommand::Proposal {
                        command:
                            WorkerProposalCommand::Submit {
                                surface,
                                kind,
                                summary,
                                json,
                                ..
                            },
                    },
            }) => {
                assert_eq!(surface, "cmux_lane");
                assert_eq!(kind, "evidence");
                assert!(summary.contains("provider contracts"));
                assert!(json);
                assert_eq!(
                    storage_mode_for_command(&Commands::Worker {
                        command: WorkerCommand::Proposal {
                            command: WorkerProposalCommand::Submit {
                                surface,
                                kind,
                                summary,
                                worker_id: None,
                                session_id: None,
                                workflow_id: None,
                                decision_scope: None,
                                json,
                            }
                        }
                    }),
                    StorageMode::SessionWrite
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let list = Cli::try_parse_from([
            "quant-m",
            "worker",
            "proposal",
            "list",
            "--surface",
            "cmux_lane",
            "--status",
            "pending_review",
            "--json",
        ])
        .expect("parse worker proposal list");
        match list.command {
            Some(Commands::Worker {
                command:
                    WorkerCommand::Proposal {
                        command:
                            WorkerProposalCommand::List {
                                surface,
                                status,
                                json,
                            },
                    },
            }) => {
                assert_eq!(surface.as_deref(), Some("cmux_lane"));
                assert_eq!(status.as_deref(), Some("pending_review"));
                assert!(json);
                assert_eq!(
                    storage_mode_for_command(&Commands::Worker {
                        command: WorkerCommand::Proposal {
                            command: WorkerProposalCommand::List {
                                surface,
                                status,
                                json,
                            }
                        }
                    }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn context_packet_command_parses_and_writes_packet_artifacts_only() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "context",
            "packet",
            "--state",
            "QUESTION_TO_WORKER_PROPOSAL_01_VALIDATED",
            "--size",
            "small",
            "--task",
            "Generate the next bounded agent packet.",
            "--json",
        ])
        .expect("parse context packet");

        match cli.command {
            Some(Commands::Context {
                command:
                    ContextCommand::Packet {
                        state,
                        size,
                        task,
                        json,
                    },
            }) => {
                assert_eq!(state, "QUESTION_TO_WORKER_PROPOSAL_01_VALIDATED");
                assert_eq!(size, "small");
                assert_eq!(
                    task.as_deref(),
                    Some("Generate the next bounded agent packet.")
                );
                assert!(json);
                assert_eq!(
                    storage_mode_for_command(&Commands::Context {
                        command: ContextCommand::Packet {
                            state,
                            size,
                            task,
                            json,
                        }
                    }),
                    StorageMode::SessionWrite
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn consensus_command_parses_and_uses_session_write_mode() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "consensus",
            "--dry-run",
            "Should we adopt this API design?",
        ])
        .expect("parse consensus");
        match cli.command {
            Some(Commands::Consensus { dry_run, question }) => {
                assert!(dry_run);
                assert_eq!(question, "Should we adopt this API design?");
                assert_eq!(
                    storage_mode_for_command(&Commands::Consensus { dry_run, question }),
                    StorageMode::SessionWrite
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn consensus_command_requires_question() {
        let err = Cli::try_parse_from(["quant-m", "consensus", "--dry-run"])
            .expect_err("missing question fails");
        assert!(err.to_string().contains("required"));
    }

    #[test]
    fn council_shadow_commands_use_safe_storage_modes() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "council",
            "shadow",
            "--input",
            "configs/council-shadow.example.json",
            "--json",
        ])
        .expect("parse council shadow");
        match cli.command {
            Some(Commands::Council {
                command:
                    CouncilCommand::Shadow {
                        input,
                        json,
                        record,
                    },
            }) => {
                assert_eq!(input, PathBuf::from("configs/council-shadow.example.json"));
                assert!(json);
                assert!(!record);
                assert_eq!(
                    storage_mode_for_command(&Commands::Council {
                        command: CouncilCommand::Shadow {
                            input,
                            json,
                            record,
                        }
                    }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        assert_eq!(
            storage_mode_for_command(&Commands::Council {
                command: CouncilCommand::Shadow {
                    input: PathBuf::from("packet.json"),
                    json: false,
                    record: true,
                }
            }),
            StorageMode::SessionWrite
        );
    }

    #[test]
    fn strategist_dry_run_commands_parse_and_use_session_write_mode() {
        let cli =
            Cli::try_parse_from(["quant-m", "strategist", "--dry-run"]).expect("parse strategist");
        match cli.command {
            Some(Commands::Strategist { dry_run, json }) => {
                assert!(dry_run);
                assert!(!json);
                assert_eq!(
                    storage_mode_for_command(&Commands::Strategist { dry_run, json }),
                    StorageMode::SessionWrite
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from(["quant-m", "strategist", "--dry-run", "--json"])
            .expect("parse strategist json");
        match cli.command {
            Some(Commands::Strategist { dry_run, json }) => {
                assert!(dry_run);
                assert!(json);
                assert_eq!(
                    storage_mode_for_command(&Commands::Strategist { dry_run, json }),
                    StorageMode::SessionWrite
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn question_command_parses_three_modes_and_uses_inspect_mode() {
        for (mode, text, json) in [
            ("agent-cluster", "How should this be reviewed?", false),
            ("handoff", "What should Codex implement next?", true),
            ("harness", "Which model route should handle this?", true),
        ] {
            let mut args = vec!["quant-m", "question", "ask", "--mode", mode, text];
            if json {
                args.push("--json");
            }
            let cli = Cli::try_parse_from(args).expect("parse question ask");
            match cli.command {
                Some(Commands::Question { command }) => match command {
                    QuestionCommand::Ask {
                        mode,
                        question,
                        json: parsed_json,
                        write_proposals,
                    } => {
                        assert_eq!(question, text);
                        assert_eq!(parsed_json, json);
                        assert!(!write_proposals);
                        assert!(mode.parse::<question::QuantMQuestionMode>().is_ok());
                        assert_eq!(
                            storage_mode_for_command(&Commands::Question {
                                command: QuestionCommand::Ask {
                                    mode,
                                    question,
                                    json: parsed_json,
                                    write_proposals,
                                }
                            }),
                            StorageMode::Inspect
                        );
                    }
                },
                other => panic!("unexpected command: {other:?}"),
            }
        }
    }

    #[test]
    fn question_write_proposals_flag_uses_session_write_mode() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "question",
            "ask",
            "--mode",
            "agent-cluster",
            "Review this API design decision",
            "--write-proposals",
            "--json",
        ])
        .expect("parse question write proposals");
        match cli.command {
            Some(Commands::Question { command }) => match command {
                QuestionCommand::Ask {
                    mode,
                    question,
                    json,
                    write_proposals,
                } => {
                    assert_eq!(mode, "agent-cluster");
                    assert_eq!(question, "Review this API design decision");
                    assert!(json);
                    assert!(write_proposals);
                    assert_eq!(
                        storage_mode_for_command(&Commands::Question {
                            command: QuestionCommand::Ask {
                                mode,
                                question,
                                json,
                                write_proposals,
                            }
                        }),
                        StorageMode::SessionWrite
                    );
                }
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn replay_command_parses_and_uses_inspect_mode() {
        let cli = Cli::try_parse_from(["quant-m", "replay", "session-1", "--json"])
            .expect("parse replay");
        match cli.command {
            Some(Commands::Replay { session_id, json }) => {
                assert_eq!(session_id, "session-1");
                assert!(json);
                assert_eq!(
                    storage_mode_for_command(&Commands::Replay { session_id, json }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn cost_summary_command_parses_and_uses_inspect_mode() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "cost",
            "summary",
            "--json",
            "--workflow",
            "workflow:one",
            "--session",
            "session-one",
        ])
        .expect("parse cost summary");
        match cli.command {
            Some(Commands::Cost { command }) => match command {
                CostCommand::Summary {
                    json,
                    workflow,
                    session,
                } => {
                    assert!(json);
                    assert_eq!(workflow.as_deref(), Some("workflow:one"));
                    assert_eq!(session.as_deref(), Some("session-one"));
                    assert_eq!(
                        storage_mode_for_command(&Commands::Cost {
                            command: CostCommand::Summary {
                                json,
                                workflow,
                                session,
                            }
                        }),
                        StorageMode::Inspect
                    );
                }
            },
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn boil_command_parses_and_uses_expected_storage_modes() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "boil",
            "session-1",
            "--json",
            "--pricing-profile",
            "manual-config",
        ])
        .expect("parse boil");
        match cli.command {
            Some(Commands::Boil {
                args,
                json,
                dry_run,
                pricing_profile,
            }) => {
                assert_eq!(args, vec!["session-1"]);
                assert!(json);
                assert!(!dry_run);
                assert_eq!(pricing_profile, "manual-config");
                assert_eq!(
                    storage_mode_for_command(&Commands::Boil {
                        args,
                        json,
                        dry_run,
                        pricing_profile,
                    }),
                    StorageMode::SessionWrite
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from(["quant-m", "boil", "session-1", "--dry-run"])
            .expect("parse boil dry-run");
        match cli.command {
            Some(Commands::Boil {
                args,
                json,
                dry_run,
                pricing_profile,
            }) => {
                assert_eq!(args, vec!["session-1"]);
                assert!(!json);
                assert!(dry_run);
                assert_eq!(pricing_profile, "rough-default");
                assert_eq!(
                    storage_mode_for_command(&Commands::Boil {
                        args,
                        json,
                        dry_run,
                        pricing_profile,
                    }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from(["quant-m", "boil", "evidence", "session-1", "step-000001"])
            .expect("parse boil evidence");
        match cli.command {
            Some(Commands::Boil {
                args,
                json,
                dry_run,
                pricing_profile,
            }) => {
                assert_eq!(args, vec!["evidence", "session-1", "step-000001"]);
                assert!(!json);
                assert!(!dry_run);
                assert_eq!(
                    storage_mode_for_command(&Commands::Boil {
                        args,
                        json,
                        dry_run,
                        pricing_profile,
                    }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }
    }

    #[test]
    fn capabilities_command_parses_and_uses_inspect_mode() {
        let cli = Cli::try_parse_from([
            "quant-m",
            "capabilities",
            "--json",
            "--category",
            "provider",
            "--status",
            "external_required",
        ])
        .expect("parse capabilities");
        match cli.command {
            Some(Commands::Capabilities {
                command,
                json,
                category,
                status,
            }) => {
                assert!(command.is_none());
                assert!(json);
                assert_eq!(category.as_deref(), Some("provider"));
                assert_eq!(status.as_deref(), Some("external_required"));
                assert_eq!(
                    storage_mode_for_command(&Commands::Capabilities {
                        command,
                        json,
                        category,
                        status,
                    }),
                    StorageMode::Inspect
                );
            }
            other => panic!("unexpected command: {other:?}"),
        }

        let cli = Cli::try_parse_from([
            "quant-m",
            "capabilities",
            "show",
            "providers.openrouter",
            "--json",
        ])
        .expect("parse capability show");
        assert!(matches!(
            cli.command,
            Some(Commands::Capabilities {
                command: Some(CapabilitiesCommand::Show { .. }),
                ..
            })
        ));

        let cli = Cli::try_parse_from(["quant-m", "capabilities", "audit-docs"])
            .expect("parse capability docs audit");
        assert!(matches!(
            cli.command,
            Some(Commands::Capabilities {
                command: Some(CapabilitiesCommand::AuditDocs { .. }),
                ..
            })
        ));
    }

    #[test]
    fn shared_state_commands_use_inspect_mode() {
        assert_eq!(
            storage_mode_for_command(&Commands::State {
                command: StateCommand::List { domain: None }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::State {
                command: StateCommand::Show {
                    key: "shared.alpha".to_string()
                }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::State {
                command: StateCommand::Snapshot { domain: None }
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::State {
                command: StateCommand::ExpireStale
            }),
            StorageMode::Inspect
        );
        assert_eq!(
            storage_mode_for_command(&Commands::State {
                command: StateCommand::Review {
                    domain: Some("consensus".to_string()),
                    json: true,
                }
            }),
            StorageMode::Inspect
        );
    }

    #[test]
    fn domain_list_and_show_do_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Domain {
                command: DomainCommand::List,
            },
        )
        .expect("domain list inspect");
        let listed = domain::builtin_registry().expect("registry").list();
        assert_eq!(listed.len(), 2);

        prepare_storage_for_command(
            &cfg,
            &Commands::Domain {
                command: DomainCommand::Show {
                    domain_id: "domain:mock-trading".to_string(),
                },
            },
        )
        .expect("domain show inspect");
        let shown = domain::builtin_registry()
            .expect("registry")
            .show(&"domain:mock-trading".parse().expect("domain id"))
            .expect("show domain");
        assert_eq!(shown.name, "Mock Trading");
    }

    #[test]
    fn read_only_session_commands_do_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let context = sessions::runtime_context("test-node", "worker");
        sessions::append_event(
            &cfg,
            &context,
            sessions::SessionEvent::Observation {
                message: "inspect".to_string(),
                job_id: None,
                detail: Some("read-only".to_string()),
            },
        )
        .expect("append session event");

        let _lock = lock_forex_db(&cfg);

        for command in [
            SessionCommand::List,
            SessionCommand::Show {
                id: context.session_id.to_string(),
            },
            SessionCommand::Replay {
                id: context.session_id.to_string(),
            },
            SessionCommand::ResumePlan {
                id: context.session_id.to_string(),
            },
        ] {
            prepare_storage_for_command(&cfg, &Commands::Session { command })
                .expect("session inspect");
        }

        assert_eq!(sessions::list_sessions(&cfg).expect("list").len(), 1);
        assert_eq!(
            sessions::show_session(&cfg, &context.session_id)
                .expect("show")
                .events
                .len(),
            1
        );
        assert_eq!(
            sessions::replay_session(&cfg, &context.session_id)
                .expect("replay")
                .summary
                .session_id,
            context.session_id
        );
        assert_eq!(
            sessions::resume_plan_session(&cfg, &context.session_id)
                .expect("resume plan")
                .session_id,
            context.session_id
        );
    }

    #[test]
    fn operator_decision_commands_do_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let context = sessions::runtime_context("test-node", "worker");
        sessions::append_event(
            &cfg,
            &context,
            sessions::SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled".to_string(),
            },
        )
        .expect("append policy");

        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Session {
                command: SessionCommand::Approve {
                    id: context.session_id.to_string(),
                    reason: "approved for follow-up".to_string(),
                },
            },
        )
        .expect("session approve mode");
        let record = sessions::record_operator_decision(
            &cfg,
            &context.session_id,
            sessions::OperatorDecision::Approved,
            "approved for follow-up",
            "operator:test",
        )
        .expect("record decision");
        assert_eq!(record.session_id, context.session_id);
    }

    #[test]
    fn skill_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Skill {
                command: SkillCommand::List {
                    domain: Some("domain:mock-trading".to_string()),
                    side_effect: Some("read_only".to_string()),
                },
            },
        )
        .expect("skill list inspect");
        let readonly = skill_registry::builtin_registry().expect("registry").list(
            Some(&"domain:mock-trading".parse().expect("domain id")),
            Some(&"read_only".parse().expect("side effect")),
        );
        assert!(!readonly.is_empty());

        prepare_storage_for_command(
            &cfg,
            &Commands::Skill {
                command: SkillCommand::Show {
                    skill_id: "mock-research.capture-brief".to_string(),
                },
            },
        )
        .expect("skill show inspect");
        let shown = skill_registry::builtin_registry()
            .expect("registry")
            .show("mock-research.capture-brief")
            .expect("show skill");
        assert_eq!(
            shown.side_effect_level,
            skill_registry::SideEffectLevel::ReadOnly
        );
    }

    #[test]
    fn policy_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Policy {
                command: PolicyCommand::List {
                    domain: Some("domain:mock-trading".to_string()),
                    side_effect: Some("local_write".to_string()),
                },
            },
        )
        .expect("policy list inspect");
        let listed = policy_registry::builtin_registry().expect("registry").list(
            Some(&"domain:mock-trading".parse().expect("domain id")),
            Some(&"local_write".parse().expect("side effect")),
        );
        assert!(!listed.is_empty());

        prepare_storage_for_command(
            &cfg,
            &Commands::Policy {
                command: PolicyCommand::Show {
                    policy_id: "mock-trading.local-write".to_string(),
                },
            },
        )
        .expect("policy show inspect");
        let shown = policy_registry::builtin_registry()
            .expect("registry")
            .show("mock-trading.local-write")
            .expect("show policy");
        assert_eq!(
            shown.default_decision,
            policy_registry::PolicyDecision::RequireApproval
        );
    }

    #[test]
    fn workflow_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Workflow {
                command: WorkflowCommand::List {
                    domain: Some("domain:mock-trading".to_string()),
                },
            },
        )
        .expect("workflow list inspect");
        let listed = workflow_registry::builtin_registry()
            .expect("registry")
            .list(Some(&"domain:mock-trading".parse().expect("domain id")));
        assert!(!listed.is_empty());

        prepare_storage_for_command(
            &cfg,
            &Commands::Workflow {
                command: WorkflowCommand::Show {
                    workflow_id: "workflow:mock-research-brief".to_string(),
                },
            },
        )
        .expect("workflow show inspect");
        let shown = workflow_registry::builtin_registry()
            .expect("registry")
            .show(&"workflow:mock-research-brief".parse().expect("workflow id"))
            .expect("show workflow");
        assert_eq!(shown.steps.len(), 1);
    }

    #[test]
    fn fsm_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Fsm {
                command: FsmCommand::List {
                    domain: Some("domain:mock-trading".to_string()),
                },
            },
        )
        .expect("fsm list inspect");
        let listed = fsm_registry::builtin_registry()
            .expect("registry")
            .list(Some(&"domain:mock-trading".parse().expect("domain id")));
        assert!(!listed.is_empty());

        prepare_storage_for_command(
            &cfg,
            &Commands::Fsm {
                command: FsmCommand::Show {
                    fsm_id: "fsm:mock-research-brief".to_string(),
                },
            },
        )
        .expect("fsm show inspect");
        let shown = fsm_registry::builtin_registry()
            .expect("registry")
            .show(&"fsm:mock-research-brief".parse().expect("fsm id"))
            .expect("show fsm");
        assert_eq!(shown.transitions.len(), 1);
    }

    #[test]
    fn scheduler_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Scheduler {
                command: SchedulerCommand::List {
                    domain: Some("domain:mock-trading".to_string()),
                    trigger: Some("polling".to_string()),
                },
            },
        )
        .expect("scheduler list inspect");
        let listed = scheduler_registry::builtin_registry()
            .expect("registry")
            .list(
                Some(&"domain:mock-trading".parse().expect("domain id")),
                Some(&"polling".parse().expect("trigger")),
            );
        assert!(!listed.is_empty());

        prepare_storage_for_command(
            &cfg,
            &Commands::Scheduler {
                command: SchedulerCommand::Show {
                    scheduler_id: "scheduler:mock-research-brief".to_string(),
                },
            },
        )
        .expect("scheduler show inspect");
        let shown = scheduler_registry::builtin_registry()
            .expect("registry")
            .show(
                &"scheduler:mock-research-brief"
                    .parse()
                    .expect("scheduler id"),
            )
            .expect("show scheduler");
        assert_eq!(
            shown.cadence.trigger_kind,
            scheduler_registry::ScheduleTriggerKind::Cron
        );
    }

    #[test]
    fn desk_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Desk {
                command: DeskCommand::List {
                    category: Some("forex".to_string()),
                    domain: Some("domain:mock-trading".to_string()),
                },
            },
        )
        .expect("desk list inspect");
        let listed = desk_registry::builtin_registry().expect("registry").list(
            Some(&"forex".parse().expect("category")),
            Some(&"domain:mock-trading".parse().expect("domain id")),
        );
        assert!(!listed.is_empty());

        prepare_storage_for_command(
            &cfg,
            &Commands::Desk {
                command: DeskCommand::Show {
                    desk_id: "desk:mock-trading-paper".to_string(),
                },
            },
        )
        .expect("desk show inspect");
        let shown = desk_registry::builtin_registry()
            .expect("registry")
            .show(&"desk:mock-trading-paper".parse().expect("desk id"))
            .expect("show desk");
        assert!(shown.storage_profile.paper_only);
    }

    #[test]
    fn mock_research_workflow_run_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::Run {
                command: RunCommand::Workflow {
                    workflow_id: "workflow:mock-research-brief".to_string(),
                },
            },
        )
        .expect("run workflow mode");
        let result = execution_runtime::run_workflow(
            &cfg,
            &"workflow:mock-research-brief".parse().expect("workflow id"),
        )
        .expect("run workflow");
        assert_eq!(result.status, "ok");
    }

    #[test]
    fn policy_evaluation_does_not_execute_skills() {
        let (_tmp, cfg) = temp_cfg();
        let marker = cfg.workspace_dir.join("policy-eval-marker.txt");

        prepare_storage_for_command(
            &cfg,
            &Commands::Policy {
                command: PolicyCommand::EvaluateSkill {
                    skill_id: "mock-trading.prepare-paper-review".to_string(),
                },
            },
        )
        .expect("policy evaluate mode");
        let skills = skill_registry::builtin_registry().expect("skills");
        let policies = policy_registry::builtin_registry().expect("policies");
        let skill = skills
            .show("mock-trading.prepare-paper-review")
            .expect("show skill");
        let evaluation = policies.evaluate_skill(&skill);
        assert_eq!(
            evaluation.decision,
            policy_registry::PolicyDecision::RequireApproval
        );
        assert!(!marker.exists());
    }

    #[test]
    fn workflow_inspection_does_not_execute_workflows() {
        let (_tmp, cfg) = temp_cfg();
        let marker = cfg.workspace_dir.join("workflow-exec-marker.txt");

        prepare_storage_for_command(
            &cfg,
            &Commands::Workflow {
                command: WorkflowCommand::Show {
                    workflow_id: "workflow:mock-trading-paper-review".to_string(),
                },
            },
        )
        .expect("workflow show mode");
        let workflow = workflow_registry::builtin_registry()
            .expect("registry")
            .show(
                &"workflow:mock-trading-paper-review"
                    .parse()
                    .expect("workflow id"),
            )
            .expect("show workflow");
        assert_eq!(workflow.steps.len(), 2);
        assert!(!marker.exists());
    }

    #[test]
    fn fsm_inspection_does_not_execute_fsms() {
        let (_tmp, cfg) = temp_cfg();
        let marker = cfg.workspace_dir.join("fsm-exec-marker.txt");

        prepare_storage_for_command(
            &cfg,
            &Commands::Fsm {
                command: FsmCommand::Show {
                    fsm_id: "fsm:mock-trading-paper-review".to_string(),
                },
            },
        )
        .expect("fsm show mode");
        let fsm = fsm_registry::builtin_registry()
            .expect("registry")
            .show(&"fsm:mock-trading-paper-review".parse().expect("fsm id"))
            .expect("show fsm");
        assert_eq!(fsm.transitions.len(), 2);
        assert!(!marker.exists());
    }

    #[test]
    fn fsm_authority_inspection_does_not_mutate_workspace() {
        let (_tmp, cfg) = temp_cfg();
        assert!(!cfg.workspace_dir.exists());

        prepare_storage_for_command(
            &cfg,
            &Commands::Fsm {
                command: FsmCommand::Authority { json: true },
            },
        )
        .expect("fsm authority mode");
        let records = fsm_authority::authority_records();

        assert!(!cfg.workspace_dir.exists());
        assert!(records.iter().any(|record| record.fsm_id == "worker_job"));
    }

    #[test]
    fn scheduler_inspection_does_not_execute_schedulers() {
        let (_tmp, cfg) = temp_cfg();
        let marker = cfg.workspace_dir.join("scheduler-exec-marker.txt");

        prepare_storage_for_command(
            &cfg,
            &Commands::Scheduler {
                command: SchedulerCommand::Show {
                    scheduler_id: "scheduler:mock-trading-paper-review".to_string(),
                },
            },
        )
        .expect("scheduler show mode");
        let scheduler = scheduler_registry::builtin_registry()
            .expect("registry")
            .show(
                &"scheduler:mock-trading-paper-review"
                    .parse()
                    .expect("scheduler id"),
            )
            .expect("show scheduler");
        assert_eq!(
            scheduler.cadence.trigger_kind,
            scheduler_registry::ScheduleTriggerKind::Polling
        );
        assert!(!marker.exists());
    }

    #[test]
    fn desk_inspection_does_not_execute_desks() {
        let (_tmp, cfg) = temp_cfg();
        let marker = cfg.workspace_dir.join("desk-exec-marker.txt");

        prepare_storage_for_command(
            &cfg,
            &Commands::Desk {
                command: DeskCommand::Show {
                    desk_id: "desk:mock-trading-paper".to_string(),
                },
            },
        )
        .expect("desk show mode");
        let desk = desk_registry::builtin_registry()
            .expect("registry")
            .show(&"desk:mock-trading-paper".parse().expect("desk id"))
            .expect("show desk");
        assert!(desk.storage_profile.paper_only);
        assert!(!marker.exists());
    }

    #[test]
    fn shared_state_inspection_does_not_open_forex_redb() {
        let (_tmp, cfg) = temp_cfg();
        let store = shared_state::HybridSharedStateStore::from_config(&cfg);
        store
            .put(sample_shared_state_record())
            .expect("put shared state");
        let _lock = lock_forex_db(&cfg);

        prepare_storage_for_command(
            &cfg,
            &Commands::State {
                command: StateCommand::List { domain: None },
            },
        )
        .expect("state list inspect");
        prepare_storage_for_command(
            &cfg,
            &Commands::State {
                command: StateCommand::Show {
                    key: "shared.alpha".to_string(),
                },
            },
        )
        .expect("state show inspect");
        prepare_storage_for_command(
            &cfg,
            &Commands::State {
                command: StateCommand::Snapshot { domain: None },
            },
        )
        .expect("state snapshot inspect");
        prepare_storage_for_command(
            &cfg,
            &Commands::State {
                command: StateCommand::ExpireStale,
            },
        )
        .expect("state expire inspect");

        assert_eq!(shared_state::list_state(&cfg, None).expect("list").len(), 1);
        assert!(
            shared_state::show_state(&cfg, &"shared.alpha".parse().expect("key"))
                .expect("show")
                .is_some()
        );
    }

    #[test]
    fn shared_state_inspection_does_not_trigger_worker_execution() {
        let (_tmp, cfg) = temp_cfg();
        let store = shared_state::HybridSharedStateStore::from_config(&cfg);
        store
            .put(sample_shared_state_record())
            .expect("put shared state");

        let before_inbox = worker::queue_depth(&cfg.worker.inbox_path).expect("inbox depth");
        let before_outbox = worker::queue_depth(&cfg.worker.outbox_path).expect("outbox depth");

        let _ = shared_state::snapshot_state(&cfg, None).expect("snapshot");
        let _ =
            shared_state::show_state(&cfg, &"shared.alpha".parse().expect("key")).expect("show");

        let after_inbox = worker::queue_depth(&cfg.worker.inbox_path).expect("inbox depth");
        let after_outbox = worker::queue_depth(&cfg.worker.outbox_path).expect("outbox depth");
        assert_eq!(before_inbox, after_inbox);
        assert_eq!(before_outbox, after_outbox);
    }

    #[test]
    fn runtime_preflight_still_opens_required_stores() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        let err = prepare_storage_for_mode(&cfg, StorageMode::RuntimePreflight)
            .expect_err("runtime preflight should fail when forex db is locked");
        let message = err.to_string();
        assert!(
            message.contains("forex redb preflight failed")
                || message.contains("failed to open redb")
                || message.contains("Database already open")
        );
    }

    #[test]
    fn parallel_read_only_cli_commands_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let context = sessions::runtime_context("test-node", "worker");
        sessions::append_event(
            &cfg,
            &context,
            sessions::SessionEvent::Observation {
                message: "parallel inspect".to_string(),
                job_id: None,
                detail: Some("safe".to_string()),
            },
        )
        .expect("append event");
        let _lock = lock_forex_db(&cfg);

        let cfg_for_domain = cfg.clone();
        let cfg_for_skill = cfg.clone();

        thread::scope(|scope| {
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });
            let skill_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_skill,
                    &Commands::Skill {
                        command: SkillCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                            side_effect: Some("read_only".to_string()),
                        },
                    },
                )
                .expect("skill inspect");
                skill_registry::builtin_registry().expect("registry").list(
                    Some(&"domain:mock-trading".parse().expect("domain id")),
                    Some(&"read_only".parse().expect("side effect")),
                )
            });

            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
            assert!(!skill_thread.join().expect("join skill").is_empty());
        });
    }

    #[test]
    fn parallel_policy_skill_domain_inspection_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        let cfg_for_policy = cfg.clone();
        let cfg_for_skill = cfg.clone();
        let cfg_for_domain = cfg.clone();

        thread::scope(|scope| {
            let policy_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_policy,
                    &Commands::Policy {
                        command: PolicyCommand::EvaluateSkill {
                            skill_id: "mock-trading.prepare-paper-review".to_string(),
                        },
                    },
                )
                .expect("policy inspect");
                let skills = skill_registry::builtin_registry().expect("skills");
                let policies = policy_registry::builtin_registry().expect("policies");
                let skill = skills
                    .show("mock-trading.prepare-paper-review")
                    .expect("show skill");
                policies.evaluate_skill(&skill)
            });
            let skill_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_skill,
                    &Commands::Skill {
                        command: SkillCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                            side_effect: Some("read_only".to_string()),
                        },
                    },
                )
                .expect("skill inspect");
                skill_registry::builtin_registry().expect("registry").list(
                    Some(&"domain:mock-trading".parse().expect("domain id")),
                    Some(&"read_only".parse().expect("side effect")),
                )
            });
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });

            assert_eq!(
                policy_thread.join().expect("join policy").decision,
                policy_registry::PolicyDecision::RequireApproval
            );
            assert!(!skill_thread.join().expect("join skill").is_empty());
            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
        });
    }

    #[test]
    fn parallel_workflow_skill_domain_inspection_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        let cfg_for_workflow = cfg.clone();
        let cfg_for_skill = cfg.clone();
        let cfg_for_domain = cfg.clone();

        thread::scope(|scope| {
            let workflow_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_workflow,
                    &Commands::Workflow {
                        command: WorkflowCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                        },
                    },
                )
                .expect("workflow inspect");
                workflow_registry::builtin_registry()
                    .expect("registry")
                    .list(Some(&"domain:mock-trading".parse().expect("domain id")))
            });
            let skill_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_skill,
                    &Commands::Skill {
                        command: SkillCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                            side_effect: Some("read_only".to_string()),
                        },
                    },
                )
                .expect("skill inspect");
                skill_registry::builtin_registry().expect("registry").list(
                    Some(&"domain:mock-trading".parse().expect("domain id")),
                    Some(&"read_only".parse().expect("side effect")),
                )
            });
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });

            assert!(!workflow_thread.join().expect("join workflow").is_empty());
            assert!(!skill_thread.join().expect("join skill").is_empty());
            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
        });
    }

    #[test]
    fn parallel_fsm_workflow_domain_inspection_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        let cfg_for_fsm = cfg.clone();
        let cfg_for_workflow = cfg.clone();
        let cfg_for_domain = cfg.clone();

        thread::scope(|scope| {
            let fsm_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_fsm,
                    &Commands::Fsm {
                        command: FsmCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                        },
                    },
                )
                .expect("fsm inspect");
                fsm_registry::builtin_registry()
                    .expect("registry")
                    .list(Some(&"domain:mock-trading".parse().expect("domain id")))
            });
            let workflow_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_workflow,
                    &Commands::Workflow {
                        command: WorkflowCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                        },
                    },
                )
                .expect("workflow inspect");
                workflow_registry::builtin_registry()
                    .expect("registry")
                    .list(Some(&"domain:mock-trading".parse().expect("domain id")))
            });
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });

            assert!(!fsm_thread.join().expect("join fsm").is_empty());
            assert!(!workflow_thread.join().expect("join workflow").is_empty());
            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
        });
    }

    #[test]
    fn parallel_scheduler_fsm_workflow_domain_inspection_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        let cfg_for_scheduler = cfg.clone();
        let cfg_for_fsm = cfg.clone();
        let cfg_for_workflow = cfg.clone();
        let cfg_for_domain = cfg.clone();

        thread::scope(|scope| {
            let scheduler_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_scheduler,
                    &Commands::Scheduler {
                        command: SchedulerCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                            trigger: Some("polling".to_string()),
                        },
                    },
                )
                .expect("scheduler inspect");
                scheduler_registry::builtin_registry()
                    .expect("registry")
                    .list(
                        Some(&"domain:mock-trading".parse().expect("domain id")),
                        Some(&"polling".parse().expect("trigger")),
                    )
            });
            let fsm_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_fsm,
                    &Commands::Fsm {
                        command: FsmCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                        },
                    },
                )
                .expect("fsm inspect");
                fsm_registry::builtin_registry()
                    .expect("registry")
                    .list(Some(&"domain:mock-trading".parse().expect("domain id")))
            });
            let workflow_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_workflow,
                    &Commands::Workflow {
                        command: WorkflowCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                        },
                    },
                )
                .expect("workflow inspect");
                workflow_registry::builtin_registry()
                    .expect("registry")
                    .list(Some(&"domain:mock-trading".parse().expect("domain id")))
            });
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });

            assert!(!scheduler_thread.join().expect("join scheduler").is_empty());
            assert!(!fsm_thread.join().expect("join fsm").is_empty());
            assert!(!workflow_thread.join().expect("join workflow").is_empty());
            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
        });
    }

    #[test]
    fn parallel_desk_scheduler_domain_inspection_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let _lock = lock_forex_db(&cfg);

        let cfg_for_desk = cfg.clone();
        let cfg_for_scheduler = cfg.clone();
        let cfg_for_domain = cfg.clone();

        thread::scope(|scope| {
            let desk_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_desk,
                    &Commands::Desk {
                        command: DeskCommand::List {
                            category: Some("forex".to_string()),
                            domain: Some("domain:mock-trading".to_string()),
                        },
                    },
                )
                .expect("desk inspect");
                desk_registry::builtin_registry().expect("registry").list(
                    Some(&"forex".parse().expect("category")),
                    Some(&"domain:mock-trading".parse().expect("domain id")),
                )
            });
            let scheduler_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_scheduler,
                    &Commands::Scheduler {
                        command: SchedulerCommand::List {
                            domain: Some("domain:mock-trading".to_string()),
                            trigger: Some("polling".to_string()),
                        },
                    },
                )
                .expect("scheduler inspect");
                scheduler_registry::builtin_registry()
                    .expect("registry")
                    .list(
                        Some(&"domain:mock-trading".parse().expect("domain id")),
                        Some(&"polling".parse().expect("trigger")),
                    )
            });
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });

            assert!(!desk_thread.join().expect("join desk").is_empty());
            assert!(!scheduler_thread.join().expect("join scheduler").is_empty());
            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
        });
    }

    #[test]
    fn parallel_shared_state_inspection_can_run_safely() {
        let (_tmp, cfg) = temp_cfg();
        let store = shared_state::HybridSharedStateStore::from_config(&cfg);
        store
            .put(sample_shared_state_record())
            .expect("put shared state");
        let _lock = lock_forex_db(&cfg);

        let cfg_for_state = cfg.clone();
        let cfg_for_domain = cfg.clone();

        thread::scope(|scope| {
            let state_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_state,
                    &Commands::State {
                        command: StateCommand::Snapshot { domain: None },
                    },
                )
                .expect("state inspect");
                shared_state::snapshot_state(&cfg_for_state, None).expect("snapshot")
            });
            let domain_thread = scope.spawn(move || {
                prepare_storage_for_command(
                    &cfg_for_domain,
                    &Commands::Domain {
                        command: DomainCommand::List,
                    },
                )
                .expect("domain inspect");
                domain::builtin_registry().expect("registry").list()
            });

            assert_eq!(state_thread.join().expect("join state").len(), 1);
            assert_eq!(domain_thread.join().expect("join domain").len(), 2);
        });
    }
}
