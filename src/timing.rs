use crate::config::Config;
use crate::desk_registry::DeskId;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Component, Path, PathBuf};

const GENERAL_MIN_POLL_SECONDS: u64 = 1;
const TABLET_MIN_POLL_SECONDS: u64 = 60;
const TABLET_HEARTBEAT_SECONDS: u64 = 60;
const TABLET_ROLE_LEASE_SECONDS: u64 = 30 * 60;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimingTrigger {
    Cron {
        expression: String,
    },
    Polling {
        interval_seconds: u64,
    },
    Mtime {
        path: String,
    },
    SessionWindow {
        session: MarketSession,
    },
    EventWindow {
        event_type: String,
        before_seconds: u64,
        after_seconds: u64,
    },
    Heartbeat {
        interval_seconds: u64,
    },
    Cooldown {
        duration_seconds: u64,
    },
}

impl TimingTrigger {
    fn kind(&self) -> &'static str {
        match self {
            Self::Cron { .. } => "cron",
            Self::Polling { .. } => "polling",
            Self::Mtime { .. } => "mtime",
            Self::SessionWindow { .. } => "session_window",
            Self::EventWindow { .. } => "event_window",
            Self::Heartbeat { .. } => "heartbeat",
            Self::Cooldown { .. } => "cooldown",
        }
    }
}

