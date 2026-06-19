use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const ENV_WORKSPACE_DIR: &str = "QUANT_M_WORKSPACE_DIR";
const ENV_MEMORY_SQLITE_PATH: &str = "QUANT_M_MEMORY_SQLITE_PATH";
const ENV_MEMORY_CORE_MARKDOWN: &str = "QUANT_M_MEMORY_CORE_MARKDOWN";
const ENV_MEMORY_DAILY_DIR: &str = "QUANT_M_MEMORY_DAILY_DIR";
const ENV_STATE_SQLITE_PATH: &str = "QUANT_M_STATE_SQLITE_PATH";
const ENV_HEARTBEAT_TASKS_FILE: &str = "QUANT_M_HEARTBEAT_TASKS_FILE";
const ENV_WORKER_INBOX_PATH: &str = "QUANT_M_WORKER_INBOX_PATH";
const ENV_WORKER_OUTBOX_PATH: &str = "QUANT_M_WORKER_OUTBOX_PATH";
const ENV_WORKER_INFLIGHT_PATH: &str = "QUANT_M_WORKER_INFLIGHT_PATH";
const ENV_WORKER_STATE_PATH: &str = "QUANT_M_WORKER_STATE_PATH";
const ENV_WORKER_DEAD_LETTER_PATH: &str = "QUANT_M_WORKER_DEAD_LETTER_PATH";
const ENV_LOG_FILE: &str = "QUANT_M_LOG_FILE";
const ENV_SKILLS_DIR: &str = "QUANT_M_SKILLS_DIR";
const ENV_FOREX_REDB_PATH: &str = "QUANT_M_FOREX_REDB_PATH";
const ENV_SESSION_DIR: &str = "QUANT_M_SESSION_DIR";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node_id: String,
    pub workspace_dir: PathBuf,
    pub memory: MemoryConfig,
    #[serde(default)]
    pub state_sql: StateSqlConfig,
    pub heartbeat: HeartbeatConfig,
    pub worker: WorkerConfig,
    pub adapters: AdapterConfig,
    pub logging: LoggingConfig,
    pub skills: SkillsConfig,
    #[serde(default)]
    pub llm: LlmConfig,
    #[serde(default)]
    pub telegram: TelegramConfig,
    #[serde(default)]
    pub chat_channels: ChatChannelsConfig,
    #[serde(default)]
    pub forex: ForexConfig,
    #[serde(default)]
    pub runtime: RuntimeConfig,
    #[serde(default)]
    pub context_guardian: ContextGuardianConfig,
    #[serde(default)]
    pub preferences: PreferenceConfig,
    #[serde(default)]
    pub providers: BTreeMap<String, ProviderConfig>,
    #[serde(default)]
    pub tools: BTreeMap<String, ToolConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryConfig {
    pub sqlite_path: PathBuf,
    pub core_markdown: PathBuf,
    pub daily_dir: PathBuf,
    pub vector_weight: f32,
    pub keyword_weight: f32,
    pub vector_dims: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSqlConfig {
    pub sqlite_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatConfig {
    pub enabled: bool,
    pub interval_seconds: u64,
    pub tasks_file: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerConfig {
    pub inbox_path: PathBuf,
    pub outbox_path: PathBuf,
    pub inflight_path: PathBuf,
    pub state_path: PathBuf,
    #[serde(default = "default_dead_letter_path")]
    pub dead_letter_path: PathBuf,
    pub poll_interval_seconds: u64,
    pub command_timeout_seconds: u64,
    pub concurrency: usize,
    pub max_retries: u8,
    #[serde(default = "default_max_inbox_depth")]
    pub max_inbox_depth: usize,
    #[serde(default)]
    pub allow_shell_commands: bool,
    #[serde(default)]
    pub allow_http_get: bool,
    #[serde(default)]
    pub allow_insecure_https: bool,
    #[serde(default = "default_http_get_mode")]
    pub http_get_mode: String,
    #[serde(default)]
    pub http_get_sandbox_hosts: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    pub terminal_enabled: bool,
    pub webhook_url: Option<String>,
    pub webhook_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    pub file: PathBuf,
    pub max_bytes: u64,
    pub keep_files: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillsConfig {
    pub dir: PathBuf,
    #[serde(default)]
    pub allow_shell_commands: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    pub enabled: bool,
    pub api_base: String,
    pub model: String,
    pub api_key: Option<String>,
    pub api_key_env: String,
    pub temperature: f32,
    pub max_tokens: u32,
    pub request_timeout_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ProviderConfig {
    #[serde(default)]
    pub enabled: bool,
    pub kind: ProviderKind,
    pub api_base: String,
    pub api_key_env: String,
    #[serde(default)]
    pub preferred_models: Vec<String>,
    #[serde(default)]
    pub live_validation_allowed: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProviderKind {
    OpenRouter,
    OpenAi,
    Ollama,
    LmStudio,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolConfig {
    #[serde(default)]
    pub enabled: bool,
    pub kind: ToolKind,
    pub command: String,
    #[serde(default)]
    pub validation_args: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ToolKind {
    Codex,
    OpenAi,
    Gemini,
    Anthropic,
    Claude,
    OpenCode,
    Antigravity,
    Antgravity,
    Hermes,
    PiAgent,
    OpenClaw,
    Ollama,
    LmStudio,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelegramConfig {
    pub enabled: bool,
    pub bot_token: Option<String>,
    pub allowed_chat_id: Option<i64>,
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatChannelsConfig {
    pub enabled: bool,
    pub allowed_channels: Vec<ExternalChannel>,
    pub default_channel: ExternalChannel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForexConfig {
    pub redb_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RuntimeConfig {
    pub profile: RuntimeProfile,
    #[serde(default = "default_session_dir")]
    pub session_dir: PathBuf,
    #[serde(default)]
    pub external_network_enabled: bool,
    #[serde(default)]
    pub multi_model_enabled: bool,
    #[serde(default)]
    pub search_enabled: bool,
    #[serde(default)]
    pub browser_harness_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextGuardianConfig {
    #[serde(default = "default_context_guardian_enabled")]
    pub enabled: bool,
    #[serde(default = "default_context_guardian_interval_seconds")]
    pub interval_seconds: u64,
    #[serde(default = "default_context_guardian_min_event_count")]
    pub min_event_count: usize,
    #[serde(default = "default_context_guardian_high_risk_event_count")]
    pub high_risk_event_count: usize,
    #[serde(default = "default_context_guardian_branch_packet_path")]
    pub branch_packet_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct PreferenceConfig {
    #[serde(default)]
    pub preferred_local_model: Option<ModelPreference>,
    #[serde(default)]
    pub preferred_remote_model: Option<ModelPreference>,
    #[serde(default)]
    pub preferred_openrouter_model: Option<String>,
    #[serde(default)]
    pub preferred_channel: ChannelPreference,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ModelPreference {
    pub provider: String,
    pub model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelPreference {
    pub channel: ExternalChannel,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeProfile {
    Edge,
    #[default]
    Laptop,
    Vps,
    StaffOsWorker,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ExternalChannel {
    Telegram,
    Discord,
    Slack,
    Signal,
    Whatsapp,
    Ichat,
    Email,
    #[default]
    None,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            node_id: default_node_id(),
            workspace_dir: PathBuf::from("workspace"),
            memory: MemoryConfig {
                sqlite_path: PathBuf::from("workspace/memory/brain.db"),
                core_markdown: PathBuf::from("workspace/MEMORY.md"),
                daily_dir: PathBuf::from("workspace/daily"),
                vector_weight: 0.7,
                keyword_weight: 0.3,
                vector_dims: 64,
            },
            state_sql: StateSqlConfig::default(),
            heartbeat: HeartbeatConfig {
                enabled: true,
                interval_seconds: 1800,
                tasks_file: PathBuf::from("workspace/HEARTBEAT.md"),
            },
            worker: WorkerConfig {
                inbox_path: PathBuf::from("workspace/queue/inbox.ndjson"),
                outbox_path: PathBuf::from("workspace/queue/outbox.ndjson"),
                inflight_path: PathBuf::from("workspace/queue/inflight.json"),
                state_path: PathBuf::from("workspace/state/worker_state.json"),
                dead_letter_path: default_dead_letter_path(),
                poll_interval_seconds: 3,
                command_timeout_seconds: 60,
                concurrency: 1,
                max_retries: 1,
                max_inbox_depth: default_max_inbox_depth(),
                allow_shell_commands: false,
                allow_http_get: false,
                allow_insecure_https: false,
                http_get_mode: default_http_get_mode(),
                http_get_sandbox_hosts: vec![],
            },
            adapters: AdapterConfig {
                terminal_enabled: true,
                webhook_url: None,
                webhook_timeout_seconds: 10,
            },
            logging: LoggingConfig {
                file: PathBuf::from("workspace/logs/quant-m.log"),
                max_bytes: 1_048_576,
                keep_files: 3,
            },
            skills: SkillsConfig {
                dir: PathBuf::from("workspace/skills"),
                allow_shell_commands: false,
            },
            llm: LlmConfig::default(),
            telegram: TelegramConfig::default(),
            chat_channels: ChatChannelsConfig::default(),
            forex: ForexConfig::default(),
            runtime: RuntimeConfig::default(),
            context_guardian: ContextGuardianConfig::default(),
            preferences: PreferenceConfig::default(),
            providers: default_provider_registry(),
            tools: default_tool_registry(),
        }
    }
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base: "https://openrouter.ai/api/v1".to_string(),
            model: "openai/gpt-4o-mini".to_string(),
            api_key: None,
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            temperature: 0.3,
            max_tokens: 512,
            request_timeout_seconds: 60,
        }
    }
}

impl Default for TelegramConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            bot_token: None,
            allowed_chat_id: None,
            poll_interval_seconds: 3,
        }
    }
}

impl Default for ChatChannelsConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            allowed_channels: vec![ExternalChannel::Telegram],
            default_channel: ExternalChannel::Telegram,
        }
    }
}

impl Default for StateSqlConfig {
    fn default() -> Self {
        Self {
            sqlite_path: PathBuf::from("workspace/state/shared-state.db"),
        }
    }
}

impl Default for ForexConfig {
    fn default() -> Self {
        Self {
            redb_path: PathBuf::from("workspace/state/forex.redb"),
        }
    }
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            profile: RuntimeProfile::default(),
            session_dir: default_session_dir(),
            external_network_enabled: false,
            multi_model_enabled: false,
            search_enabled: false,
            browser_harness_enabled: false,
        }
    }
}

impl Default for ContextGuardianConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_seconds: default_context_guardian_interval_seconds(),
            min_event_count: default_context_guardian_min_event_count(),
            high_risk_event_count: default_context_guardian_high_risk_event_count(),
            branch_packet_path: default_context_guardian_branch_packet_path(),
        }
    }
}

impl Default for ChannelPreference {
    fn default() -> Self {
        Self {
            channel: ExternalChannel::None,
            value: None,
        }
    }
}

impl Config {
    #[allow(dead_code)]
    pub fn load_existing(config_path: &Path) -> Result<Self> {
        if !config_path.exists() {
            anyhow::bail!("config not found at {}", config_path.display());
        }
        let raw = std::fs::read_to_string(config_path)
            .with_context(|| format!("failed to read config {}", config_path.display()))?;
        let cfg: Self = toml::from_str(&raw)
            .with_context(|| format!("invalid config TOML at {}", config_path.display()))?;
        Ok(cfg
            .apply_env_overrides()
            .resolve_paths(config_path)
            .sanitize())
    }

    pub fn load_or_create(config_path: &Path) -> Result<Self> {
        let cfg = Self::load_source_or_create(config_path)?;
        Ok(cfg
            .apply_env_overrides()
            .resolve_paths(config_path)
            .sanitize())
    }

    pub fn save(&self, config_path: &Path) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let raw = toml::to_string_pretty(&self.portable_for_save(config_path))
            .context("failed to serialize config")?;
        std::fs::write(config_path, raw)
            .with_context(|| format!("failed to write {}", config_path.display()))
    }

    pub fn render_toml(&self, config_path: &Path) -> Result<String> {
        toml::to_string_pretty(&self.portable_for_save(config_path))
            .context("failed to serialize config")
    }

    pub fn portable_view(&self, config_path: &Path) -> Self {
        self.portable_for_save(config_path)
    }

    fn load_source_or_create(config_path: &Path) -> Result<Self> {
        if config_path.exists() {
            let raw = std::fs::read_to_string(config_path)
                .with_context(|| format!("failed to read config {}", config_path.display()))?;
            return toml::from_str(&raw)
                .with_context(|| format!("invalid config TOML at {}", config_path.display()));
        }

        let cfg = Self::default();
        cfg.save(config_path)?;
        Ok(cfg)
    }

    fn apply_env_overrides(self) -> Self {
        self.apply_env_overrides_from(|key| std::env::var(key).ok())
    }

    fn apply_env_overrides_from<F>(mut self, mut get_env: F) -> Self
    where
        F: FnMut(&str) -> Option<String>,
    {
        let original_workspace_dir = self.workspace_dir.clone();
        let workspace_overridden =
            override_path_from_env(&mut self.workspace_dir, ENV_WORKSPACE_DIR, &mut get_env);
        let memory_sqlite_overridden = override_path_from_env(
            &mut self.memory.sqlite_path,
            ENV_MEMORY_SQLITE_PATH,
            &mut get_env,
        );
        let memory_core_overridden = override_path_from_env(
            &mut self.memory.core_markdown,
            ENV_MEMORY_CORE_MARKDOWN,
            &mut get_env,
        );
        let memory_daily_overridden = override_path_from_env(
            &mut self.memory.daily_dir,
            ENV_MEMORY_DAILY_DIR,
            &mut get_env,
        );
        let state_sql_overridden = override_path_from_env(
            &mut self.state_sql.sqlite_path,
            ENV_STATE_SQLITE_PATH,
            &mut get_env,
        );
        let heartbeat_tasks_overridden = override_path_from_env(
            &mut self.heartbeat.tasks_file,
            ENV_HEARTBEAT_TASKS_FILE,
            &mut get_env,
        );
        let worker_inbox_overridden = override_path_from_env(
            &mut self.worker.inbox_path,
            ENV_WORKER_INBOX_PATH,
            &mut get_env,
        );
        let worker_outbox_overridden = override_path_from_env(
            &mut self.worker.outbox_path,
            ENV_WORKER_OUTBOX_PATH,
            &mut get_env,
        );
        let worker_inflight_overridden = override_path_from_env(
            &mut self.worker.inflight_path,
            ENV_WORKER_INFLIGHT_PATH,
            &mut get_env,
        );
        let worker_state_overridden = override_path_from_env(
            &mut self.worker.state_path,
            ENV_WORKER_STATE_PATH,
            &mut get_env,
        );
        let worker_dead_letter_overridden = override_path_from_env(
            &mut self.worker.dead_letter_path,
            ENV_WORKER_DEAD_LETTER_PATH,
            &mut get_env,
        );
        let log_file_overridden =
            override_path_from_env(&mut self.logging.file, ENV_LOG_FILE, &mut get_env);
        let skills_dir_overridden =
            override_path_from_env(&mut self.skills.dir, ENV_SKILLS_DIR, &mut get_env);
        let forex_redb_overridden =
            override_path_from_env(&mut self.forex.redb_path, ENV_FOREX_REDB_PATH, &mut get_env);
        let session_dir_overridden =
            override_path_from_env(&mut self.runtime.session_dir, ENV_SESSION_DIR, &mut get_env);

        if workspace_overridden {
            rebase_workspace_child(
                &mut self.memory.sqlite_path,
                &original_workspace_dir,
                &self.workspace_dir,
                memory_sqlite_overridden,
            );
            rebase_workspace_child(
                &mut self.memory.core_markdown,
                &original_workspace_dir,
                &self.workspace_dir,
                memory_core_overridden,
            );
            rebase_workspace_child(
                &mut self.memory.daily_dir,
                &original_workspace_dir,
                &self.workspace_dir,
                memory_daily_overridden,
            );
            rebase_workspace_child(
                &mut self.state_sql.sqlite_path,
                &original_workspace_dir,
                &self.workspace_dir,
                state_sql_overridden,
            );
            rebase_workspace_child(
                &mut self.heartbeat.tasks_file,
                &original_workspace_dir,
                &self.workspace_dir,
                heartbeat_tasks_overridden,
            );
            rebase_workspace_child(
                &mut self.worker.inbox_path,
                &original_workspace_dir,
                &self.workspace_dir,
                worker_inbox_overridden,
            );
            rebase_workspace_child(
                &mut self.worker.outbox_path,
                &original_workspace_dir,
                &self.workspace_dir,
                worker_outbox_overridden,
            );
            rebase_workspace_child(
                &mut self.worker.inflight_path,
                &original_workspace_dir,
                &self.workspace_dir,
                worker_inflight_overridden,
            );
            rebase_workspace_child(
                &mut self.worker.state_path,
                &original_workspace_dir,
                &self.workspace_dir,
                worker_state_overridden,
            );
            rebase_workspace_child(
                &mut self.worker.dead_letter_path,
                &original_workspace_dir,
                &self.workspace_dir,
                worker_dead_letter_overridden,
            );
            rebase_workspace_child(
                &mut self.logging.file,
                &original_workspace_dir,
                &self.workspace_dir,
                log_file_overridden,
            );
            rebase_workspace_child(
                &mut self.skills.dir,
                &original_workspace_dir,
                &self.workspace_dir,
                skills_dir_overridden,
            );
            rebase_workspace_child(
                &mut self.forex.redb_path,
                &original_workspace_dir,
                &self.workspace_dir,
                forex_redb_overridden,
            );
            rebase_workspace_child(
                &mut self.runtime.session_dir,
                &original_workspace_dir,
                &self.workspace_dir,
                session_dir_overridden,
            );
        }

        self
    }

    pub fn resolve_paths(&self, config_path: &Path) -> Self {
        let mut cfg = self.clone();
        let base = config_base(config_path);

        cfg.workspace_dir = absolutize(&base, &cfg.workspace_dir);
        cfg.memory.sqlite_path = absolutize(&base, &cfg.memory.sqlite_path);
        cfg.state_sql.sqlite_path = absolutize(&base, &cfg.state_sql.sqlite_path);
        cfg.memory.core_markdown = absolutize(&base, &cfg.memory.core_markdown);
        cfg.memory.daily_dir = absolutize(&base, &cfg.memory.daily_dir);
        cfg.heartbeat.tasks_file = absolutize(&base, &cfg.heartbeat.tasks_file);

        cfg.worker.inbox_path = absolutize(&base, &cfg.worker.inbox_path);
        cfg.worker.outbox_path = absolutize(&base, &cfg.worker.outbox_path);
        cfg.worker.inflight_path = absolutize(&base, &cfg.worker.inflight_path);
        cfg.worker.state_path = absolutize(&base, &cfg.worker.state_path);
        cfg.worker.dead_letter_path = absolutize(&base, &cfg.worker.dead_letter_path);

        cfg.logging.file = absolutize(&base, &cfg.logging.file);
        cfg.skills.dir = absolutize(&base, &cfg.skills.dir);
        cfg.forex.redb_path = absolutize(&base, &cfg.forex.redb_path);
        cfg.runtime.session_dir = absolutize(&base, &cfg.runtime.session_dir);
        cfg.context_guardian.branch_packet_path =
            absolutize(&base, &cfg.context_guardian.branch_packet_path);

        cfg
    }

    fn portable_for_save(&self, config_path: &Path) -> Self {
        let mut cfg = self.clone();
        let base = config_base(config_path);

        cfg.workspace_dir = relativize_for_save(&base, &cfg.workspace_dir);
        cfg.memory.sqlite_path = relativize_for_save(&base, &cfg.memory.sqlite_path);
        cfg.state_sql.sqlite_path = relativize_for_save(&base, &cfg.state_sql.sqlite_path);
        cfg.memory.core_markdown = relativize_for_save(&base, &cfg.memory.core_markdown);
        cfg.memory.daily_dir = relativize_for_save(&base, &cfg.memory.daily_dir);
        cfg.heartbeat.tasks_file = relativize_for_save(&base, &cfg.heartbeat.tasks_file);

        cfg.worker.inbox_path = relativize_for_save(&base, &cfg.worker.inbox_path);
        cfg.worker.outbox_path = relativize_for_save(&base, &cfg.worker.outbox_path);
        cfg.worker.inflight_path = relativize_for_save(&base, &cfg.worker.inflight_path);
        cfg.worker.state_path = relativize_for_save(&base, &cfg.worker.state_path);
        cfg.worker.dead_letter_path = relativize_for_save(&base, &cfg.worker.dead_letter_path);

        cfg.logging.file = relativize_for_save(&base, &cfg.logging.file);
        cfg.skills.dir = relativize_for_save(&base, &cfg.skills.dir);
        cfg.forex.redb_path = relativize_for_save(&base, &cfg.forex.redb_path);
        cfg.runtime.session_dir = relativize_for_save(&base, &cfg.runtime.session_dir);
        cfg.context_guardian.branch_packet_path =
            relativize_for_save(&base, &cfg.context_guardian.branch_packet_path);

        cfg
    }

    pub fn sanitize(mut self) -> Self {
        self.memory.vector_dims = self.memory.vector_dims.clamp(8, 1024);
        self.memory.vector_weight = self.memory.vector_weight.clamp(0.0, 1.0);
        self.memory.keyword_weight = self.memory.keyword_weight.clamp(0.0, 1.0);

        let total = self.memory.vector_weight + self.memory.keyword_weight;
        if total > 0.0 {
            self.memory.vector_weight /= total;
            self.memory.keyword_weight /= total;
        } else {
            self.memory.vector_weight = 0.7;
            self.memory.keyword_weight = 0.3;
        }

        self.heartbeat.interval_seconds = self.heartbeat.interval_seconds.clamp(5, 86_400);

        self.worker.poll_interval_seconds = self.worker.poll_interval_seconds.clamp(1, 3600);
        self.worker.command_timeout_seconds = self.worker.command_timeout_seconds.clamp(1, 3600);
        self.worker.concurrency = self.worker.concurrency.clamp(1, 8);
        self.worker.max_retries = self.worker.max_retries.min(10);
        self.worker.max_inbox_depth = self.worker.max_inbox_depth.clamp(32, 100_000);
        self.worker.http_get_mode = {
            let mode = self.worker.http_get_mode.trim().to_ascii_lowercase();
            match mode.as_str() {
                "dry_run" | "sandbox" | "live" => mode,
                _ => default_http_get_mode(),
            }
        };
        self.worker.http_get_sandbox_hosts = self
            .worker
            .http_get_sandbox_hosts
            .iter()
            .map(|host| host.trim().to_ascii_lowercase())
            .filter(|host| !host.is_empty())
            .collect();

        if self.logging.max_bytes < 16 * 1024 {
            self.logging.max_bytes = 16 * 1024;
        }
        self.logging.keep_files = self.logging.keep_files.clamp(1, 20);

        self.adapters.webhook_timeout_seconds = self.adapters.webhook_timeout_seconds.clamp(1, 60);
        self.adapters.webhook_url = self.adapters.webhook_url.and_then(|url| {
            let trimmed = url.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });

        self.llm.temperature = self.llm.temperature.clamp(0.0, 2.0);
        self.llm.max_tokens = self.llm.max_tokens.clamp(1, 8_192);
        self.llm.request_timeout_seconds = self.llm.request_timeout_seconds.clamp(5, 300);
        self.telegram.poll_interval_seconds = self.telegram.poll_interval_seconds.clamp(1, 30);
        self.llm.api_key = self.llm.api_key.and_then(|key| {
            let trimmed = key.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        self.telegram.bot_token = self.telegram.bot_token.and_then(|token| {
            let trimmed = token.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        });
        self.runtime.session_dir = if self.runtime.session_dir.as_os_str().is_empty() {
            default_session_dir()
        } else {
            self.runtime.session_dir.clone()
        };
        self.context_guardian.interval_seconds =
            self.context_guardian.interval_seconds.clamp(5, 86_400);
        self.context_guardian.min_event_count =
            self.context_guardian.min_event_count.clamp(1, 100_000);
        self.context_guardian.high_risk_event_count = self
            .context_guardian
            .high_risk_event_count
            .clamp(self.context_guardian.min_event_count, 100_000);
        self.context_guardian.branch_packet_path = if self
            .context_guardian
            .branch_packet_path
            .as_os_str()
            .is_empty()
        {
            default_context_guardian_branch_packet_path()
        } else {
            self.context_guardian.branch_packet_path.clone()
        };
        self.preferences.preferred_local_model =
            sanitize_model_preference(self.preferences.preferred_local_model.take());
        self.preferences.preferred_remote_model =
            sanitize_model_preference(self.preferences.preferred_remote_model.take());
        self.preferences.preferred_openrouter_model = self
            .preferences
            .preferred_openrouter_model
            .take()
            .and_then(trimmed_option_string);
        self.preferences.preferred_channel.value = self
            .preferences
            .preferred_channel
            .value
            .take()
            .and_then(|value| normalize_channel_value(&value));
        self.providers = sanitize_provider_registry(std::mem::take(&mut self.providers));
        self.tools = sanitize_tool_registry(std::mem::take(&mut self.tools));
        self.chat_channels.allowed_channels =
            sanitize_allowed_channels(std::mem::take(&mut self.chat_channels.allowed_channels));
        if self.chat_channels.default_channel == ExternalChannel::None {
            self.chat_channels.default_channel = ExternalChannel::Telegram;
        }
        if !self
            .chat_channels
            .allowed_channels
            .contains(&self.chat_channels.default_channel)
        {
            self.chat_channels
                .allowed_channels
                .push(self.chat_channels.default_channel);
        }

        self
    }

    pub fn validate(&self) -> Result<()> {
        ensure_path_present(
            &self.workspace_dir,
            "workspace_dir",
            Some(ENV_WORKSPACE_DIR),
        )?;
        ensure_path_present(
            &self.memory.daily_dir,
            "memory.daily_dir",
            Some(ENV_MEMORY_DAILY_DIR),
        )?;
        ensure_path_present(&self.skills.dir, "skills.dir", Some(ENV_SKILLS_DIR))?;
        ensure_path_present(
            &self.runtime.session_dir,
            "runtime.session_dir",
            Some(ENV_SESSION_DIR),
        )?;

        ensure_file_path(
            &self.memory.sqlite_path,
            "memory.sqlite_path",
            Some(ENV_MEMORY_SQLITE_PATH),
        )?;
        ensure_file_path(
            &self.memory.core_markdown,
            "memory.core_markdown",
            Some(ENV_MEMORY_CORE_MARKDOWN),
        )?;
        ensure_file_path(
            &self.state_sql.sqlite_path,
            "state_sql.sqlite_path",
            Some(ENV_STATE_SQLITE_PATH),
        )?;
        ensure_file_path(
            &self.forex.redb_path,
            "forex.redb_path",
            Some(ENV_FOREX_REDB_PATH),
        )?;
        ensure_file_path(
            &self.heartbeat.tasks_file,
            "heartbeat.tasks_file",
            Some(ENV_HEARTBEAT_TASKS_FILE),
        )?;
        ensure_file_path(
            &self.worker.inbox_path,
            "worker.inbox_path",
            Some(ENV_WORKER_INBOX_PATH),
        )?;
        ensure_file_path(
            &self.worker.outbox_path,
            "worker.outbox_path",
            Some(ENV_WORKER_OUTBOX_PATH),
        )?;
        ensure_file_path(
            &self.worker.inflight_path,
            "worker.inflight_path",
            Some(ENV_WORKER_INFLIGHT_PATH),
        )?;
        ensure_file_path(
            &self.worker.state_path,
            "worker.state_path",
            Some(ENV_WORKER_STATE_PATH),
        )?;
        ensure_file_path(
            &self.worker.dead_letter_path,
            "worker.dead_letter_path",
            Some(ENV_WORKER_DEAD_LETTER_PATH),
        )?;
        ensure_file_path(&self.logging.file, "logging.file", Some(ENV_LOG_FILE))?;
        ensure_file_path(
            &self.context_guardian.branch_packet_path,
            "context_guardian.branch_packet_path",
            None,
        )?;

        let mut seen = std::collections::BTreeSet::new();
        for (name, path) in [
            ("inbox", &self.worker.inbox_path),
            ("outbox", &self.worker.outbox_path),
            ("inflight", &self.worker.inflight_path),
            ("state", &self.worker.state_path),
            ("dead_letter", &self.worker.dead_letter_path),
        ] {
            if !seen.insert(path.clone()) {
                anyhow::bail!(
                    "worker path collision detected at '{}': {}",
                    name,
                    path.display()
                );
            }
        }

        if self.llm.enabled && self.resolve_llm_api_key().is_none() {
            anyhow::bail!(
                "llm.enabled=true but no API key found in llm.api_key or env {}",
                self.llm.api_key_env
            );
        }
        if self.telegram.enabled && self.resolve_telegram_bot_token().is_none() {
            anyhow::bail!(
                "telegram.enabled=true but no bot token found in telegram.bot_token or TELEGRAM_BOT_TOKEN"
            );
        }
        if self.worker.allow_http_get
            && self.worker.http_get_mode == "sandbox"
            && self.worker.http_get_sandbox_hosts.is_empty()
        {
            anyhow::bail!(
                "worker.http_get_mode=sandbox requires at least one host in worker.http_get_sandbox_hosts"
            );
        }
        for (id, provider) in &self.providers {
            if id.trim().is_empty() {
                anyhow::bail!("provider registry contains an empty provider id");
            }
            if provider.api_base.trim().is_empty() {
                anyhow::bail!("providers.{}.api_base is empty", id);
            }
            if provider.api_key_env.trim().is_empty()
                && !matches!(provider.kind, ProviderKind::Ollama | ProviderKind::LmStudio)
            {
                anyhow::bail!("providers.{}.api_key_env is empty", id);
            }
        }
        for (id, tool) in &self.tools {
            if id.trim().is_empty() {
                anyhow::bail!("tool registry contains an empty tool id");
            }
            if tool.command.trim().is_empty() {
                anyhow::bail!("tools.{}.command is empty", id);
            }
        }

        Ok(())
    }

    pub fn resolve_llm_api_key(&self) -> Option<String> {
        if let Some(inline) = self
            .llm
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(inline.to_string());
        }
        std::env::var(&self.llm.api_key_env)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    pub fn resolve_telegram_bot_token(&self) -> Option<String> {
        if let Some(inline) = self
            .telegram
            .bot_token
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            return Some(inline.to_string());
        }
        std::env::var("TELEGRAM_BOT_TOKEN")
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }

    pub fn set_preferred_model(&mut self, provider: &str, model: &str) -> Result<()> {
        let provider = provider.trim();
        let model = model.trim();
        if provider.is_empty() {
            anyhow::bail!("provider is empty");
        }
        if model.is_empty() {
            anyhow::bail!("model is empty");
        }

        let preference = ModelPreference {
            provider: provider.to_string(),
            model: model.to_string(),
        };
        if is_local_provider(provider) {
            self.preferences.preferred_local_model = Some(preference);
        } else {
            self.preferences.preferred_remote_model = Some(preference);
        }
        if provider.eq_ignore_ascii_case("openrouter") {
            self.preferences.preferred_openrouter_model = Some(model.to_string());
            self.llm.model = model.to_string();
        }
        Ok(())
    }

    pub fn ensure_onboarding_registries(&mut self) {
        let defaults = default_provider_registry();
        for (id, provider) in defaults {
            self.providers.entry(id).or_insert(provider);
        }
        let defaults = default_tool_registry();
        for (id, tool) in defaults {
            self.tools.entry(id).or_insert(tool);
        }
    }

    pub fn set_channel_preference(
        &mut self,
        channel: ExternalChannel,
        value: Option<&str>,
    ) -> Result<()> {
        self.preferences.preferred_channel = ChannelPreference {
            channel,
            value: value.and_then(normalize_channel_value),
        };
        Ok(())
    }
}

impl std::str::FromStr for RuntimeProfile {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "edge" => Ok(Self::Edge),
            "laptop" => Ok(Self::Laptop),
            "vps" => Ok(Self::Vps),
            "staff-os-worker" | "staff_os_worker" => Ok(Self::StaffOsWorker),
            other => anyhow::bail!("unknown runtime profile '{}'", other),
        }
    }
}

impl std::str::FromStr for ExternalChannel {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "telegram" => Ok(Self::Telegram),
            "discord" => Ok(Self::Discord),
            "slack" => Ok(Self::Slack),
            "signal" => Ok(Self::Signal),
            "whatsapp" | "whats_app" => Ok(Self::Whatsapp),
            "ichat" | "i_chat" | "imessage" | "apple_messages" => Ok(Self::Ichat),
            "email" => Ok(Self::Email),
            "none" => Ok(Self::None),
            other => anyhow::bail!("unknown external channel '{}'", other),
        }
    }
}

