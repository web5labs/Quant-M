use crate::config::Config;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharedSignalInput {
    pub signal_id: String,
    pub desk: String,
    pub source_venue: String,
    pub execution_adapter: String,
    pub account_scope: String,
    pub symbol: String,
    pub freshness_ms: i64,
    pub confidence: f64,
    // TODO(serde-normalization): keep this raw JSON field at the intake boundary only.
    // Future desk slices should normalize the subset they need into typed Serde structs
    // before runtime logic depends on it.
    pub payload_json: Value,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeskHandoffInput {
    pub desk: String,
    pub source_venue: String,
    pub symbol: Option<String>,
    pub signal_id: String,
    pub created_at: Option<String>,
    pub producer_role: String,
    pub producer_model: Option<String>,
    pub thesis: String,
    // TODO(serde-normalization): normalize recurring evidence/risk payloads into typed
    // desk-local structs when a slice needs them beyond storage intake and operator review.
    pub evidence_json: Option<Value>,
    pub risk_flags_json: Option<Value>,
    pub confidence: Option<f64>,
    pub recommended_action: Option<String>,
    pub execution_adapter: String,
    pub account_scope: String,
    pub paper_trade_only: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeskHandoff {
    pub id: i64,
    pub desk: String,
    pub source_venue: String,
    pub symbol: Option<String>,
    pub signal_id: String,
    pub created_at: String,
    pub producer_role: String,
    pub producer_model: Option<String>,
    pub thesis: String,
    pub evidence_json: Option<Value>,
    pub risk_flags_json: Option<Value>,
    pub confidence: Option<f64>,
    pub recommended_action: Option<String>,
    pub execution_adapter: String,
    pub account_scope: String,
    pub paper_trade_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskReviewInput {
    pub signal_id: String,
    pub desk: String,
    pub reviewer_role: String,
    pub score: Option<f64>,
    pub decision: String,
    pub notes: Option<String>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperOrderInput {
    pub signal_id: String,
    pub desk: String,
    pub symbol: String,
    pub side: String,
    pub quantity: f64,
    pub venue: String,
    pub status: String,
    // TODO(serde-normalization): keep raw order details at the intake/storage boundary.
    // Runtime truth should use typed Serde-backed fields when execution logic needs them.
    pub details_json: Option<Value>,
    pub created_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateSummary {
    pub shared_signals: i64,
    pub desk_handoffs: i64,
    pub risk_reviews: i64,
    pub paper_orders: i64,
    pub db_path: String,
}

pub fn init_schema(cfg: &Config) -> Result<()> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)
}

pub fn sanity_check(cfg: &Config) -> Result<()> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;
    let quick_check: String = conn
        .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
        .context("failed to run shared-state sqlite quick_check")?;
    if quick_check.to_lowercase() != "ok" {
        anyhow::bail!("shared-state sqlite quick_check failed: {}", quick_check);
    }
    Ok(())
}

pub fn upsert_shared_signal(cfg: &Config, input: &SharedSignalInput) -> Result<()> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;

    let created_at = input.created_at.clone().unwrap_or_else(now_rfc3339);
    let payload_json =
        serde_json::to_string(&input.payload_json).context("failed to serialize payload_json")?;

    conn.execute(
        "INSERT INTO shared_signals (
            signal_id, desk, source_venue, execution_adapter, account_scope,
            symbol, freshness_ms, confidence, payload_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
         ON CONFLICT(signal_id) DO UPDATE SET
            desk=excluded.desk,
            source_venue=excluded.source_venue,
            execution_adapter=excluded.execution_adapter,
            account_scope=excluded.account_scope,
            symbol=excluded.symbol,
            freshness_ms=excluded.freshness_ms,
            confidence=excluded.confidence,
            payload_json=excluded.payload_json,
            created_at=excluded.created_at",
        params![
            input.signal_id,
            input.desk,
            input.source_venue,
            input.execution_adapter,
            input.account_scope,
            input.symbol,
            input.freshness_ms,
            input.confidence,
            payload_json,
            created_at
        ],
    )
    .context("failed to upsert shared_signals row")?;

    Ok(())
}

pub fn insert_handoff(cfg: &Config, input: &DeskHandoffInput) -> Result<DeskHandoff> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;

    let created_at = input.created_at.clone().unwrap_or_else(now_rfc3339);
    let evidence_json = opt_json_string(&input.evidence_json)?;
    let risk_flags_json = opt_json_string(&input.risk_flags_json)?;
    let paper_trade_only = input.paper_trade_only.unwrap_or(true);

    conn.execute(
        "INSERT INTO desk_handoffs (
            desk, source_venue, symbol, signal_id, created_at,
            producer_role, producer_model, thesis, evidence_json, risk_flags_json,
            confidence, recommended_action, execution_adapter, account_scope, paper_trade_only
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15)",
        params![
            input.desk,
            input.source_venue,
            input.symbol,
            input.signal_id,
            created_at,
            input.producer_role,
            input.producer_model,
            input.thesis,
            evidence_json,
            risk_flags_json,
            input.confidence,
            input.recommended_action,
            input.execution_adapter,
            input.account_scope,
            if paper_trade_only { 1 } else { 0 },
        ],
    )
    .context("failed to insert desk_handoffs row")?;

    let row_id = conn.last_insert_rowid();
    fetch_handoff_by_id(&conn, row_id)?.ok_or_else(|| anyhow!("failed to fetch inserted handoff"))
}