impl fmt::Display for TimingTrigger {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cron { expression } => write!(f, "cron({expression})"),
            Self::Polling { interval_seconds } => write!(f, "polling({interval_seconds}s)"),
            Self::Mtime { path } => write!(f, "mtime({path})"),
            Self::SessionWindow { session } => write!(f, "session_window({session})"),
            Self::EventWindow {
                event_type,
                before_seconds,
                after_seconds,
            } => write!(
                f,
                "event_window({event_type},-{before_seconds}s,+{after_seconds}s)"
            ),
            Self::Heartbeat { interval_seconds } => write!(f, "heartbeat({interval_seconds}s)"),
            Self::Cooldown { duration_seconds } => write!(f, "cooldown({duration_seconds}s)"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MarketSession {
    ForexTokyo,
    ForexLondon,
    ForexNewYork,
    CryptoAlwaysOn,
    UsEquitiesPremarket,
    UsEquitiesRegular,
    UsEquitiesAfterHours,
    SportsPregame,
    SportsLive,
    SportsPostgame,
}

impl fmt::Display for MarketSession {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::ForexTokyo => "forex_tokyo",
            Self::ForexLondon => "forex_london",
            Self::ForexNewYork => "forex_new_york",
            Self::CryptoAlwaysOn => "crypto_always_on",
            Self::UsEquitiesPremarket => "us_equities_premarket",
            Self::UsEquitiesRegular => "us_equities_regular",
            Self::UsEquitiesAfterHours => "us_equities_after_hours",
            Self::SportsPregame => "sports_pregame",
            Self::SportsLive => "sports_live",
            Self::SportsPostgame => "sports_postgame",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeskTimingPolicy {
    pub desk_id: DeskId,
    pub role_id: String,
    pub allowed_triggers: Vec<TimingTrigger>,
    pub stale_after_seconds: u64,
    pub min_refresh_seconds: u64,
    pub max_refresh_seconds: u64,
    pub cooldown_after_proposal_seconds: u64,
    pub tablet_safe: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TimingFsmDecision {
    WaitTiming,
    RejectStaleEvidence,
    RejectCooldown,
    WatchOnlyEventWindow,
    AllowEvaluation,
}

impl fmt::Display for TimingFsmDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let value = match self {
            Self::WaitTiming => "wait_timing",
            Self::RejectStaleEvidence => "reject_stale_evidence",
            Self::RejectCooldown => "reject_cooldown",
            Self::WatchOnlyEventWindow => "watch_only_event_window",
            Self::AllowEvaluation => "allow_evaluation",
        };
        f.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimingCooldown {
    pub desk_id: DeskId,
    pub role_id: String,
    pub until: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimingDecisionRecord {
    pub desk_id: DeskId,
    pub role_id: String,
    pub trigger_type: String,
    pub timestamp: String,
    pub allowed: bool,
    pub decision: TimingFsmDecision,
    pub reason: String,
    pub stale_after_seconds: u64,
    pub evidence_timestamp: Option<String>,
    pub child_node_id: Option<String>,
    pub replay_safe: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimingCheckRequest {
    pub desk_id: DeskId,
    pub role_id: Option<String>,
    pub child_node_id: Option<String>,
    pub trigger: Option<TimingTrigger>,
    pub evidence_timestamp: Option<DateTime<Utc>>,
    pub proposal_requested: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimingNextSummary {
    pub desk_id: DeskId,
    pub role_id: String,
    pub next_allowed_trigger: String,
    pub stale_after_seconds: u64,
    pub min_refresh_seconds: u64,
    pub tablet_safe: bool,
}

#[allow(dead_code)]
struct TimingPaths {
    policies: PathBuf,
    events: PathBuf,
    cooldowns: PathBuf,
    stale_rejections: PathBuf,
}

impl TimingPaths {
    fn new(cfg: &Config) -> Self {
        let dir = cfg.workspace_dir.join("state/timing");
        Self {
            policies: dir.join("policies.json"),
            events: dir.join("events.jsonl"),
            cooldowns: dir.join("cooldowns.json"),
            stale_rejections: dir.join("stale-rejections.jsonl"),
        }
    }
}

pub fn default_policies() -> Vec<DeskTimingPolicy> {
    vec![
        DeskTimingPolicy {
            desk_id: desk_id("forex"),
            role_id: "forex_calendar_watcher".to_string(),
            allowed_triggers: vec![
                cron("0 0 * * *", "daily swap review"),
                cron("0 22 * * 1-5", "rollover window"),
                TimingTrigger::Polling {
                    interval_seconds: 60,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::ForexTokyo,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::ForexLondon,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::ForexNewYork,
                },
                TimingTrigger::EventWindow {
                    event_type: "high_impact_currency_news".to_string(),
                    before_seconds: 1800,
                    after_seconds: 1800,
                },
                TimingTrigger::Heartbeat {
                    interval_seconds: TABLET_HEARTBEAT_SECONDS,
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 900,
                },
            ],
            stale_after_seconds: 3600,
            min_refresh_seconds: 60,
            max_refresh_seconds: 3600,
            cooldown_after_proposal_seconds: 900,
            tablet_safe: true,
        },
        DeskTimingPolicy {
            desk_id: desk_id("crypto"),
            role_id: "bitcoin_dca_monitor".to_string(),
            allowed_triggers: vec![
                cron("0 12 * * *", "daily DCA check"),
                cron("0 12 * * 0", "weekly accumulation review"),
                TimingTrigger::Polling {
                    interval_seconds: 300,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::CryptoAlwaysOn,
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 3600,
                },
            ],
            stale_after_seconds: 900,
            min_refresh_seconds: 300,
            max_refresh_seconds: 86_400,
            cooldown_after_proposal_seconds: 3600,
            tablet_safe: false,
        },
        DeskTimingPolicy {
            desk_id: desk_id("crypto"),
            role_id: "stablecoin_peg_watcher".to_string(),
            allowed_triggers: vec![
                TimingTrigger::Polling {
                    interval_seconds: 120,
                },
                TimingTrigger::Heartbeat {
                    interval_seconds: TABLET_HEARTBEAT_SECONDS,
                },
                TimingTrigger::EventWindow {
                    event_type: "depeg_or_exchange_outage".to_string(),
                    before_seconds: 0,
                    after_seconds: 7200,
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 1800,
                },
            ],
            stale_after_seconds: 120,
            min_refresh_seconds: 120,
            max_refresh_seconds: 1800,
            cooldown_after_proposal_seconds: 1800,
            tablet_safe: true,
        },
        DeskTimingPolicy {
            desk_id: desk_id("stocks_options"),
            role_id: "stock_index_session_watcher".to_string(),
            allowed_triggers: vec![
                cron("30 8 * * 1-5", "premarket scan"),
                cron("30 9 * * 1-5", "market open scan"),
                cron("0 12 * * 1-5", "midday check"),
                cron("0 15 * * 1-5", "power hour check"),
                TimingTrigger::Polling {
                    interval_seconds: 120,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::UsEquitiesPremarket,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::UsEquitiesRegular,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::UsEquitiesAfterHours,
                },
                TimingTrigger::EventWindow {
                    event_type: "macro_or_earnings".to_string(),
                    before_seconds: 1800,
                    after_seconds: 1800,
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 1200,
                },
            ],
            stale_after_seconds: 120,
            min_refresh_seconds: 120,
            max_refresh_seconds: 3600,
            cooldown_after_proposal_seconds: 1200,
            tablet_safe: false,
        },
        DeskTimingPolicy {
            desk_id: desk_id("sports"),
            role_id: "sports_scout".to_string(),
            allowed_triggers: vec![
                cron("0 9 * * *", "daily slate scan"),
                cron("0 10 * * 1", "weekly major event scan"),
                TimingTrigger::Polling {
                    interval_seconds: 300,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::SportsPregame,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::SportsLive,
                },
                TimingTrigger::SessionWindow {
                    session: MarketSession::SportsPostgame,
                },
                TimingTrigger::EventWindow {
                    event_type: "pregame_lineup_or_injury".to_string(),
                    before_seconds: 7200,
                    after_seconds: 0,
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 1800,
                },
            ],
            stale_after_seconds: 300,
            min_refresh_seconds: 300,
            max_refresh_seconds: 86_400,
            cooldown_after_proposal_seconds: 1800,
            tablet_safe: true,
        },
        DeskTimingPolicy {
            desk_id: desk_id("prediction_markets"),
            role_id: "prediction_market_watcher".to_string(),
            allowed_triggers: vec![
                TimingTrigger::Polling {
                    interval_seconds: 300,
                },
                TimingTrigger::Mtime {
                    path: "workspace/state/evidence".to_string(),
                },
                TimingTrigger::EventWindow {
                    event_type: "debate_election_court_earnings_or_settlement".to_string(),
                    before_seconds: 3600,
                    after_seconds: 3600,
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 1800,
                },
            ],
            stale_after_seconds: 600,
            min_refresh_seconds: 300,
            max_refresh_seconds: 86_400,
            cooldown_after_proposal_seconds: 1800,
            tablet_safe: false,
        },
        DeskTimingPolicy {
            desk_id: desk_id("research"),
            role_id: "generic_evidence_collector".to_string(),
            allowed_triggers: vec![
                TimingTrigger::Polling {
                    interval_seconds: 60,
                },
                TimingTrigger::Heartbeat {
                    interval_seconds: TABLET_HEARTBEAT_SECONDS,
                },
                TimingTrigger::Mtime {
                    path: "workspace/state/evidence".to_string(),
                },
                TimingTrigger::Cooldown {
                    duration_seconds: 900,
                },
            ],
            stale_after_seconds: 600,
            min_refresh_seconds: 60,
            max_refresh_seconds: 3600,
            cooldown_after_proposal_seconds: 900,
            tablet_safe: true,
        },
    ]
}

pub fn load_policies(cfg: &Config) -> Result<Vec<DeskTimingPolicy>> {
    let paths = TimingPaths::new(cfg);
    if paths.policies.exists() {
        let raw = fs::read_to_string(&paths.policies)
            .with_context(|| format!("failed to read {}", paths.policies.display()))?;
        serde_json::from_str(&raw).context("failed to parse timing policies")
    } else {
        Ok(default_policies())
    }
}

#[cfg(test)]
pub fn write_default_policy_fixture(cfg: &Config) -> Result<()> {
    let paths = TimingPaths::new(cfg);
    if let Some(parent) = paths.policies.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        &paths.policies,
        serde_json::to_string_pretty(&default_policies())?,
    )?;
    if !paths.cooldowns.exists() {
        fs::write(&paths.cooldowns, "[]")?;
    }
    Ok(())
}

pub fn cooldowns(cfg: &Config) -> Result<Vec<TimingCooldown>> {
    let paths = TimingPaths::new(cfg);
    if !paths.cooldowns.exists() {
        return Ok(vec![]);
    }
    serde_json::from_str(&fs::read_to_string(&paths.cooldowns)?)
        .context("failed to parse timing cooldowns")
}

pub fn policies_for_desk(cfg: &Config, desk: &DeskId) -> Result<Vec<DeskTimingPolicy>> {
    Ok(load_policies(cfg)?
        .into_iter()
        .filter(|policy| &policy.desk_id == desk)
        .collect())
}

pub fn next_summary(cfg: &Config, desk: &DeskId) -> Result<Vec<TimingNextSummary>> {
    Ok(policies_for_desk(cfg, desk)?
        .into_iter()
        .map(|policy| TimingNextSummary {
            desk_id: policy.desk_id.clone(),
            role_id: policy.role_id.clone(),
            next_allowed_trigger: policy
                .allowed_triggers
                .first()
                .map(ToString::to_string)
                .unwrap_or_else(|| "none".to_string()),
            stale_after_seconds: policy.stale_after_seconds,
            min_refresh_seconds: policy.min_refresh_seconds,
            tablet_safe: policy.tablet_safe,
        })
        .collect())
}

pub fn check_timing(cfg: &Config, request: TimingCheckRequest) -> Result<TimingDecisionRecord> {
    let policies = load_policies(cfg)?;
    let policy = select_policy(&policies, &request)?;
    let active_cooldowns = cooldowns(cfg)?;
    evaluate_policy(cfg, policy, &active_cooldowns, request)
}

pub fn evaluate_policy(
    cfg: &Config,
    policy: &DeskTimingPolicy,
    active_cooldowns: &[TimingCooldown],
    request: TimingCheckRequest,
) -> Result<TimingDecisionRecord> {
    validate_policy(cfg, policy)?;
    if request
        .child_node_id
        .as_deref()
        .is_some_and(|node| node.contains("tablet"))
    {
        validate_tablet_policy(policy)?;
    }
    let trigger = request
        .trigger
        .clone()
        .or_else(|| policy.allowed_triggers.first().cloned())
        .ok_or_else(|| anyhow!("timing policy has no allowed triggers"))?;
    validate_trigger(cfg, policy, &trigger)?;
    let trigger_allowed = policy
        .allowed_triggers
        .iter()
        .any(|allowed| same_trigger_kind(allowed, &trigger));
    if !trigger_allowed {
        return Ok(record(
            policy,
            &request,
            &trigger,
            TimingFsmDecision::WaitTiming,
            "trigger is not allowed for role",
        ));
    }
    if let Some(evidence_ts) = request.evidence_timestamp {
        let age = Utc::now() - evidence_ts;
        if age > Duration::seconds(policy.stale_after_seconds as i64) {
            return Ok(record(
                policy,
                &request,
                &trigger,
                TimingFsmDecision::RejectStaleEvidence,
                "required evidence is stale",
            ));
        }
    }
    if active_cooldowns.iter().any(|cooldown| {
        cooldown.desk_id == policy.desk_id
            && cooldown.role_id == policy.role_id
            && parse_ts(&cooldown.until).is_ok_and(|until| until > Utc::now())
    }) {
        return Ok(record(
            policy,
            &request,
            &trigger,
            TimingFsmDecision::RejectCooldown,
            "desk role is in cooldown",
        ));
    }
    if request.proposal_requested && matches!(trigger, TimingTrigger::EventWindow { .. }) {
        return Ok(record(
            policy,
            &request,
            &trigger,
            TimingFsmDecision::WatchOnlyEventWindow,
            "event window permits observation only",
        ));
    }
    Ok(record(
        policy,
        &request,
        &trigger,
        TimingFsmDecision::AllowEvaluation,
        "timing policy allows evidence evaluation",
    ))
}

#[allow(dead_code)]
pub fn record_timing_decision(cfg: &Config, decision: &TimingDecisionRecord) -> Result<()> {
    let paths = TimingPaths::new(cfg);
    if let Some(parent) = paths.events.parent() {
        fs::create_dir_all(parent)?;
    }
    append_json_line(&paths.events, decision)?;
    if decision.decision == TimingFsmDecision::RejectStaleEvidence {
        append_json_line(&paths.stale_rejections, decision)?;
    }
    Ok(())
}

pub fn render_policy_list(policies: &[DeskTimingPolicy]) -> String {
    let mut out = String::from("timing policies\n");
    for policy in policies {
        out.push_str(&format!(
            "{} role={} triggers={} stale_after={}s min_refresh={}s tablet_safe={}\n",
            policy.desk_id,
            policy.role_id,
            policy.allowed_triggers.len(),
            policy.stale_after_seconds,
            policy.min_refresh_seconds,
            policy.tablet_safe
        ));
    }
    out
}

pub fn render_decision(decision: &TimingDecisionRecord) -> String {
    format!(
        "timing check\ndesk: {}\nrole: {}\ndecision: {}\nallowed: {}\nreason: {}\nreplay_safe: {}\n",
        decision.desk_id,
        decision.role_id,
        decision.decision,
        decision.allowed,
        decision.reason,
        decision.replay_safe
    )
}

pub fn render_next(summaries: &[TimingNextSummary]) -> String {
    let mut out = String::from("timing next\n");
    for summary in summaries {
        out.push_str(&format!(
            "{} role={} next={} stale_after={}s min_refresh={}s tablet_safe={}\n",
            summary.desk_id,
            summary.role_id,
            summary.next_allowed_trigger,
            summary.stale_after_seconds,
            summary.min_refresh_seconds,
            summary.tablet_safe
        ));
    }
    out
}

pub fn render_cooldowns(cooldowns: &[TimingCooldown]) -> String {
    if cooldowns.is_empty() {
        return "timing cooldowns\nnone\n".to_string();
    }
    let mut out = String::from("timing cooldowns\n");
    for cooldown in cooldowns {
        out.push_str(&format!(
            "{} role={} until={} reason={}\n",
            cooldown.desk_id, cooldown.role_id, cooldown.until, cooldown.reason
        ));
    }
    out
}

pub fn validate_cron_expression(expression: &str) -> Result<()> {
    let fields: Vec<_> = expression.split_whitespace().collect();
    if !(fields.len() == 5 || fields.len() == 6) {
        return Err(anyhow!("cron expression must have 5 or 6 fields"));
    }
    if fields.iter().any(|field| field.trim().is_empty()) {
        return Err(anyhow!("cron expression contains empty field"));
    }
    Ok(())
}

pub fn validate_trigger(
    cfg: &Config,
    policy: &DeskTimingPolicy,
    trigger: &TimingTrigger,
) -> Result<()> {
    match trigger {
        TimingTrigger::Cron { expression } => validate_cron_expression(expression),
        TimingTrigger::Polling { interval_seconds } => {
            if *interval_seconds < GENERAL_MIN_POLL_SECONDS {
                return Err(anyhow!("polling interval must not be sub-second or zero"));
            }
            if policy.tablet_safe && *interval_seconds < TABLET_MIN_POLL_SECONDS {
                return Err(anyhow!(
                    "tablet-safe polling interval must be at least {TABLET_MIN_POLL_SECONDS}s"
                ));
            }
            if *interval_seconds < policy.min_refresh_seconds {
                return Err(anyhow!(
                    "polling interval is below role min_refresh_seconds"
                ));
            }
            Ok(())
        }
        TimingTrigger::Mtime { path } => validate_mtime_path(&cfg.workspace_dir, Path::new(path)),
        TimingTrigger::Heartbeat { interval_seconds } => {
            if policy.tablet_safe && *interval_seconds < TABLET_HEARTBEAT_SECONDS {
                return Err(anyhow!(
                    "tablet heartbeat interval must be at least {TABLET_HEARTBEAT_SECONDS}s"
                ));
            }
            Ok(())
        }
        TimingTrigger::SessionWindow { .. }
        | TimingTrigger::EventWindow { .. }
        | TimingTrigger::Cooldown { .. } => Ok(()),
    }
}

pub fn validate_mtime_path(workspace_dir: &Path, path: &Path) -> Result<()> {
    if path
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        return Err(anyhow!("mtime path cannot contain parent traversal"));
    }
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace_dir.join(path)
    };
    if !candidate.starts_with(workspace_dir) {
        return Err(anyhow!("mtime path must stay inside workspace"));
    }
    let lowered = candidate.to_string_lossy().to_ascii_lowercase();
    if lowered.contains("secret") || lowered.contains(".ssh") || lowered.contains("key") {
        return Err(anyhow!("mtime path cannot watch secrets or keys"));
    }
    Ok(())
}

#[cfg(test)]
pub fn has_session(policy: &DeskTimingPolicy, session: MarketSession) -> bool {
    policy.allowed_triggers.iter().any(|trigger| {
        matches!(
            trigger,
            TimingTrigger::SessionWindow { session: found } if *found == session
        )
    })
}

#[cfg(test)]
pub fn has_event_window(policy: &DeskTimingPolicy, event_type: &str) -> bool {
    policy.allowed_triggers.iter().any(|trigger| {
        matches!(
            trigger,
            TimingTrigger::EventWindow { event_type: found, .. } if found == event_type
        )
    })
}

#[cfg(test)]
pub fn validate_child_policy_override(
    assigned: &DeskTimingPolicy,
    proposed: &DeskTimingPolicy,
) -> Result<()> {
    if assigned != proposed {
        return Err(anyhow!("child nodes cannot override timing policy"));
    }
    Ok(())
}

fn validate_policy(cfg: &Config, policy: &DeskTimingPolicy) -> Result<()> {
    if policy.min_refresh_seconds == 0 {
        return Err(anyhow!("min_refresh_seconds must be positive"));
    }
    if policy.max_refresh_seconds < policy.min_refresh_seconds {
        return Err(anyhow!(
            "max_refresh_seconds must be >= min_refresh_seconds"
        ));
    }
    for trigger in &policy.allowed_triggers {
        validate_trigger(cfg, policy, trigger)?;
    }
    Ok(())
}

fn validate_tablet_policy(policy: &DeskTimingPolicy) -> Result<()> {
    if !policy.tablet_safe {
        return Err(anyhow!("role timing policy is not tablet-safe"));
    }
    if policy.min_refresh_seconds < TABLET_MIN_POLL_SECONDS {
        return Err(anyhow!("tablet polling minimum is not enforced"));
    }
    if policy.max_refresh_seconds < policy.min_refresh_seconds {
        return Err(anyhow!("tablet policy refresh range is invalid"));
    }
    if policy
        .allowed_triggers
        .iter()
        .filter_map(|trigger| match trigger {
            TimingTrigger::Heartbeat { interval_seconds } => Some(*interval_seconds),
            _ => None,
        })
        .any(|interval| interval < TABLET_HEARTBEAT_SECONDS)
    {
        return Err(anyhow!("tablet heartbeat interval is too aggressive"));
    }
    let _lease = TABLET_ROLE_LEASE_SECONDS;
    Ok(())
}

fn select_policy<'a>(
    policies: &'a [DeskTimingPolicy],
    request: &TimingCheckRequest,
) -> Result<&'a DeskTimingPolicy> {
    policies
        .iter()
        .find(|policy| {
            policy.desk_id == request.desk_id
                && request
                    .role_id
                    .as_ref()
                    .is_none_or(|role| role == &policy.role_id)
        })
        .ok_or_else(|| anyhow!("no timing policy found for desk '{}'", request.desk_id))
}

fn record(
    policy: &DeskTimingPolicy,
    request: &TimingCheckRequest,
    trigger: &TimingTrigger,
    decision: TimingFsmDecision,
    reason: &str,
) -> TimingDecisionRecord {
    TimingDecisionRecord {
        desk_id: policy.desk_id.clone(),
        role_id: policy.role_id.clone(),
        trigger_type: trigger.kind().to_string(),
        timestamp: Utc::now().to_rfc3339(),
        allowed: decision == TimingFsmDecision::AllowEvaluation,
        decision,
        reason: reason.to_string(),
        stale_after_seconds: policy.stale_after_seconds,
        evidence_timestamp: request.evidence_timestamp.map(|ts| ts.to_rfc3339()),
        child_node_id: request.child_node_id.clone(),
        replay_safe: true,
    }
}

fn same_trigger_kind(left: &TimingTrigger, right: &TimingTrigger) -> bool {
    left.kind() == right.kind()
}

#[allow(dead_code)]
fn append_json_line<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", serde_json::to_string(value)?)?;
    Ok(())
}

fn parse_ts(ts: &str) -> Result<DateTime<Utc>> {
    Ok(DateTime::parse_from_rfc3339(ts)?.with_timezone(&Utc))
}

fn cron(expression: &str, _label: &str) -> TimingTrigger {
    TimingTrigger::Cron {
        expression: expression.to_string(),
    }
}

fn desk_id(value: &str) -> DeskId {
    DeskId::new(value)
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
        (tmp, cfg)
    }

