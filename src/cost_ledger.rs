use crate::config::Config;
use anyhow::{Context, Result};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostLedgerRecord {
    pub cost_record_id: String,
    pub session_id: String,
    pub workflow_id: String,
    pub workflow_kind: String,
    pub command: String,
    pub provider: String,
    pub model: String,
    pub estimated_cost: f64,
    pub actual_cost: f64,
    pub currency: String,
    pub dry_run: bool,
    pub created_at: String,
    pub input_units: Option<u64>,
    pub output_units: Option<u64>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostLedgerInvalidRecord {
    pub line_number: usize,
    pub error: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostSummary {
    pub total_estimated_cost: f64,
    pub total_actual_cost: f64,
    pub currency: String,
    pub record_count: usize,
    pub dry_run_count: usize,
    pub invalid_record_count: usize,
    pub by_workflow_kind: BTreeMap<String, CostSummaryBucket>,
    pub by_provider: BTreeMap<String, CostSummaryBucket>,
    pub latest_records: Vec<CostLedgerRecord>,
    pub invalid_records: Vec<CostLedgerInvalidRecord>,
    pub next_recommended_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CostSummaryBucket {
    pub record_count: usize,
    pub estimated_cost: f64,
    pub actual_cost: f64,
}

pub fn cost_ledger_path(cfg: &Config) -> PathBuf {
    cfg.workspace_dir.join("state/cost/cost-ledger.jsonl")
}

pub fn append_cost_record(cfg: &Config, record: &CostLedgerRecord) -> Result<()> {
    let path = cost_ledger_path(cfg);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let line = serde_json::to_string(record).context("failed to encode cost ledger record")?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    writeln!(file, "{line}").with_context(|| format!("failed to append {}", path.display()))
}

pub fn consensus_dry_run_record(
    session_id: &str,
    workflow_id: &str,
    command: &str,
    input_units: Option<u64>,
    output_units: Option<u64>,
) -> CostLedgerRecord {
    CostLedgerRecord {
        cost_record_id: format!(
            "cost:{}:{}",
            stable_hex(session_id),
            stable_hex(workflow_id)
        ),
        session_id: session_id.to_string(),
        workflow_id: workflow_id.to_string(),
        workflow_kind: "consensus_dry_run".to_string(),
        command: command.to_string(),
        provider: "mock".to_string(),
        model: "deterministic-reviewer-lanes".to_string(),
        estimated_cost: 0.0,
        actual_cost: 0.0,
        currency: "USD".to_string(),
        dry_run: true,
        created_at: Utc::now().to_rfc3339(),
        input_units,
        output_units,
        notes: "$0.00 actual, mock-only".to_string(),
    }
}

pub fn summarize_costs(
    cfg: &Config,
    workflow: Option<&str>,
    session: Option<&str>,
) -> Result<CostSummary> {
    let path = cost_ledger_path(cfg);
    let (records, invalid_records) = read_records(&path)?;
    let mut records: Vec<_> = records
        .into_iter()
        .filter(|record| workflow.is_none_or(|value| record.workflow_id == value))
        .filter(|record| session.is_none_or(|value| record.session_id == value))
        .collect();
    records.sort_by(|a, b| a.created_at.cmp(&b.created_at));

    let total_estimated_cost = round8(records.iter().map(|record| record.estimated_cost).sum());
    let total_actual_cost = round8(records.iter().map(|record| record.actual_cost).sum());
    let dry_run_count = records.iter().filter(|record| record.dry_run).count();
    let by_workflow_kind = bucket_by(&records, |record| record.workflow_kind.as_str());
    let by_provider = bucket_by(&records, |record| record.provider.as_str());
    let mut latest_records = records.iter().rev().take(5).cloned().collect::<Vec<_>>();
    latest_records.reverse();
    let next_recommended_command = if records.is_empty() {
        "quant-m consensus --dry-run \"Should we adopt this API design?\"".to_string()
    } else {
        "quant-m state review --domain consensus".to_string()
    };

    Ok(CostSummary {
        total_estimated_cost,
        total_actual_cost,
        currency: "USD".to_string(),
        record_count: records.len(),
        dry_run_count,
        invalid_record_count: invalid_records.len(),
        by_workflow_kind,
        by_provider,
        latest_records,
        invalid_records,
        next_recommended_command,
    })
}

pub fn render_cost_summary(summary: &CostSummary) -> String {
    if summary.record_count == 0 {
        return format!(
            "Cost summary\nrecords: 0\ninvalid_records: {}\ntotal_estimated_cost: {}\ntotal_actual_cost: {}\nnext: {}\n",
            summary.invalid_record_count,
            format_currency_amount(summary.total_estimated_cost, &summary.currency),
            format_currency_amount(summary.total_actual_cost, &summary.currency),
            summary.next_recommended_command
        );
    }
    let latest_session = summary
        .latest_records
        .last()
        .map(|record| record.session_id.as_str())
        .unwrap_or("none");
    format!(
        "Cost summary\nrecords: {}\ndry_run_records: {}\ninvalid_records: {}\ntotal_estimated_cost: {}\ntotal_actual_cost: {}\nby_workflow_kind: {}\nby_provider: {}\nlatest_session_id: {}\nnext: {}\n",
        summary.record_count,
        summary.dry_run_count,
        summary.invalid_record_count,
        format_currency_amount(summary.total_estimated_cost, &summary.currency),
        format_currency_amount(summary.total_actual_cost, &summary.currency),
        format_buckets(&summary.by_workflow_kind),
        format_buckets(&summary.by_provider),
        latest_session,
        summary.next_recommended_command
    )
}

fn read_records(path: &Path) -> Result<(Vec<CostLedgerRecord>, Vec<CostLedgerInvalidRecord>)> {
    if !path.exists() {
        return Ok((vec![], vec![]));
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut records = Vec::new();
    let mut invalid = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<CostLedgerRecord>(line) {
            Ok(record) => records.push(record),
            Err(err) => invalid.push(CostLedgerInvalidRecord {
                line_number: index + 1,
                error: err.to_string(),
            }),
        }
    }
    Ok((records, invalid))
}

fn bucket_by<'a>(
    records: &'a [CostLedgerRecord],
    key_for: impl Fn(&'a CostLedgerRecord) -> &'a str,
) -> BTreeMap<String, CostSummaryBucket> {
    let mut buckets = BTreeMap::new();
    for record in records {
        let key = key_for(record).to_string();
        let bucket = buckets.entry(key).or_insert(CostSummaryBucket {
            record_count: 0,
            estimated_cost: 0.0,
            actual_cost: 0.0,
        });
        bucket.record_count += 1;
        bucket.estimated_cost += record.estimated_cost;
        bucket.actual_cost += record.actual_cost;
    }
    for bucket in buckets.values_mut() {
        bucket.estimated_cost = round8(bucket.estimated_cost);
        bucket.actual_cost = round8(bucket.actual_cost);
    }
    buckets
}

fn format_buckets(buckets: &BTreeMap<String, CostSummaryBucket>) -> String {
    if buckets.is_empty() {
        return "none".to_string();
    }
    buckets
        .iter()
        .map(|(key, bucket)| {
            format!(
                "{} records={} estimated={} actual={}",
                key,
                bucket.record_count,
                format_currency_amount(bucket.estimated_cost, "USD"),
                format_currency_amount(bucket.actual_cost, "USD")
            )
        })
        .collect::<Vec<_>>()
        .join("; ")
}

pub fn format_currency_amount(value: f64, currency: &str) -> String {
    let value = round8(value);
    let value = if value.abs() < 0.000000005 {
        0.0
    } else {
        value
    };
    if currency.eq_ignore_ascii_case("usd") {
        format!("${value:.8} USD")
    } else {
        format!("{value:.8} {}", currency.trim())
    }
}

fn stable_hex(value: &str) -> String {
    let mut hash = 2166136261u32;
    for byte in value.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16777619);
    }
    format!("{hash:08x}")
}

fn round8(value: f64) -> f64 {
    let rounded = (value * 100_000_000.0).round() / 100_000_000.0;
    if rounded.abs() < 0.00000001 {
        0.0
    } else {
        rounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().join("workspace");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        (tmp, cfg)
    }

    #[test]
    fn append_and_summarize_one_record() {
        let (_tmp, cfg) = temp_cfg();
        let record = consensus_dry_run_record(
            "session-1",
            "workflow-1",
            "quant-m consensus",
            Some(4),
            Some(3),
        );
        append_cost_record(&cfg, &record).expect("append");
        let summary = summarize_costs(&cfg, None, None).expect("summary");

        assert_eq!(summary.record_count, 1);
        assert_eq!(summary.dry_run_count, 1);
        assert_eq!(summary.total_actual_cost, 0.0);
        assert_eq!(summary.latest_records[0].session_id, "session-1");
        assert_eq!(summary.latest_records[0].provider, "mock");
        assert_eq!(
            summary.latest_records[0].model,
            "deterministic-reviewer-lanes"
        );
    }

    #[test]
    fn multiple_records_aggregate() {
        let (_tmp, cfg) = temp_cfg();
        append_cost_record(
            &cfg,
            &consensus_dry_run_record("session-1", "workflow-1", "cmd", None, None),
        )
        .expect("append 1");
        append_cost_record(
            &cfg,
            &consensus_dry_run_record("session-2", "workflow-2", "cmd", None, None),
        )
        .expect("append 2");
        let summary = summarize_costs(&cfg, None, None).expect("summary");

        assert_eq!(summary.record_count, 2);
        assert_eq!(
            summary.by_workflow_kind["consensus_dry_run"].record_count,
            2
        );
        assert_eq!(summary.by_provider["mock"].record_count, 2);
    }

    #[test]
    fn filters_by_workflow_and_session() {
        let (_tmp, cfg) = temp_cfg();
        append_cost_record(
            &cfg,
            &consensus_dry_run_record("session-1", "workflow-1", "cmd", None, None),
        )
        .expect("append 1");
        append_cost_record(
            &cfg,
            &consensus_dry_run_record("session-2", "workflow-2", "cmd", None, None),
        )
        .expect("append 2");

        assert_eq!(
            summarize_costs(&cfg, Some("workflow-1"), None)
                .expect("workflow")
                .record_count,
            1
        );
        assert_eq!(
            summarize_costs(&cfg, None, Some("session-2"))
                .expect("session")
                .record_count,
            1
        );
    }

    #[test]
    fn empty_ledger_is_helpful() {
        let (_tmp, cfg) = temp_cfg();
        let summary = summarize_costs(&cfg, None, None).expect("summary");
        let rendered = render_cost_summary(&summary);

        assert_eq!(summary.record_count, 0);
        assert_eq!(summary.total_estimated_cost, 0.0);
        assert_eq!(summary.total_actual_cost, 0.0);
        assert!(rendered.contains("records: 0"));
        assert!(rendered.contains("total_actual_cost: $0.00000000 USD"));
        assert!(rendered.contains("quant-m consensus --dry-run"));
    }

    #[test]
    fn tiny_costs_render_as_precise_dollar_amounts() {
        let (_tmp, cfg) = temp_cfg();
        let mut record =
            consensus_dry_run_record("session-tiny", "workflow-tiny", "cmd", Some(19), None);
        record.estimated_cost = 0.000019;
        record.actual_cost = 0.000019;
        append_cost_record(&cfg, &record).expect("append");

        let summary = summarize_costs(&cfg, None, None).expect("summary");
        let rendered = render_cost_summary(&summary);

        assert_eq!(summary.total_estimated_cost, 0.000019);
        assert_eq!(summary.total_actual_cost, 0.000019);
        assert!(rendered.contains("total_estimated_cost: $0.00001900 USD"));
        assert!(rendered.contains("total_actual_cost: $0.00001900 USD"));
        assert!(rendered.contains("estimated=$0.00001900 USD"));
    }

    #[test]
    fn malformed_records_are_reported() {
        let (_tmp, cfg) = temp_cfg();
        let path = cost_ledger_path(&cfg);
        fs::create_dir_all(path.parent().expect("parent")).expect("dir");
        fs::write(&path, "{not-json}\n").expect("write malformed");
        let summary = summarize_costs(&cfg, None, None).expect("summary");

        assert_eq!(summary.invalid_record_count, 1);
        assert_eq!(summary.record_count, 0);
    }

    #[test]
    fn summary_does_not_mutate_ledger() {
        let (_tmp, cfg) = temp_cfg();
        append_cost_record(
            &cfg,
            &consensus_dry_run_record("session-1", "workflow-1", "cmd", None, None),
        )
        .expect("append");
        let before = fs::read(cost_ledger_path(&cfg)).expect("before");
        summarize_costs(&cfg, None, None).expect("summary");
        let after = fs::read(cost_ledger_path(&cfg)).expect("after");
        assert_eq!(before, after);
    }

    #[test]
    fn summary_does_not_require_provider_keys_or_network() {
        let (_tmp, mut cfg) = temp_cfg();
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        append_cost_record(
            &cfg,
            &consensus_dry_run_record("session-1", "workflow-1", "cmd", None, None),
        )
        .expect("append");
        let summary = summarize_costs(&cfg, None, None).expect("summary");
        assert_eq!(summary.total_actual_cost, 0.0);
    }
}
