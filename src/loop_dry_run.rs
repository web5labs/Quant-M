use crate::config::Config;
use crate::context_decay::{
    ContextDecayScore, ContextItem, MemoryClass, is_canonical_truth_file, score_context_item,
};
use crate::context_status::{self, ContextStatusReport};
use crate::fsm_core::ContextRecommendedAction;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum LoopScope {
    Repo,
    Docs,
    Sessions,
    Truth,
    All,
}

impl std::str::FromStr for LoopScope {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "repo" => Ok(Self::Repo),
            "docs" => Ok(Self::Docs),
            "sessions" => Ok(Self::Sessions),
            "truth" => Ok(Self::Truth),
            "all" => Ok(Self::All),
            other => Err(anyhow!(
                "invalid loop scope '{other}'; expected repo, docs, sessions, truth, or all"
            )),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopDryRunRequest {
    pub scope: LoopScope,
    pub max_candidates: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LoopDryRunReport {
    pub loop_id: String,
    pub created_at: String,
    pub scope: LoopScope,
    pub fsm_states: Vec<LoopFsmState>,
    pub execution_readiness: ExecutionReadiness,
    pub context_status: ContextStatusReport,
    pub candidates: Vec<LoopCandidate>,
    pub evidence_index: Vec<LoopEvidenceRef>,
    pub context_decay: Vec<ContextDecayScore>,
    pub output_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopFsmState {
    pub state: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ExecutionReadiness {
    Ready,
    Blocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopCandidate {
    pub candidate_id: String,
    pub title: String,
    pub category: CandidateCategory,
    pub reason: String,
    pub expected_benefit: String,
    pub risk_level: RiskLevel,
    pub files_likely_touched: Vec<PathBuf>,
    pub validation_required: Vec<String>,
    pub approval_required: bool,
    pub blocked_until: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CandidateCategory {
    Docs,
    Tests,
    Policy,
    Compaction,
    Context,
    Sessions,
    Code,
    Validation,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoopEvidenceRef {
    pub item_id: String,
    pub source_path: PathBuf,
    pub evidence_kind: String,
    pub summary: String,
}

pub fn run_loop_dry_run(cfg: &Config, request: LoopDryRunRequest) -> Result<LoopDryRunReport> {
    if request.max_candidates == 0 {
        return Err(anyhow!("max-candidates must be greater than 0"));
    }
    let created_at = Utc::now().to_rfc3339();
    let loop_id = format!("loop-{}", Utc::now().timestamp_micros());
    let context_status = context_status::context_status(cfg)?;
    let output_dir = cfg.workspace_dir.join("state").join("loops").join(&loop_id);
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let mut candidates = generate_candidates(cfg, &request, &context_status);
    candidates.truncate(request.max_candidates);
    let evidence_index = build_evidence_index(cfg, request.scope)?;
    let context_decay = build_context_decay(cfg, request.scope)?;
    let execution_readiness = if context_status.recommended_action
        == ContextRecommendedAction::Continue
        && !context_status.blocked
    {
        ExecutionReadiness::Ready
    } else {
        ExecutionReadiness::Blocked
    };
    let report = LoopDryRunReport {
        loop_id,
        created_at,
        scope: request.scope,
        fsm_states: completed_fsm_states(),
        execution_readiness,
        context_status,
        candidates,
        evidence_index,
        context_decay,
        output_dir,
    };
    write_loop_outputs(&report)?;
    Ok(report)
}

pub fn render_loop_report(report: &LoopDryRunReport) -> String {
    format!(
        "loop_id: {}\nscope: {:?}\nexecution_readiness: {:?}\ncontext_state: {:?}\ncandidates: {}\noutput_dir: {}\nrecommended_next_action: {}\n",
        report.loop_id,
        report.scope,
        report.execution_readiness,
        report.context_status.context_state,
        report.candidates.len(),
        report.output_dir.display(),
        report.context_status.recommended_next_action
    )
}

fn write_loop_outputs(report: &LoopDryRunReport) -> Result<()> {
    write_json(&report.output_dir.join("loop-report.json"), report)?;
    write_json(
        &report.output_dir.join("candidates.json"),
        &report.candidates,
    )?;
    write_json(
        &report.output_dir.join("evidence-index.json"),
        &report.evidence_index,
    )?;
    write_json(
        &report.output_dir.join("context-decay.json"),
        &report.context_decay,
    )?;
    fs::write(
        report.output_dir.join("loop-report.md"),
        render_loop_markdown(report),
    )
    .with_context(|| "failed to write loop-report.md")?;
    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
        .with_context(|| format!("failed to write {}", path.display()))
}

fn render_loop_markdown(report: &LoopDryRunReport) -> String {
    format!(
        "# Loop Dry Run Report\n\n## Summary\n\n- loop_id: {}\n- scope: {:?}\n- execution_readiness: {:?}\n- context_state: {:?}\n\n## Recommended Next Action\n\n{}\n\n## Candidates\n\n{}\n\n## Missing Context\n\n{}\n",
        report.loop_id,
        report.scope,
        report.execution_readiness,
        report.context_status.context_state,
        report.context_status.recommended_next_action,
        render_candidates(&report.candidates),
        render_missing(&report.context_status.missing_required_context),
    )
}

fn render_candidates(candidates: &[LoopCandidate]) -> String {
    if candidates.is_empty() {
        return "- No useful improvement found.".to_string();
    }
    candidates
        .iter()
        .map(|candidate| {
            format!(
                "- {} ({:?}, {:?}): {}",
                candidate.title, candidate.category, candidate.risk_level, candidate.reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_missing(items: &[String]) -> String {
    if items.is_empty() {
        return "- none".to_string();
    }
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn completed_fsm_states() -> Vec<LoopFsmState> {
    [
        "Idle",
        "LoopRequested",
        "ScopeResolved",
        "ContextStatusLoaded",
        "TruthFilesLoaded",
        "CompactedContextLoaded",
        "QualityScored",
        "CandidatesGenerated",
        "CandidatesRanked",
        "ContextDecayScored",
        "EvidenceIndexed",
        "ReportWritten",
        "Completed",
    ]
    .into_iter()
    .map(|state| LoopFsmState {
        state: state.to_string(),
        status: "completed".to_string(),
    })
    .collect()
}

fn generate_candidates(
    cfg: &Config,
    request: &LoopDryRunRequest,
    status: &ContextStatusReport,
) -> Vec<LoopCandidate> {
    let mut candidates = Vec::new();
    let mut next_id = 1usize;
    if !status.compact_packet_present {
        push_candidate(
            &mut candidates,
            &mut next_id,
            "Create compact packet for latest session",
            CandidateCategory::Compaction,
            "context-status reports no compact packet for the latest session",
            vec![cfg.workspace_dir.join("state/compacted")],
            vec!["Run quant-m compact <session_id> and then quant-m context-status".to_string()],
            false,
            None,
        );
    }
    if !status.policy_block_present {
        push_candidate(
            &mut candidates,
            &mut next_id,
            "Capture or review policy evidence",
            CandidateCategory::Policy,
            "execution sessions need policy evidence before continuation",
            vec![cfg.workspace_dir.join("POLICY.md")],
            vec!["Run context-status after policy evidence is captured".to_string()],
            true,
            Some("policy evidence exists for the latest execution session".to_string()),
        );
    }
    if !status.validation_evidence_present {
        push_candidate(
            &mut candidates,
            &mut next_id,
            "Collect validation evidence",
            CandidateCategory::Validation,
            "validation evidence is missing; Quant-M must not infer success",
            vec![],
            vec!["Run the relevant validation command and preserve evidence".to_string()],
            true,
            Some("validation evidence is present".to_string()),
        );
    }
    if !status.shippable_definition_present {
        push_candidate(
            &mut candidates,
            &mut next_id,
            "Refine shippable definition",
            CandidateCategory::Docs,
            "shippable definition is missing from compacted evidence",
            vec![cfg.workspace_dir.join("SHIPPABLE.md")],
            vec![
                "Review SHIPPABLE.md and capture the relevant definition in session evidence"
                    .to_string(),
            ],
            false,
            None,
        );
    }
    for missing in &status.missing_required_context {
        if let Some(file) = missing.strip_suffix(" not found.") {
            push_candidate(
                &mut candidates,
                &mut next_id,
                &format!("Create missing {file}"),
                CandidateCategory::Context,
                missing,
                vec![cfg.workspace_dir.join(file)],
                vec!["Run quant-m init-truth".to_string()],
                false,
                None,
            );
        }
    }
    if candidates.is_empty() && matches!(request.scope, LoopScope::Repo | LoopScope::All) {
        push_candidate(
            &mut candidates,
            &mut next_id,
            "Review repo validation coverage",
            CandidateCategory::Tests,
            "context is green; next useful dry-run candidate is validation coverage review",
            vec![PathBuf::from("tests"), PathBuf::from("src")],
            vec!["Run targeted tests after any future approved change".to_string()],
            false,
            None,
        );
    }
    candidates
}

#[allow(clippy::too_many_arguments)]
fn push_candidate(
    candidates: &mut Vec<LoopCandidate>,
    next_id: &mut usize,
    title: &str,
    category: CandidateCategory,
    reason: impl Into<String>,
    files_likely_touched: Vec<PathBuf>,
    validation_required: Vec<String>,
    approval_required: bool,
    blocked_until: Option<String>,
) {
    let risk_level = if approval_required {
        RiskLevel::Medium
    } else {
        RiskLevel::Low
    };
    candidates.push(LoopCandidate {
        candidate_id: format!("candidate-{next_id:03}"),
        title: title.to_string(),
        category,
        reason: reason.into(),
        expected_benefit: "Improves evidence quality or continuation safety.".to_string(),
        risk_level,
        files_likely_touched,
        validation_required,
        approval_required,
        blocked_until,
    });
    *next_id += 1;
}

fn build_evidence_index(cfg: &Config, scope: LoopScope) -> Result<Vec<LoopEvidenceRef>> {
    let mut refs = Vec::new();
    let mut next_id = 1usize;
    for path in scoped_paths(cfg, scope)? {
        if path.is_file() {
            refs.push(LoopEvidenceRef {
                item_id: format!("evidence-{next_id:03}"),
                evidence_kind: evidence_kind_for_path(&path).to_string(),
                summary: summarize_path(&path),
                source_path: path,
            });
            next_id += 1;
        }
    }
    Ok(refs)
}

fn build_context_decay(cfg: &Config, scope: LoopScope) -> Result<Vec<ContextDecayScore>> {
    let mut items = Vec::new();
    let mut next_id = 1usize;
    for path in scoped_paths(cfg, scope)? {
        if !path.is_file() {
            continue;
        }
        let canonical = is_canonical_truth_file(&path);
        let docs = path
            .components()
            .any(|component| component.as_os_str() == "docs");
        let path_text = path.to_string_lossy();
        let compact_path = path_text.contains("state/compacted");
        let session_path = path_text.contains("state/sessions");
        let validation_evidence_present = compact_path
            || path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.contains("validation") || name.contains("test") || name.contains("compact")
                });
        let memory_class = if canonical {
            MemoryClass::Canonical
        } else if compact_path {
            MemoryClass::Tactical
        } else if session_path {
            MemoryClass::Ephemeral
        } else if docs {
            MemoryClass::Strategic
        } else {
            MemoryClass::Tactical
        };
        let item = ContextItem {
            item_id: format!("decay-{next_id:03}"),
            source_path: path.clone(),
            memory_class,
            freshness_score: freshness_score(&path),
            validation_evidence_present,
            usage_count: usage_count_for_path(&path, scope),
            shippable_relevance_score: if path
                .file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.eq_ignore_ascii_case("SHIPPABLE.md"))
            {
                1.0
            } else if canonical || compact_path {
                0.8
            } else if docs {
                0.6
            } else {
                0.4
            },
            contradiction_count: contradiction_count_for_path(&path),
            compact_packet_stale: compact_path && freshness_score(&path) < 0.7,
        };
        items.push(score_context_item(&item));
        next_id += 1;
    }
    Ok(items)
}

fn scoped_paths(cfg: &Config, scope: LoopScope) -> Result<Vec<PathBuf>> {
    let mut paths = Vec::new();
    match scope {
        LoopScope::Repo => {
            collect_existing(&mut paths, &PathBuf::from("src"))?;
            collect_existing(&mut paths, &PathBuf::from("tests"))?;
        }
        LoopScope::Docs => collect_existing(&mut paths, &PathBuf::from("docs"))?,
        LoopScope::Sessions => {
            collect_existing(&mut paths, &cfg.runtime.session_dir)?;
            collect_existing(&mut paths, &cfg.workspace_dir.join("state/compacted"))?;
        }
        LoopScope::Truth => collect_truth_paths(&mut paths, cfg),
        LoopScope::All => {
            collect_existing(&mut paths, &PathBuf::from("src"))?;
            collect_existing(&mut paths, &PathBuf::from("tests"))?;
            collect_existing(&mut paths, &PathBuf::from("docs"))?;
            collect_existing(&mut paths, &cfg.runtime.session_dir)?;
            collect_existing(&mut paths, &cfg.workspace_dir.join("state/compacted"))?;
            collect_truth_paths(&mut paths, cfg);
        }
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn collect_truth_paths(paths: &mut Vec<PathBuf>, cfg: &Config) {
    for name in ["QUANTM.md", "POLICY.md", "SHIPPABLE.md", "AGENTS.md"] {
        let workspace_path = cfg.workspace_dir.join(name);
        if workspace_path.exists() {
            paths.push(workspace_path);
        }
        let repo_path = PathBuf::from(name);
        if repo_path.exists() {
            paths.push(repo_path);
        }
    }
}

fn collect_existing(paths: &mut Vec<PathBuf>, root: &Path) -> Result<()> {
    if !root.exists() {
        return Ok(());
    }
    if root.is_file() {
        paths.push(root.to_path_buf());
        return Ok(());
    }
    for entry in fs::read_dir(root).with_context(|| format!("failed to read {}", root.display()))? {
        let entry = entry.context("failed to read directory entry")?;
        let path = entry.path();
        if should_skip_path(&path) {
            continue;
        }
        if path.is_dir() {
            collect_existing(paths, &path)?;
        } else if path.is_file() {
            paths.push(path);
        }
    }
    Ok(())
}

fn should_skip_path(path: &Path) -> bool {
    path.components().any(|component| {
        let value = component.as_os_str().to_string_lossy();
        matches!(
            value.as_ref(),
            "target" | ".git" | "node_modules" | "__pycache__"
        )
    })
}

fn evidence_kind_for_path(path: &Path) -> &'static str {
    let path_text = path.to_string_lossy();
    if path_text.contains("state/sessions") {
        "session"
    } else if path_text.contains("state/compacted") {
        "compact"
    } else if is_truth_file(path) {
        "truth"
    } else if path_text.contains("docs") {
        "docs"
    } else {
        "repo"
    }
}

fn summarize_path(path: &Path) -> String {
    let bytes = fs::metadata(path).map(|meta| meta.len()).unwrap_or(0);
    format!("{} bytes", bytes)
}

fn is_truth_file(path: &Path) -> bool {
    is_canonical_truth_file(path)
}

fn freshness_score(path: &Path) -> f64 {
    let Ok(metadata) = fs::metadata(path) else {
        return 0.0;
    };
    let Ok(modified) = metadata.modified() else {
        return 0.5;
    };
    let Ok(elapsed) = modified.elapsed() else {
        return 0.5;
    };
    if elapsed.as_secs() < 86_400 {
        1.0
    } else if elapsed.as_secs() < 604_800 {
        0.7
    } else {
        0.4
    }
}

fn usage_count_for_path(path: &Path, scope: LoopScope) -> u32 {
    if is_canonical_truth_file(path) {
        return 5;
    }
    let path_text = path.to_string_lossy();
    if path_text.contains("state/compacted") {
        return 4;
    }
    if path_text.contains("state/sessions") {
        return 2;
    }
    match scope {
        LoopScope::Truth => 5,
        LoopScope::Docs => 3,
        LoopScope::Sessions => 2,
        LoopScope::Repo => 1,
        LoopScope::All => 2,
    }
}

fn contradiction_count_for_path(path: &Path) -> u32 {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    if name.contains("deprecated") || name.contains("contradiction") {
        1
    } else {
        0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::compaction;
    use crate::config::Config;
    use crate::context_decay::DecayAction;
    use crate::context_status::ContextState;
    use crate::sessions::{SessionEvent, append_event, runtime_context};
    use crate::truth_files;
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        fs::create_dir_all(&cfg.workspace_dir).expect("workspace");
        (tmp, cfg)
    }

    fn create_session(cfg: &Config) -> crate::sessions::SessionContext {
        let context = runtime_context("loop-node", "worker");
        append_event(
            cfg,
            &context,
            SessionEvent::Observation {
                message: "goal".to_string(),
                job_id: None,
                detail: Some("update quantm/src/loop_dry_run.rs shippable".to_string()),
            },
        )
        .expect("goal");
        append_event(
            cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled".to_string(),
            },
        )
        .expect("policy");
        append_event(
            cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "validation".to_string(),
                input_preview: "test".to_string(),
                command_preview: Some("cargo test loop_dry_run".to_string()),
                status: "ok".to_string(),
            },
        )
        .expect("skill");
        append_event(
            cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "changed quantm/src/loop_dry_run.rs shippable definition captured"
                    .to_string(),
                job_id: None,
            },
        )
        .expect("output");
        context
    }

    #[test]
    fn dry_run_creates_expected_output_files() {
        let (_tmp, cfg) = temp_cfg();
        truth_files::init_truth_files(&cfg, false).expect("truth");
        let context = create_session(&cfg);
        compaction::compact_session(&cfg, &context.session_id).expect("compact");

        let report = run_loop_dry_run(
            &cfg,
            LoopDryRunRequest {
                scope: LoopScope::All,
                max_candidates: 5,
            },
        )
        .expect("loop");

        for file in [
            "loop-report.md",
            "loop-report.json",
            "candidates.json",
            "evidence-index.json",
            "context-decay.json",
        ] {
            assert!(report.output_dir.join(file).exists());
        }
    }

    #[test]
    fn dry_run_does_not_mutate_truth_files_or_compact_packets() {
        let (_tmp, cfg) = temp_cfg();
        truth_files::init_truth_files(&cfg, false).expect("truth");
        let context = create_session(&cfg);
        compaction::compact_session(&cfg, &context.session_id).expect("compact");
        let policy_path = cfg.workspace_dir.join("POLICY.md");
        let compact_path = cfg
            .workspace_dir
            .join("state/compacted")
            .join(context.session_id.as_str())
            .join("compact.json");
        let policy_before = fs::read_to_string(&policy_path).expect("policy");
        let compact_before = fs::read_to_string(&compact_path).expect("compact");

        run_loop_dry_run(
            &cfg,
            LoopDryRunRequest {
                scope: LoopScope::All,
                max_candidates: 5,
            },
        )
        .expect("loop");

        assert_eq!(
            fs::read_to_string(&policy_path).expect("policy"),
            policy_before
        );
        assert_eq!(
            fs::read_to_string(&compact_path).expect("compact"),
            compact_before
        );
    }

    #[test]
    fn red_context_status_blocks_execution_readiness() {
        let (_tmp, cfg) = temp_cfg();
        truth_files::init_truth_files(&cfg, false).expect("truth");
        create_session(&cfg);

        let report = run_loop_dry_run(
            &cfg,
            LoopDryRunRequest {
                scope: LoopScope::All,
                max_candidates: 5,
            },
        )
        .expect("loop");

        assert_eq!(report.execution_readiness, ExecutionReadiness::Blocked);
        assert_eq!(report.context_status.context_state, ContextState::Red);
    }

    #[test]
    fn max_candidates_is_respected() {
        let (_tmp, cfg) = temp_cfg();
        truth_files::init_truth_files(&cfg, false).expect("truth");
        create_session(&cfg);

        let report = run_loop_dry_run(
            &cfg,
            LoopDryRunRequest {
                scope: LoopScope::All,
                max_candidates: 2,
            },
        )
        .expect("loop");

        assert!(report.candidates.len() <= 2);
    }

    #[test]
    fn invalid_scope_fails_safely() {
        let error = "bogus".parse::<LoopScope>().expect_err("invalid scope");

        assert!(error.to_string().contains("invalid loop scope"));
    }

    #[test]
    fn missing_truth_files_are_reported_clearly() {
        let (_tmp, cfg) = temp_cfg();
        create_session(&cfg);

        let report = run_loop_dry_run(
            &cfg,
            LoopDryRunRequest {
                scope: LoopScope::All,
                max_candidates: 10,
            },
        )
        .expect("loop");

        assert!(
            report
                .context_status
                .missing_required_context
                .iter()
                .any(|item| item.contains("POLICY.md"))
        );
    }

    #[test]
    fn context_decay_never_auto_deprecates_truth_files() {
        let (_tmp, cfg) = temp_cfg();
        truth_files::init_truth_files(&cfg, false).expect("truth");

        let items = build_context_decay(&cfg, LoopScope::Truth).expect("decay");

        assert!(!items.is_empty());
        assert!(items.iter().all(|item| {
            !matches!(
                item.decay_action,
                DecayAction::Deprecate | DecayAction::Archive
            )
        }));
        assert!(
            items
                .iter()
                .any(|item| item.source_path.ends_with("POLICY.md")
                    && item.memory_class == MemoryClass::Canonical)
        );
    }
}