    fn policy(desk: &str, role: &str) -> DeskTimingPolicy {
        default_policies()
            .into_iter()
            .find(|policy| policy.desk_id == DeskId::new(desk) && policy.role_id == role)
            .expect("policy")
    }

    #[test]
    fn timing_policy_loads() {
        let (_tmp, cfg) = test_config();
        write_default_policy_fixture(&cfg).expect("write fixture");
        let policies = load_policies(&cfg).expect("load");
        assert!(
            policies
                .iter()
                .any(|policy| policy.desk_id == DeskId::new("forex"))
        );
        assert!(
            policies
                .iter()
                .any(|policy| policy.role_id == "sports_scout")
        );
    }

    #[test]
    fn cron_trigger_parses() {
        validate_cron_expression("0 22 * * 1-5").expect("valid cron");
        assert!(validate_cron_expression("not enough").is_err());
    }

    #[test]
    fn polling_rejects_too_fast_interval() {
        let (_tmp, cfg) = test_config();
        let forex = policy("forex", "forex_calendar_watcher");
        let err = validate_trigger(
            &cfg,
            &forex,
            &TimingTrigger::Polling {
                interval_seconds: 30,
            },
        )
        .expect_err("too fast");
        assert!(err.to_string().contains("tablet-safe polling"));
    }

