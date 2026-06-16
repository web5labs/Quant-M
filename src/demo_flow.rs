use crate::compaction;
use crate::config::Config;
use crate::consensus;
use crate::context_guardian::{self, GuardianTickOptions};
use crate::cost_ledger;
use crate::sessions::SessionId;
use anyhow::{Context, Result};
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};

const DEMO_QUESTION: &str = "What should a new Quant-M user inspect first?";
const EXCERPT_LINE_LIMIT: usize = 10;

#[derive(Debug, Clone, Serialize)]
pub struct DemoArtifacts {
    pub session_evidence: PathBuf,
    pub replay_record: PathBuf,
    pub compact_packet: PathBuf,
    pub guardian_handoff: PathBuf,
    pub cost_ledger: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct DemoResult {
    pub session_id: String,
    pub workflow_id: String,
    pub evidence_created: bool,
    pub replay_validated: bool,
    pub compact_packet_generated: bool,
    pub guardian_handoff_created: bool,
    pub cost_record_written: bool,
    pub artifacts: DemoArtifacts,
    pub excerpt_source: PathBuf,
    pub excerpt: String,
}

pub fn run(cfg: &Config) -> Result<DemoResult> {
    let mut demo_cfg = cfg.clone();
    demo_cfg.context_guardian.enabled = true;

    let report = consensus::run_consensus_dry_run(&demo_cfg, DEMO_QUESTION)?;
    let session_id = SessionId::new(report.session_id.clone());
    let replay = consensus::replay_consensus_session(&demo_cfg, &session_id)?;
    let compact = compaction::compact_session(&demo_cfg, &session_id)?;
    let guardian = context_guardian::tick_with_options(&demo_cfg, GuardianTickOptions::force())?;
    let cost_summary = cost_ledger::summarize_costs(&demo_cfg, None, Some(&report.session_id))?;

    let compact_packet = guardian
        .compact_packet_path
        .clone()
        .unwrap_or_else(|| compact.artifacts.compact_md.clone());
    let guardian_handoff = guardian
        .handoff_path
        .clone()
        .context("demo expected context guardian handoff path")?;
    let excerpt = read_excerpt(&guardian_handoff)
        .or_else(|_| read_excerpt(&compact_packet))
        .context("failed to read demo excerpt")?;

    Ok(DemoResult {
        session_id: report.session_id,
        workflow_id: report.workflow_id,
        evidence_created: report.artifact_paths.evidence_index_json.exists(),
        replay_validated: matches!(
            replay.replay_status,
            consensus::ConsensusReplayStatus::ValidatedEvidenceOnly
        ),
        compact_packet_generated: compact_packet.exists(),
        guardian_handoff_created: guardian_handoff.exists(),
        cost_record_written: cost_summary.record_count > 0,
        artifacts: DemoArtifacts {
            session_evidence: report.artifact_paths.session_dir,
            replay_record: report.artifact_paths.report_json,
            compact_packet,
            guardian_handoff: guardian_handoff.clone(),
            cost_ledger: cost_ledger::cost_ledger_path(&demo_cfg),
        },
        excerpt_source: guardian_handoff,
        excerpt,
    })
}

pub fn render(result: &DemoResult) -> String {
    format!(
        "Quant-M demo\n\
session_id: {}\n\
workflow_id: {}\n\n\
{} Evidence created\n\
{} Replay validated\n\
{} Compact packet generated\n\
{} Context Guardian handoff created\n\
{} Cost record written\n\n\
Artifacts:\n\
session evidence: {}\n\
replay record: {}\n\
compact packet: {}\n\
guardian handoff: {}\n\
cost ledger: {}\n\n\
Excerpt from {}:\n{}\n\n\
Next:\n\
open the compact packet or guardian handoff above to inspect the continuity proof.\n",
        result.session_id,
        result.workflow_id,
        check(result.evidence_created),
        check(result.replay_validated),
        check(result.compact_packet_generated),
        check(result.guardian_handoff_created),
        check(result.cost_record_written),
        result.artifacts.session_evidence.display(),
        result.artifacts.replay_record.display(),
        result.artifacts.compact_packet.display(),
        result.artifacts.guardian_handoff.display(),
        result.artifacts.cost_ledger.display(),
        result.excerpt_source.display(),
        result.excerpt,
    )
}

fn check(ok: bool) -> &'static str {
    if ok { "✓" } else { "!" }
}

fn read_excerpt(path: &Path) -> Result<String> {
    let text =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(text
        .lines()
        .take(EXCERPT_LINE_LIMIT)
        .collect::<Vec<_>>()
        .join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn temp_cfg() -> (TempDir, Config) {
        let temp = TempDir::new().expect("temp");
        let mut cfg = Config::default();
        cfg.workspace_dir = temp.path().join("workspace");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.context_guardian.branch_packet_path = cfg
            .workspace_dir
            .join("state/context-guardian/continuity-handoff.md");
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        (temp, cfg)
    }

    #[test]
    fn demo_flow_creates_first_success_artifacts() {
        let (_temp, cfg) = temp_cfg();
        let result = run(&cfg).expect("demo flow");
        assert!(result.evidence_created);
        assert!(result.replay_validated);
        assert!(result.compact_packet_generated);
        assert!(result.guardian_handoff_created);
        assert!(result.cost_record_written);
        assert!(result.artifacts.session_evidence.exists());
        assert!(result.artifacts.compact_packet.exists());
        assert!(result.artifacts.guardian_handoff.exists());
        assert!(result.artifacts.cost_ledger.exists());

        let rendered = render(&result);
        assert!(rendered.contains("Quant-M demo"));
        assert!(rendered.contains("Evidence created"));
        assert!(rendered.contains("Replay validated"));
        assert!(rendered.contains("Compact packet generated"));
        assert!(rendered.contains("Context Guardian handoff created"));
        assert!(rendered.contains("Cost record written"));
        assert!(rendered.contains("Artifacts:"));
        assert!(rendered.contains("Excerpt"));
    }
}
