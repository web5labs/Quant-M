use crate::adapters::AdapterHub;
use crate::config::Config;
use crate::{context_guardian, heartbeat, logutil, shutdown, telegram, worker};
use anyhow::Result;
use tokio::sync::watch;
use tokio::task::JoinHandle;
use tokio::time::Duration;

const MAX_BACKOFF_SECONDS: u64 = 60;

pub async fn run(cfg: Config) -> Result<()> {
    logutil::append_log(&cfg.logging, "daemon starting")?;
    let adapters = AdapterHub::new(&cfg)?;
    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let worker_shutdown_rx = shutdown_rx.clone();
    let worker_task = spawn_supervisor(
        "worker",
        cfg.clone(),
        adapters.clone(),
        move |cfg, adapters| {
            worker::run_loop_with_shutdown(cfg, adapters, Some(worker_shutdown_rx.clone()))
        },
    );
    let heartbeat_shutdown_rx = shutdown_rx.clone();
    let heartbeat_task = spawn_supervisor(
        "heartbeat",
        cfg.clone(),
        adapters.clone(),
        move |cfg, adapters| {
            heartbeat::run_loop_with_shutdown(cfg, adapters, Some(heartbeat_shutdown_rx.clone()))
        },
    );
    let telegram_shutdown_rx = shutdown_rx.clone();
    let telegram_task = spawn_supervisor(
        "telegram",
        cfg.clone(),
        adapters.clone(),
        move |cfg, adapters| {
            telegram::run_loop_with_shutdown(cfg, adapters, Some(telegram_shutdown_rx.clone()))
        },
    );
    let guardian_shutdown_rx = shutdown_rx.clone();
    let guardian_task = spawn_supervisor(
        "context-guardian",
        cfg.clone(),
        adapters.clone(),
        move |cfg, _adapters| {
            context_guardian::run_loop_with_shutdown(cfg, Some(guardian_shutdown_rx.clone()))
        },
    );

    shutdown::wait_for_shutdown_signal().await?;
    logutil::append_log(&cfg.logging, "daemon shutdown signal received")?;
    let _ = shutdown_tx.send(true);

    let mut worker_task = worker_task;
    let mut heartbeat_task = heartbeat_task;

    let worker_shutdown = tokio::time::timeout(Duration::from_secs(10), &mut worker_task).await;
    if worker_shutdown.is_err() {
        worker_task.abort();
    }

    let heartbeat_shutdown =
        tokio::time::timeout(Duration::from_secs(10), &mut heartbeat_task).await;
    if heartbeat_shutdown.is_err() {
        heartbeat_task.abort();
    }

    let mut telegram_task = telegram_task;
    let telegram_shutdown = tokio::time::timeout(Duration::from_secs(10), &mut telegram_task).await;
    if telegram_shutdown.is_err() {
        telegram_task.abort();
    }

    let mut guardian_task = guardian_task;
    let guardian_shutdown = tokio::time::timeout(Duration::from_secs(10), &mut guardian_task).await;
    if guardian_shutdown.is_err() {
        guardian_task.abort();
    }

    logutil::append_log(&cfg.logging, "daemon stopped")?;
    Ok(())
}

fn spawn_supervisor<F, Fut>(
    name: &'static str,
    cfg: Config,
    adapters: AdapterHub,
    runner: F,
) -> JoinHandle<()>
where
    F: Fn(Config, AdapterHub) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<()>> + Send + 'static,
{
    tokio::spawn(async move {
        let mut backoff = 1_u64;
        loop {
            match runner(cfg.clone(), adapters.clone()).await {
                Ok(()) => {
                    let _ = logutil::append_log(
                        &cfg.logging,
                        &format!("daemon component '{}' stopped cleanly", name),
                    );
                    break;
                }
                Err(err) => {
                    let _ = logutil::append_log(
                        &cfg.logging,
                        &format!(
                            "daemon component '{}' failed: {} (restart in {}s)",
                            name, err, backoff
                        ),
                    );
                    tokio::time::sleep(Duration::from_secs(backoff)).await;
                    backoff = (backoff.saturating_mul(2)).min(MAX_BACKOFF_SECONDS);
                }
            }
        }
    })
}
