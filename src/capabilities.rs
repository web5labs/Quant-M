use crate::config::{Config, ProviderKind};
use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::env;
use std::fmt;
use std::path::Path;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityStatus {
    Shipped,
    Guarded,
    DryRun,
    Mock,
    Experimental,
    DesignOnly,
    ExternalRequired,
    Unavailable,
    Deprecated,
}

impl Default for CapabilityStatus {
    fn default() -> Self {
        Self::Experimental
    }
}

impl fmt::Display for CapabilityStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Shipped => "shipped",
            Self::Guarded => "guarded",
            Self::DryRun => "dry_run",
            Self::Mock => "mock",
            Self::Experimental => "experimental",
            Self::DesignOnly => "design_only",
            Self::ExternalRequired => "external_required",
            Self::Unavailable => "unavailable",
            Self::Deprecated => "deprecated",
        })
    }
}

impl FromStr for CapabilityStatus {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "shipped" => Ok(Self::Shipped),
            "guarded" => Ok(Self::Guarded),
            "dry_run" | "dry-run" => Ok(Self::DryRun),
            "mock" => Ok(Self::Mock),
            "experimental" => Ok(Self::Experimental),
            "design_only" | "design-only" => Ok(Self::DesignOnly),
            "external_required" | "external-required" => Ok(Self::ExternalRequired),
            "unavailable" => Ok(Self::Unavailable),
            "deprecated" => Ok(Self::Deprecated),
            other => Err(anyhow!("unknown capability status '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum CapabilityCategory {
    Onboarding,
    OperatorSurface,
    Memory,
    Sessions,
    Context,
    Cost,
    Decisioning,
    Worker,
    Adapter,
    Provider,
    Skills,
    Governance,
    State,
    Cockpit,
    Truth,
    DomainPack,
    DesignPattern,
}

impl fmt::Display for CapabilityCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Onboarding => "onboarding",
            Self::OperatorSurface => "operator_surface",
            Self::Memory => "memory",
            Self::Sessions => "sessions",
            Self::Context => "context",
            Self::Cost => "cost",
            Self::Decisioning => "decisioning",
            Self::Worker => "worker",
            Self::Adapter => "adapter",
            Self::Provider => "provider",
            Self::Skills => "skills",
            Self::Governance => "governance",
            Self::State => "state",
            Self::Cockpit => "cockpit",
            Self::Truth => "truth",
            Self::DomainPack => "domain_pack",
            Self::DesignPattern => "design_pattern",
        })
    }
}