fn config_base(config_path: &Path) -> PathBuf {
    config_path
        .parent()
        .map_or_else(|| PathBuf::from("."), PathBuf::from)
}

fn absolutize(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        return path.to_path_buf();
    }

    let joined = base.join(path);
    joined.components().collect()
}

fn relativize_for_save(base: &Path, path: &Path) -> PathBuf {
    if !path.is_absolute() {
        return path.to_path_buf();
    }

    match path.strip_prefix(base) {
        Ok(relative) if !relative.as_os_str().is_empty() => relative.to_path_buf(),
        Ok(_) => PathBuf::from("."),
        Err(_) => path.to_path_buf(),
    }
}

fn override_path_from_env<F>(path: &mut PathBuf, env_var: &str, get_env: &mut F) -> bool
where
    F: FnMut(&str) -> Option<String>,
{
    let Some(value) = get_env(env_var) else {
        return false;
    };
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return false;
    }
    *path = PathBuf::from(trimmed);
    true
}

fn rebase_workspace_child(
    path: &mut PathBuf,
    original_workspace_dir: &Path,
    overridden_workspace_dir: &Path,
    explicit_override: bool,
) {
    if explicit_override {
        return;
    }

    let Ok(relative) = path.strip_prefix(original_workspace_dir) else {
        return;
    };
    *path = if relative.as_os_str().is_empty() {
        overridden_workspace_dir.to_path_buf()
    } else {
        overridden_workspace_dir.join(relative)
    };
}

