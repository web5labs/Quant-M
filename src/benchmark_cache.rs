use crate::config::{Config, ProviderKind};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::env;
use std::path::PathBuf;

const CACHE_TTL_HOURS: i64 = 24;
const AA_URL: &str = "https://artificialanalysis.ai/api/v2/data/llms/models";
const AA_KEY_ENV: &str = "ARTIFICIAL_ANALYSIS_API_KEY";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkCacheStatus {
    pub path: PathBuf,
    pub initialized: bool,
    pub model_count: usize,
    pub last_refresh_at: Option<String>,
    pub stale: bool,
    pub ttl_hours: i64,
    pub key_env: String,
    pub key_present: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BenchmarkRefreshReport {
    pub path: PathBuf,
    pub refreshed: bool,
    pub model_count: usize,
    pub fetched_at: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRouteReport {
    pub task: String,
    pub selected_model: Option<ModelRouteCandidate>,
    pub candidates: Vec<ModelRouteCandidate>,
    pub cache_status: BenchmarkCacheStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelRouteCandidate {
    pub provider: String,
    pub model: String,
    pub endpoint: String,
    pub score: f64,
    pub benchmark_match: Option<String>,
    pub coding_score: Option<f64>,
    pub intelligence_score: Option<f64>,
    pub speed_score: Option<f64>,
    pub cost_score: Option<f64>,
    pub source: String,
}

#[derive(Debug, Clone)]
struct CachedBenchmarkModel {
    id: String,
    name: String,
    slug: String,
    creator_name: Option<String>,
    coding: Option<f64>,
    intelligence: Option<f64>,
    output_tps: Option<f64>,
    ttft_seconds: Option<f64>,
    input_price: Option<f64>,
    output_price: Option<f64>,
}

pub fn cache_path(cfg: &Config) -> PathBuf {
    cfg.workspace_dir
        .join("state")
        .join("artificial-analysis-cache.sqlite")
}

pub fn status(cfg: &Config) -> Result<BenchmarkCacheStatus> {
    let path = cache_path(cfg);
    let conn = open_cache(cfg)?;
    let model_count: usize =
        conn.query_row("SELECT COUNT(*) FROM aa_llm_models", [], |row| row.get(0))?;
    let last_refresh_at = metadata(&conn, "last_refresh_at")?;
    let stale = last_refresh_at
        .as_deref()
        .and_then(parse_rfc3339)
        .map(|value| Utc::now().signed_duration_since(value).num_hours() >= CACHE_TTL_HOURS)
        .unwrap_or(true);
    Ok(BenchmarkCacheStatus {
        path,
        initialized: true,
        model_count,
        last_refresh_at,
        stale,
        ttl_hours: CACHE_TTL_HOURS,
        key_env: AA_KEY_ENV.to_string(),
        key_present: env::var(AA_KEY_ENV)
            .ok()
            .is_some_and(|value| !value.trim().is_empty()),
    })
}

pub async fn refresh(cfg: &Config, live: bool) -> Result<BenchmarkRefreshReport> {
    let path = cache_path(cfg);
    let conn = open_cache(cfg)?;
    if !live {
        let current = status(cfg)?;
        return Ok(BenchmarkRefreshReport {
            path,
            refreshed: false,
            model_count: current.model_count,
            fetched_at: current.last_refresh_at,
            message: "cache initialized; pass --live to fetch Artificial Analysis once explicitly"
                .to_string(),
        });
    }
    if !cfg.runtime.external_network_enabled {
        return Err(anyhow!(
            "external network is disabled; enable it before running a live benchmark refresh"
        ));
    }
    let key = env::var(AA_KEY_ENV)
        .ok()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| anyhow!("missing {AA_KEY_ENV}"))?;

    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .context("failed to build Artificial Analysis HTTP client")?;
    let payload = client
        .get(AA_URL)
        .header("x-api-key", key)
        .send()
        .await
        .context("failed to fetch Artificial Analysis LLM data")?
        .error_for_status()
        .context("Artificial Analysis returned non-success status")?
        .json::<Value>()
        .await
        .context("failed to decode Artificial Analysis response")?;

    let fetched_at = Utc::now().to_rfc3339();
    let count = upsert_payload(&conn, &payload, &fetched_at)?;
    set_metadata(&conn, "last_refresh_at", &fetched_at)?;
    set_metadata(&conn, "source", AA_URL)?;
    Ok(BenchmarkRefreshReport {
        path,
        refreshed: true,
        model_count: count,
        fetched_at: Some(fetched_at),
        message:
            "Artificial Analysis LLM benchmarks cached locally; use at most once per day in cron"
                .to_string(),
    })
}

pub fn route_report(
    cfg: &Config,
    task: &str,
    explicit_models: &[String],
) -> Result<ModelRouteReport> {
    let cache_status = status(cfg)?;
    let benchmarks = load_models(cfg)?;
    let mut candidates = selected_candidates(cfg, explicit_models);
    for candidate in &mut candidates {
        score_candidate(candidate, task, &benchmarks);
    }
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then(a.provider.cmp(&b.provider))
            .then(a.model.cmp(&b.model))
    });
    Ok(ModelRouteReport {
        task: task.to_string(),
        selected_model: candidates.first().cloned(),
        candidates,
        cache_status,
        note: "selected model wins when it is the only candidate; multiple candidates are ranked by cached benchmark, cost, and endpoint fit without making provider calls".to_string(),
    })
}

