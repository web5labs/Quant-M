use crate::config::Config;
use crate::sessions::{DomainId, SessionId};
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use redb::{Database, ReadOnlyDatabase, ReadableDatabase, ReadableTable, TableDefinition};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;
use std::str::FromStr;

const HOT_STATE_TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("shared_state_v1");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[serde(transparent)]
pub struct SharedStateKey(String);

impl SharedStateKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for SharedStateKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

impl FromStr for SharedStateKey {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            return Err(anyhow!("SharedStateKey is empty"));
        }
        Ok(Self::new(trimmed))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SharedStateValue {
    Text(String),
    Json(Value),
    Number(f64),
    Bool(bool),
    Timestamp(String),
    Score(f64),
    Status(String),
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SharedStateRecord {
    pub key: SharedStateKey,
    pub value: SharedStateValue,
    pub domain_id: DomainId,
    pub source: String,
    pub confidence: f64,
    pub updated_at: String,
    pub expires_at: Option<String>,
    pub session_id: Option<SessionId>,
}

#[allow(dead_code)]
pub trait SharedStateStore {
    fn put(&self, record: SharedStateRecord) -> Result<()>;
    fn get(&self, key: &SharedStateKey) -> Result<Option<SharedStateRecord>>;
    fn list(&self, domain_id: Option<&DomainId>) -> Result<Vec<SharedStateRecord>>;
    fn expire_stale(&self, now: &str) -> Result<usize>;
    fn snapshot(&self, domain_id: Option<&DomainId>) -> Result<Vec<SharedStateRecord>>;
}

pub struct HybridSharedStateStore {
    sqlite_path: PathBuf,
    redb_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedStateExpireSummary {
    pub expired: usize,
    pub as_of: String,
}

impl HybridSharedStateStore {
    pub fn from_config(cfg: &Config) -> Self {
        Self {
            sqlite_path: cfg.state_sql.sqlite_path.clone(),
            redb_path: hot_store_path(cfg),
        }
    }

    fn ensure_hot_parent_dir(&self) -> Result<()> {
        if let Some(parent) = self.redb_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        Ok(())
    }

    fn open_hot_db_for_write(&self) -> Result<Database> {
        self.ensure_hot_parent_dir()?;
        Database::create(&self.redb_path)
            .with_context(|| format!("failed to open {}", self.redb_path.display()))
    }

    fn open_hot_db_for_read(&self) -> Result<Option<ReadOnlyDatabase>> {
        if !self.redb_path.exists() {
            return Ok(None);
        }
        ReadOnlyDatabase::open(&self.redb_path)
            .map(Some)
            .with_context(|| format!("failed to open {}", self.redb_path.display()))
    }

    fn ensure_hot_table(db: &Database) -> Result<()> {
        let write_txn = db
            .begin_write()
            .context("failed to start shared-state redb write txn")?;
        {
            let _ = write_txn
                .open_table(HOT_STATE_TABLE)
                .context("failed to open shared-state redb table")?;
        }
        write_txn
            .commit()
            .context("failed to commit shared-state redb table init")?;
        Ok(())
    }

    fn open_history_conn(&self) -> Result<Connection> {
        if let Some(parent) = self.sqlite_path.parent() {
            std::fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let conn = Connection::open(&self.sqlite_path)
            .with_context(|| format!("failed to open {}", self.sqlite_path.display()))?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
             PRAGMA foreign_keys = ON;
             PRAGMA busy_timeout = 5000;
             PRAGMA synchronous = NORMAL;
             CREATE TABLE IF NOT EXISTS shared_state_history (
                 id INTEGER PRIMARY KEY AUTOINCREMENT,
                 state_key TEXT NOT NULL,
                 value_json TEXT NOT NULL,
                 domain_id TEXT NOT NULL,
                 source TEXT NOT NULL,
                 confidence REAL NOT NULL,
                 updated_at TEXT NOT NULL,
                 expires_at TEXT,
                 session_id TEXT,
                 operation TEXT NOT NULL
             );
             CREATE INDEX IF NOT EXISTS idx_shared_state_history_key_updated
                 ON shared_state_history(state_key, updated_at DESC);
             CREATE INDEX IF NOT EXISTS idx_shared_state_history_domain_updated
                 ON shared_state_history(domain_id, updated_at DESC);",
        )
        .context("failed to initialize shared-state history schema")?;
        Ok(conn)
    }

    fn open_history_conn_for_read(&self) -> Result<Option<Connection>> {
        if !self.sqlite_path.exists() {
            return Ok(None);
        }
        let conn = Connection::open(&self.sqlite_path)
            .with_context(|| format!("failed to open {}", self.sqlite_path.display()))?;
        conn.execute_batch(
            "PRAGMA busy_timeout = 5000;
             PRAGMA foreign_keys = ON;",
        )
        .context("failed to configure shared-state history read connection")?;
        Ok(Some(conn))
    }

    fn history_table_exists(conn: &Connection) -> Result<bool> {
        conn.query_row(
            "SELECT EXISTS(
                SELECT 1
                FROM sqlite_master
                WHERE type = 'table' AND name = 'shared_state_history'
            )",
            [],
            |row| row.get::<_, i64>(0),
        )
        .map(|value| value != 0)
        .context("failed to inspect shared-state history schema")
    }

    fn snapshot_from_history(
        &self,
        domain_id: Option<&DomainId>,
    ) -> Result<Vec<SharedStateRecord>> {
        let Some(conn) = self.open_history_conn_for_read()? else {
            return Ok(vec![]);
        };
        if !Self::history_table_exists(&conn)? {
            return Ok(vec![]);
        }

        let mut stmt = conn
            .prepare(
                "SELECT state_key, value_json, domain_id, source, confidence,
                        updated_at, expires_at, session_id, operation
                 FROM shared_state_history
                 ORDER BY id ASC",
            )
            .context("failed to prepare shared-state history snapshot query")?;
        let mut rows = stmt
            .query([])
            .context("failed to query shared-state history snapshot rows")?;
        let mut current = BTreeMap::<SharedStateKey, SharedStateRecord>::new();

        while let Some(row) = rows
            .next()
            .context("failed to read shared-state history snapshot row")?
        {
            let key = SharedStateKey::from_str(
                &row.get::<_, String>(0)
                    .context("failed to decode shared-state history key")?,
            )?;
            let value = serde_json::from_str::<SharedStateValue>(
                &row.get::<_, String>(1)
                    .context("failed to decode shared-state history value")?,
            )
            .context("failed to parse shared-state history value_json")?;
            let record = SharedStateRecord {
                key: key.clone(),
                value,
                domain_id: DomainId::from_str(
                    &row.get::<_, String>(2)
                        .context("failed to decode shared-state history domain_id")?,
                )?,
                source: row
                    .get::<_, String>(3)
                    .context("failed to decode shared-state history source")?,
                confidence: row
                    .get::<_, f64>(4)
                    .context("failed to decode shared-state history confidence")?,
                updated_at: row
                    .get::<_, String>(5)
                    .context("failed to decode shared-state history updated_at")?,
                expires_at: row
                    .get::<_, Option<String>>(6)
                    .context("failed to decode shared-state history expires_at")?,
                session_id: row
                    .get::<_, Option<String>>(7)
                    .context("failed to decode shared-state history session_id")?
                    .map(|value| SessionId::from_str(&value))
                    .transpose()?,
            };
            let operation = row
                .get::<_, String>(8)
                .context("failed to decode shared-state history operation")?;
            if operation == "expired" {
                current.remove(&key);
            } else {
                current.insert(key, record);
            }
        }

        let mut records: Vec<SharedStateRecord> = current
            .into_values()
            .filter(|record| domain_id.is_none_or(|value| &record.domain_id == value))
            .collect();
        records.sort_by(|a, b| {
            a.key
                .cmp(&b.key)
                .then_with(|| a.updated_at.cmp(&b.updated_at))
                .then_with(|| a.source.cmp(&b.source))
        });
        Ok(records)
    }

    fn get_from_history(&self, key: &SharedStateKey) -> Result<Option<SharedStateRecord>> {
        Ok(self
            .snapshot_from_history(None)?
            .into_iter()
            .find(|record| &record.key == key))
    }

    fn append_history(&self, record: &SharedStateRecord, operation: &str) -> Result<()> {
        let conn = self.open_history_conn()?;
        let value_json = serde_json::to_string(&record.value)
            .context("failed to serialize shared-state value")?;
        conn.execute(
            "INSERT INTO shared_state_history (
                state_key, value_json, domain_id, source, confidence,
                updated_at, expires_at, session_id, operation
             ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            params![
                record.key.as_str(),
                value_json,
                record.domain_id.as_str(),
                record.source,
                record.confidence,
                record.updated_at,
                record.expires_at,
                record.session_id.as_ref().map(|value| value.as_str()),
                operation,
            ],
        )
        .context("failed to append shared-state history row")?;
        Ok(())
    }
}

impl SharedStateStore for HybridSharedStateStore {
    fn put(&self, record: SharedStateRecord) -> Result<()> {
        validate_record(&record)?;
        let db = self.open_hot_db_for_write()?;
        Self::ensure_hot_table(&db)?;
        let write_txn = db
            .begin_write()
            .context("failed to start shared-state redb write txn")?;
        {
            let mut table = write_txn
                .open_table(HOT_STATE_TABLE)
                .context("failed to open shared-state redb table for write")?;
            let encoded =
                serde_json::to_vec(&record).context("failed to encode shared-state record")?;
            table
                .insert(record.key.as_str(), encoded.as_slice())
                .with_context(|| format!("failed to upsert shared-state key '{}'", record.key))?;
        }
        write_txn
            .commit()
            .context("failed to commit shared-state redb write txn")?;
        self.append_history(&record, "put")
    }

    fn get(&self, key: &SharedStateKey) -> Result<Option<SharedStateRecord>> {
        let Some(db) = self.open_hot_db_for_read()? else {
            return Ok(None);
        };
        let read_txn = db
            .begin_read()
            .context("failed to start shared-state redb read txn")?;
        let table = match read_txn.open_table(HOT_STATE_TABLE) {
            Ok(table) => table,
            Err(_) => return Ok(None),
        };
        let maybe = table
            .get(key.as_str())
            .with_context(|| format!("failed to read shared-state key '{}'", key))?;
        maybe
            .map(|value| {
                serde_json::from_slice::<SharedStateRecord>(value.value())
                    .context("failed to decode shared-state record")
            })
            .transpose()
    }

    fn list(&self, domain_id: Option<&DomainId>) -> Result<Vec<SharedStateRecord>> {
        self.snapshot(domain_id)
    }

    fn expire_stale(&self, now: &str) -> Result<usize> {
        let now_ts = parse_timestamp(now)?;
        let existing = self.snapshot(None)?;
        let stale: Vec<SharedStateRecord> = existing
            .into_iter()
            .filter(|record| {
                record.expires_at.as_deref().is_some_and(|expires_at| {
                    parse_timestamp(expires_at).is_ok_and(|ts| ts <= now_ts)
                })
            })
            .collect();
        if stale.is_empty() {
            return Ok(0);
        }

        let db = self.open_hot_db_for_write()?;
        Self::ensure_hot_table(&db)?;
        let write_txn = db
            .begin_write()
            .context("failed to start shared-state redb delete txn")?;
        {
            let mut table = write_txn
                .open_table(HOT_STATE_TABLE)
                .context("failed to open shared-state redb table for delete")?;
            for record in &stale {
                table
                    .remove(record.key.as_str())
                    .with_context(|| format!("failed to remove stale key '{}'", record.key))?;
            }
        }
        write_txn
            .commit()
            .context("failed to commit shared-state redb delete txn")?;
        for record in &stale {
            self.append_history(record, "expired")?;
        }
        Ok(stale.len())
    }

    fn snapshot(&self, domain_id: Option<&DomainId>) -> Result<Vec<SharedStateRecord>> {
        let Some(db) = self.open_hot_db_for_read()? else {
            return Ok(vec![]);
        };
        let read_txn = db
            .begin_read()
            .context("failed to start shared-state redb read txn")?;
        let table = match read_txn.open_table(HOT_STATE_TABLE) {
            Ok(table) => table,
            Err(_) => return Ok(vec![]),
        };
        let mut records = Vec::new();
        for row in table
            .iter()
            .context("failed to iterate shared-state redb snapshot")?
        {
            let (_key, value) = row.context("failed to decode shared-state redb row")?;
            let record = serde_json::from_slice::<SharedStateRecord>(value.value())
                .context("failed to decode shared-state snapshot record")?;
            if domain_id.is_none_or(|value| &record.domain_id == value) {
                records.push(record);
            }
        }
        records.sort_by(|a, b| {
            a.key
                .cmp(&b.key)
                .then_with(|| a.updated_at.cmp(&b.updated_at))
                .then_with(|| a.source.cmp(&b.source))
        });
        Ok(records)
    }
}

pub fn hot_store_path(cfg: &Config) -> PathBuf {
    cfg.workspace_dir.join("state").join("shared-state.redb")
}

pub fn list_state(cfg: &Config, domain_id: Option<&DomainId>) -> Result<Vec<SharedStateRecord>> {
    HybridSharedStateStore::from_config(cfg).snapshot_from_history(domain_id)
}

pub fn show_state(cfg: &Config, key: &SharedStateKey) -> Result<Option<SharedStateRecord>> {
    HybridSharedStateStore::from_config(cfg).get_from_history(key)
}

pub fn snapshot_state(
    cfg: &Config,
    domain_id: Option<&DomainId>,
) -> Result<Vec<SharedStateRecord>> {
    HybridSharedStateStore::from_config(cfg).snapshot_from_history(domain_id)
}

pub fn expire_stale_now(cfg: &Config) -> Result<SharedStateExpireSummary> {
    let now = now_rfc3339();
    let expired = HybridSharedStateStore::from_config(cfg).expire_stale(&now)?;
    Ok(SharedStateExpireSummary {
        expired,
        as_of: now,
    })
}

#[allow(dead_code)]
fn validate_record(record: &SharedStateRecord) -> Result<()> {
    if record.key.as_str().trim().is_empty() {
        return Err(anyhow!("shared-state key is empty"));
    }
    if record.source.trim().is_empty() {
        return Err(anyhow!("shared-state source is empty"));
    }
    if !(0.0..=1.0).contains(&record.confidence) {
        return Err(anyhow!(
            "shared-state confidence must be between 0.0 and 1.0"
        ));
    }
    let updated_at = parse_timestamp(&record.updated_at)?;
    if let Some(expires_at) = &record.expires_at {
        let expires = parse_timestamp(expires_at)?;
        if expires < updated_at {
            return Err(anyhow!(
                "shared-state expires_at cannot be earlier than updated_at"
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "fuzzing_hooks")]
#[allow(dead_code)]
pub fn parse_and_validate_record_for_fuzz(raw: &str) -> Result<()> {
    let record = serde_json::from_str::<SharedStateRecord>(raw)?;
    validate_record(&record)
}

fn parse_timestamp(value: &str) -> Result<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .with_context(|| format!("invalid RFC3339 timestamp '{}'", value))
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sessions::{DomainId, SessionId};
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        (tmp, cfg)
    }

    fn sample_record(key: &str) -> SharedStateRecord {
        SharedStateRecord {
            key: SharedStateKey::new(key),
            value: SharedStateValue::Status("ready".to_string()),
            domain_id: DomainId::new("domain:test"),
            source: "test".to_string(),
            confidence: 0.8,
            updated_at: "2026-05-31T00:00:00+00:00".to_string(),
            expires_at: None,
            session_id: Some(SessionId::new("session:test")),
        }
    }

    #[test]
    fn shared_state_writes_and_reads_typed_records() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let record = SharedStateRecord {
            value: SharedStateValue::Json(serde_json::json!({"edge": 1})),
            ..sample_record("alpha")
        };
        store.put(record.clone()).expect("put record");

        let fetched = store
            .get(&SharedStateKey::new("alpha"))
            .expect("get record")
            .expect("record exists");
        assert_eq!(fetched, record);
    }

    #[test]
    fn stale_records_expire() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let fresh = sample_record("fresh");
        let stale = SharedStateRecord {
            expires_at: Some("2026-05-31T01:00:00+00:00".to_string()),
            ..sample_record("stale")
        };
        store.put(fresh).expect("put fresh");
        store.put(stale).expect("put stale");

        let expired = store
            .expire_stale("2026-05-31T02:00:00+00:00")
            .expect("expire stale");
        assert_eq!(expired, 1);
        assert!(
            store
                .get(&SharedStateKey::new("stale"))
                .expect("get stale")
                .is_none()
        );
    }

    #[test]
    fn snapshots_are_deterministic() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        store.put(sample_record("bravo")).expect("put bravo");
        store.put(sample_record("alpha")).expect("put alpha");

        let a = store.snapshot(None).expect("snapshot a");
        let b = store.snapshot(None).expect("snapshot b");
        assert_eq!(a, b);
        assert_eq!(a[0].key.as_str(), "alpha");
        assert_eq!(a[1].key.as_str(), "bravo");
    }

    #[test]
    fn session_id_links_state_to_session_evidence() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let record = sample_record("linked");
        let expected = record.session_id.clone();
        store.put(record).expect("put linked");

        let fetched = store
            .get(&SharedStateKey::new("linked"))
            .expect("get linked")
            .expect("exists");
        assert_eq!(fetched.session_id, expected);
    }

    #[test]
    fn no_live_trading_or_external_calls_occur() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let record = SharedStateRecord {
            value: SharedStateValue::Text("offline".to_string()),
            ..sample_record("quiet")
        };
        store.put(record).expect("put quiet");
        let snapshot = store.snapshot(None).expect("snapshot");
        assert_eq!(snapshot.len(), 1);
        assert_eq!(snapshot[0].source, "test");
    }

    #[test]
    fn parallel_shared_state_readers_can_open_redb_safely() {
        use std::sync::{Arc, Barrier};
        use std::thread;

        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        store
            .put(sample_record("alpha"))
            .expect("seed shared state");

        let cfg = Arc::new(cfg);
        let barrier = Arc::new(Barrier::new(3));
        let mut threads = Vec::new();
        for _ in 0..2 {
            let cfg = Arc::clone(&cfg);
            let barrier = Arc::clone(&barrier);
            threads.push(thread::spawn(move || {
                barrier.wait();
                snapshot_state(&cfg, None).expect("parallel snapshot")
            }));
        }

        barrier.wait();
        let snapshots: Vec<Vec<SharedStateRecord>> = threads
            .into_iter()
            .map(|thread| thread.join().expect("join parallel reader"))
            .collect();
        assert_eq!(snapshots.len(), 2);
        assert_eq!(snapshots[0], snapshots[1]);
        assert_eq!(snapshots[0].len(), 1);
        assert_eq!(snapshots[0][0].key.as_str(), "alpha");
    }

    #[test]
    fn history_snapshot_reconstructs_current_state_for_cli_inspection() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        store.put(sample_record("alpha")).expect("put alpha");
        store
            .put(SharedStateRecord {
                value: SharedStateValue::Number(2.0),
                ..sample_record("bravo")
            })
            .expect("put bravo");
        store
            .expire_stale("2026-05-30T23:59:59+00:00")
            .expect("expire none");

        let snapshot = snapshot_state(&cfg, None).expect("history snapshot");
        assert_eq!(snapshot.len(), 2);
        assert_eq!(snapshot[0].key.as_str(), "alpha");
        assert_eq!(snapshot[1].key.as_str(), "bravo");
    }
}
