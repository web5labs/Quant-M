use crate::compaction;
use crate::config::Config;
use crate::context_status::{self, ContextState};
use crate::cost_ledger::format_currency_amount;
use crate::fsm_core::{ContextGuardianState, ContextRecommendedAction};
use crate::sessions::{self, SessionId};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct BoilReport {
    pub session_id: String,
    pub created_at: String,
    pub dry_run: bool,
    pub compact_packet_created: bool,
    pub raw_context_paths: Vec<PathBuf>,
    pub boiled_context_paths: Vec<PathBuf>,
    pub report_markdown_path: Option<PathBuf>,
    pub report_json_path: Option<PathBuf>,
    pub token_estimate_method: String,
    pub token_estimate_confidence: String,
    pub pricing_profile: String,
    pub pricing_profile_source: String,
    pub pricing_profile_updated_at: Option<String>,
    pub raw_input_tokens_estimate: usize,
    pub boiled_input_tokens_estimate: usize,
    pub expected_output_tokens_estimate: usize,
    pub token_savings: isize,
    pub compression_ratio: f64,
    pub raw_estimated_cost: f64,
    pub boiled_estimated_cost: f64,
    pub estimated_cost_avoided: f64,
    pub raw_estimated_cost_display: String,
    pub boiled_estimated_cost_display: String,
    pub estimated_cost_avoided_display: String,
    pub packet_status: BoilPacketStatus,
    pub context_state: ContextState,
    pub guardian_state: ContextGuardianState,
    pub recommended_action: ContextRecommendedAction,
    pub blocked: bool,
    pub evidence_ref_count: usize,
    pub excluded_context_count: usize,
    pub risk_count: usize,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum BoilPacketStatus {
    Created,
    Present,
    Missing,
    Unsafe,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoilRequest {
    pub session_id: SessionId,
    pub dry_run: bool,
    pub pricing_profile: String,
}

#[derive(Debug, Clone, PartialEq)]
struct BoilPricingProfile {
    name: String,
    input_cost_per_million_tokens: f64,
    output_cost_per_million_tokens: f64,
    expected_output_tokens: usize,
    source: String,
    updated_at: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct CompactPacketForBoil {
    evidence_refs: Vec<BoilEvidenceRef>,
    open_risks: Vec<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoilEvidenceRef {
    pub sequence: u64,
    pub step_id: String,
    pub occurred_at: String,
    pub event_kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BoilEvidenceLookup {
    pub session_id: String,
    pub evidence_id: String,
    pub compact_packet_path: PathBuf,
    pub matched: Option<BoilEvidenceRef>,
    pub warnings: Vec<String>,
}

pub fn run_boil(cfg: &Config, request: BoilRequest) -> Result<BoilReport> {
    sessions::show_session(cfg, &request.session_id)?;
    let created_at = Utc::now().to_rfc3339();
    let pricing = pricing_profile(&request.pricing_profile)?;
    let mut warnings = Vec::new();
    let raw_context_paths = raw_context_paths(cfg, &request.session_id);
    let compact_paths = compact_paths(cfg, &request.session_id);
    let mut compact_packet_created = false;

    let mut boiled_context_paths = existing_compact_paths(&compact_paths);
    let packet_status = if compact_paths.compact_json.exists() {
        BoilPacketStatus::Present
    } else if request.dry_run {
        warnings.push(format!(
            "No compact packet exists for {}; dry-run did not create one.",
            request.session_id
        ));
        BoilPacketStatus::Missing
    } else {
        compaction::compact_session(cfg, &request.session_id)?;
        compact_packet_created = true;
        boiled_context_paths = existing_compact_paths(&compact_paths);
        warnings.push("Created compact packet because none existed.".to_string());
        BoilPacketStatus::Created
    };

    if raw_context_paths.is_empty() {
        warnings.push("Raw context estimate has no readable session log path.".to_string());
    }
    if boiled_context_paths.is_empty() {
        warnings.push("Boiled context estimate has no compact artifact paths.".to_string());
    }

    let raw_input_tokens_estimate = estimate_paths_tokens(&raw_context_paths)?;
    let boiled_input_tokens_estimate = estimate_paths_tokens(&boiled_context_paths)?;
    let expected_output_tokens_estimate = pricing.expected_output_tokens;
    let token_savings = raw_input_tokens_estimate as isize - boiled_input_tokens_estimate as isize;
    let compression_ratio = if raw_input_tokens_estimate == 0 {
        0.0
    } else {
        round4(boiled_input_tokens_estimate as f64 / raw_input_tokens_estimate as f64)
    };
    let raw_estimated_cost = estimate_cost(
        raw_input_tokens_estimate,
        expected_output_tokens_estimate,
        &pricing,
    );
    let boiled_estimated_cost = estimate_cost(
        boiled_input_tokens_estimate,
        expected_output_tokens_estimate,
        &pricing,
    );
    let estimated_cost_avoided = round8(raw_estimated_cost - boiled_estimated_cost);
    let (context_state, guardian_state, recommended_action, blocked) =
        match context_status::context_status(cfg) {
            Ok(context) => (
                context.context_state,
                context.guardian_state,
                context.recommended_action,
                context.blocked,
            ),
            Err(err) => {
                warnings.push(format!("Context status could not be read safely: {err}"));
                (
                    ContextState::Red,
                    ContextGuardianState::Blocked,
                    ContextRecommendedAction::BlockContinuation,
                    true,
                )
            }
        };
    let mut compact_metrics = CompactMetrics::default();
    let mut packet_status = packet_status;
    if compact_paths.compact_json.exists() {
        match read_compact_metrics(&compact_paths.compact_json) {
            Ok(metrics) => compact_metrics = metrics,
            Err(err) => {
                warnings.push(format!("Compact JSON could not be read safely: {err}"));
                packet_status = BoilPacketStatus::Unsafe;
            }
        }
    }
    if compact_paths.compact_json.exists() && !compact_paths.evidence_index_json.exists() {
        warnings.push("Compact packet is missing evidence-index.json.".to_string());
        packet_status = BoilPacketStatus::Unsafe;
    }
    if compact_paths.compact_json.exists() && compact_metrics.evidence_ref_count == 0 {
        warnings.push("Compact packet has no evidence refs.".to_string());
        packet_status = BoilPacketStatus::Unsafe;
    }
    if token_savings < 0 {
        warnings.push(
            "Boiled estimate is larger than raw estimate; compression value is not established."
                .to_string(),
        );
    }
    if !matches!(context_state, ContextState::Green) {
        warnings.push(format!(
            "Context status is {:?}; savings are not a safety verdict.",
            context_state
        ));
        if !matches!(packet_status, BoilPacketStatus::Missing) {
            packet_status = BoilPacketStatus::Unsafe;
        }
    }

    let mut report = BoilReport {
        session_id: request.session_id.to_string(),
        created_at,
        dry_run: request.dry_run,
        compact_packet_created,
        raw_context_paths,
        boiled_context_paths,
        report_markdown_path: None,
        report_json_path: None,
        token_estimate_method: "rough_whitespace_tokens".to_string(),
        token_estimate_confidence: "low".to_string(),
        pricing_profile: pricing.name,
        pricing_profile_source: pricing.source,
        pricing_profile_updated_at: pricing.updated_at,
        raw_input_tokens_estimate,
        boiled_input_tokens_estimate,
        expected_output_tokens_estimate,
        token_savings,
        compression_ratio,
        raw_estimated_cost,
        boiled_estimated_cost,
        estimated_cost_avoided,
        raw_estimated_cost_display: format_currency_amount(raw_estimated_cost, "USD"),
        boiled_estimated_cost_display: format_currency_amount(boiled_estimated_cost, "USD"),
        estimated_cost_avoided_display: format_currency_amount(estimated_cost_avoided, "USD"),
        packet_status,
        context_state,
        guardian_state,
        recommended_action,
        blocked,
        evidence_ref_count: compact_metrics.evidence_ref_count,
        excluded_context_count: 0,
        risk_count: compact_metrics.risk_count,
        warnings,
    };

    if !request.dry_run {
        write_reports(cfg, &mut report)?;
    }

    Ok(report)
}

pub fn render_boil_report(report: &BoilReport) -> String {
    let savings_percent = if report.raw_input_tokens_estimate == 0 {
        0.0
    } else {
        round4(
            (report.token_savings.max(0) as f64 / report.raw_input_tokens_estimate as f64) * 100.0,
        )
    };
    let mut out = format!(
        "Quant-M Boil report\nsession_id: {}\npacket_status: {:?}\ncontext_state: {:?}\nguardian_state: {}\nrecommended_action: {}\nblocked: {}\ntoken_estimate_method: {}\ntoken_estimate_confidence: {}\nraw_input_tokens_estimate: {}\nboiled_input_tokens_estimate: {}\nEstimated continuation savings: {:.1}% fewer input tokens.\nRaw continuation estimate: {}\nBoiled continuation estimate: {}\nEstimated cost avoided: {}\npricing_profile: {}\n",
        report.session_id,
        report.packet_status,
        report.context_state,
        report.guardian_state,
        report.recommended_action,
        report.blocked,
        report.token_estimate_method,
        report.token_estimate_confidence,
        report.raw_input_tokens_estimate,
        report.boiled_input_tokens_estimate,
        savings_percent,
        report.raw_estimated_cost_display,
        report.boiled_estimated_cost_display,
        report.estimated_cost_avoided_display,
        report.pricing_profile,
    );
    out.push_str("raw_context_paths:\n");
    for path in &report.raw_context_paths {
        out.push_str(&format!("- {}\n", path.display()));
    }
    out.push_str("boiled_context_paths:\n");
    for path in &report.boiled_context_paths {
        out.push_str(&format!("- {}\n", path.display()));
    }
    if report.compact_packet_created {
        out.push_str("Created compact packet because none existed.\n");
    }
    if let Some(path) = &report.report_markdown_path {
        out.push_str(&format!("report: {}\n", path.display()));
    }
    if let Some(path) = &report.report_json_path {
        out.push_str(&format!("report_json: {}\n", path.display()));
    }
    if !report.warnings.is_empty() {
        out.push_str("warnings:\n");
        for warning in &report.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }
    out
}

pub fn lookup_evidence(
    cfg: &Config,
    session_id: &SessionId,
    evidence_id: &str,
) -> Result<BoilEvidenceLookup> {
    sessions::show_session(cfg, session_id)?;
    let compact_paths = compact_paths(cfg, session_id);
    let mut warnings = Vec::new();
    if !compact_paths.evidence_index_json.exists() {
        warnings.push("No evidence-index.json exists for this session.".to_string());
        return Ok(BoilEvidenceLookup {
            session_id: session_id.to_string(),
            evidence_id: evidence_id.to_string(),
            compact_packet_path: compact_paths.compact_json,
            matched: None,
            warnings,
        });
    }
    let raw = fs::read_to_string(&compact_paths.evidence_index_json).with_context(|| {
        format!(
            "failed to read {}",
            compact_paths.evidence_index_json.display()
        )
    })?;
    let refs: Vec<BoilEvidenceRef> = serde_json::from_str(&raw).with_context(|| {
        format!(
            "failed to decode {}",
            compact_paths.evidence_index_json.display()
        )
    })?;
    let needle = evidence_id.trim();
    let matched = refs
        .into_iter()
        .find(|item| item.step_id == needle || item.sequence.to_string() == needle);
    if matched.is_none() {
        warnings.push(format!("Evidence id '{needle}' was not found."));
    }
    Ok(BoilEvidenceLookup {
        session_id: session_id.to_string(),
        evidence_id: needle.to_string(),
        compact_packet_path: compact_paths.compact_json,
        matched,
        warnings,
    })
}

pub fn render_evidence_lookup(lookup: &BoilEvidenceLookup) -> String {
    let mut out = format!(
        "Quant-M Boil evidence\nsession_id: {}\nevidence_id: {}\ncompact_packet: {}\n",
        lookup.session_id,
        lookup.evidence_id,
        lookup.compact_packet_path.display()
    );
    if let Some(evidence) = &lookup.matched {
        out.push_str(&format!(
            "matched: true\nsequence: {}\nstep_id: {}\nevent_kind: {}\noccurred_at: {}\nsummary: {}\n",
            evidence.sequence,
            evidence.step_id,
            evidence.event_kind,
            evidence.occurred_at,
            evidence.summary
        ));
    } else {
        out.push_str("matched: false\n");
    }
    if !lookup.warnings.is_empty() {
        out.push_str("warnings:\n");
        for warning in &lookup.warnings {
            out.push_str(&format!("- {warning}\n"));
        }
    }
    out
}

fn write_reports(cfg: &Config, report: &mut BoilReport) -> Result<()> {
    let output_dir = cfg
        .workspace_dir
        .join("state")
        .join("boil")
        .join(&report.session_id);
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;
    let markdown_path = output_dir.join("boil-report.md");
    let json_path = output_dir.join("boil-report.json");
    report.report_markdown_path = Some(markdown_path.clone());
    report.report_json_path = Some(json_path.clone());
    fs::write(&markdown_path, render_boil_markdown(report))
        .with_context(|| format!("failed to write {}", markdown_path.display()))?;
    fs::write(
        &json_path,
        format!("{}\n", serde_json::to_string_pretty(report)?),
    )
    .with_context(|| format!("failed to write {}", json_path.display()))?;
    Ok(())
}

fn render_boil_markdown(report: &BoilReport) -> String {
    format!(
        "# Quant-M Boil Report\n\n- session_id: {}\n- packet_status: {:?}\n- context_state: {:?}\n- guardian_state: {}\n- recommended_action: {}\n- blocked: {}\n- raw_input_tokens_estimate: {}\n- boiled_input_tokens_estimate: {}\n- expected_output_tokens_estimate: {}\n- token_savings: {}\n- compression_ratio: {:.4}\n- raw_estimated_cost: {}\n- boiled_estimated_cost: {}\n- estimated_cost_avoided: {}\n- pricing_profile: {}\n- token_estimate_method: {}\n- token_estimate_confidence: {}\n\n## Raw Context Paths\n\n{}\n## Boiled Context Paths\n\n{}\n## Safety\n\n{}\n\n## Warnings\n\n{}\n",
        report.session_id,
        report.packet_status,
        report.context_state,
        report.guardian_state,
        report.recommended_action,
        report.blocked,
        report.raw_input_tokens_estimate,
        report.boiled_input_tokens_estimate,
        report.expected_output_tokens_estimate,
        report.token_savings,
        report.compression_ratio,
        report.raw_estimated_cost_display,
        report.boiled_estimated_cost_display,
        report.estimated_cost_avoided_display,
        report.pricing_profile,
        report.token_estimate_method,
        report.token_estimate_confidence,
        format_markdown_paths(&report.raw_context_paths),
        format_markdown_paths(&report.boiled_context_paths),
        if matches!(
            report.packet_status,
            BoilPacketStatus::Present | BoilPacketStatus::Created
        ) && matches!(report.context_state, ContextState::Green)
        {
            "Boiled packet appears safe to inspect for continuation."
        } else {
            "Do not treat estimated savings as continuation approval."
        },
        if report.warnings.is_empty() {
            "- none\n".to_string()
        } else {
            report
                .warnings
                .iter()
                .map(|warning| format!("- {warning}\n"))
                .collect::<String>()
        }
    )
}

fn format_markdown_paths(paths: &[PathBuf]) -> String {
    if paths.is_empty() {
        return "- none\n\n".to_string();
    }
    let mut out = String::new();
    for path in paths {
        out.push_str(&format!("- `{}`\n", path.display()));
    }
    out.push('\n');
    out
}

#[derive(Debug, Clone)]
struct CompactPaths {
    compact_md: PathBuf,
    compact_json: PathBuf,
    evidence_index_json: PathBuf,
    next_action_md: PathBuf,
    risks_md: PathBuf,
}

fn raw_context_paths(cfg: &Config, session_id: &SessionId) -> Vec<PathBuf> {
    existing_paths(&[cfg
        .runtime
        .session_dir
        .join(format!("{}.ndjson", session_id.as_str()))])
}

fn compact_paths(cfg: &Config, session_id: &SessionId) -> CompactPaths {
    let output_dir = cfg
        .workspace_dir
        .join("state")
        .join("compacted")
        .join(session_id.as_str());
    CompactPaths {
        compact_md: output_dir.join("compact.md"),
        compact_json: output_dir.join("compact.json"),
        evidence_index_json: output_dir.join("evidence-index.json"),
        next_action_md: output_dir.join("next-action.md"),
        risks_md: output_dir.join("risks.md"),
    }
}

fn existing_paths(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut paths = paths
        .iter()
        .filter(|path| path.exists())
        .cloned()
        .collect::<Vec<_>>();
    paths.sort_by_key(|path| path.display().to_string());
    paths
}

impl CompactPaths {
    fn as_vec(&self) -> Vec<PathBuf> {
        vec![
            self.compact_md.clone(),
            self.compact_json.clone(),
            self.evidence_index_json.clone(),
            self.next_action_md.clone(),
            self.risks_md.clone(),
        ]
    }
}

fn existing_compact_paths(compact_paths: &CompactPaths) -> Vec<PathBuf> {
    existing_paths(&compact_paths.as_vec())
}

fn estimate_paths_tokens(paths: &[PathBuf]) -> Result<usize> {
    let mut tokens = 0usize;
    for path in paths {
        let raw = fs::read_to_string(path)
            .with_context(|| format!("failed to read {}", path.display()))?;
        tokens = tokens.saturating_add(estimate_tokens(&raw));
    }
    Ok(tokens)
}

fn estimate_tokens(raw: &str) -> usize {
    raw.split_whitespace().count()
}

#[derive(Debug, Default)]
struct CompactMetrics {
    evidence_ref_count: usize,
    risk_count: usize,
}

fn read_compact_metrics(path: &Path) -> Result<CompactMetrics> {
    if !path.exists() {
        return Ok(CompactMetrics::default());
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let packet: CompactPacketForBoil = serde_json::from_str(&raw)
        .with_context(|| format!("failed to decode {}", path.display()))?;
    Ok(CompactMetrics {
        evidence_ref_count: packet.evidence_refs.len(),
        risk_count: packet.open_risks.len(),
    })
}

fn pricing_profile(name: &str) -> Result<BoilPricingProfile> {
    match name.trim() {
        "" | "rough-default" => Ok(BoilPricingProfile {
            name: "rough-default".to_string(),
            input_cost_per_million_tokens: 3.0,
            output_cost_per_million_tokens: 15.0,
            expected_output_tokens: 0,
            source: "local rough default; not provider billing".to_string(),
            updated_at: None,
        }),
        "local-zero-cost" => Ok(BoilPricingProfile {
            name: "local-zero-cost".to_string(),
            input_cost_per_million_tokens: 0.0,
            output_cost_per_million_tokens: 0.0,
            expected_output_tokens: 0,
            source: "local profile".to_string(),
            updated_at: None,
        }),
        "manual-config" => Ok(BoilPricingProfile {
            name: "manual-config".to_string(),
            input_cost_per_million_tokens: 3.0,
            output_cost_per_million_tokens: 15.0,
            expected_output_tokens: 0,
            source: "built-in manual placeholder".to_string(),
            updated_at: None,
        }),
        other => Err(anyhow!(
            "unsupported pricing profile '{other}'; expected rough-default, local-zero-cost, or manual-config"
        )),
    }
}

fn estimate_cost(input_tokens: usize, output_tokens: usize, pricing: &BoilPricingProfile) -> f64 {
    round8(
        (input_tokens as f64 / 1_000_000.0) * pricing.input_cost_per_million_tokens
            + (output_tokens as f64 / 1_000_000.0) * pricing.output_cost_per_million_tokens,
    )
}

fn round4(value: f64) -> f64 {
    (value * 10_000.0).round() / 10_000.0
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
    use crate::sessions::{self, SessionEvent};
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn temp_cfg() -> (TempDir, Config, SessionId) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config {
            workspace_dir: tmp.path().join("workspace"),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        let context = sessions::runtime_context("node-boil", "boil");
        sessions::append_event(
            &cfg,
            &context,
            SessionEvent::Observation {
                message: "goal: validate boil tiny costs".to_string(),
                job_id: None,
                detail: Some("raw session context should be measured".to_string()),
            },
        )
        .expect("event");
        sessions::append_event(
            &cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "boil safety".to_string(),
                allowed: false,
                reason: "measurement only".to_string(),
            },
        )
        .expect("policy");
        sessions::append_event(
            &cfg,
            &context,
            SessionEvent::Output {
                channel: "validation".to_string(),
                summary: "validation evidence for boil report".to_string(),
                job_id: None,
            },
        )
        .expect("output");
        (tmp, cfg, context.session_id)
    }

    #[test]
    fn boil_creates_reports_and_keeps_json_costs_numeric() {
        let (_tmp, cfg, session_id) = temp_cfg();
        let report = run_boil(
            &cfg,
            BoilRequest {
                session_id,
                dry_run: false,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");

        assert!(report.compact_packet_created);
        assert!(
            report
                .report_markdown_path
                .as_ref()
                .is_some_and(|p| p.exists())
        );
        assert!(report.report_json_path.as_ref().is_some_and(|p| p.exists()));
        let json_path = report.report_json_path.as_ref().expect("json path");
        let raw_json = fs::read_to_string(json_path).expect("json");
        let value: serde_json::Value = serde_json::from_str(&raw_json).expect("value");
        assert!(value["raw_estimated_cost"].is_number());
        assert!(
            value["raw_estimated_cost_display"]
                .as_str()
                .expect("display")
                .starts_with('$')
        );
    }

    #[test]
    fn dry_run_does_not_write_reports_or_create_compact_packet() {
        let (_tmp, cfg, session_id) = temp_cfg();
        let report = run_boil(
            &cfg,
            BoilRequest {
                session_id: session_id.clone(),
                dry_run: true,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil dry-run");
        assert_eq!(report.packet_status, BoilPacketStatus::Missing);
        assert_eq!(report.guardian_state, ContextGuardianState::NeedsCompact);
        assert_eq!(report.recommended_action, ContextRecommendedAction::Compact);
        assert!(report.report_json_path.is_none());
        assert!(
            !cfg.workspace_dir
                .join("state/compacted")
                .join(session_id.as_str())
                .join("compact.json")
                .exists()
        );
    }

    #[test]
    fn tiny_costs_stay_visible_in_terminal_and_markdown() {
        assert_eq!(format_currency_amount(0.000019, "USD"), "$0.00001900 USD");
        let (_tmp, cfg, session_id) = temp_cfg();
        let mut report = run_boil(
            &cfg,
            BoilRequest {
                session_id,
                dry_run: false,
                pricing_profile: "manual-config".to_string(),
            },
        )
        .expect("boil");
        report.raw_estimated_cost = 0.000214;
        report.boiled_estimated_cost = 0.000019;
        report.estimated_cost_avoided = 0.000195;
        report.raw_estimated_cost_display = format_currency_amount(0.000214, "USD");
        report.boiled_estimated_cost_display = format_currency_amount(0.000019, "USD");
        report.estimated_cost_avoided_display = format_currency_amount(0.000195, "USD");

        let rendered = render_boil_report(&report);
        let markdown = render_boil_markdown(&report);
        assert!(rendered.contains("Raw continuation estimate: $0.00021400 USD"));
        assert!(rendered.contains("Boiled continuation estimate: $0.00001900 USD"));
        assert!(rendered.contains("Estimated cost avoided: $0.00019500 USD"));
        assert!(markdown.contains("- raw_estimated_cost: $0.00021400 USD"));
    }

    #[test]
    fn evidence_lookup_finds_sequence_or_step_id() {
        let (_tmp, cfg, session_id) = temp_cfg();
        run_boil(
            &cfg,
            BoilRequest {
                session_id: session_id.clone(),
                dry_run: false,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");

        let by_sequence = lookup_evidence(&cfg, &session_id, "1").expect("lookup sequence");
        assert!(by_sequence.matched.is_some());
        let step_id = by_sequence
            .matched
            .as_ref()
            .expect("matched")
            .step_id
            .clone();
        let by_step = lookup_evidence(&cfg, &session_id, &step_id).expect("lookup step");
        assert_eq!(by_step.matched.as_ref().expect("matched").step_id, step_id);
        assert!(render_evidence_lookup(&by_step).contains("matched: true"));
    }

    #[test]
    fn missing_evidence_index_marks_packet_unsafe() {
        let (_tmp, cfg, session_id) = temp_cfg();
        run_boil(
            &cfg,
            BoilRequest {
                session_id: session_id.clone(),
                dry_run: false,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");
        fs::remove_file(
            cfg.workspace_dir
                .join("state/compacted")
                .join(session_id.as_str())
                .join("evidence-index.json"),
        )
        .expect("remove evidence index");

        let report = run_boil(
            &cfg,
            BoilRequest {
                session_id,
                dry_run: true,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");
        assert_eq!(report.packet_status, BoilPacketStatus::Unsafe);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("missing evidence-index"))
        );
    }

    #[test]
    fn corrupted_compact_json_marks_packet_unsafe() {
        let (_tmp, cfg, session_id) = temp_cfg();
        run_boil(
            &cfg,
            BoilRequest {
                session_id: session_id.clone(),
                dry_run: false,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");
        fs::write(
            cfg.workspace_dir
                .join("state/compacted")
                .join(session_id.as_str())
                .join("compact.json"),
            "{not-json}",
        )
        .expect("corrupt compact");

        let report = run_boil(
            &cfg,
            BoilRequest {
                session_id,
                dry_run: true,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");
        assert_eq!(report.packet_status, BoilPacketStatus::Unsafe);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("Compact JSON could not be read safely"))
        );
    }

    #[test]
    fn empty_session_fails_before_report_generation() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config {
            workspace_dir: tmp.path().join("workspace"),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        fs::write(cfg.runtime.session_dir.join("session-empty.ndjson"), "").expect("empty");
        let error = run_boil(
            &cfg,
            BoilRequest {
                session_id: SessionId::new("session-empty"),
                dry_run: true,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect_err("empty session should fail");
        assert!(error.to_string().contains("has no events"));
    }

    #[test]
    fn boiled_larger_than_raw_is_warned_not_celebrated() {
        let (_tmp, cfg, session_id) = temp_cfg();
        run_boil(
            &cfg,
            BoilRequest {
                session_id: session_id.clone(),
                dry_run: false,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");
        let compact_dir = cfg
            .workspace_dir
            .join("state/compacted")
            .join(session_id.as_str());
        fs::write(compact_dir.join("compact.md"), "extra ".repeat(5_000)).expect("bloat compact");

        let report = run_boil(
            &cfg,
            BoilRequest {
                session_id,
                dry_run: true,
                pricing_profile: "rough-default".to_string(),
            },
        )
        .expect("boil");
        assert!(report.boiled_input_tokens_estimate > report.raw_input_tokens_estimate);
        assert!(report.token_savings < 0);
        assert!(
            report
                .warnings
                .iter()
                .any(|warning| warning.contains("larger than raw"))
        );
    }
}