pub fn render_status(status: &BenchmarkCacheStatus) -> String {
    format!(
        "Artificial Analysis benchmark cache\npath: {}\nmodels: {}\nlast_refresh_at: {}\nstale: {}\nttl_hours: {}\nkey_env: {}\nkey_present: {}\n",
        status.path.display(),
        status.model_count,
        status.last_refresh_at.as_deref().unwrap_or("never"),
        status.stale,
        status.ttl_hours,
        status.key_env,
        status.key_present
    )
}

pub fn render_refresh(report: &BenchmarkRefreshReport) -> String {
    format!(
        "Artificial Analysis benchmark refresh\npath: {}\nrefreshed: {}\nmodels: {}\nfetched_at: {}\nmessage: {}\n",
        report.path.display(),
        report.refreshed,
        report.model_count,
        report.fetched_at.as_deref().unwrap_or("none"),
        report.message
    )
}

pub fn render_route(report: &ModelRouteReport) -> String {
    let mut out = format!(
        "Quant-M model route\n task: {}\n selected: {}\n cache_models: {}\n cache_stale: {}\n\n",
        report.task,
        report
            .selected_model
            .as_ref()
            .map(|candidate| format!("{} {}", candidate.provider, candidate.model))
            .unwrap_or_else(|| "none".to_string()),
        report.cache_status.model_count,
        report.cache_status.stale
    );
    for candidate in &report.candidates {
        out.push_str(&format!(
            "{} {} endpoint={} score={:.3} benchmark_match={}\n",
            candidate.provider,
            candidate.model,
            candidate.endpoint,
            candidate.score,
            candidate.benchmark_match.as_deref().unwrap_or("none")
        ));
    }
    out.push('\n');
    out.push_str(&report.note);
    out.push('\n');
    out
}

pub fn render_cron(binary: &str, config_path: &str) -> String {
    format!(
        "Daily Artificial Analysis benchmark cache cron\n\n0 3 * * * ARTIFICIAL_ANALYSIS_API_KEY=$ARTIFICIAL_ANALYSIS_API_KEY {binary} --config {config_path} provider benchmark refresh --live >/tmp/quant-m-aa-cache.log 2>&1\n"
    )
}

fn open_cache(cfg: &Config) -> Result<Connection> {
    let path = cache_path(cfg);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create cache directory {}", parent.display()))?;
    }
    let conn = Connection::open(&path)
        .with_context(|| format!("failed to open benchmark cache {}", path.display()))?;
    init_schema(&conn)?;
    Ok(conn)
}

fn init_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS aa_cache_metadata (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS aa_llm_models (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            slug TEXT NOT NULL,
            creator_id TEXT,
            creator_name TEXT,
            creator_slug TEXT,
            intelligence REAL,
            coding REAL,
            math REAL,
            mmlu_pro REAL,
            gpqa REAL,
            livecodebench REAL,
            input_price REAL,
            output_price REAL,
            blended_price REAL,
            output_tps REAL,
            ttft_seconds REAL,
            raw_json TEXT NOT NULL,
            fetched_at TEXT NOT NULL
        );
        ",
    )?;
    Ok(())
}

fn metadata(conn: &Connection, key: &str) -> Result<Option<String>> {
    conn.query_row(
        "SELECT value FROM aa_cache_metadata WHERE key = ?1",
        params![key],
        |row| row.get(0),
    )
    .optional()
    .map_err(Into::into)
}

fn set_metadata(conn: &Connection, key: &str, value: &str) -> Result<()> {
    conn.execute(
        "INSERT INTO aa_cache_metadata (key, value) VALUES (?1, ?2)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value",
        params![key, value],
    )?;
    Ok(())
}