    #[test]
    fn mtime_rejects_path_outside_workspace() {
        let (_tmp, cfg) = test_config();
        assert!(validate_mtime_path(&cfg.workspace_dir, Path::new("../secret.csv")).is_err());
        assert!(validate_mtime_path(&cfg.workspace_dir, Path::new("/etc/passwd")).is_err());
    }

    #[test]
    fn stale_evidence_rejected() {
        let (_tmp, cfg) = test_config();
        let forex = policy("forex", "forex_calendar_watcher");
        let request = TimingCheckRequest {
            desk_id: DeskId::new("forex"),
            role_id: Some("forex_calendar_watcher".to_string()),
            child_node_id: None,
            trigger: Some(TimingTrigger::Polling {
                interval_seconds: 60,
            }),
            evidence_timestamp: Some(Utc::now() - Duration::seconds(7200)),
            proposal_requested: false,
        };
        let decision = evaluate_policy(&cfg, &forex, &[], request).expect("decision");
        assert_eq!(decision.decision, TimingFsmDecision::RejectStaleEvidence);
    }

    #[test]
    fn cooldown_blocks_proposal() {
        let (_tmp, cfg) = test_config();
        let forex = policy("forex", "forex_calendar_watcher");
        let cooldown = TimingCooldown {
            desk_id: DeskId::new("forex"),
            role_id: "forex_calendar_watcher".to_string(),
            until: (Utc::now() + Duration::minutes(10)).to_rfc3339(),
            reason: "failed proposal".to_string(),
        };
        let request = TimingCheckRequest {
            desk_id: DeskId::new("forex"),
            role_id: Some("forex_calendar_watcher".to_string()),
            child_node_id: None,
            trigger: Some(TimingTrigger::Polling {
                interval_seconds: 60,
            }),
            evidence_timestamp: Some(Utc::now()),
            proposal_requested: true,
        };
        let decision = evaluate_policy(&cfg, &forex, &[cooldown], request).expect("decision");
        assert_eq!(decision.decision, TimingFsmDecision::RejectCooldown);
    }