impl FromStr for CapabilityCategory {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "onboarding" => Ok(Self::Onboarding),
            "operator_surface" | "operator-surface" => Ok(Self::OperatorSurface),
            "memory" => Ok(Self::Memory),
            "sessions" | "session" => Ok(Self::Sessions),
            "context" => Ok(Self::Context),
            "cost" => Ok(Self::Cost),
            "decisioning" | "decision" => Ok(Self::Decisioning),
            "worker" => Ok(Self::Worker),
            "adapter" | "adapters" => Ok(Self::Adapter),
            "provider" | "providers" => Ok(Self::Provider),
            "skills" | "skill" => Ok(Self::Skills),
            "governance" => Ok(Self::Governance),
            "state" => Ok(Self::State),
            "cockpit" => Ok(Self::Cockpit),
            "truth" => Ok(Self::Truth),
            "domain_pack" | "domain-pack" => Ok(Self::DomainPack),
            "design_pattern" | "design-pattern" => Ok(Self::DesignPattern),
            other => Err(anyhow!("unknown capability category '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRequirement {
    pub kind: String,
    pub value: String,
    pub satisfied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityProof {
    pub command: String,
    pub expected_artifact: Option<String>,
    pub validation: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityRecord {
    pub id: String,
    pub name: String,
    pub category: CapabilityCategory,
    pub status: CapabilityStatus,
    pub summary: String,
    pub commands: Vec<String>,
    pub requirements: Vec<CapabilityRequirement>,
    pub config_gates: Vec<String>,
    pub policy_gates: Vec<String>,
    pub artifacts_created: Vec<String>,
    pub proofs: Vec<CapabilityProof>,
    pub proof_commands: Vec<String>,
    pub validation_commands: Vec<String>,
    pub docs: Vec<String>,
    pub risks: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CapabilityAuditReport {
    pub status: String,
    pub checked_docs: Vec<String>,
    pub missing_markers: Vec<String>,
    pub capability_count: usize,
}

#[derive(Debug, Clone)]
pub struct CapabilityFilter {
    pub category: Option<CapabilityCategory>,
    pub status: Option<CapabilityStatus>,
}

pub fn inventory(cfg: &Config) -> Result<Vec<CapabilityRecord>> {
    let mut records = base_records(cfg);
    records.extend(provider_records(cfg));
    records.extend(tool_records(cfg));
    records.sort_by(|a, b| a.id.cmp(&b.id));
    validate_unique_ids(&records)?;
    Ok(records)
}

pub fn filtered_inventory(cfg: &Config, filter: CapabilityFilter) -> Result<Vec<CapabilityRecord>> {
    Ok(inventory(cfg)?
        .into_iter()
        .filter(|record| filter.category.is_none_or(|value| record.category == value))
        .filter(|record| filter.status.is_none_or(|value| record.status == value))
        .collect())
}

pub fn show_capability(cfg: &Config, id: &str) -> Result<CapabilityRecord> {
    inventory(cfg)?
        .into_iter()
        .find(|record| record.id == id)
        .ok_or_else(|| anyhow!("capability '{id}' not found"))
}

pub fn validate_unique_ids(records: &[CapabilityRecord]) -> Result<()> {
    let mut seen = BTreeSet::new();
    for record in records {
        if !seen.insert(record.id.as_str()) {
            return Err(anyhow!("duplicate capability id '{}'", record.id));
        }
    }
    Ok(())
}

pub fn audit_docs(cfg: &Config) -> Result<CapabilityAuditReport> {
    let checked_docs = vec!["README.md".to_string(), "Cargo.toml".to_string()];
    let required = [
        ("README.md", "What Quant-M Is"),
        ("README.md", "Detection does not equal permission"),
        ("README.md", "Authority Snapshot"),
        ("README.md", "Five-Minute Proof"),
        ("Cargo.toml", "name = \"quant-m\""),
    ];
    let mut missing_markers = Vec::new();
    for (path, marker) in required {
        let raw = std::fs::read_to_string(path).unwrap_or_default();
        if !raw.contains(marker) {
            missing_markers.push(format!("{path}: missing '{marker}'"));
        }
    }
    Ok(CapabilityAuditReport {
        status: if missing_markers.is_empty() {
            "ok".to_string()
        } else {
            "needs_review".to_string()
        },
        checked_docs,
        missing_markers,
        capability_count: inventory(cfg)?.len(),
    })
}

fn base_records(cfg: &Config) -> Vec<CapabilityRecord> {
    vec![
        record(
            "onboarding.setup",
            "Onboarding and setup",
            CapabilityCategory::Onboarding,
            CapabilityStatus::Shipped,
            "Guided and scripted project-local setup for workspace, models, tools, channels, and context guard.",
            &[
                "quant-m onboard",
                "quant-m setup",
                "quant-m init",
                "quant-m settings",
            ],
            &["quant-m onboard", "quant-m settings"],
            &["quant-m config validate"],
            &["quant-m.toml", "workspace/"],
            &["README.md"],
        ),
        record(
            "operator.shell",
            "Local shell",
            CapabilityCategory::OperatorSurface,
            CapabilityStatus::Shipped,
            "Local CLI shell for running Quant-M commands without granting extra authority.",
            &["quant-m agent", "quant-m shell"],
            &["quant-m agent"],
            &["cargo test agent_shell"],
            &["workspace/state/sessions/"],
            &["README.md"],
        ),
        record(
            "operator.tui",
            "Terminal UI",
            CapabilityCategory::OperatorSurface,
            CapabilityStatus::Experimental,
            "Terminal UI exists, but should not be described as a stable public contract yet.",
            &["quant-m tui"],
            &["quant-m tui"],
            &["cargo test tui_shell"],
            &[],
            &["README.md"],
        ),
        record(
            "demo.proof-loop",
            "Local proof demo",
            CapabilityCategory::OperatorSurface,
            CapabilityStatus::Shipped,
            "Runs a local proof path without provider calls.",
            &["quant-m demo"],
            &["quant-m demo"],
            &["cargo test demo_flow"],
            &[
                "workspace/state/sessions/",
                "workspace/state/compacted/",
                "workspace/state/context-guardian/",
            ],
            &["README.md"],
        ),
        record(
            "memory.local",
            "Markdown and SQLite memory",
            CapabilityCategory::Memory,
            CapabilityStatus::Shipped,
            "Local markdown memory plus SQLite-backed search/indexing.",
            &[
                "quant-m memory add",
                "quant-m memory search",
                "quant-m memory list",
            ],
            &[
                "quant-m memory add demo \"hello\"",
                "quant-m memory search hello",
            ],
            &["cargo test memory"],
            &[
                "workspace/MEMORY.md",
                "workspace/memory/brain.db",
                "workspace/daily/",
            ],
            &["README.md"],
        ),
        record(
            "sessions.replay",
            "Sessions and replay",
            CapabilityCategory::Sessions,
            CapabilityStatus::Shipped,
            "Append-only session evidence and side-effect-free replay/inspection.",
            &[
                "quant-m session list",
                "quant-m session show <id>",
                "quant-m session replay <id>",
                "quant-m replay <session_id>",
            ],
            &["quant-m demo", "quant-m session list"],
            &["cargo test sessions", "cargo test replay"],
            &["workspace/state/sessions/"],
            &["README.md"],
        ),
        record(
            "context.compact-guard",
            "Compact packets and context guard",
            CapabilityCategory::Context,
            CapabilityStatus::Shipped,
            "Compacts session evidence and prepares local continuation handoffs.",
            &[
                "quant-m compact <session_id>",
                "quant-m context-status",
                "quant-m context guard",
            ],
            &[
                "quant-m compact <session_id>",
                "quant-m context guard --json",
            ],
            &[
                "cargo test compaction",
                "cargo test context_guardian",
                "cargo test context_status",
            ],
            &[
                "workspace/state/compacted/",
                "workspace/state/context-guardian/",
            ],
            &["README.md"],
        ),
        record(
            "context.boil",
            "Boil context savings report",
            CapabilityCategory::Context,
            CapabilityStatus::Experimental,
            "Estimates continuation context savings; useful but still an emerging reporting surface.",
            &[
                "quant-m boil <session_id>",
                "quant-m boil <session_id> --dry-run",
            ],
            &["quant-m boil <session_id> --dry-run"],
            &["cargo test boil"],
            &["workspace/state/boil/"],
            &["README.md"],
        ),
        record(
            "loop.dry-run",
            "Loop dry-run",
            CapabilityCategory::Context,
            CapabilityStatus::DryRun,
            "Read-only self-check loop that reports gaps without mutating source, truth files, or sessions.",
            &["quant-m loop --dry-run"],
            &["quant-m loop --dry-run --json"],
            &["cargo test loop_dry_run"],
            &["workspace/state/loops/"],
            &["README.md"],
        ),
        record(
            "cost.ledger",
            "Cost ledger",
            CapabilityCategory::Cost,
            CapabilityStatus::Shipped,
            "Local cost summary over recorded dry-run/provider-path cost records.",
            &["quant-m cost summary"],
            &["quant-m cost summary"],
            &["cargo test cost_ledger"],
            &["workspace/state/cost-ledger.jsonl"],
            &["README.md"],
        ),
        record(
            "consensus.dry-run",
            "Consensus utility",
            CapabilityCategory::Decisioning,
            CapabilityStatus::DryRun,
            "Deterministic evidence-oriented consensus path; currently dry-run only.",
            &["quant-m consensus --dry-run \"<question>\""],
            &["quant-m consensus --dry-run \"What should we inspect first?\""],
            &["cargo test consensus"],
            &[
                "workspace/state/sessions/",
                "workspace/state/shared-state.db",
                "workspace/state/cost-ledger.jsonl",
            ],
            &["README.md", "README.md"],
        ),
        record(
            "strategist.dry-run",
            "Strategist utility",
            CapabilityCategory::Decisioning,
            CapabilityStatus::DryRun,
            "Multi-lane strategist proof path using deterministic mock lanes only.",
            &["quant-m strategist --dry-run"],
            &["quant-m strategist --dry-run --json"],
            &["cargo test strategist"],
            &[
                "workspace/state/strategist/",
                "workspace/state/worker-proposals/",
            ],
            &["README.md"],
        ),
        record(
            "question.utility",
            "Universal question utility",
            CapabilityCategory::Decisioning,
            CapabilityStatus::Experimental,
            "Builds bounded question/proposal structures; proposal writes remain explicit and non-authoritative.",
            &[
                "quant-m question ask --mode agent-cluster \"<question>\"",
                "quant-m question ask --mode agent-cluster \"<question>\" --write-proposals",
            ],
            &["quant-m question ask --mode agent-cluster \"What next?\""],
            &["cargo test question"],
            &["workspace/state/worker-proposals/"],
            &["README.md"],
        ),
        worker_runtime_record(cfg),
        record(
            "worker.proposals",
            "Worker proposals",
            CapabilityCategory::Worker,
            CapabilityStatus::Shipped,
            "Non-authoritative worker proposal records; workers propose and the core decides.",
            &[
                "quant-m worker proposal submit",
                "quant-m worker proposal list",
            ],
            &[
                "quant-m worker proposal submit --surface local_worker --kind evidence --summary \"demo\"",
            ],
            &["cargo test worker_proposals", "cargo test cluster_boundary"],
            &["workspace/state/worker-proposals/"],
            &["README.md"],
        ),
        record(
            "adapters.terminal",
            "Terminal adapter",
            CapabilityCategory::Adapter,
            CapabilityStatus::Shipped,
            "Default local terminal/JSON notification surface.",
            &["quant-m adapter send \"hello\""],
            &["quant-m adapter send \"hello\""],
            &["cargo test adapters"],
            &["workspace/logs/"],
            &["README.md"],
        ),
        webhook_record(cfg),
        telegram_record(cfg),
        skills_record(cfg),
        record(
            "governance.registries",
            "Registry-backed governance",
            CapabilityCategory::Governance,
            CapabilityStatus::Shipped,
            "Typed domain, skill, policy, workflow, FSM, scheduler, and desk registries.",
            &[
                "quant-m domain list",
                "quant-m skill list",
                "quant-m policy list",
                "quant-m workflow list",
                "quant-m fsm list",
                "quant-m scheduler list",
                "quant-m desk list",
            ],
            &[
                "quant-m domain list",
                "quant-m policy evaluate-skill mock-trading.prepare-paper-review",
            ],
            &[
                "cargo test skill_registry",
                "cargo test policy_registry",
                "cargo test workflow_registry",
                "cargo test fsm_registry",
                "cargo test scheduler_registry",
                "cargo test desk_registry",
            ],
            &[],
            &["README.md", "README.md"],
        ),
        record(
            "state.shared",
            "Shared state and domain state",
            CapabilityCategory::State,
            CapabilityStatus::Guarded,
            "Typed local state store and domain state commands. Trading-like records are paper/state modeling only.",
            &[
                "quant-m state init",
                "quant-m state list",
                "quant-m state review",
                "quant-m state handoff-add",
                "quant-m state order-add",
            ],
            &["quant-m state init", "quant-m state summary"],
            &[
                "cargo test shared_state",
                "cargo test state_sql",
                "cargo test state_review",
            ],
            &[
                "workspace/state/shared-state.db",
                "workspace/state/shared-state.redb",
            ],
            &["README.md", "README.md"],
        ),
        record(
            "cockpit.planning",
            "Cockpit planning",
            CapabilityCategory::Cockpit,
            CapabilityStatus::Experimental,
            "Produces terminal/cockpit lane plans; does not launch external terminal panes.",
            &["quant-m cockpit plan"],
            &["quant-m cockpit plan --host auto"],
            &["cargo test terminal_cockpit"],
            &[],
            &["README.md"],
        ),
        record(
            "truth.files",
            "Truth/project files",
            CapabilityCategory::Truth,
            CapabilityStatus::Shipped,
            "Creates local truth files for policy, shippable criteria, and agent-facing runtime context.",
            &["quant-m init-truth"],
            &["quant-m init-truth --json"],
            &["cargo test truth_files"],
            &[
                "workspace/POLICY.md",
                "workspace/SHIPPABLE.md",
                "workspace/AGENTS.md",
            ],
            &["README.md"],
        ),
        record(
            "domain.mock-research",
            "Mock research pack",
            CapabilityCategory::DomainPack,
            CapabilityStatus::Mock,
            "Built-in mock research descriptors for registry, workflow, FSM, scheduler, and desk proof.",
            &[
                "quant-m domain show domain:mock-research",
                "quant-m run workflow workflow:mock-research-brief",
            ],
            &["quant-m domain show domain:mock-research"],
            &["cargo test domain", "cargo test execution_runtime"],
            &[
                "workspace/state/sessions/",
                "workspace/state/shared-state.db",
            ],
            &["README.md"],
        ),
        record(
            "domain.mock-trading-paper",
            "Mock trading paper pack",
            CapabilityCategory::DomainPack,
            CapabilityStatus::Mock,
            "Paper-only mock trading descriptors. No broker, exchange, live order, or live trading adapter exists.",
            &[
                "quant-m domain show domain:mock-trading",
                "quant-m policy evaluate-skill mock-trading.prepare-paper-review",
            ],
            &["quant-m policy evaluate-skill mock-trading.prepare-paper-review"],
            &["cargo test domain", "cargo test policy_registry"],
            &["workspace/state/shared-state.db"],
            &["README.md"],
        ),
        record(
            "live-trading.execution",
            "Live trading execution",
            CapabilityCategory::DomainPack,
            CapabilityStatus::Unavailable,
            "Live trading is intentionally unavailable and denied by policy in the current repo.",
            &[],
            &["quant-m policy show mock-trading.trading-action-deny"],
            &["cargo test mock_trading"],
            &[],
            &["README.md", "README.md"],
        ),
        record(
            "design.repeatable-project-skills",
            "Designed repeatable project skills",
            CapabilityCategory::DesignPattern,
            CapabilityStatus::DesignOnly,
            "Markdown-described reusable project skill patterns. They are not all installed runtime skills.",
            &[],
            &[],
            &[],
            &["README.md"],
            &["README.md"],
        ),
    ]
}

fn worker_runtime_record(cfg: &Config) -> CapabilityRecord {
    let mut record = record(
        "worker.queue-runtime",
        "Worker queue runtime",
        CapabilityCategory::Worker,
        CapabilityStatus::Guarded,
        "Local queue runtime. Echo/sleep are local; shell and HTTP lanes are gated by config.",
        &[
            "quant-m worker submit <job-json>",
            "quant-m worker once <job-json>",
            "quant-m worker run",
        ],
        &["quant-m worker once '{\"kind\":\"echo\",\"text\":\"hello\"}'"],
        &["cargo test worker"],
        &[
            "workspace/queue/inbox.ndjson",
            "workspace/queue/outbox.ndjson",
            "workspace/queue/dead-letter.ndjson",
            "workspace/state/worker_state.json",
        ],
        &["README.md"],
    );
    record.config_gates = vec![
        format!(
            "worker.allow_shell_commands={}",
            cfg.worker.allow_shell_commands
        ),
        format!("worker.allow_http_get={}", cfg.worker.allow_http_get),
        format!("worker.http_get_mode={}", cfg.worker.http_get_mode),
    ];
    record.policy_gates = vec![
        "shell commands require worker.allow_shell_commands=true".to_string(),
        "HTTP requires worker.allow_http_get=true and http_get_mode policy".to_string(),
    ];
    record
}

fn webhook_record(cfg: &Config) -> CapabilityRecord {
    let configured = cfg.adapters.webhook_url.is_some();
    let mut record = record(
        "adapters.webhook",
        "Webhook adapter",
        CapabilityCategory::Adapter,
        CapabilityStatus::Guarded,
        "Optional webhook delivery; disabled unless a webhook URL is configured.",
        &["quant-m adapter send \"hello\" --kind webhook"],
        &["quant-m channel list"],
        &["cargo test adapters"],
        &[],
        &["README.md"],
    );
    record
        .requirements
        .push(req("config", "adapters.webhook_url", configured));
    record.config_gates = vec!["adapters.webhook_url".to_string()];
    record
}

fn telegram_record(cfg: &Config) -> CapabilityRecord {
    let configured = cfg.telegram.enabled && cfg.resolve_telegram_bot_token().is_some();
    let mut record = record(
        "channels.telegram",
        "Telegram channel",
        CapabilityCategory::Adapter,
        if cfg.telegram.enabled {
            CapabilityStatus::Guarded
        } else {
            CapabilityStatus::Unavailable
        },
        "Optional Telegram polling channel. Channel text is evidence/input only, not execution authority.",
        &["quant-m telegram run", "quant-m channel list"],
        &["quant-m channel list --json"],
        &["cargo test telegram", "cargo test channels"],
        &["workspace/state/sessions/"],
        &["README.md"],
    );
    record
        .requirements
        .push(req("config", "telegram.enabled", cfg.telegram.enabled));
    record.requirements.push(req(
        "secret",
        "TELEGRAM_BOT_TOKEN or config token",
        configured,
    ));
    record.config_gates = vec![
        "telegram.enabled".to_string(),
        "telegram.bot_token".to_string(),
    ];
    record.policy_gates = vec!["channels are not execution authority".to_string()];
    record
}

fn skills_record(cfg: &Config) -> CapabilityRecord {
    let mut record = record(
        "skills.filesystem",
        "Local filesystem skills",
        CapabilityCategory::Skills,
        CapabilityStatus::Guarded,
        "Local SKILL.md discovery is available; shell-backed skill execution is gated.",
        &[
            "quant-m skills list",
            "quant-m skills show <name>",
            "quant-m skills run <name> <input>",
        ],
        &["quant-m skills list"],
        &["cargo test skills"],
        &["workspace/skills/<skill>/SKILL.md"],
        &["README.md", "README.md"],
    );
    record.requirements.push(req(
        "config",
        "skills.allow_shell_commands",
        cfg.skills.allow_shell_commands,
    ));
    record.config_gates = vec!["skills.allow_shell_commands".to_string()];
    record.policy_gates = vec!["skill shell execution is disabled by default".to_string()];
    record
}

fn provider_records(cfg: &Config) -> Vec<CapabilityRecord> {
    let mut records = Vec::new();
    for (id, provider) in &cfg.providers {
        let key_present = !provider.api_key_env.trim().is_empty()
            && env::var(&provider.api_key_env)
                .ok()
                .map(|value| !value.trim().is_empty())
                .unwrap_or(false);
        let local_provider = matches!(provider.kind, ProviderKind::Ollama | ProviderKind::LmStudio);
        let detected = match provider.kind {
            ProviderKind::Ollama => command_present("ollama"),
            ProviderKind::LmStudio => command_present("lms"),
            _ => key_present,
        };
        let status = if local_provider && !detected {
            CapabilityStatus::Unavailable
        } else if local_provider {
            CapabilityStatus::ExternalRequired
        } else if key_present && provider.enabled && cfg.runtime.external_network_enabled {
            CapabilityStatus::Guarded
        } else {
            CapabilityStatus::ExternalRequired
        };
        let mut record = record(
            &format!("providers.{id}"),
            &format!("Provider: {id}"),
            CapabilityCategory::Provider,
            status,
            "Provider configuration/detection only. Registry presence is not permission to call the provider.",
            &[
                "quant-m provider list",
                "quant-m provider validate <provider>",
                "quant-m llm ask <prompt>",
            ],
            &["quant-m provider list --json"],
            &["cargo test provider", "cargo test llm"],
            &[],
            &["README.md", "README.md"],
        );
        record.requirements.push(req(
            "config",
            &format!("providers.{id}.enabled"),
            provider.enabled,
        ));
        if !provider.api_key_env.trim().is_empty() {
            record
                .requirements
                .push(req("secret", &provider.api_key_env, key_present));
        }
        if local_provider {
            record.requirements.push(req(
                "external_tool",
                if provider.kind == ProviderKind::Ollama {
                    "ollama"
                } else {
                    "lms"
                },
                detected,
            ));
        }
        record.config_gates = vec![
            format!("providers.{id}.enabled"),
            "runtime.external_network_enabled".to_string(),
            "provider live validation is never automatic".to_string(),
        ];
        record.risks =
            vec!["Detection/configuration does not grant runtime permission.".to_string()];
        records.push(record);
    }
    records
}

fn tool_records(cfg: &Config) -> Vec<CapabilityRecord> {
    cfg.tools
        .iter()
        .map(|(id, tool)| {
            let present = command_present(&tool.command);
            let status = if present {
                CapabilityStatus::ExternalRequired
            } else {
                CapabilityStatus::Unavailable
            };
            let mut record = record(
                &format!("tools.{id}"),
                &format!("External tool: {id}"),
                CapabilityCategory::Provider,
                status,
                "External developer/model tool detection only. Detection is not permission.",
                &[
                    "quant-m tool list",
                    "quant-m tool scan",
                    "quant-m tool validate <tool>",
                ],
                &["quant-m tool list --json"],
                &["cargo test tool"],
                &[],
                &["README.md"],
            );
            record
                .requirements
                .push(req("external_tool", &tool.command, present));
            record
                .requirements
                .push(req("config", &format!("tools.{id}.enabled"), tool.enabled));
            record.config_gates = vec![format!("tools.{id}.enabled")];
            record
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn record(
    id: &str,
    name: &str,
    category: CapabilityCategory,
    status: CapabilityStatus,
    summary: &str,
    commands: &[&str],
    proof_commands: &[&str],
    validation_commands: &[&str],
    artifacts_created: &[&str],
    docs: &[&str],
) -> CapabilityRecord {
    let proof_commands_vec = strings(proof_commands);
    let artifacts_vec = strings(artifacts_created);
    let proofs = proof_commands_vec
        .iter()
        .map(|command| CapabilityProof {
            command: command.clone(),
            expected_artifact: artifacts_vec.first().cloned(),
            validation: validation_commands
                .first()
                .map(|value| (*value).to_string()),
        })
        .collect();
    CapabilityRecord {
        id: id.to_string(),
        name: name.to_string(),
        category,
        status,
        summary: summary.to_string(),
        commands: strings(commands),
        requirements: Vec::new(),
        config_gates: Vec::new(),
        policy_gates: Vec::new(),
        artifacts_created: artifacts_vec,
        proofs,
        proof_commands: proof_commands_vec,
        validation_commands: strings(validation_commands),
        docs: strings(docs),
        risks: Vec::new(),
        notes: Vec::new(),
    }
}

fn req(kind: &str, value: &str, satisfied: bool) -> CapabilityRequirement {
    CapabilityRequirement {
        kind: kind.to_string(),
        value: value.to_string(),
        satisfied,
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|value| (*value).to_string()).collect()
}

fn command_present(command: &str) -> bool {
    let command = command.trim();
    if command.is_empty() {
        return false;
    }
    if command.contains(std::path::MAIN_SEPARATOR) {
        return Path::new(command).is_file();
    }
    let Some(path) = env::var_os("PATH") else {
        return false;
    };
    env::split_paths(&path).any(|dir| dir.join(command).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn capability_status_serializes_as_enum_like_string() {
        assert_eq!(
            serde_json::to_string(&CapabilityStatus::DryRun).expect("serialize"),
            "\"dry_run\""
        );
        assert_eq!(
            "external_required"
                .parse::<CapabilityStatus>()
                .expect("parse status"),
            CapabilityStatus::ExternalRequired
        );
    }

    #[test]
    fn inventory_json_order_is_deterministic() {
        let cfg = Config::default();
        let first = serde_json::to_string(&inventory(&cfg).expect("inventory")).expect("json");
        let second = serde_json::to_string(&inventory(&cfg).expect("inventory")).expect("json");
        assert_eq!(first, second);
    }

    #[test]
    fn inventory_contains_required_maturity_examples() {
        let cfg = Config::default();
        let records = inventory(&cfg).expect("inventory");
        assert!(
            records
                .iter()
                .any(|item| item.status == CapabilityStatus::Shipped)
        );
        assert!(
            records
                .iter()
                .any(|item| item.status == CapabilityStatus::Guarded)
        );
        assert!(
            records
                .iter()
                .any(|item| item.status == CapabilityStatus::DryRun)
        );
        assert!(
            records
                .iter()
                .any(|item| item.status == CapabilityStatus::Mock)
        );
        assert!(records.iter().any(|item| {
            matches!(
                item.status,
                CapabilityStatus::Unavailable | CapabilityStatus::ExternalRequired
            )
        }));
    }

    #[test]
    fn shell_skills_are_guarded_when_shell_execution_is_disabled() {
        let mut cfg = Config::default();
        cfg.skills.allow_shell_commands = false;
        let skill = show_capability(&cfg, "skills.filesystem").expect("skill capability");
        assert_eq!(skill.status, CapabilityStatus::Guarded);
        assert!(
            skill
                .config_gates
                .contains(&"skills.allow_shell_commands".to_string())
        );
    }

    #[test]
    fn provider_registry_presence_does_not_report_shipped() {
        let cfg = Config::default();
        let provider = show_capability(&cfg, "providers.openrouter").expect("provider capability");
        assert_ne!(provider.status, CapabilityStatus::Shipped);
    }

    #[test]
    fn mock_trading_never_reports_live_trading() {
        let cfg = Config::default();
        let mock = show_capability(&cfg, "domain.mock-trading-paper").expect("mock trading");
        let live = show_capability(&cfg, "live-trading.execution").expect("live trading");
        assert_eq!(mock.status, CapabilityStatus::Mock);
        assert_eq!(live.status, CapabilityStatus::Unavailable);
        assert!(live.summary.contains("denied"));
    }

    #[test]
    fn duplicate_capability_ids_are_rejected() {
        let mut records = vec![record(
            "duplicate",
            "Duplicate",
            CapabilityCategory::Truth,
            CapabilityStatus::Experimental,
            "duplicate test",
            &[],
            &[],
            &[],
            &[],
            &[],
        )];
        records.push(records[0].clone());
        assert!(validate_unique_ids(&records).is_err());
    }
}
