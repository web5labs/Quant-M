use crate::config::Config;
use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params};
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MemoryEntry {
    pub id: i64,
    pub key: String,
    pub content: String,
    pub category: String,
    pub created_at: String,
    pub score: Option<f32>,
}

pub struct MemoryStore {
    core_markdown: PathBuf,
    daily_dir: PathBuf,
    vector_weight: f32,
    keyword_weight: f32,
    vector_dims: usize,
    conn: Connection,
}

impl MemoryStore {
    pub fn open(cfg: &Config) -> Result<Self> {
        if let Some(parent) = cfg.memory.sqlite_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        fs::create_dir_all(&cfg.memory.daily_dir)
            .with_context(|| format!("failed to create {}", cfg.memory.daily_dir.display()))?;

        let conn = Connection::open(&cfg.memory.sqlite_path).with_context(|| {
            format!(
                "failed to open sqlite memory DB {}",
                cfg.memory.sqlite_path.display()
            )
        })?;
        conn.execute_batch(
            "PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;
            PRAGMA busy_timeout = 5000;
            PRAGMA synchronous = NORMAL;
            PRAGMA temp_store = MEMORY;
            PRAGMA cache_size = -2000;
            CREATE TABLE IF NOT EXISTS memories (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                key TEXT NOT NULL,
                content TEXT NOT NULL,
                category TEXT NOT NULL,
                created_at TEXT NOT NULL,
                embedding TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_memories_category ON memories(category);
            CREATE INDEX IF NOT EXISTS idx_memories_created_at ON memories(created_at);",
        )
        .context("failed to initialize memory schema")?;

        let user_version: i64 = conn
            .query_row("PRAGMA user_version", [], |row| row.get(0))
            .context("failed to query memory sqlite user_version")?;
        if user_version < 1 {
            conn.execute_batch("PRAGMA user_version = 1;")
                .context("failed to set memory sqlite user_version")?;
        }

        let quick_check: String = conn
            .query_row("PRAGMA quick_check(1)", [], |row| row.get(0))
            .context("failed to run memory sqlite quick_check")?;
        if quick_check.to_lowercase() != "ok" {
            return Err(anyhow!("memory sqlite quick_check failed: {}", quick_check));
        }

        Ok(Self {
            core_markdown: cfg.memory.core_markdown.clone(),
            daily_dir: cfg.memory.daily_dir.clone(),
            vector_weight: cfg.memory.vector_weight,
            keyword_weight: cfg.memory.keyword_weight,
            vector_dims: cfg.memory.vector_dims.max(8),
            conn,
        })
    }

    pub fn add_entry(&self, key: &str, content: &str, category: &str) -> Result<MemoryEntry> {
        let created_at = Utc::now().to_rfc3339();
        let embedding = embed(content, self.vector_dims);
        let embedding_json =
            serde_json::to_string(&embedding).context("failed to serialize embedding")?;

        self.conn
            .prepare_cached(
                "INSERT INTO memories (key, content, category, created_at, embedding)
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .context("failed to prepare insert statement")?
            .execute(params![key, content, category, created_at, embedding_json])
            .context("failed to insert memory entry")?;

        self.append_markdown(key, content, category)?;

        Ok(MemoryEntry {
            id: self.conn.last_insert_rowid(),
            key: key.to_string(),
            content: content.to_string(),
            category: category.to_string(),
            created_at,
            score: None,
        })
    }

    pub fn list(&self, limit: usize, category: Option<&str>) -> Result<Vec<MemoryEntry>> {
        let rows = self.fetch_entries_with_embeddings(category, limit.max(1))?;
        Ok(rows.into_iter().map(|(entry, _)| entry).collect())
    }

    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<MemoryEntry>> {
        let query_embedding = embed(query, self.vector_dims);
        let query_tokens = tokenize(query);
        let mut scored: Vec<MemoryEntry> = Vec::new();

        for (mut entry, embedding) in self.fetch_entries_with_embeddings(None, 2_000)? {
            let vec_score = cosine_similarity(&query_embedding, &embedding);
            let key_score = keyword_score(&query_tokens, &entry.content);
            let recency_score = recency_decay(&entry.created_at);
            let final_score = self.vector_weight * vec_score
                + self.keyword_weight * key_score
                + 0.05 * recency_score;
            entry.score = Some(final_score);
            scored.push(entry);
        }

        scored.sort_by(|a, b| {
            b.score
                .unwrap_or_default()
                .partial_cmp(&a.score.unwrap_or_default())
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        scored.truncate(limit.max(1));
        Ok(scored)
    }

    pub fn count(&self) -> Result<usize> {
        let count: i64 = self
            .conn
            .query_row("SELECT COUNT(*) FROM memories", [], |row| row.get(0))
            .context("failed to count memories")?;
        Ok(usize::try_from(count).unwrap_or(0))
    }

    fn fetch_entries_with_embeddings(
        &self,
        category: Option<&str>,
        limit: usize,
    ) -> Result<Vec<(MemoryEntry, Vec<f32>)>> {
        let query = if category.is_some() {
            "SELECT id, key, content, category, created_at, embedding
             FROM memories WHERE category = ?1
             ORDER BY id DESC LIMIT ?2"
        } else {
            "SELECT id, key, content, category, created_at, embedding
             FROM memories
             ORDER BY id DESC LIMIT ?1"
        };

        let mut rows_out = Vec::new();
        if let Some(category) = category {
            let mut stmt = self
                .conn
                .prepare(query)
                .context("failed to prepare query")?;
            let mapped = stmt
                .query_map(
                    params![category, i64::try_from(limit).unwrap_or(i64::MAX)],
                    |row| {
                        let embedding_json: String = row.get(5)?;
                        let embedding: Vec<f32> =
                            serde_json::from_str(&embedding_json).unwrap_or_else(|_| vec![]);
                        Ok((
                            MemoryEntry {
                                id: row.get(0)?,
                                key: row.get(1)?,
                                content: row.get(2)?,
                                category: row.get(3)?,
                                created_at: row.get(4)?,
                                score: None,
                            },
                            embedding,
                        ))
                    },
                )
                .context("failed to query memory rows")?;

            for row in mapped {
                rows_out.push(row.context("failed to decode memory row")?);
            }
            return Ok(rows_out);
        }

        let mut stmt = self
            .conn
            .prepare(query)
            .context("failed to prepare query")?;
        let mapped = stmt
            .query_map(params![i64::try_from(limit).unwrap_or(i64::MAX)], |row| {
                let embedding_json: String = row.get(5)?;
                let embedding: Vec<f32> =
                    serde_json::from_str(&embedding_json).unwrap_or_else(|_| vec![]);
                Ok((
                    MemoryEntry {
                        id: row.get(0)?,
                        key: row.get(1)?,
                        content: row.get(2)?,
                        category: row.get(3)?,
                        created_at: row.get(4)?,
                        score: None,
                    },
                    embedding,
                ))
            })
            .context("failed to query memory rows")?;

        for row in mapped {
            rows_out.push(row.context("failed to decode memory row")?);
        }
        Ok(rows_out)
    }

    fn append_markdown(&self, key: &str, content: &str, category: &str) -> Result<()> {
        let (path, header) = if category.eq_ignore_ascii_case("core") {
            (self.core_markdown.clone(), "# MEMORY\n\n".to_string())
        } else {
            let date = Utc::now().format("%Y-%m-%d").to_string();
            (
                self.daily_dir.join(format!("{date}.md")),
                format!("# DAILY {date}\n\n"),
            )
        };

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create {}", parent.display()))?;
        }
        let line = format!("- **{key}**: {content}\n");

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .with_context(|| format!("failed to open {}", path.display()))?;
        let is_empty = file
            .metadata()
            .with_context(|| format!("failed to stat {}", path.display()))?
            .len()
            == 0;
        if is_empty {
            file.write_all(header.as_bytes())
                .with_context(|| format!("failed to write header {}", path.display()))?;
        }
        file.write_all(line.as_bytes())
            .with_context(|| format!("failed to append {}", path.display()))
    }
}

fn tokenize(text: &str) -> Vec<String> {
    text.split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|raw| {
            let t = raw.trim().to_lowercase();
            if t.is_empty() { None } else { Some(t) }
        })
        .collect()
}

fn embed(text: &str, dims: usize) -> Vec<f32> {
    let mut vector = vec![0.0_f32; dims.max(8)];
    for token in tokenize(text) {
        let mut hasher = DefaultHasher::new();
        token.hash(&mut hasher);
        let idx = (hasher.finish() as usize) % vector.len();
        vector[idx] += 1.0;
    }
    normalize(&mut vector);
    vector
}

fn normalize(vec: &mut [f32]) {
    let norm = vec.iter().map(|v| v * v).sum::<f32>().sqrt();
    if norm > 0.0 {
        for value in vec {
            *value /= norm;
        }
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.is_empty() || b.is_empty() || a.len() != b.len() {
        return 0.0;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum::<f32>()
}

fn keyword_score(query_tokens: &[String], content: &str) -> f32 {
    if query_tokens.is_empty() {
        return 0.0;
    }
    let body = content.to_lowercase();
    let matched = query_tokens
        .iter()
        .filter(|token| body.contains(token.as_str()))
        .count();
    matched as f32 / query_tokens.len() as f32
}

fn recency_decay(created_at: &str) -> f32 {
    let parsed: DateTime<Utc> = match DateTime::parse_from_rfc3339(created_at) {
        Ok(value) => value.with_timezone(&Utc),
        Err(_) => return 0.0,
    };
    let age_seconds = (Utc::now() - parsed).num_seconds().max(0) as f32;
    let age_days = age_seconds / 86_400.0;
    1.0 / (1.0 + age_days)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Config {
        let root = tmp.path().join("workspace");
        Config {
            workspace_dir: root.clone(),
            memory: crate::config::MemoryConfig {
                sqlite_path: root.join("memory/brain.db"),
                core_markdown: root.join("MEMORY.md"),
                daily_dir: root.join("daily"),
                vector_weight: 0.7,
                keyword_weight: 0.3,
                vector_dims: 64,
            },
            ..Config::default()
        }
    }

    #[test]
    fn add_and_count_memories() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let store = MemoryStore::open(&cfg).expect("memory open");
        store
            .add_entry("goal", "Track EURUSD breakout patterns", "core")
            .expect("insert");
        store
            .add_entry("daily-note", "Checked spread and swap rates", "daily")
            .expect("insert");
        assert_eq!(store.count().expect("count"), 2);
        assert!(cfg.memory.sqlite_path.exists());
    }

    #[test]
    fn hybrid_search_prioritizes_matching_text() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let store = MemoryStore::open(&cfg).expect("memory open");
        store
            .add_entry("alpha", "quant momentum signal on usd jpy", "daily")
            .expect("insert");
        store
            .add_entry("beta", "calendar reminder for dentist", "daily")
            .expect("insert");

        let results = store.search("usd momentum signal", 5).expect("search");
        assert!(!results.is_empty());
        assert_eq!(results[0].key, "alpha");
    }
}