    #[test]
    fn event_window_can_force_watch_only() {
        let (_tmp, cfg) = test_config();
        let forex = policy("forex", "forex_calendar_watcher");
        let request = TimingCheckRequest {
            desk_id: DeskId::new("forex"),
            role_id: Some("forex_calendar_watcher".to_string()),
            child_node_id: None,
            trigger: Some(TimingTrigger::EventWindow {
                event_type: "high_impact_currency_news".to_string(),
                before_seconds: 1800,
                after_seconds: 1800,
            }),
            evidence_timestamp: Some(Utc::now()),
            proposal_requested: true,
        };
        let decision = evaluate_policy(&cfg, &forex, &[], request).expect("decision");
        assert_eq!(decision.decision, TimingFsmDecision::WatchOnlyEventWindow);
    }

    #[test]
    fn forex_rollover_window_detected() {
        let forex = policy("forex", "forex_calendar_watcher");
        assert!(forex.allowed_triggers.iter().any(|trigger| {
            matches!(
                trigger,
                TimingTrigger::Cron { expression } if expression == "0 22 * * 1-5"
            )
        }));
        assert!(has_event_window(&forex, "high_impact_currency_news"));
    }

    #[test]
    fn sports_pregame_window_detected() {
        let sports = policy("sports", "sports_scout");
        assert!(has_session(&sports, MarketSession::SportsPregame));
        assert!(has_event_window(&sports, "pregame_lineup_or_injury"));
    }