fn upsert_payload(conn: &Connection, payload: &Value, fetched_at: &str) -> Result<usize> {
    let data = payload
        .get("data")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("Artificial Analysis payload missing data array"))?;
    for item in data {
        let id = text(item, "id").unwrap_or_else(|| stable_model_key(item));
        let evaluations = item.get("evaluations").unwrap_or(&Value::Null);
        let pricing = item.get("pricing").unwrap_or(&Value::Null);
        let creator = item.get("model_creator").unwrap_or(&Value::Null);
        conn.execute(
            "INSERT INTO aa_llm_models (
                id, name, slug, creator_id, creator_name, creator_slug,
                intelligence, coding, math, mmlu_pro, gpqa, livecodebench,
                input_price, output_price, blended_price, output_tps, ttft_seconds,
                raw_json, fetched_at
            ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19)
            ON CONFLICT(id) DO UPDATE SET
                name = excluded.name,
                slug = excluded.slug,
                creator_id = excluded.creator_id,
                creator_name = excluded.creator_name,
                creator_slug = excluded.creator_slug,
                intelligence = excluded.intelligence,
                coding = excluded.coding,
                math = excluded.math,
                mmlu_pro = excluded.mmlu_pro,
                gpqa = excluded.gpqa,
                livecodebench = excluded.livecodebench,
                input_price = excluded.input_price,
                output_price = excluded.output_price,
                blended_price = excluded.blended_price,
                output_tps = excluded.output_tps,
                ttft_seconds = excluded.ttft_seconds,
                raw_json = excluded.raw_json,
                fetched_at = excluded.fetched_at",
            params![
                id,
                text(item, "name").unwrap_or_default(),
                text(item, "slug").unwrap_or_default(),
                text(creator, "id"),
                text(creator, "name"),
                text(creator, "slug"),
                number(evaluations, "artificial_analysis_intelligence_index"),
                number(evaluations, "artificial_analysis_coding_index"),
                number(evaluations, "artificial_analysis_math_index"),
                number(evaluations, "mmlu_pro"),
                number(evaluations, "gpqa"),
                number(evaluations, "livecodebench"),
                number(pricing, "price_1m_input_tokens"),
                number(pricing, "price_1m_output_tokens"),
                number(pricing, "price_1m_blended_3_to_1"),
                number(item, "median_output_tokens_per_second"),
                number(item, "median_time_to_first_token_seconds")
                    .or_else(|| number(item, "median_time_to_first_answer_token")),
                serde_json::to_string(item)?,
                fetched_at,
            ],
        )?;
    }
    Ok(data.len())
}

fn load_models(cfg: &Config) -> Result<Vec<CachedBenchmarkModel>> {
    let conn = open_cache(cfg)?;
    let mut stmt = conn.prepare(
        "SELECT id, name, slug, creator_name, coding, intelligence, output_tps, ttft_seconds, input_price, output_price
         FROM aa_llm_models",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(CachedBenchmarkModel {
            id: row.get(0)?,
            name: row.get(1)?,
            slug: row.get(2)?,
            creator_name: row.get(3)?,
            coding: row.get(4)?,
            intelligence: row.get(5)?,
            output_tps: row.get(6)?,
            ttft_seconds: row.get(7)?,
            input_price: row.get(8)?,
            output_price: row.get(9)?,
        })
    })?;
    rows.collect::<std::result::Result<Vec<_>, _>>()
        .map_err(Into::into)
}

fn selected_candidates(cfg: &Config, explicit_models: &[String]) -> Vec<ModelRouteCandidate> {
    let mut out = Vec::new();
    if explicit_models.is_empty() {
        if let Some(model) = &cfg.preferences.preferred_openrouter_model {
            push_candidate(&mut out, "openrouter", model);
        }
        if let Some(pref) = &cfg.preferences.preferred_remote_model {
            push_candidate(&mut out, &pref.provider, &pref.model);
        }
        if let Some(pref) = &cfg.preferences.preferred_local_model {
            push_candidate(&mut out, &pref.provider, &pref.model);
        }
        for (provider_id, provider) in &cfg.providers {
            if provider.enabled {
                for model in &provider.preferred_models {
                    push_candidate(&mut out, provider_id, model);
                }
            }
        }
    } else {
        for raw in explicit_models {
            if let Some((provider, model)) = raw.split_once(':') {
                push_candidate(&mut out, provider, model);
            } else if raw.contains('/') {
                push_candidate(&mut out, "openrouter", raw);
            } else {
                push_candidate(&mut out, "openai", raw);
            }
        }
    }
    out.sort_by(|a, b| a.provider.cmp(&b.provider).then(a.model.cmp(&b.model)));
    out.dedup_by(|a, b| a.provider == b.provider && a.model == b.model);
    out
}

