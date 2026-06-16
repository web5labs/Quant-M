use crate::config::Config;
use anyhow::{Context, Result, anyhow};

pub fn ensure_workspace(cfg: &Config) -> Result<()> {
    std::fs::create_dir_all(&cfg.workspace_dir)
        .with_context(|| format!("failed to create {}", cfg.workspace_dir.display()))?;

    let queue_dir = path_parent(&cfg.worker.inbox_path, "worker.inbox_path")?;
    let state_dir = path_parent(&cfg.worker.state_path, "worker.state_path")?;
    let dead_letter_dir = path_parent(&cfg.worker.dead_letter_path, "worker.dead_letter_path")?;
    let log_dir = path_parent(&cfg.logging.file, "logging.file")?;
    let sqlite_dir = path_parent(&cfg.memory.sqlite_path, "memory.sqlite_path")?;
    let shared_state_dir = path_parent(&cfg.state_sql.sqlite_path, "state_sql.sqlite_path")?;
    let session_dir = cfg.runtime.session_dir.clone();

    for dir in [
        &cfg.memory.daily_dir,
        &cfg.skills.dir,
        &queue_dir,
        &state_dir,
        &session_dir,
        &dead_letter_dir,
        &log_dir,
        &sqlite_dir,
        &shared_state_dir,
    ] {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("failed to create {}", dir.display()))?;
    }

    write_if_missing(
        cfg.workspace_dir.join("SOUL.md"),
        "# SOUL\n\n- mission: Quant-M is a lean local-first worker runtime\n- behavior: deterministic, bounded, low-memory\n",
    )?;
    write_if_missing(
        cfg.workspace_dir.join("USER.md"),
        "# USER\n\n- preferred_workflow: coordinator -> worker -> return JSON\n- deployment: android termux + ssh\n",
    )?;
    write_if_missing(
        cfg.workspace_dir.join("AGENTS.md"),
        "# AGENTS\n\n- worker: executes narrow jobs from queue\n- heartbeat: runs periodic checks from HEARTBEAT.md\n",
    )?;
    write_if_missing(
        cfg.memory.core_markdown.clone(),
        "# MEMORY\n\n- Quant-M initialized\n",
    )?;
    write_if_missing(
        cfg.heartbeat.tasks_file.clone(),
        "# HEARTBEAT\n\n# Add periodic tasks with bullet syntax.\n# Example: - echo:heartbeat alive\n",
    )?;
    write_if_missing(cfg.worker.inbox_path.clone(), "")?;
    write_if_missing(cfg.worker.outbox_path.clone(), "")?;
    write_if_missing(cfg.worker.dead_letter_path.clone(), "")?;

    Ok(())
}

fn write_if_missing(path: impl AsRef<std::path::Path>, content: &str) -> Result<()> {
    let path = path.as_ref();
    if path.exists() {
        return Ok(());
    }
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn path_parent(path: &std::path::Path, field_name: &str) -> Result<std::path::PathBuf> {
    path.parent()
        .map(std::path::Path::to_path_buf)
        .ok_or_else(|| anyhow!("{} has no parent: {}", field_name, path.display()))
}