#[cfg(feature = "fuzzing_hooks")]
pub fn parse_and_validate_toml_for_fuzz(raw: &str) -> Result<()> {
    if let Ok(cfg) = toml::from_str::<Config>(raw) {
        return cfg.validate();
    }
    if toml::from_str::<RuntimeConfig>(raw).is_ok() {
        return Ok(());
    }
    if toml::from_str::<PreferenceConfig>(raw).is_ok() {
        return Ok(());
    }
    let _ = toml::from_str::<ModelPreference>(raw)?;
    Ok(())
}

fn default_node_id() -> String {
    std::env::var("HOSTNAME")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .unwrap_or_else(|| "quant-m-node".to_string())
}

fn default_dead_letter_path() -> PathBuf {
    PathBuf::from("workspace/queue/dead-letter.ndjson")
}

fn default_max_inbox_depth() -> usize {
    2_000
}

fn default_http_get_mode() -> String {
    "dry_run".to_string()
}

fn default_session_dir() -> PathBuf {
    PathBuf::from("workspace/state/sessions")
}

fn default_context_guardian_enabled() -> bool {
    true
}

fn default_context_guardian_interval_seconds() -> u64 {
    300
}

fn default_context_guardian_min_event_count() -> usize {
    1
}

fn default_context_guardian_high_risk_event_count() -> usize {
    40
}