pub fn list_handoffs(cfg: &Config, desk: Option<&str>, limit: usize) -> Result<Vec<DeskHandoff>> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;
    let limit = i64::try_from(limit.max(1)).unwrap_or(i64::MAX);

    let mut out = Vec::new();
    if let Some(desk_name) = desk {
        let mut stmt = conn
            .prepare(
                "SELECT id, desk, source_venue, symbol, signal_id, created_at,
                        producer_role, producer_model, thesis, evidence_json, risk_flags_json,
                        confidence, recommended_action, execution_adapter, account_scope, paper_trade_only
                 FROM desk_handoffs
                 WHERE desk = ?1
                 ORDER BY id DESC
                 LIMIT ?2",
            )
            .context("failed to prepare desk-filtered handoff query")?;
        let rows = stmt
            .query_map(params![desk_name, limit], map_handoff_row)
            .context("failed to execute desk-filtered handoff query")?;
        for row in rows {
            out.push(row.context("failed to decode handoff row")?);
        }
        return Ok(out);
    }

    let mut stmt = conn
        .prepare(
            "SELECT id, desk, source_venue, symbol, signal_id, created_at,
                    producer_role, producer_model, thesis, evidence_json, risk_flags_json,
                    confidence, recommended_action, execution_adapter, account_scope, paper_trade_only
             FROM desk_handoffs
             ORDER BY id DESC
             LIMIT ?1",
        )
        .context("failed to prepare handoff query")?;
    let rows = stmt
        .query_map(params![limit], map_handoff_row)
        .context("failed to execute handoff query")?;
    for row in rows {
        out.push(row.context("failed to decode handoff row")?);
    }
    Ok(out)
}

pub fn insert_risk_review(cfg: &Config, input: &RiskReviewInput) -> Result<i64> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;
    let created_at = input.created_at.clone().unwrap_or_else(now_rfc3339);
    conn.execute(
        "INSERT INTO risk_reviews (
            signal_id, desk, reviewer_role, score, decision, notes, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            input.signal_id,
            input.desk,
            input.reviewer_role,
            input.score,
            input.decision,
            input.notes,
            created_at
        ],
    )
    .context("failed to insert risk_reviews row")?;
    Ok(conn.last_insert_rowid())
}

pub fn insert_paper_order(cfg: &Config, input: &PaperOrderInput) -> Result<i64> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;
    let created_at = input.created_at.clone().unwrap_or_else(now_rfc3339);
    let details_json = opt_json_string(&input.details_json)?;
    conn.execute(
        "INSERT INTO paper_orders (
            signal_id, desk, symbol, side, quantity, venue, status, details_json, created_at
         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            input.signal_id,
            input.desk,
            input.symbol,
            input.side,
            input.quantity,
            input.venue,
            input.status,
            details_json,
            created_at
        ],
    )
    .context("failed to insert paper_orders row")?;
    Ok(conn.last_insert_rowid())
}