    #[test]
    fn tablet_polling_minimum_enforced() {
        let (_tmp, cfg) = test_config();
        let mut forex = policy("forex", "forex_calendar_watcher");
        forex.min_refresh_seconds = 30;
        forex.allowed_triggers = vec![TimingTrigger::Polling {
            interval_seconds: 30,
        }];
        assert!(validate_policy(&cfg, &forex).is_err());
    }

    #[test]
    fn child_cannot_override_timing_policy() {
        let assigned = policy("forex", "forex_calendar_watcher");
        let mut proposed = assigned.clone();
        proposed.cooldown_after_proposal_seconds = 1;
        let err =
            validate_child_policy_override(&assigned, &proposed).expect_err("override rejected");
        assert!(err.to_string().contains("cannot override"));
    }

    #[test]
    fn timing_decision_record_writes_audit_paths() {
        let (_tmp, cfg) = test_config();
        let forex = policy("forex", "forex_calendar_watcher");
        let request = TimingCheckRequest {
            desk_id: DeskId::new("forex"),
            role_id: Some("forex_calendar_watcher".to_string()),
            child_node_id: Some("node:tablet-01".to_string()),
            trigger: Some(TimingTrigger::Polling {
                interval_seconds: 60,
            }),
            evidence_timestamp: Some(Utc::now()),
            proposal_requested: false,
        };
        let allowed = evaluate_policy(&cfg, &forex, &[], request).expect("allowed");
        record_timing_decision(&cfg, &allowed).expect("record allowed");

        let stale_request = TimingCheckRequest {
            desk_id: DeskId::new("forex"),
            role_id: Some("forex_calendar_watcher".to_string()),
            child_node_id: Some("node:tablet-01".to_string()),
            trigger: Some(TimingTrigger::Polling {
                interval_seconds: 60,
            }),
            evidence_timestamp: Some(Utc::now() - Duration::hours(2)),
            proposal_requested: false,
        };
        let stale = evaluate_policy(&cfg, &forex, &[], stale_request).expect("stale");
        record_timing_decision(&cfg, &stale).expect("record stale");

        let paths = TimingPaths::new(&cfg);
        assert!(paths.events.exists());
        assert!(paths.stale_rejections.exists());
    }
}