fn default_context_guardian_branch_packet_path() -> PathBuf {
    PathBuf::from("workspace/state/context-guardian/continuity-handoff.md")
}

fn trimmed_option_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn sanitize_model_preference(value: Option<ModelPreference>) -> Option<ModelPreference> {
    let model = value?;
    let provider = trimmed_option_string(model.provider)?;
    let model_name = trimmed_option_string(model.model)?;
    Some(ModelPreference {
        provider,
        model: model_name,
    })
}

fn normalize_channel_value(value: &str) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty()
        || trimmed.eq_ignore_ascii_case("disabled")
        || trimmed.eq_ignore_ascii_case("none")
    {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn sanitize_allowed_channels(values: Vec<ExternalChannel>) -> Vec<ExternalChannel> {
    let mut out = Vec::new();
    for value in values {
        if value == ExternalChannel::None || out.contains(&value) {
            continue;
        }
        out.push(value);
    }
    if out.is_empty() {
        out.push(ExternalChannel::Telegram);
    }
    out
}

fn sanitize_provider_registry(
    values: BTreeMap<String, ProviderConfig>,
) -> BTreeMap<String, ProviderConfig> {
    let mut out = default_provider_registry();
    for (id, mut provider) in values {
        let Some(id) = trimmed_option_string(id) else {
            continue;
        };
        provider.api_base = provider.api_base.trim().trim_end_matches('/').to_string();
        provider.api_key_env = provider.api_key_env.trim().to_string();
        provider.preferred_models = provider
            .preferred_models
            .into_iter()
            .filter_map(trimmed_option_string)
            .collect();
        out.insert(id, provider);
    }
    out
}

fn sanitize_tool_registry(values: BTreeMap<String, ToolConfig>) -> BTreeMap<String, ToolConfig> {
    let mut out = default_tool_registry();
    for (id, mut tool) in values {
        let Some(id) = trimmed_option_string(id) else {
            continue;
        };
        tool.command = tool.command.trim().to_string();
        tool.validation_args = tool
            .validation_args
            .into_iter()
            .filter_map(trimmed_option_string)
            .collect();
        out.insert(id, tool);
    }
    out
}

fn default_provider_registry() -> BTreeMap<String, ProviderConfig> {
    let mut providers = BTreeMap::new();
    providers.insert(
        "openrouter".to_string(),
        ProviderConfig {
            enabled: false,
            kind: ProviderKind::OpenRouter,
            api_base: "https://openrouter.ai/api/v1".to_string(),
            api_key_env: "OPENROUTER_API_KEY".to_string(),
            preferred_models: vec![
                "qwen/qwen3-coder".to_string(),
                "openai/gpt-4o-mini".to_string(),
            ],
            live_validation_allowed: false,
        },
    );
    providers.insert(
        "openai".to_string(),
        ProviderConfig {
            enabled: false,
            kind: ProviderKind::OpenAi,
            api_base: "https://api.openai.com/v1".to_string(),
            api_key_env: "OPENAI_API_KEY".to_string(),
            preferred_models: vec!["gpt-5-codex".to_string(), "gpt-5".to_string()],
            live_validation_allowed: false,
        },
    );
    providers.insert(
        "ollama".to_string(),
        ProviderConfig {
            enabled: false,
            kind: ProviderKind::Ollama,
            api_base: "http://127.0.0.1:11434".to_string(),
            api_key_env: String::new(),
            preferred_models: vec!["qwen3-coder:7b".to_string()],
            live_validation_allowed: false,
        },
    );
    providers.insert(
        "lmstudio".to_string(),
        ProviderConfig {
            enabled: false,
            kind: ProviderKind::LmStudio,
            api_base: "http://127.0.0.1:1234/v1".to_string(),
            api_key_env: String::new(),
            preferred_models: vec![],
            live_validation_allowed: false,
        },
    );
    providers
}

fn default_tool_registry() -> BTreeMap<String, ToolConfig> {
    let mut tools = BTreeMap::new();
    for (id, kind, command, args) in [
        ("codex", ToolKind::Codex, "codex", vec!["--version"]),
        ("openai", ToolKind::OpenAi, "openai", vec!["--version"]),
        ("gemini", ToolKind::Gemini, "gemini", vec!["--version"]),
        (
            "anthropic",
            ToolKind::Anthropic,
            "anthropic",
            vec!["--version"],
        ),
        ("claude", ToolKind::Claude, "claude", vec!["--version"]),
        (
            "opencode",
            ToolKind::OpenCode,
            "opencode",
            vec!["--version"],
        ),
        (
            "antigravity",
            ToolKind::Antigravity,
            "antigravity",
            vec!["--version"],
        ),
        (
            "antgravity",
            ToolKind::Antgravity,
            "antgravity",
            vec!["--version"],
        ),
        ("hermes", ToolKind::Hermes, "hermes", vec!["--version"]),
        ("pi-agent", ToolKind::PiAgent, "pi-agent", vec!["--version"]),
        (
            "openclaw",
            ToolKind::OpenClaw,
            "openclaw",
            vec!["--version"],
        ),
        ("ollama", ToolKind::Ollama, "ollama", vec!["--version"]),
        ("lmstudio", ToolKind::LmStudio, "lms", vec!["--version"]),
    ] {
        tools.insert(
            id.to_string(),
            ToolConfig {
                enabled: false,
                kind,
                command: command.to_string(),
                validation_args: args.into_iter().map(str::to_string).collect(),
            },
        );
    }
    tools
}

fn is_local_provider(provider: &str) -> bool {
    matches!(
        provider.trim().to_ascii_lowercase().as_str(),
        "local" | "ollama" | "llama.cpp" | "llamacpp"
    )
}

fn ensure_path_present(path: &Path, field: &str, env_var: Option<&str>) -> Result<()> {
    if path.as_os_str().is_empty() {
        match env_var {
            Some(env_var) => anyhow::bail!(
                "{} resolved to an empty path; set a non-empty value in quant-m.toml or {}",
                field,
                env_var
            ),
            None => anyhow::bail!("{} resolved to an empty path; set a non-empty value", field),
        }
    }
    Ok(())
}

fn ensure_file_path(path: &Path, field: &str, env_var: Option<&str>) -> Result<()> {
    ensure_path_present(path, field, env_var)?;
    if path.parent().is_none() {
        match env_var {
            Some(env_var) => anyhow::bail!(
                "{} has no parent directory after resolution: {}; use a nested path in quant-m.toml or {}",
                field,
                path.display(),
                env_var
            ),
            None => anyhow::bail!(
                "{} has no parent directory after resolution: {}",
                field,
                path.display()
            ),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use tempfile::tempdir;

    #[test]
    fn sanitize_normalizes_memory_weights_and_limits() {
        let mut cfg = Config::default();
        cfg.memory.vector_weight = 10.0;
        cfg.memory.keyword_weight = 10.0;
        cfg.worker.command_timeout_seconds = 0;
        cfg.worker.concurrency = 0;
        cfg.worker.max_inbox_depth = 0;
        cfg.worker.http_get_mode = " INVALID ".to_string();
        cfg.worker.http_get_sandbox_hosts =
            vec!["  API.SANDBOX.EXAMPLE.COM  ".to_string(), " ".to_string()];
        cfg.adapters.webhook_url = Some("   ".to_string());

        let cfg = cfg.sanitize();
        let sum = cfg.memory.vector_weight + cfg.memory.keyword_weight;
        assert!((sum - 1.0).abs() < 0.001);
        assert!(cfg.worker.command_timeout_seconds >= 1);
        assert_eq!(cfg.worker.concurrency, 1);
        assert!(cfg.worker.max_inbox_depth >= 32);
        assert_eq!(cfg.worker.http_get_mode, "dry_run");
        assert_eq!(
            cfg.worker.http_get_sandbox_hosts,
            vec!["api.sandbox.example.com".to_string()]
        );
        assert!(cfg.adapters.webhook_url.is_none());
    }

    #[test]
    fn validate_requires_llm_key_when_llm_enabled() {
        let mut cfg = Config::default();
        cfg.llm.enabled = true;
        cfg.llm.api_key = None;
        cfg.llm.api_key_env = "QUANT_M_TEST_MISSING_KEY_DO_NOT_SET".to_string();
        let result = cfg.validate();
        assert!(result.is_err());
    }

    #[test]
    fn validate_requires_sandbox_hosts_when_http_sandbox_enabled() {
        let mut cfg = Config::default();
        cfg.worker.allow_http_get = true;
        cfg.worker.http_get_mode = "sandbox".to_string();
        cfg.worker.http_get_sandbox_hosts.clear();
        let result = cfg.validate();
        assert!(result.is_err());
    }

    #[test]
    fn save_keeps_workspace_paths_relative() {
        let temp = tempdir().expect("tempdir");
        let config_path = temp.path().join("quant-m.toml");

        let mut cfg = Config::default().resolve_paths(&config_path);
        cfg.workspace_dir = temp.path().join("workspace");
        cfg.memory.sqlite_path = temp.path().join("workspace/memory/brain.db");
        cfg.memory.core_markdown = temp.path().join("workspace/MEMORY.md");
        cfg.memory.daily_dir = temp.path().join("workspace/daily");
        cfg.state_sql.sqlite_path = temp.path().join("workspace/state/shared-state.db");
        cfg.forex.redb_path = temp.path().join("workspace/state/forex.redb");
        cfg.heartbeat.tasks_file = temp.path().join("workspace/HEARTBEAT.md");
        cfg.worker.inbox_path = temp.path().join("workspace/queue/inbox.ndjson");
        cfg.worker.outbox_path = temp.path().join("workspace/queue/outbox.ndjson");
        cfg.worker.inflight_path = temp.path().join("workspace/queue/inflight.json");
        cfg.worker.state_path = temp.path().join("workspace/state/worker_state.json");
        cfg.worker.dead_letter_path = temp.path().join("workspace/queue/dead-letter.ndjson");
        cfg.logging.file = temp.path().join("workspace/logs/quant-m.log");
        cfg.skills.dir = temp.path().join("workspace/skills");
        cfg.runtime.session_dir = temp.path().join("workspace/state/sessions");

        cfg.save(&config_path).expect("save config");
        let raw = std::fs::read_to_string(&config_path).expect("read saved config");

        assert!(raw.contains("workspace_dir = \"workspace\""));
        assert!(raw.contains("sqlite_path = \"workspace/memory/brain.db\""));
        assert!(raw.contains("session_dir = \"workspace/state/sessions\""));
        assert!(!raw.contains(&temp.path().display().to_string()));
    }

    #[test]
    fn env_overrides_apply_to_runtime_paths() {
        let cfg = Config::default().apply_env_overrides_from(|key| match key {
            ENV_WORKSPACE_DIR => Some("portable-workspace".to_string()),
            _ => None,
        });

        assert_eq!(cfg.workspace_dir, PathBuf::from("portable-workspace"));
        assert_eq!(
            cfg.memory.sqlite_path,
            PathBuf::from("portable-workspace/memory/brain.db")
        );
    }

    #[test]
    fn explicit_path_override_wins_over_workspace_rebase() {
        let cfg = Config::default().apply_env_overrides_from(|key| match key {
            ENV_WORKSPACE_DIR => Some("portable-workspace".to_string()),
            ENV_MEMORY_SQLITE_PATH => Some("custom/brain.db".to_string()),
            _ => None,
        });

        assert_eq!(cfg.workspace_dir, PathBuf::from("portable-workspace"));
        assert_eq!(cfg.memory.sqlite_path, PathBuf::from("custom/brain.db"));
    }

    #[test]
    fn copied_workspace_resolves_paths_under_new_root() {
        let source = tempdir().expect("source tempdir");
        let copied = tempdir().expect("copied tempdir");
        let source_config_path = source.path().join("quant-m.toml");
        let copied_config_path = copied.path().join("quant-m.toml");

        Config::default()
            .save(&source_config_path)
            .expect("save source config");
        std::fs::copy(&source_config_path, &copied_config_path).expect("copy config");

        let cfg = Config::load_or_create(&copied_config_path).expect("load copied config");
        bootstrap::ensure_workspace(&cfg).expect("ensure workspace");

        assert_eq!(cfg.workspace_dir, copied.path().join("workspace"));
        assert_eq!(
            cfg.memory.sqlite_path,
            copied.path().join("workspace/memory/brain.db")
        );
        assert_eq!(
            cfg.runtime.session_dir,
            copied.path().join("workspace/state/sessions")
        );
        assert!(cfg.workspace_dir.join("SOUL.md").exists());
        assert!(cfg.memory.core_markdown.exists());
        assert!(cfg.worker.inbox_path.exists());
    }

    #[test]
    fn validate_reports_invalid_path_with_override_name() {
        let mut cfg = Config::default();
        cfg.memory.sqlite_path = PathBuf::new();

        let err = cfg.validate().expect_err("validation should fail");
        let message = err.to_string();
        assert!(message.contains("memory.sqlite_path"));
        assert!(message.contains(ENV_MEMORY_SQLITE_PATH));
    }

    #[test]
    fn set_preferred_model_updates_typed_preferences() {
        let mut cfg = Config::default();
        cfg.set_preferred_model("openrouter", "qwen/qwen3-coder")
            .expect("set model");
        assert_eq!(
            cfg.preferences.preferred_remote_model,
            Some(ModelPreference {
                provider: "openrouter".to_string(),
                model: "qwen/qwen3-coder".to_string(),
            })
        );
        assert_eq!(
            cfg.preferences.preferred_openrouter_model.as_deref(),
            Some("qwen/qwen3-coder")
        );
        assert_eq!(cfg.llm.model, "qwen/qwen3-coder");
    }

    #[test]
    fn set_channel_preference_updates_typed_preferences() {
        let mut cfg = Config::default();
        cfg.set_channel_preference(ExternalChannel::Telegram, Some("disabled"))
            .expect("set channel");
        assert_eq!(
            cfg.preferences.preferred_channel.channel,
            ExternalChannel::Telegram
        );
        assert!(cfg.preferences.preferred_channel.value.is_none());
    }

    #[test]
    fn validate_accepts_safe_relative_session_path() {
        let mut cfg = Config::default();
        cfg.runtime.session_dir = PathBuf::from("workspace/state/sessions");
        cfg.validate()
            .expect("relative session path should validate");
    }
}