pub fn summary(cfg: &Config) -> Result<StateSummary> {
    let conn = open_conn(cfg)?;
    apply_schema(&conn)?;
    Ok(StateSummary {
        shared_signals: count_rows(&conn, "shared_signals")?,
        desk_handoffs: count_rows(&conn, "desk_handoffs")?,
        risk_reviews: count_rows(&conn, "risk_reviews")?,
        paper_orders: count_rows(&conn, "paper_orders")?,
        db_path: cfg.state_sql.sqlite_path.display().to_string(),
    })
}

fn open_conn(cfg: &Config) -> Result<Connection> {
    if let Some(parent) = cfg.state_sql.sqlite_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    Connection::open(&cfg.state_sql.sqlite_path)
        .with_context(|| format!("failed to open {}", cfg.state_sql.sqlite_path.display()))
}

fn apply_schema(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "PRAGMA journal_mode = WAL;
         PRAGMA foreign_keys = ON;
         PRAGMA busy_timeout = 5000;
         PRAGMA synchronous = NORMAL;
         PRAGMA temp_store = MEMORY;
         CREATE TABLE IF NOT EXISTS shared_signals (
             signal_id TEXT PRIMARY KEY,
             desk TEXT NOT NULL,
             source_venue TEXT NOT NULL,
             execution_adapter TEXT NOT NULL,
             account_scope TEXT NOT NULL,
             symbol TEXT NOT NULL,
             freshness_ms INTEGER NOT NULL,
             confidence REAL NOT NULL,
             payload_json TEXT NOT NULL,
             created_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_shared_signals_desk_created
             ON shared_signals(desk, created_at DESC);
         CREATE TABLE IF NOT EXISTS desk_handoffs (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             desk TEXT NOT NULL,
             source_venue TEXT NOT NULL,
             symbol TEXT,
             signal_id TEXT NOT NULL,
             created_at TEXT NOT NULL,
             producer_role TEXT NOT NULL,
             producer_model TEXT,
             thesis TEXT NOT NULL,
             evidence_json TEXT,
             risk_flags_json TEXT,
             confidence REAL,
             recommended_action TEXT,
             execution_adapter TEXT NOT NULL,
             account_scope TEXT NOT NULL,
             paper_trade_only INTEGER NOT NULL DEFAULT 1
         );
         CREATE INDEX IF NOT EXISTS idx_desk_handoffs_desk_created
             ON desk_handoffs(desk, created_at DESC);
         CREATE TABLE IF NOT EXISTS risk_reviews (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             signal_id TEXT NOT NULL,
             desk TEXT NOT NULL,
             reviewer_role TEXT NOT NULL,
             score REAL,
             decision TEXT NOT NULL,
             notes TEXT,
             created_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_risk_reviews_signal_created
             ON risk_reviews(signal_id, created_at DESC);
         CREATE TABLE IF NOT EXISTS paper_orders (
             id INTEGER PRIMARY KEY AUTOINCREMENT,
             signal_id TEXT NOT NULL,
             desk TEXT NOT NULL,
             symbol TEXT NOT NULL,
             side TEXT NOT NULL,
             quantity REAL NOT NULL,
             venue TEXT NOT NULL,
             status TEXT NOT NULL,
             details_json TEXT,
             created_at TEXT NOT NULL
         );
         CREATE INDEX IF NOT EXISTS idx_paper_orders_signal_created
             ON paper_orders(signal_id, created_at DESC);",
    )
    .context("failed to initialize shared state schema")?;

    let user_version: i64 = conn
        .query_row("PRAGMA user_version", [], |row| row.get(0))
        .context("failed to query shared-state sqlite user_version")?;
    if user_version < 1 {
        conn.execute_batch("PRAGMA user_version = 1;")
            .context("failed to set shared-state sqlite user_version")?;
    }

    Ok(())
}

fn fetch_handoff_by_id(conn: &Connection, id: i64) -> Result<Option<DeskHandoff>> {
    conn.query_row(
        "SELECT id, desk, source_venue, symbol, signal_id, created_at,
                producer_role, producer_model, thesis, evidence_json, risk_flags_json,
                confidence, recommended_action, execution_adapter, account_scope, paper_trade_only
         FROM desk_handoffs
         WHERE id = ?1",
        params![id],
        map_handoff_row,
    )
    .optional()
    .context("failed to fetch handoff by id")
}

fn map_handoff_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DeskHandoff> {
    let evidence_json_raw: Option<String> = row.get(9)?;
    let risk_flags_json_raw: Option<String> = row.get(10)?;
    let paper_trade_only_int: i64 = row.get(15)?;

    Ok(DeskHandoff {
        id: row.get(0)?,
        desk: row.get(1)?,
        source_venue: row.get(2)?,
        symbol: row.get(3)?,
        signal_id: row.get(4)?,
        created_at: row.get(5)?,
        producer_role: row.get(6)?,
        producer_model: row.get(7)?,
        thesis: row.get(8)?,
        evidence_json: parse_optional_json(evidence_json_raw),
        risk_flags_json: parse_optional_json(risk_flags_json_raw),
        confidence: row.get(11)?,
        recommended_action: row.get(12)?,
        execution_adapter: row.get(13)?,
        account_scope: row.get(14)?,
        paper_trade_only: paper_trade_only_int != 0,
    })
}

fn parse_optional_json(raw: Option<String>) -> Option<Value> {
    raw.and_then(|value| serde_json::from_str::<Value>(&value).ok())
}

fn opt_json_string(value: &Option<Value>) -> Result<Option<String>> {
    value
        .as_ref()
        .map(|item| serde_json::to_string(item).context("failed to serialize json field"))
        .transpose()
}

fn count_rows(conn: &Connection, table_name: &str) -> Result<i64> {
    let sql = format!("SELECT COUNT(*) FROM {table_name}");
    conn.query_row(&sql, [], |row| row.get(0))
        .with_context(|| format!("failed counting table {table_name}"))
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn can_insert_and_list_handoffs() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.state_sql.sqlite_path = tmp.path().join("shared-state.db");

        init_schema(&cfg).expect("init schema");
        upsert_shared_signal(
            &cfg,
            &SharedSignalInput {
                signal_id: "sig-1".to_string(),
                desk: "forex".to_string(),
                source_venue: "dukascopy".to_string(),
                execution_adapter: "paper_fx".to_string(),
                account_scope: "sandbox".to_string(),
                symbol: "EURUSD".to_string(),
                freshness_ms: 1000,
                confidence: 0.72,
                payload_json: serde_json::json!({"edge": "test"}),
                created_at: None,
            },
        )
        .expect("upsert signal");

        let inserted = insert_handoff(
            &cfg,
            &DeskHandoffInput {
                desk: "forex".to_string(),
                source_venue: "dukascopy".to_string(),
                symbol: Some("EURUSD".to_string()),
                signal_id: "sig-1".to_string(),
                created_at: None,
                producer_role: "desk_worker".to_string(),
                producer_model: Some("openai/gpt-4o-mini".to_string()),
                thesis: "mean reversion setup".to_string(),
                evidence_json: Some(serde_json::json!({"rsi": 28})),
                risk_flags_json: Some(serde_json::json!({"macro_event": false})),
                confidence: Some(0.72),
                recommended_action: Some("buy".to_string()),
                execution_adapter: "paper_fx".to_string(),
                account_scope: "sandbox".to_string(),
                paper_trade_only: Some(true),
            },
        )
        .expect("insert handoff");

        assert!(inserted.id > 0);
        let listed = list_handoffs(&cfg, Some("forex"), 10).expect("list handoffs");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].signal_id, "sig-1");

        let counts = summary(&cfg).expect("summary");
        assert_eq!(counts.shared_signals, 1);
        assert_eq!(counts.desk_handoffs, 1);

        sanity_check(&cfg).expect("sanity check");
    }
}
