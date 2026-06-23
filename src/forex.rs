use crate::config::Config;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use redb::{Database, ReadableDatabase, TableDefinition};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use tokio::process::Command;

// v2 table uses postcard encoding for values.
const FOREX_STATE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("forex_state_v2");
const DESK: &str = "forex";
const SOURCE_VENUE: &str = "forex_com";
const SOURCE_CHANNEL: &str = "live_prices_stream";
const EXECUTION_ADAPTER: &str = "stonex_forex_exec";
const ACCOUNT_SCOPE: &str = "paper_primary";
const MARKET_TYPE: &str = "spot_fx";
const STALE_THRESHOLD_MS: i64 = 5_000;
const MACRO_BLACKOUT_MINUTES: i64 = 30;
const SPREAD_ELEVATED_BPS: f64 = 8.0;
const SPREAD_BLOCK_BPS: f64 = 25.0;
const MACRO_HIGH_PRE_BLACKOUT_MINUTES: i64 = 30;
const MACRO_HIGH_POST_BLACKOUT_MINUTES: i64 = 15;
const MACRO_MEDIUM_PRE_WINDOW_MINUTES: i64 = 30;
const MACRO_UPCOMING_LIMIT: usize = 12;
const MQL5_CALENDAR_URL: &str = "https://www.mql5.com/en/economic-calendar/content";
const MQL5_IMPORTANCE_MEDIUM_HIGH_MASK: u32 = 12; // 4 (medium) + 8 (high)
const MQL5_FOREX_CURRENCY_MASK: u32 = 131_199; // AUD,CAD,CHF,EUR,GBP,JPY,NOK,USD
pub const FOREX_ALLOWED_PAIRS: &[&str] = &[
    "AUDCHF", "AUDJPY", "EURAUD", "EURCAD", "EURGBP", "EURNOK", "EURUSD", "GBPJPY", "USDCHF",
    "USDJPY",
];
pub const FOREX_LONG_BIAS_PAIRS: &[&str] = &["AUDCHF", "AUDJPY", "GBPJPY", "USDCHF", "USDJPY"];
pub const FOREX_SHORT_BIAS_PAIRS: &[&str] = &["EURAUD", "EURGBP", "EURNOK", "EURUSD"];
pub const FOREX_CARRY_BLOCKED_PAIRS: &[&str] = &["EURCAD"];
const SWAP_HEALTH_SOURCE_DEFAULT: &str = "stonex_swap_health";
const MACRO_SOURCE_MQL5: &str = "mql5_calendar";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PairPolicy {
    LongOnly,
    ShortOnly,
    CarryBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteTick {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    pub event_ts_ms: i64,
    pub recv_ts_ms: i64,
    pub processed_ts_ms: i64,
    pub freshness_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CarryBias {
    #[serde(rename = "LongFavorable")]
    LongFavorable,
    #[serde(rename = "ShortFavorable")]
    ShortFavorable,
    #[serde(rename = "Neutral")]
    Neutral,
    #[serde(rename = "Unknown")]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum Trend {
    Up,
    Down,
    Sideways,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "PascalCase")]
pub enum FreshnessQuality {
    Hot,
    Warm,
    Cold,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SignalType {
    NoTrade,
    Hold,
    LongBiasHold,
    ShortBiasHold,
    LongBiasEntryCandidate,
    ShortBiasEntryCandidate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ForexPayload {
    pub bid: f64,
    pub ask: f64,
    pub spread_bps: f64,
    pub carry_bias: CarryBias,
    pub swap_long: f64,
    pub swap_short: f64,
    pub trend_m15: Trend,
    pub trend_h1: Trend,
    pub trend_h4: Trend,
    pub ma20: f64,
    pub ma50: f64,
    pub next_high_impact_minutes: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingMetadata {
    pub desk: String,
    pub source_venue: String,
    pub source_channel: String,
    pub execution_adapter: String,
    pub account_scope: String,
    pub market_type: String,
    pub instrument_id: String,
    pub symbol: String,
    pub paper_trade_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FreshnessMetadata {
    pub event_ts_ms: i64,
    pub recv_ts_ms: i64,
    pub processed_ts_ms: i64,
    pub freshness_ms: i64,
    pub stale: bool,
    pub quality: FreshnessQuality,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SignalMetadata {
    pub signal_type: SignalType,
    pub confidence: f32,
    pub thesis: String,
    pub recommended_action: SignalType,
    pub requires_review: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetadata {
    pub risk_flags: Vec<String>,
    pub blockers: Vec<String>,
    pub risk_score: f32,
    pub exposure_ok: bool,
    pub policy_ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSignal {
    pub signal_id: String,
    pub routing: RoutingMetadata,
    pub freshness: FreshnessMetadata,
    pub signal: SignalMetadata,
    pub risk: RiskMetadata,
    pub evidence: Vec<String>,
    pub desk_payload: ForexPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeskHandoff {
    pub signal_id: String,
    pub desk: String,
    pub source_venue: String,
    pub execution_adapter: String,
    pub account_scope: String,
    pub symbol: String,
    pub thesis: String,
    pub evidence: Vec<String>,
    pub risk_flags: Vec<String>,
    pub confidence: f32,
    pub recommended_action: SignalType,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IngestResult {
    pub quote_tick: QuoteTick,
    pub shared_signal: SharedSignal,
    pub handoff: DeskHandoff,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoneXQuotePayload {
    pub symbol: String,
    pub bid: f64,
    pub ask: f64,
    #[serde(default)]
    pub event_ts_ms: Option<i64>,
    #[serde(default)]
    pub event_timestamp_ms: Option<i64>,
    #[serde(default)]
    pub ts_ms: Option<i64>,
    #[serde(default)]
    pub instrument_id: Option<String>,
    #[serde(default)]
    pub swap_long: Option<f64>,
    #[serde(default)]
    pub swap_short: Option<f64>,
    #[serde(default)]
    pub trend_m15: Option<String>,
    #[serde(default)]
    pub trend_h1: Option<String>,
    #[serde(default)]
    pub trend_h4: Option<String>,
    #[serde(default)]
    pub ma20: Option<f64>,
    #[serde(default)]
    pub ma50: Option<f64>,
    #[serde(default)]
    pub next_high_impact_minutes: Option<i64>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SwapDirectionPolicy {
    LongOnly,
    ShortOnly,
    CarryBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHealthPairInput {
    pub symbol: String,
    pub swap_long: f64,
    pub swap_short: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHealthInput {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub as_of_ms: Option<i64>,
    pub pairs: Vec<SwapHealthPairInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHealthRecord {
    pub symbol: String,
    pub swap_long: f64,
    pub swap_short: f64,
    pub observed_carry_bias: CarryBias,
    pub effective_policy: SwapDirectionPolicy,
    pub source: String,
    pub as_of_ms: i64,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SwapHealthSummary {
    pub source: String,
    pub as_of_ms: i64,
    pub processed: usize,
    pub long_only: usize,
    pub short_only: usize,
    pub carry_blocked: usize,
    pub skipped_symbols: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MacroImpact {
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroEvent {
    pub event_id: String,
    pub source: String,
    pub currency: String,
    pub impact: MacroImpact,
    pub title: String,
    pub scheduled_at_ms: i64,
    pub actual: Option<String>,
    pub forecast: Option<String>,
    pub previous: Option<String>,
    pub affected_pairs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PairMacroState {
    pub pair: String,
    pub next_medium_event_minutes: Option<u32>,
    pub next_high_event_minutes: Option<u32>,
    pub macro_blackout: bool,
    pub upcoming_events: Vec<MacroEvent>,
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MacroRefreshSummary {
    pub source: String,
    pub from_ms: i64,
    pub to_ms: i64,
    pub events_written: usize,
    pub pairs_written: usize,
    pub blackout_pairs: usize,
}

#[derive(Debug, Deserialize)]
struct Mql5EventRow {
    #[serde(rename = "Id")]
    id: i64,
    #[serde(rename = "EventName")]
    event_name: String,
    #[serde(rename = "Importance")]
    importance: String,
    #[serde(rename = "CurrencyCode")]
    currency_code: String,
    #[serde(rename = "ReleaseDate")]
    release_date: i64,
    #[serde(rename = "ActualValue", default)]
    actual_value: String,
    #[serde(rename = "ForecastValue", default)]
    forecast_value: String,
    #[serde(rename = "PreviousValue", default)]
    previous_value: String,
}

pub fn preflight(cfg: &Config) -> Result<()> {
    let _ = open_db(cfg)?;
    Ok(())
}

pub fn key_latest_signal(symbol: &str) -> String {
    format!("latest_signal:{}:{}", DESK, normalize_symbol(symbol))
}

pub fn key_handoff(symbol: &str) -> String {
    format!("handoff:{}:{}", DESK, normalize_symbol(symbol))
}

pub fn key_swap_health(symbol: &str) -> String {
    format!("swap_health:{}:{}", DESK, normalize_symbol(symbol))
}

pub fn key_macro_event(event_id: &str) -> String {
    format!("macro_event:{}", event_id)
}

pub fn key_pair_macro(symbol: &str) -> String {
    format!("pair_macro:{}:{}", DESK, normalize_symbol(symbol))
}

pub fn put_latest_signal(cfg: &Config, symbol: &str, signal: &SharedSignal) -> Result<()> {
    put_value(cfg, &key_latest_signal(symbol), signal)
}

pub fn get_latest_signal(cfg: &Config, symbol: &str) -> Result<Option<SharedSignal>> {
    get_value(cfg, &key_latest_signal(symbol))
}

pub fn put_handoff(cfg: &Config, symbol: &str, handoff: &DeskHandoff) -> Result<()> {
    put_value(cfg, &key_handoff(symbol), handoff)
}

pub fn get_handoff(cfg: &Config, symbol: &str) -> Result<Option<DeskHandoff>> {
    get_value(cfg, &key_handoff(symbol))
}

pub fn put_swap_health(cfg: &Config, symbol: &str, record: &SwapHealthRecord) -> Result<()> {
    put_value(cfg, &key_swap_health(symbol), record)
}

pub fn get_swap_health(cfg: &Config, symbol: &str) -> Result<Option<SwapHealthRecord>> {
    get_value(cfg, &key_swap_health(symbol))
}

pub fn get_pair_macro_state(cfg: &Config, symbol: &str) -> Result<Option<PairMacroState>> {
    get_value(cfg, &key_pair_macro(symbol))
}

pub async fn refresh_mql5_macro(cfg: &Config, hours_ahead: i64) -> Result<MacroRefreshSummary> {
    let now = now_ms();
    let safe_hours = hours_ahead.clamp(1, 24 * 14);
    let from_ms = now;
    let to_ms = now.saturating_add(safe_hours.saturating_mul(60 * 60 * 1000));
    let from = iso_utc_seconds(from_ms);
    let to = iso_utc_seconds(to_ms);
    let importance = MQL5_IMPORTANCE_MEDIUM_HIGH_MASK.to_string();
    let currencies = MQL5_FOREX_CURRENCY_MASK.to_string();

    let raw = match fetch_mql5_calendar_reqwest(&from, &to, &importance, &currencies).await {
        Ok(body) => body,
        Err(primary_err) => fetch_mql5_calendar_curl(&from, &to, &importance, &currencies)
            .await
            .with_context(|| format!("reqwest failed first: {primary_err}"))?,
    };
    let rows: Vec<Mql5EventRow> =
        serde_json::from_str(&raw).context("invalid JSON from MQL5 calendar endpoint")?;
    let events = normalize_mql5_rows(rows);
    apply_macro_events(cfg, &events, now, from_ms, to_ms, MACRO_SOURCE_MQL5)
}

async fn fetch_mql5_calendar_reqwest(
    from: &str,
    to: &str,
    importance: &str,
    currencies: &str,
) -> Result<String> {
    let client = reqwest::Client::builder()
        .http1_only()
        .build()
        .context("failed to build HTTP client for MQL5")?;
    let response = client
        .post(MQL5_CALENDAR_URL)
        .header("User-Agent", "Quant-M/0.1")
        .header("Accept", "*/*")
        .header(
            "Content-Type",
            "application/x-www-form-urlencoded; charset=UTF-8",
        )
        .header("Origin", "https://www.mql5.com")
        .header("X-Requested-With", "XMLHttpRequest")
        .header("Referer", "https://www.mql5.com/en/economic-calendar")
        .form(&[
            ("date_mode", "0"),
            ("from", from),
            ("to", to),
            ("importance", importance),
            ("currencies", currencies),
        ])
        .send()
        .await
        .context("failed calling MQL5 calendar endpoint via reqwest")?;
    let status = response.status();
    let raw = response
        .text()
        .await
        .context("failed reading MQL5 response body")?;
    if !status.is_success() {
        return Err(anyhow!(
            "MQL5 reqwest status={} body_preview={}",
            status,
            body_preview(&raw)
        ));
    }
    Ok(raw)
}

async fn fetch_mql5_calendar_curl(
    from: &str,
    to: &str,
    importance: &str,
    currencies: &str,
) -> Result<String> {
    let output = Command::new("curl")
        .arg("-sS")
        .arg("-X")
        .arg("POST")
        .arg(MQL5_CALENDAR_URL)
        .arg("-H")
        .arg("User-Agent: Quant-M/0.1")
        .arg("-H")
        .arg("Accept: */*")
        .arg("-H")
        .arg("Origin: https://www.mql5.com")
        .arg("-H")
        .arg("X-Requested-With: XMLHttpRequest")
        .arg("-H")
        .arg("Referer: https://www.mql5.com/en/economic-calendar")
        .arg("--data-urlencode")
        .arg("date_mode=0")
        .arg("--data-urlencode")
        .arg(format!("from={from}"))
        .arg("--data-urlencode")
        .arg(format!("to={to}"))
        .arg("--data-urlencode")
        .arg(format!("importance={importance}"))
        .arg("--data-urlencode")
        .arg(format!("currencies={currencies}"))
        .output()
        .await
        .context("failed calling MQL5 calendar endpoint via curl fallback")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow!(
            "curl fallback failed status={} stderr={}",
            output.status,
            body_preview(&stderr)
        ));
    }
    let raw = String::from_utf8(output.stdout).context("invalid UTF-8 from curl response")?;
    Ok(raw)
}

pub fn apply_macro_events(
    cfg: &Config,
    events: &[MacroEvent],
    now_ms: i64,
    from_ms: i64,
    to_ms: i64,
    source: &str,
) -> Result<MacroRefreshSummary> {
    for event in events {
        put_value(cfg, &key_macro_event(&event.event_id), event)?;
    }

    let mut by_pair: BTreeMap<String, Vec<MacroEvent>> = BTreeMap::new();
    for pair in FOREX_ALLOWED_PAIRS {
        by_pair.insert((*pair).to_string(), Vec::new());
    }
    for event in events {
        for pair in &event.affected_pairs {
            if let Some(list) = by_pair.get_mut(pair) {
                list.push(event.clone());
            }
        }
    }

    let mut blackout_pairs = 0_usize;
    for pair in FOREX_ALLOWED_PAIRS {
        let mut upcoming = by_pair.remove(*pair).unwrap_or_default();
        upcoming.sort_by_key(|event| event.scheduled_at_ms);

        let next_medium = upcoming
            .iter()
            .filter(|event| event.impact == MacroImpact::Medium && event.scheduled_at_ms >= now_ms)
            .map(|event| ms_to_minutes(event.scheduled_at_ms.saturating_sub(now_ms)))
            .min();
        let next_high = upcoming
            .iter()
            .filter(|event| event.impact == MacroImpact::High && event.scheduled_at_ms >= now_ms)
            .map(|event| ms_to_minutes(event.scheduled_at_ms.saturating_sub(now_ms)))
            .min();

        let macro_blackout = upcoming.iter().any(|event| match event.impact {
            MacroImpact::High => {
                let delta = event.scheduled_at_ms.saturating_sub(now_ms);
                (-MACRO_HIGH_POST_BLACKOUT_MINUTES * 60 * 1000
                    ..=MACRO_HIGH_PRE_BLACKOUT_MINUTES * 60 * 1000)
                    .contains(&delta)
            }
            MacroImpact::Medium => false,
        });
        if macro_blackout {
            blackout_pairs = blackout_pairs.saturating_add(1);
        }

        let state = PairMacroState {
            pair: (*pair).to_string(),
            next_medium_event_minutes: next_medium,
            next_high_event_minutes: next_high,
            macro_blackout,
            upcoming_events: upcoming.into_iter().take(MACRO_UPCOMING_LIMIT).collect(),
            updated_at_ms: now_ms,
        };
        put_value(cfg, &key_pair_macro(pair), &state)?;
    }

    Ok(MacroRefreshSummary {
        source: source.to_string(),
        from_ms,
        to_ms,
        events_written: events.len(),
        pairs_written: FOREX_ALLOWED_PAIRS.len(),
        blackout_pairs,
    })
}

pub fn apply_swap_health(cfg: &Config, input: &SwapHealthInput) -> Result<SwapHealthSummary> {
    let source = input
        .source
        .as_deref()
        .unwrap_or(SWAP_HEALTH_SOURCE_DEFAULT)
        .trim()
        .to_string();
    let as_of_ms = input.as_of_ms.unwrap_or_else(now_ms);

    let mut processed = 0_usize;
    let mut long_only = 0_usize;
    let mut short_only = 0_usize;
    let mut carry_blocked = 0_usize;
    let mut skipped_symbols = Vec::new();

    for item in &input.pairs {
        let symbol = normalize_symbol(&item.symbol);
        if !is_allowed_forex_pair(&symbol) {
            skipped_symbols.push(symbol);
            continue;
        }
        let observed_carry_bias = classify_carry_bias(item.swap_long, item.swap_short);
        let effective_policy = match observed_carry_bias {
            CarryBias::LongFavorable => SwapDirectionPolicy::LongOnly,
            CarryBias::ShortFavorable => SwapDirectionPolicy::ShortOnly,
            CarryBias::Neutral | CarryBias::Unknown => SwapDirectionPolicy::CarryBlocked,
        };

        match effective_policy {
            SwapDirectionPolicy::LongOnly => long_only = long_only.saturating_add(1),
            SwapDirectionPolicy::ShortOnly => short_only = short_only.saturating_add(1),
            SwapDirectionPolicy::CarryBlocked => carry_blocked = carry_blocked.saturating_add(1),
        }

        let record = SwapHealthRecord {
            symbol: symbol.clone(),
            swap_long: item.swap_long,
            swap_short: item.swap_short,
            observed_carry_bias,
            effective_policy,
            source: source.clone(),
            as_of_ms,
            updated_at_ms: now_ms(),
        };
        put_swap_health(cfg, &symbol, &record)?;
        processed = processed.saturating_add(1);
    }

    Ok(SwapHealthSummary {
        source,
        as_of_ms,
        processed,
        long_only,
        short_only,
        carry_blocked,
        skipped_symbols,
    })
}

fn normalize_mql5_rows(rows: Vec<Mql5EventRow>) -> Vec<MacroEvent> {
    let mut out = Vec::new();
    for row in rows {
        let currency = row.currency_code.trim().to_ascii_uppercase();
        if !is_allowed_macro_currency(&currency) {
            continue;
        }
        let Some(impact) = parse_macro_impact(&row.importance) else {
            continue;
        };
        if row.release_date <= 0 {
            continue;
        }
        let affected_pairs = map_currency_to_pairs(&currency);
        if affected_pairs.is_empty() {
            continue;
        }
        let event_id = format!(
            "mql5-{}-{}-{}",
            currency.to_ascii_lowercase(),
            row.id,
            row.release_date
        );
        out.push(MacroEvent {
            event_id,
            source: MACRO_SOURCE_MQL5.to_string(),
            currency,
            impact,
            title: row.event_name.trim().to_string(),
            scheduled_at_ms: row.release_date,
            actual: opt_nonempty(&row.actual_value),
            forecast: opt_nonempty(&row.forecast_value),
            previous: opt_nonempty(&row.previous_value),
            affected_pairs,
        });
    }
    out
}

fn is_allowed_macro_currency(currency: &str) -> bool {
    matches!(
        currency,
        "AUD" | "CAD" | "CHF" | "EUR" | "GBP" | "JPY" | "NOK" | "USD"
    )
}

fn parse_macro_impact(raw: &str) -> Option<MacroImpact> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "high" => Some(MacroImpact::High),
        "medium" => Some(MacroImpact::Medium),
        _ => None,
    }
}

fn map_currency_to_pairs(currency: &str) -> Vec<String> {
    let pairs = match currency {
        "AUD" => &["AUDCHF", "AUDJPY", "EURAUD"][..],
        "CAD" => &["EURCAD"][..],
        "CHF" => &["AUDCHF", "USDCHF"][..],
        "EUR" => &["EURAUD", "EURCAD", "EURGBP", "EURNOK", "EURUSD"][..],
        "GBP" => &["EURGBP", "GBPJPY"][..],
        "JPY" => &["AUDJPY", "GBPJPY", "USDJPY"][..],
        "NOK" => &["EURNOK"][..],
        "USD" => &["EURUSD", "USDCHF", "USDJPY"][..],
        _ => &[][..],
    };
    pairs.iter().map(|pair| (*pair).to_string()).collect()
}

fn opt_nonempty(raw: &str) -> Option<String> {
    let cleaned = raw.trim();
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned.to_string())
    }
}

fn body_preview(raw: &str) -> String {
    let trimmed = raw.trim();
    let mut preview = String::new();
    for ch in trimmed.chars().take(200) {
        preview.push(ch);
    }
    preview
}

pub fn ingest_stonex_payload(cfg: &Config, raw_json: &str) -> Result<IngestResult> {
    let mapped = map_stonex_payload(cfg, raw_json)?;

    put_latest_signal(cfg, &mapped.quote_tick.symbol, &mapped.shared_signal)?;
    put_handoff(cfg, &mapped.quote_tick.symbol, &mapped.handoff)?;

    Ok(mapped)
}

pub fn map_stonex_payload(cfg: &Config, raw_json: &str) -> Result<IngestResult> {
    let payload: StoneXQuotePayload =
        serde_json::from_str(raw_json).context("invalid StoneX/FOREX.com quote payload JSON")?;
    let swap_policy_override = get_swap_health(cfg, &payload.symbol)?;
    let pair_macro_state = get_pair_macro_state(cfg, &payload.symbol)?;
    map_payload_with_overrides(payload, swap_policy_override, pair_macro_state)
}

#[cfg(feature = "fuzzing_hooks")]
#[allow(dead_code)]
pub fn map_stonex_payload_for_fuzz(raw_json: &str) -> Result<IngestResult> {
    let payload: StoneXQuotePayload =
        serde_json::from_str(raw_json).context("invalid StoneX/FOREX.com quote payload JSON")?;
    map_payload_with_overrides(payload, None, None)
}

#[cfg(feature = "fuzzing_hooks")]
#[allow(dead_code)]
pub fn parse_mql5_rows_for_fuzz(raw_json: &str) -> Result<Vec<MacroEvent>> {
    let rows: Vec<Mql5EventRow> =
        serde_json::from_str(raw_json).context("invalid JSON for MQL5 row list")?;
    Ok(normalize_mql5_rows(rows))
}

fn map_payload_with_overrides(
    payload: StoneXQuotePayload,
    swap_policy_override: Option<SwapHealthRecord>,
    pair_macro_state: Option<PairMacroState>,
) -> Result<IngestResult> {
    let symbol = normalize_symbol(&payload.symbol);
    if symbol.is_empty() {
        return Err(anyhow!("symbol is empty"));
    }
    if !is_allowed_forex_pair(&symbol) {
        return Err(anyhow!(
            "symbol '{}' is not in enabled forex universe",
            symbol
        ));
    }
    let static_pair_policy = pair_policy(&symbol);
    let pair_policy = swap_policy_override
        .as_ref()
        .map(|record| swap_policy_to_pair_policy(record.effective_policy))
        .unwrap_or(static_pair_policy);
    if payload.bid <= 0.0 || payload.ask <= 0.0 {
        return Err(anyhow!("bid and ask must be positive"));
    }
    if payload.ask < payload.bid {
        return Err(anyhow!("ask must be >= bid"));
    }

    let recv_ts_ms = now_ms();
    let event_ts_ms = payload
        .event_ts_ms
        .or(payload.event_timestamp_ms)
        .or(payload.ts_ms)
        .unwrap_or(recv_ts_ms);
    let processed_ts_ms = now_ms();
    let freshness_ms = (processed_ts_ms - event_ts_ms).max(0);
    let stale = freshness_ms > STALE_THRESHOLD_MS;

    let quote_tick = QuoteTick {
        symbol: symbol.clone(),
        bid: payload.bid,
        ask: payload.ask,
        event_ts_ms,
        recv_ts_ms,
        processed_ts_ms,
        freshness_ms,
    };

    let spread_bps = spread_bps(payload.bid, payload.ask);
    let swap_long = payload.swap_long.unwrap_or(0.0);
    let swap_short = payload.swap_short.unwrap_or(0.0);
    let observed_carry_bias = if payload.swap_long.is_some() || payload.swap_short.is_some() {
        Some(classify_carry_bias(swap_long, swap_short))
    } else {
        None
    };
    let carry_bias = policy_carry_bias(pair_policy);

    let trend_m15 = parse_trend(payload.trend_m15.as_deref());
    let trend_h1 = parse_trend(payload.trend_h1.as_deref());
    let trend_h4 = parse_trend(payload.trend_h4.as_deref());
    let mut next_high_impact_minutes = payload.next_high_impact_minutes.unwrap_or(9_999);
    let mut medium_event_near = false;

    let mut risk_flags = Vec::new();
    let mut blockers = Vec::new();
    let mut evidence = Vec::new();

    match pair_policy {
        PairPolicy::LongOnly => {
            evidence.push("Pair policy allows long-only carry direction".to_string())
        }
        PairPolicy::ShortOnly => {
            evidence.push("Pair policy allows short-only carry direction".to_string())
        }
        PairPolicy::CarryBlocked => {
            risk_flags.push("carry_pair_blocked".to_string());
            blockers.push("pair_not_carry_eligible".to_string());
        }
    }
    if let Some(record) = &swap_policy_override {
        evidence.push(format!(
            "swap_health policy={} as_of_ms={} source={}",
            swap_policy_label(record.effective_policy),
            record.as_of_ms,
            record.source
        ));
        if record.effective_policy != pair_policy_to_swap_policy(static_pair_policy) {
            risk_flags.push("swap_policy_changed".to_string());
        }
    }
    if let Some(state) = &pair_macro_state {
        evidence.push(format!(
            "pair_macro blackout={} next_high={:?} next_medium={:?}",
            state.macro_blackout, state.next_high_event_minutes, state.next_medium_event_minutes
        ));
        if let Some(minutes) = state.next_high_event_minutes {
            next_high_impact_minutes = next_high_impact_minutes.min(minutes as i64);
        }
        if state.macro_blackout {
            push_unique(&mut risk_flags, "macro_event_near");
            push_unique(&mut blockers, "macro_event_near");
        }
        if let Some(minutes) = state.next_medium_event_minutes
            && (minutes as i64) <= MACRO_MEDIUM_PRE_WINDOW_MINUTES
        {
            medium_event_near = true;
            push_unique(&mut risk_flags, "macro_event_medium_near");
        }
    }

    if spread_bps > SPREAD_ELEVATED_BPS {
        risk_flags.push("spread_elevated".to_string());
    }
    if spread_bps > SPREAD_BLOCK_BPS {
        blockers.push("spread_unrealistic".to_string());
    }
    if stale {
        risk_flags.push("stale_feed".to_string());
        blockers.push("stale_feed".to_string());
    }
    if next_high_impact_minutes <= MACRO_BLACKOUT_MINUTES {
        push_unique(&mut risk_flags, "macro_event_near");
        push_unique(&mut blockers, "macro_event_near");
    }
    if matches!(carry_bias, CarryBias::Unknown) {
        risk_flags.push("carry_unknown".to_string());
    }
    if let Some(observed) = observed_carry_bias
        && !observed_matches_policy(pair_policy, &observed)
    {
        risk_flags.push("swap_direction_conflict".to_string());
        blockers.push("swap_direction_conflict".to_string());
    }
    if trend_conflict(&carry_bias, &trend_h1) {
        risk_flags.push("trend_conflict".to_string());
    }

    if matches!(carry_bias, CarryBias::LongFavorable) {
        evidence.push("Carry favors long".to_string());
    } else if matches!(carry_bias, CarryBias::ShortFavorable) {
        evidence.push("Carry favors short".to_string());
    }

    if trend_h1 == Trend::Up {
        evidence.push("H1 trend up".to_string());
    } else if trend_h1 == Trend::Down {
        evidence.push("H1 trend down".to_string());
    }

    if trend_h4 == Trend::Up {
        evidence.push("H4 supportive up".to_string());
    } else if trend_h4 == Trend::Down {
        evidence.push("H4 supportive down".to_string());
    }

    if spread_bps <= SPREAD_ELEVATED_BPS {
        evidence.push("Spread within normal range".to_string());
    }
    if next_high_impact_minutes > MACRO_BLACKOUT_MINUTES {
        evidence.push("No high-impact event in blackout window".to_string());
    }

    let mut recommended_action = recommended_action(
        &carry_bias,
        &trend_h1,
        &trend_h4,
        stale,
        spread_bps,
        next_high_impact_minutes,
    );
    if !blockers.is_empty() {
        recommended_action = SignalType::NoTrade;
    }
    let requires_review = matches!(
        recommended_action,
        SignalType::LongBiasEntryCandidate | SignalType::ShortBiasEntryCandidate
    );

    let mut confidence = compute_confidence(
        &carry_bias,
        &trend_h1,
        &trend_h4,
        stale,
        spread_bps,
        next_high_impact_minutes,
        medium_event_near,
    );
    if !blockers.is_empty() {
        confidence = 0.0;
    }

    let thesis = thesis_for_action(&recommended_action).to_string();
    let policy_ok = blockers.is_empty();
    let quality = if stale {
        FreshnessQuality::Cold
    } else if payload.event_ts_ms.is_none()
        && payload.event_timestamp_ms.is_none()
        && payload.ts_ms.is_none()
    {
        FreshnessQuality::Warm
    } else {
        FreshnessQuality::Hot
    };

    let desk_payload = ForexPayload {
        bid: payload.bid,
        ask: payload.ask,
        spread_bps,
        carry_bias: carry_bias.clone(),
        swap_long,
        swap_short,
        trend_m15,
        trend_h1: trend_h1.clone(),
        trend_h4: trend_h4.clone(),
        ma20: payload.ma20.unwrap_or(0.0),
        ma50: payload.ma50.unwrap_or(0.0),
        next_high_impact_minutes,
    };

    let signal_id = format!("fx-{}-{}", symbol, processed_ts_ms);

    let shared_signal = SharedSignal {
        signal_id: signal_id.clone(),
        routing: RoutingMetadata {
            desk: DESK.to_string(),
            source_venue: SOURCE_VENUE.to_string(),
            source_channel: SOURCE_CHANNEL.to_string(),
            execution_adapter: EXECUTION_ADAPTER.to_string(),
            account_scope: ACCOUNT_SCOPE.to_string(),
            market_type: MARKET_TYPE.to_string(),
            instrument_id: payload.instrument_id.unwrap_or_else(|| symbol.clone()),
            symbol: symbol.clone(),
            paper_trade_only: true,
        },
        freshness: FreshnessMetadata {
            event_ts_ms,
            recv_ts_ms,
            processed_ts_ms,
            freshness_ms,
            stale,
            quality,
        },
        signal: SignalMetadata {
            signal_type: recommended_action.clone(),
            confidence,
            thesis: thesis.clone(),
            recommended_action: recommended_action.clone(),
            requires_review,
        },
        risk: RiskMetadata {
            risk_flags: risk_flags.clone(),
            blockers: blockers.clone(),
            risk_score: (risk_flags.len() as f32 / 10.0).clamp(0.0, 1.0),
            exposure_ok: true,
            policy_ok,
        },
        evidence: if evidence.is_empty() {
            vec!["Insufficient aligned evidence".to_string()]
        } else {
            evidence.clone()
        },
        desk_payload,
    };

    let handoff = DeskHandoff {
        signal_id,
        desk: DESK.to_string(),
        source_venue: SOURCE_VENUE.to_string(),
        execution_adapter: EXECUTION_ADAPTER.to_string(),
        account_scope: ACCOUNT_SCOPE.to_string(),
        symbol,
        thesis,
        evidence,
        risk_flags,
        confidence,
        recommended_action,
        created_at_ms: processed_ts_ms,
    };

    Ok(IngestResult {
        quote_tick,
        shared_signal,
        handoff,
    })
}

fn recommended_action(
    carry_bias: &CarryBias,
    trend_h1: &Trend,
    trend_h4: &Trend,
    stale: bool,
    spread_bps: f64,
    next_high_impact_minutes: i64,
) -> SignalType {
    if stale || spread_bps > SPREAD_BLOCK_BPS || next_high_impact_minutes <= MACRO_BLACKOUT_MINUTES
    {
        return SignalType::NoTrade;
    }

    match carry_bias {
        CarryBias::LongFavorable => {
            if *trend_h1 == Trend::Up && matches!(trend_h4, Trend::Up | Trend::Sideways) {
                SignalType::LongBiasEntryCandidate
            } else {
                SignalType::LongBiasHold
            }
        }
        CarryBias::ShortFavorable => {
            if *trend_h1 == Trend::Down && matches!(trend_h4, Trend::Down | Trend::Sideways) {
                SignalType::ShortBiasEntryCandidate
            } else {
                SignalType::ShortBiasHold
            }
        }
        CarryBias::Neutral => SignalType::Hold,
        CarryBias::Unknown => SignalType::NoTrade,
    }
}

fn compute_confidence(
    carry_bias: &CarryBias,
    trend_h1: &Trend,
    trend_h4: &Trend,
    stale: bool,
    spread_bps: f64,
    next_high_impact_minutes: i64,
    medium_event_near: bool,
) -> f32 {
    let mut value = 0.35_f32;
    match carry_bias {
        CarryBias::LongFavorable | CarryBias::ShortFavorable => value += 0.20,
        CarryBias::Neutral => value += 0.05,
        CarryBias::Unknown => value -= 0.20,
    }

    if trend_agrees(carry_bias, trend_h1) {
        value += 0.20;
    }
    if trend_supports(carry_bias, trend_h4) {
        value += 0.10;
    }

    if spread_bps > SPREAD_ELEVATED_BPS {
        value -= 0.15;
    }
    if stale {
        value -= 0.40;
    }
    if next_high_impact_minutes <= MACRO_BLACKOUT_MINUTES {
        value -= 0.20;
    }
    if medium_event_near {
        value -= 0.10;
    }

    value.clamp(0.0, 1.0)
}

fn thesis_for_action(action: &SignalType) -> &'static str {
    match action {
        SignalType::NoTrade => "Policy blockers present; no trade",
        SignalType::Hold => "Carry/trend alignment insufficient for entry",
        SignalType::LongBiasHold => "Long carry bias present; waiting for stronger trend alignment",
        SignalType::ShortBiasHold => {
            "Short carry bias present; waiting for stronger trend alignment"
        }
        SignalType::LongBiasEntryCandidate => "Carry and trend align for long-bias candidate",
        SignalType::ShortBiasEntryCandidate => "Carry and trend align for short-bias candidate",
    }
}

pub fn is_allowed_forex_pair(symbol: &str) -> bool {
    let normalized = normalize_symbol(symbol);
    FOREX_ALLOWED_PAIRS.contains(&normalized.as_str())
}

fn pair_policy(symbol: &str) -> PairPolicy {
    if FOREX_LONG_BIAS_PAIRS.contains(&symbol) {
        PairPolicy::LongOnly
    } else if FOREX_SHORT_BIAS_PAIRS.contains(&symbol) {
        PairPolicy::ShortOnly
    } else {
        debug_assert!(FOREX_CARRY_BLOCKED_PAIRS.contains(&symbol));
        PairPolicy::CarryBlocked
    }
}

fn policy_carry_bias(policy: PairPolicy) -> CarryBias {
    match policy {
        PairPolicy::LongOnly => CarryBias::LongFavorable,
        PairPolicy::ShortOnly => CarryBias::ShortFavorable,
        PairPolicy::CarryBlocked => CarryBias::Neutral,
    }
}

fn pair_policy_to_swap_policy(policy: PairPolicy) -> SwapDirectionPolicy {
    match policy {
        PairPolicy::LongOnly => SwapDirectionPolicy::LongOnly,
        PairPolicy::ShortOnly => SwapDirectionPolicy::ShortOnly,
        PairPolicy::CarryBlocked => SwapDirectionPolicy::CarryBlocked,
    }
}

fn swap_policy_to_pair_policy(policy: SwapDirectionPolicy) -> PairPolicy {
    match policy {
        SwapDirectionPolicy::LongOnly => PairPolicy::LongOnly,
        SwapDirectionPolicy::ShortOnly => PairPolicy::ShortOnly,
        SwapDirectionPolicy::CarryBlocked => PairPolicy::CarryBlocked,
    }
}

fn swap_policy_label(policy: SwapDirectionPolicy) -> &'static str {
    match policy {
        SwapDirectionPolicy::LongOnly => "long_only",
        SwapDirectionPolicy::ShortOnly => "short_only",
        SwapDirectionPolicy::CarryBlocked => "carry_blocked",
    }
}

fn observed_matches_policy(policy: PairPolicy, observed: &CarryBias) -> bool {
    match policy {
        PairPolicy::LongOnly => matches!(observed, CarryBias::LongFavorable),
        PairPolicy::ShortOnly => matches!(observed, CarryBias::ShortFavorable),
        PairPolicy::CarryBlocked => !matches!(
            observed,
            CarryBias::LongFavorable | CarryBias::ShortFavorable
        ),
    }
}

fn classify_carry_bias(swap_long: f64, swap_short: f64) -> CarryBias {
    if swap_long > 0.0 && swap_short <= 0.0 {
        CarryBias::LongFavorable
    } else if swap_short > 0.0 && swap_long <= 0.0 {
        CarryBias::ShortFavorable
    } else if swap_long == 0.0 && swap_short == 0.0 {
        CarryBias::Neutral
    } else {
        CarryBias::Unknown
    }
}

fn parse_trend(raw: Option<&str>) -> Trend {
    match raw.unwrap_or("").trim().to_ascii_lowercase().as_str() {
        "up" => Trend::Up,
        "down" => Trend::Down,
        "sideways" | "flat" | "neutral" => Trend::Sideways,
        _ => Trend::Unknown,
    }
}

fn trend_agrees(carry_bias: &CarryBias, trend: &Trend) -> bool {
    matches!(
        (carry_bias, trend),
        (CarryBias::LongFavorable, Trend::Up) | (CarryBias::ShortFavorable, Trend::Down)
    )
}

fn trend_supports(carry_bias: &CarryBias, trend: &Trend) -> bool {
    matches!(
        (carry_bias, trend),
        (CarryBias::LongFavorable, Trend::Up | Trend::Sideways)
            | (CarryBias::ShortFavorable, Trend::Down | Trend::Sideways)
    )
}

fn trend_conflict(carry_bias: &CarryBias, trend_h1: &Trend) -> bool {
    matches!(
        (carry_bias, trend_h1),
        (CarryBias::LongFavorable, Trend::Down) | (CarryBias::ShortFavorable, Trend::Up)
    )
}

fn spread_bps(bid: f64, ask: f64) -> f64 {
    let mid = (bid + ask) / 2.0;
    if mid <= 0.0 {
        return 0.0;
    }
    ((ask - bid) / mid) * 10_000.0
}

fn push_unique(target: &mut Vec<String>, value: &str) {
    if !target.iter().any(|item| item == value) {
        target.push(value.to_string());
    }
}

fn ms_to_minutes(delta_ms: i64) -> u32 {
    ((delta_ms.max(0) + 59_999) / 60_000) as u32
}

fn iso_utc_seconds(ts_ms: i64) -> String {
    let secs = ts_ms.div_euclid(1000);
    match chrono::DateTime::<Utc>::from_timestamp(secs, 0) {
        Some(dt) => dt.format("%Y-%m-%dT%H:%M:%S").to_string(),
        None => "1970-01-01T00:00:00".to_string(),
    }
}

fn now_ms() -> i64 {
    Utc::now().timestamp_millis()
}

fn normalize_symbol(raw: &str) -> String {
    raw.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect::<String>()
        .to_ascii_uppercase()
}

fn open_db(cfg: &Config) -> Result<Database> {
    let path = &cfg.forex.redb_path;
    if let Some(parent) = Path::new(path).parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let db = match Database::create(path) {
        Ok(db) => db,
        Err(err) if is_redb_manual_upgrade_error(&err.to_string()) => {
            backup_legacy_redb(path)?;
            Database::create(path)
                .with_context(|| format!("failed to recreate redb {}", path.display()))?
        }
        Err(err) => {
            return Err(err).with_context(|| format!("failed to open redb {}", path.display()));
        }
    };
    ensure_table(&db)?;
    Ok(db)
}

fn is_redb_manual_upgrade_error(message: &str) -> bool {
    message.contains("Manual upgrade required")
}

fn backup_legacy_redb(path: &Path) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let backup_path = legacy_backup_path(path);
    std::fs::rename(path, &backup_path).with_context(|| {
        format!(
            "failed to backup legacy redb '{}' to '{}'",
            path.display(),
            backup_path.display()
        )
    })?;
    eprintln!(
        "forex: backed up legacy redb to '{}'; created fresh v3 store",
        backup_path.display()
    );
    Ok(())
}

fn legacy_backup_path(path: &Path) -> std::path::PathBuf {
    let mut backup = path.as_os_str().to_os_string();
    backup.push(format!(".legacy-v2-{}.bak", now_ms()));
    std::path::PathBuf::from(backup)
}

fn ensure_table(db: &Database) -> Result<()> {
    let write_txn = db.begin_write().context("failed to start redb write txn")?;
    {
        let _ = write_txn
            .open_table(FOREX_STATE_TABLE)
            .context("failed to open redb forex table")?;
    }
    write_txn
        .commit()
        .context("failed to commit redb table init")?;
    Ok(())
}

fn put_value<T: Serialize>(cfg: &Config, key: &str, value: &T) -> Result<()> {
    let db = open_db(cfg)?;
    let write_txn = db.begin_write().context("failed to start redb write txn")?;
    {
        let mut table = write_txn
            .open_table(FOREX_STATE_TABLE)
            .context("failed to open redb forex table for write")?;
        let encoded = postcard::to_stdvec(value).context("failed to serialize redb value")?;
        table
            .insert(key, encoded.as_slice())
            .with_context(|| format!("failed redb insert key '{}'", key))?;
    }
    write_txn
        .commit()
        .context("failed to commit redb write txn")?;
    Ok(())
}

fn get_value<T: for<'de> Deserialize<'de>>(cfg: &Config, key: &str) -> Result<Option<T>> {
    let db = open_db(cfg)?;
    let read_txn = db.begin_read().context("failed to start redb read txn")?;
    let table = read_txn
        .open_table(FOREX_STATE_TABLE)
        .context("failed to open redb forex table for read")?;
    let maybe = table
        .get(key)
        .with_context(|| format!("failed redb get key '{}'", key))?;

    if let Some(value) = maybe {
        let decoded = postcard::from_bytes(value.value())
            .with_context(|| format!("failed to deserialize redb value for key '{}'", key))?;
        Ok(Some(decoded))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.forex.redb_path = tmp.path().join("state/forex.redb");
        (tmp, cfg)
    }

    #[test]
    fn redb_put_get_roundtrip_signal_and_handoff() {
        let (_tmp, cfg) = test_config();
        let result = ingest_stonex_payload(
            &cfg,
            r#"{
                "symbol":"EUR/USD",
                "bid":1.1000,
                "ask":1.1002,
                "event_ts_ms":1730000000000,
                "swap_long":0.8,
                "swap_short":-0.6,
                "trend_h1":"Up",
                "trend_h4":"Up",
                "next_high_impact_minutes":120
            }"#,
        )
        .expect("ingest");

        let got_signal = get_latest_signal(&cfg, "EURUSD").expect("get signal");
        assert!(got_signal.is_some());
        let got_signal = got_signal.expect("signal value");
        assert_eq!(got_signal.routing.symbol, "EURUSD");

        let got_handoff = get_handoff(&cfg, "eur/usd").expect("get handoff");
        assert!(got_handoff.is_some());
        let got_handoff = got_handoff.expect("handoff value");
        assert_eq!(got_handoff.symbol, "EURUSD");
        assert_eq!(result.shared_signal.routing.desk, "forex");
    }

    #[test]
    fn ingest_rejects_invalid_quotes() {
        let (_tmp, cfg) = test_config();
        let err = ingest_stonex_payload(&cfg, r#"{"symbol":"","bid":1.0,"ask":0.9}"#)
            .expect_err("should reject invalid payload");
        assert!(err.to_string().contains("symbol is empty") || err.to_string().contains("ask"));
    }

    #[test]
    fn stale_payload_is_no_trade() {
        let (_tmp, cfg) = test_config();
        let old_ts = now_ms() - 60_000;
        let result = ingest_stonex_payload(
            &cfg,
            &format!(
                "{{\"symbol\":\"USDJPY\",\"bid\":150.1,\"ask\":150.2,\"event_ts_ms\":{}}}",
                old_ts
            ),
        )
        .expect("ingest stale");

        assert!(result.shared_signal.freshness.stale);
        assert_eq!(
            result.shared_signal.signal.recommended_action,
            SignalType::NoTrade
        );
        assert!(
            result
                .shared_signal
                .risk
                .risk_flags
                .iter()
                .any(|flag| flag == "stale_feed")
        );
    }

    #[test]
    fn key_helpers_normalize_symbols() {
        assert_eq!(key_latest_signal("eur/usd"), "latest_signal:forex:EURUSD");
        assert_eq!(key_handoff(" gbp-jpy "), "handoff:forex:GBPJPY");
    }

    #[test]
    fn allowed_pairs_helper_accepts_only_universe() {
        assert!(is_allowed_forex_pair("AUDCHF"));
        assert!(is_allowed_forex_pair("eur/usd"));
        assert!(!is_allowed_forex_pair("NZDUSD"));
    }

    #[test]
    fn rejects_symbol_outside_enabled_universe() {
        let (_tmp, cfg) = test_config();
        let err = ingest_stonex_payload(&cfg, r#"{"symbol":"NZDUSD","bid":0.6200,"ask":0.6202}"#)
            .expect_err("should reject unknown pair");
        assert!(err.to_string().contains("not in enabled forex universe"));
    }

    #[test]
    fn eurgcad_is_blocked_for_carry() {
        let (_tmp, cfg) = test_config();
        let result = ingest_stonex_payload(
            &cfg,
            &format!(
                "{{\"symbol\":\"EUR/CAD\",\"bid\":1.4700,\"ask\":1.4702,\"event_ts_ms\":{},\"trend_h1\":\"Down\",\"trend_h4\":\"Down\"}}",
                now_ms()
            ),
        )
        .expect("ingest blocked pair");
        assert_eq!(
            result.shared_signal.signal.recommended_action,
            SignalType::NoTrade
        );
        assert!(
            result
                .shared_signal
                .risk
                .risk_flags
                .iter()
                .any(|flag| flag == "carry_pair_blocked")
        );
    }

    #[test]
    fn swap_direction_conflict_blocks_candidate() {
        let (_tmp, cfg) = test_config();
        let result = ingest_stonex_payload(
            &cfg,
            &format!(
                "{{\"symbol\":\"AUDJPY\",\"bid\":97.100,\"ask\":97.102,\"event_ts_ms\":{},\"swap_long\":-0.40,\"swap_short\":0.20,\"trend_h1\":\"Up\",\"trend_h4\":\"Up\"}}",
                now_ms()
            ),
        )
        .expect("ingest conflict");
        assert_eq!(
            result.shared_signal.signal.recommended_action,
            SignalType::NoTrade
        );
        assert!(
            result
                .shared_signal
                .risk
                .risk_flags
                .iter()
                .any(|flag| flag == "swap_direction_conflict")
        );
    }

    #[test]
    fn detects_manual_upgrade_error_message() {
        assert!(is_redb_manual_upgrade_error(
            "Manual upgrade required. Expected file format version 3, but file is version 2"
        ));
        assert!(!is_redb_manual_upgrade_error("some other io error"));
    }

    #[test]
    fn normalize_mql5_rows_keeps_medium_high_and_allowed_currency() {
        let rows = vec![
            Mql5EventRow {
                id: 1,
                event_name: "Fed Speech".to_string(),
                importance: "high".to_string(),
                currency_code: "USD".to_string(),
                release_date: now_ms() + 60_000,
                actual_value: "".to_string(),
                forecast_value: "".to_string(),
                previous_value: "".to_string(),
            },
            Mql5EventRow {
                id: 2,
                event_name: "Low event".to_string(),
                importance: "low".to_string(),
                currency_code: "USD".to_string(),
                release_date: now_ms() + 60_000,
                actual_value: "".to_string(),
                forecast_value: "".to_string(),
                previous_value: "".to_string(),
            },
            Mql5EventRow {
                id: 3,
                event_name: "Foreign event".to_string(),
                importance: "high".to_string(),
                currency_code: "BRL".to_string(),
                release_date: now_ms() + 60_000,
                actual_value: "".to_string(),
                forecast_value: "".to_string(),
                previous_value: "".to_string(),
            },
        ];
        let out = normalize_mql5_rows(rows);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].impact, MacroImpact::High);
        assert_eq!(out[0].currency, "USD");
        assert!(out[0].affected_pairs.iter().any(|pair| pair == "USDJPY"));
    }

    #[test]
    fn macro_pair_state_blocks_high_impact_window() {
        let (_tmp, cfg) = test_config();
        let now = now_ms();
        let event = MacroEvent {
            event_id: "mql5-usd-1".to_string(),
            source: MACRO_SOURCE_MQL5.to_string(),
            currency: "USD".to_string(),
            impact: MacroImpact::High,
            title: "Fed Event".to_string(),
            scheduled_at_ms: now + 10 * 60_000,
            actual: None,
            forecast: None,
            previous: None,
            affected_pairs: vec!["USDJPY".to_string(), "EURUSD".to_string()],
        };
        let summary = apply_macro_events(&cfg, &[event], now, now, now + 3_600_000, "unit_test")
            .expect("apply macro");
        assert_eq!(summary.events_written, 1);

        let pair_state = get_pair_macro_state(&cfg, "USDJPY")
            .expect("macro get")
            .expect("pair state");
        assert!(pair_state.macro_blackout);
        assert!(pair_state.next_high_event_minutes.is_some());

        let result = ingest_stonex_payload(
            &cfg,
            &format!(
                "{{\"symbol\":\"USDJPY\",\"bid\":150.10,\"ask\":150.12,\"event_ts_ms\":{},\"trend_h1\":\"Up\",\"trend_h4\":\"Up\"}}",
                now
            ),
        )
        .expect("ingest with macro");
        assert_eq!(
            result.shared_signal.signal.recommended_action,
            SignalType::NoTrade
        );
        assert!(
            result
                .shared_signal
                .risk
                .risk_flags
                .iter()
                .any(|flag| flag == "macro_event_near")
        );
    }

    #[test]
    fn swap_health_roundtrip_and_override_blocks_old_direction() {
        let (_tmp, cfg) = test_config();
        let health = SwapHealthInput {
            source: Some("unit_test".to_string()),
            as_of_ms: Some(now_ms()),
            pairs: vec![SwapHealthPairInput {
                symbol: "AUDJPY".to_string(),
                swap_long: -0.10,
                swap_short: 0.05,
            }],
        };
        let summary = apply_swap_health(&cfg, &health).expect("apply swap health");
        assert_eq!(summary.processed, 1);
        assert_eq!(summary.short_only, 1);

        let stored = get_swap_health(&cfg, "AUDJPY")
            .expect("get swap health")
            .expect("swap health exists");
        assert_eq!(stored.effective_policy, SwapDirectionPolicy::ShortOnly);

        let result = ingest_stonex_payload(
            &cfg,
            &format!(
                "{{\"symbol\":\"AUDJPY\",\"bid\":97.100,\"ask\":97.102,\"event_ts_ms\":{},\"trend_h1\":\"Up\",\"trend_h4\":\"Up\"}}",
                now_ms()
            ),
        )
        .expect("ingest with override");
        assert_eq!(
            result.shared_signal.desk_payload.carry_bias,
            CarryBias::ShortFavorable
        );
        assert_eq!(
            result.shared_signal.signal.recommended_action,
            SignalType::ShortBiasHold
        );
        assert!(
            result
                .shared_signal
                .risk
                .risk_flags
                .iter()
                .any(|flag| flag == "swap_policy_changed")
        );
    }
}
