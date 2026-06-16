use crate::adapters::AdapterHub;
use crate::config::Config;
use crate::logutil;
use crate::shutdown;
use crate::worker::{self, WorkerResult};
use anyhow::{Context, Result};
use std::fs;
use std::time::SystemTime;
use tokio::sync::watch;
use tokio::time::Duration;

pub fn parse_tasks(content: &str) -> Vec<String> {
    content
        .lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            trimmed
                .strip_prefix("- ")
                .map(|task| task.trim().to_string())
        })
        .filter(|task| !task.is_empty())
        .collect()
}

pub async fn tick(cfg: &Config, adapters: &AdapterHub) -> Result<Vec<WorkerResult>> {
    if !cfg.heartbeat.enabled {
        return Ok(vec![]);
    }

    if !cfg.heartbeat.tasks_file.exists() {
        return Ok(vec![]);
    }

    let content = fs::read_to_string(&cfg.heartbeat.tasks_file).with_context(|| {
        format!(
            "failed to read heartbeat file {}",
            cfg.heartbeat.tasks_file.display()
        )
    })?;
    let tasks = parse_tasks(&content);
    run_tasks(cfg, adapters, tasks).await
}

async fn run_tasks(
    cfg: &Config,
    adapters: &AdapterHub,
    tasks: Vec<String>,
) -> Result<Vec<WorkerResult>> {
    let mut results = Vec::with_capacity(tasks.len());

    for task in tasks {
        let result = worker::execute_task_spec(&task, cfg).await;
        let status_line = format!("heartbeat task='{}' status={}", task, result.status);
        logutil::append_log(&cfg.logging, &status_line)?;
        let _ = adapters.send_simple("heartbeat", &status_line).await;
        results.push(result);
    }

    Ok(results)
}

pub async fn run_loop(cfg: Config, adapters: AdapterHub) -> Result<()> {
    run_loop_with_shutdown(cfg, adapters, None).await
}

pub async fn run_loop_with_shutdown(
    cfg: Config,
    adapters: AdapterHub,
    mut shutdown_rx: Option<watch::Receiver<bool>>,
) -> Result<()> {
    let interval_secs = cfg.heartbeat.interval_seconds.max(5);
    let mut timer = tokio::time::interval(Duration::from_secs(interval_secs));
    let mut cache = TasksCache::default();

    logutil::append_log(
        &cfg.logging,
        &format!("heartbeat loop starting interval={}s", interval_secs),
    )?;

    loop {
        if let Some(rx) = shutdown_rx.as_mut() {
            tokio::select! {
                _ = shutdown::wait_for_shutdown_signal() => {
                    logutil::append_log(&cfg.logging, "heartbeat loop stopping on shutdown signal")?;
                    break;
                }
                changed = rx.changed() => {
                    if changed.is_ok() && *rx.borrow() {
                        logutil::append_log(&cfg.logging, "heartbeat loop stopping via daemon shutdown")?;
                        break;
                    }
                }
                _ = timer.tick() => {
                    if let Err(err) = tick_cached(&cfg, &adapters, &mut cache).await {
                        logutil::append_log(&cfg.logging, &format!("heartbeat tick failed: {err}"))?;
                    }
                }
            }
        } else {
            tokio::select! {
                _ = shutdown::wait_for_shutdown_signal() => {
                    logutil::append_log(&cfg.logging, "heartbeat loop stopping on shutdown signal")?;
                    break;
                }
                _ = timer.tick() => {
                    if let Err(err) = tick_cached(&cfg, &adapters, &mut cache).await {
                        logutil::append_log(&cfg.logging, &format!("heartbeat tick failed: {err}"))?;
                    }
                }
            }
        }
    }

    Ok(())
}

#[derive(Default)]
struct TasksCache {
    modified: Option<SystemTime>,
    tasks: Vec<String>,
}

async fn tick_cached(
    cfg: &Config,
    adapters: &AdapterHub,
    cache: &mut TasksCache,
) -> Result<Vec<WorkerResult>> {
    if !cfg.heartbeat.enabled {
        return Ok(vec![]);
    }
    if !cfg.heartbeat.tasks_file.exists() {
        return Ok(vec![]);
    }

    let current_modified = fs::metadata(&cfg.heartbeat.tasks_file)
        .with_context(|| {
            format!(
                "failed to stat heartbeat file {}",
                cfg.heartbeat.tasks_file.display()
            )
        })?
        .modified()
        .context("failed to read heartbeat mtime")?;

    if cache.modified != Some(current_modified) {
        let content = fs::read_to_string(&cfg.heartbeat.tasks_file).with_context(|| {
            format!(
                "failed to read heartbeat file {}",
                cfg.heartbeat.tasks_file.display()
            )
        })?;
        cache.tasks = parse_tasks(&content);
        cache.modified = Some(current_modified);
    }

    run_tasks(cfg, adapters, cache.tasks.clone()).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_bulleted_tasks() {
        let content = "# HEARTBEAT\n\n- shell:uptime\n- echo:hello\nnot-a-task\n";
        let tasks = parse_tasks(content);
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0], "shell:uptime");
        assert_eq!(tasks[1], "echo:hello");
    }
}