fn push_candidate(out: &mut Vec<ModelRouteCandidate>, provider: &str, model: &str) {
    let provider = normalize_provider(provider);
    out.push(ModelRouteCandidate {
        endpoint: endpoint_for(&provider),
        provider,
        model: model.trim().to_string(),
        score: 0.0,
        benchmark_match: None,
        coding_score: None,
        intelligence_score: None,
        speed_score: None,
        cost_score: None,
        source: "selected".to_string(),
    });
}

fn score_candidate(
    candidate: &mut ModelRouteCandidate,
    task: &str,
    benchmarks: &[CachedBenchmarkModel],
) {
    let matched = find_benchmark(&candidate.model, benchmarks);
    if let Some(model) = matched {
        candidate.benchmark_match = Some(format!("{} ({})", model.name, model.id));
        candidate.coding_score = model.coding.map(score_index);
        candidate.intelligence_score = model.intelligence.map(score_index);
        candidate.speed_score = speed_score(model.output_tps, model.ttft_seconds);
        candidate.cost_score = cost_score(model.input_price, model.output_price);
        candidate.source = "artificial_analysis_cache".to_string();
    }
    let coding = candidate.coding_score.unwrap_or(0.5);
    let intelligence = candidate.intelligence_score.unwrap_or(0.5);
    let speed = candidate.speed_score.unwrap_or(0.5);
    let cost = candidate.cost_score.unwrap_or(0.5);
    let endpoint_fit = endpoint_fit(&candidate.provider, task);
    candidate.score = match task.trim().to_ascii_lowercase().as_str() {
        "cheap" => {
            0.25 * coding + 0.15 * intelligence + 0.15 * speed + 0.35 * cost + 0.10 * endpoint_fit
        }
        "long-context" | "long_context" => {
            0.25 * coding + 0.30 * intelligence + 0.10 * speed + 0.10 * cost + 0.25 * endpoint_fit
        }
        "council" => {
            0.30 * coding + 0.30 * intelligence + 0.15 * speed + 0.10 * cost + 0.15 * endpoint_fit
        }
        _ => 0.45 * coding + 0.25 * intelligence + 0.15 * speed + 0.05 * cost + 0.10 * endpoint_fit,
    };
}

fn find_benchmark<'a>(
    model: &str,
    benchmarks: &'a [CachedBenchmarkModel],
) -> Option<&'a CachedBenchmarkModel> {
    let model_key = normalize_model(model);
    benchmarks.iter().find(|candidate| {
        let keys = [
            normalize_model(&candidate.id),
            normalize_model(&candidate.name),
            normalize_model(&candidate.slug),
            candidate
                .creator_name
                .as_deref()
                .map(normalize_model)
                .unwrap_or_default(),
        ];
        keys.iter()
            .any(|key| !key.is_empty() && (model_key.contains(key) || key.contains(&model_key)))
    })
}

fn endpoint_for(provider: &str) -> String {
    match provider {
        "openai" => "/responses".to_string(),
        "openrouter" => "/chat/completions".to_string(),
        "ollama" => "/api/chat".to_string(),
        "lmstudio" => "/v1/chat/completions".to_string(),
        "gemini" => "/v1beta/models/{model}:generateContent".to_string(),
        "zai" => "/paas/v4/chat/completions".to_string(),
        other => format!("{other}:unknown"),
    }
}

fn endpoint_fit(provider: &str, task: &str) -> f64 {
    match (provider, task.trim().to_ascii_lowercase().as_str()) {
        ("ollama" | "lmstudio", "local") => 1.0,
        ("openrouter", "council") => 0.95,
        ("openrouter", "cheap") => 0.9,
        ("openai", "coding") => 0.9,
        ("openai", _) => 0.85,
        ("openrouter", _) => 0.85,
        ("gemini", "long-context" | "long_context") => 0.85,
        ("zai", _) => 0.7,
        ("ollama" | "lmstudio", _) => 0.65,
        _ => 0.5,
    }
}

fn normalize_provider(value: &str) -> String {
    let normalized = value.trim().to_ascii_lowercase().replace(['_', '-'], "");
    match normalized.as_str() {
        "openai" => "openai".to_string(),
        "openrouter" => "openrouter".to_string(),
        "lmstudio" => "lmstudio".to_string(),
        "ollama" => "ollama".to_string(),
        "gemini" | "google" => "gemini".to_string(),
        "zai" | "z.ai" => "zai".to_string(),
        other => other.to_string(),
    }
}

