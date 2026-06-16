use crate::config::Config;
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthInitReport {
    pub workspace_dir: PathBuf,
    pub files: Vec<TruthFileReport>,
    pub recommended_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TruthFileReport {
    pub path: PathBuf,
    pub status: TruthFileStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TruthFileStatus {
    Created,
    Present,
    Overwritten,
}

pub fn init_truth_files(cfg: &Config, force: bool) -> Result<TruthInitReport> {
    fs::create_dir_all(&cfg.workspace_dir)
        .with_context(|| format!("failed to create {}", cfg.workspace_dir.display()))?;

    let mut files = Vec::new();
    for spec in truth_file_specs() {
        files.push(write_truth_file(&cfg.workspace_dir, spec, force)?);
    }

    Ok(TruthInitReport {
        workspace_dir: cfg.workspace_dir.clone(),
        files,
        recommended_next_action: "Run quant-m context-status".to_string(),
    })
}

pub fn render_truth_init_report(report: &TruthInitReport) -> String {
    let mut lines = vec![format!("workspace_dir: {}", report.workspace_dir.display())];
    for file in &report.files {
        lines.push(format!(
            "{}: {}",
            file.path.display(),
            truth_file_status_label(&file.status)
        ));
    }
    lines.push(format!(
        "recommended_next_action: {}",
        report.recommended_next_action
    ));
    lines.join("\n") + "\n"
}

fn write_truth_file(
    workspace_dir: &Path,
    spec: TruthFileSpec,
    force: bool,
) -> Result<TruthFileReport> {
    let path = workspace_dir.join(spec.name);
    if path.exists() && !force {
        return Ok(TruthFileReport {
            path,
            status: TruthFileStatus::Present,
        });
    }
    fs::write(&path, spec.content)
        .with_context(|| format!("failed to write {}", path.display()))?;
    Ok(TruthFileReport {
        path,
        status: if force {
            TruthFileStatus::Overwritten
        } else {
            TruthFileStatus::Created
        },
    })
}

fn truth_file_status_label(status: &TruthFileStatus) -> &'static str {
    match status {
        TruthFileStatus::Created => "created",
        TruthFileStatus::Present => "present",
        TruthFileStatus::Overwritten => "overwritten",
    }
}

#[derive(Debug, Clone, Copy)]
struct TruthFileSpec {
    name: &'static str,
    content: &'static str,
}

fn truth_file_specs() -> [TruthFileSpec; 4] {
    [
        TruthFileSpec {
            name: "QUANTM.md",
            content: "# QUANTM\n\nQuant-M is the local Rust runtime harness for this workspace.\n\n## Purpose\n\n- Preserve session evidence.\n- Distill long work into compact truth packets.\n- Keep terminal cockpits as surfaces, not orchestrators.\n\n## Core Files\n\n- POLICY.md defines safety boundaries.\n- SHIPPABLE.md defines what done means.\n- AGENTS.md defines lanes without granting permissions.\n",
        },
        TruthFileSpec {
            name: "POLICY.md",
            content: "# POLICY\n\nDefault posture: strict and evidence-first.\n\n## Forbidden Without Explicit Approval\n\n- No live trading.\n- No credential edits.\n- No shell escalation.\n- No HTTP or network escalation.\n- No terminal or cockpit launch escalation.\n- No provider, model, or CLI execution permission is implied by configuration.\n\n## Required\n\n- Preserve evidence for meaningful actions.\n- Do not claim validation without proof.\n- Do not claim changed files without file evidence.\n- Do not treat missing policy, missing validation, or missing shippable definition as success.\n",
        },
        TruthFileSpec {
            name: "SHIPPABLE.md",
            content: "# SHIPPABLE\n\nThis is a placeholder definition of shippable. The operator must refine it for this project.\n\n## Minimum Bar\n\n- The intended change is clear.\n- Relevant validation evidence is captured.\n- Policy blocks are preserved or resolved explicitly.\n- Missing evidence is reported honestly.\n- The next safe action is documented.\n",
        },
        TruthFileSpec {
            name: "AGENTS.md",
            content: "# AGENTS\n\nThese lanes describe responsibilities only. They do not grant permissions.\n\n## Default Lanes\n\n- operator: reviews policy, approvals, and shippable criteria.\n- planner: reads evidence and proposes safe next actions.\n- worker: performs approved narrow implementation work.\n- validator: records validation evidence.\n- auditor: checks compact packets, risks, and blocked actions.\n",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let cfg = Config {
            workspace_dir,
            ..Config::default()
        };
        (tmp, cfg)
    }

    #[test]
    fn creates_missing_truth_files() {
        let (_tmp, cfg) = temp_cfg();

        let report = init_truth_files(&cfg, false).expect("init truth");

        assert_eq!(report.files.len(), 4);
        for file in ["QUANTM.md", "POLICY.md", "SHIPPABLE.md", "AGENTS.md"] {
            assert!(cfg.workspace_dir.join(file).exists());
        }
        assert!(
            report
                .files
                .iter()
                .all(|file| file.status == TruthFileStatus::Created)
        );
    }

    #[test]
    fn does_not_overwrite_existing_files_by_default() {
        let (_tmp, cfg) = temp_cfg();
        fs::create_dir_all(&cfg.workspace_dir).expect("workspace");
        fs::write(cfg.workspace_dir.join("POLICY.md"), "# Custom policy\n").expect("policy");

        let report = init_truth_files(&cfg, false).expect("init truth");

        assert_eq!(
            fs::read_to_string(cfg.workspace_dir.join("POLICY.md")).expect("policy"),
            "# Custom policy\n"
        );
        assert!(report.files.iter().any(
            |file| file.path.ends_with("POLICY.md") && file.status == TruthFileStatus::Present
        ));
    }

    #[test]
    fn force_overwrites_existing_files() {
        let (_tmp, cfg) = temp_cfg();
        fs::create_dir_all(&cfg.workspace_dir).expect("workspace");
        fs::write(cfg.workspace_dir.join("POLICY.md"), "# Custom policy\n").expect("policy");

        let report = init_truth_files(&cfg, true).expect("init truth");

        let policy = fs::read_to_string(cfg.workspace_dir.join("POLICY.md")).expect("policy");
        assert!(policy.contains("No live trading."));
        assert!(
            report
                .files
                .iter()
                .any(|file| file.path.ends_with("POLICY.md")
                    && file.status == TruthFileStatus::Overwritten)
        );
    }

    #[test]
    fn generated_policy_contains_conservative_defaults() {
        let (_tmp, cfg) = temp_cfg();

        init_truth_files(&cfg, false).expect("init truth");
        let policy = fs::read_to_string(cfg.workspace_dir.join("POLICY.md")).expect("policy");

        assert!(policy.contains("No live trading."));
        assert!(policy.contains("No credential edits."));
        assert!(policy.contains("No shell escalation."));
        assert!(policy.contains("No HTTP or network escalation."));
        assert!(policy.contains("Do not claim validation without proof."));
    }

    #[test]
    fn generated_shippable_contains_placeholder_definition() {
        let (_tmp, cfg) = temp_cfg();

        init_truth_files(&cfg, false).expect("init truth");
        let shippable =
            fs::read_to_string(cfg.workspace_dir.join("SHIPPABLE.md")).expect("shippable");

        assert!(shippable.contains("placeholder definition of shippable"));
        assert!(shippable.contains("operator must refine"));
    }
}