fn normalize_model(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace(['/', ':', '-', '_', '.', ' '], "")
}

fn score_index(value: f64) -> f64 {
    if value > 1.0 {
        (value / 100.0).clamp(0.0, 1.0)
    } else {
        value.clamp(0.0, 1.0)
    }
}

fn speed_score(output_tps: Option<f64>, ttft_seconds: Option<f64>) -> Option<f64> {
    let tps = output_tps.map(|value| (value / 200.0).clamp(0.0, 1.0));
    let ttft = ttft_seconds.map(|value| (1.0 - (value / 20.0)).clamp(0.0, 1.0));
    match (tps, ttft) {
        (Some(a), Some(b)) => Some((a + b) / 2.0),
        (Some(value), None) | (None, Some(value)) => Some(value),
        (None, None) => None,
    }
}

fn cost_score(input_price: Option<f64>, output_price: Option<f64>) -> Option<f64> {
    let blended = match (input_price, output_price) {
        (Some(input), Some(output)) => (input * 3.0 + output) / 4.0,
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => return None,
    };
    Some((1.0 - (blended / 25.0)).clamp(0.0, 1.0))
}

fn text(value: &Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn number(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn stable_model_key(item: &Value) -> String {
    text(item, "slug")
        .or_else(|| text(item, "name"))
        .unwrap_or_else(|| format!("unknown-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0)))
}

fn parse_rfc3339(value: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|value| value.with_timezone(&Utc))
}

#[allow(dead_code)]
fn provider_kind_id(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::OpenRouter => "openrouter",
        ProviderKind::OpenAi => "openai",
        ProviderKind::Ollama => "ollama",
        ProviderKind::LmStudio => "lmstudio",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config {
            workspace_dir: tmp.path().join("workspace"),
            ..Default::default()
        };
        cfg.providers
            .get_mut("openrouter")
            .expect("openrouter")
            .enabled = true;
        cfg.providers
            .get_mut("openrouter")
            .expect("openrouter")
            .preferred_models = vec!["openai/gpt-5".to_string(), "qwen/qwen3-coder".to_string()];
        (tmp, cfg)
    }

    #[test]
    fn route_score_uses_cached_artificial_analysis_data() {
        let (_tmp, cfg) = test_cfg();
        let conn = open_cache(&cfg).expect("cache");
        let payload = serde_json::json!({
            "status": 200,
            "data": [
                {
                    "id": "gpt5",
                    "name": "GPT-5",
                    "slug": "gpt-5",
                    "model_creator": {"id": "openai", "name": "OpenAI", "slug": "openai"},
                    "evaluations": {
                        "artificial_analysis_intelligence_index": 90.0,
                        "artificial_analysis_coding_index": 92.0
                    },
                    "pricing": {
                        "price_1m_input_tokens": 1.0,
                        "price_1m_output_tokens": 5.0
                    },
                    "median_output_tokens_per_second": 120.0,
                    "median_time_to_first_token_seconds": 2.0
                },
                {
                    "id": "qwen",
                    "name": "Qwen3 Coder",
                    "slug": "qwen3-coder",
                    "model_creator": {"id": "qwen", "name": "Qwen", "slug": "qwen"},
                    "evaluations": {
                        "artificial_analysis_intelligence_index": 70.0,
                        "artificial_analysis_coding_index": 80.0
                    },
                    "pricing": {
                        "price_1m_input_tokens": 0.1,
                        "price_1m_output_tokens": 0.3
                    },
                    "median_output_tokens_per_second": 80.0,
                    "median_time_to_first_token_seconds": 1.0
                }
            ]
        });
        upsert_payload(&conn, &payload, &Utc::now().to_rfc3339()).expect("upsert");

        let report = route_report(&cfg, "coding", &[]).expect("route");

        assert_eq!(report.candidates.len(), 2);
        assert_eq!(
            report
                .selected_model
                .expect("selected")
                .benchmark_match
                .as_deref(),
            Some("GPT-5 (gpt5)")
        );
    }

    #[test]
    fn explicit_openrouter_model_gets_openrouter_endpoint() {
        let (_tmp, cfg) = test_cfg();
        let report =
            route_report(&cfg, "cheap", &["anthropic/claude-sonnet".to_string()]).expect("route");

        let selected = report.selected_model.expect("selected");
        assert_eq!(selected.provider, "openrouter");
        assert_eq!(selected.endpoint, "/chat/completions");
    }
}
