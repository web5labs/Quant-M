use crate::adapters::AdapterHub;
use crate::config::Config;
use crate::logutil;
use crate::sessions::{self, SessionContext, SessionEvent};
use crate::shutdown;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::net::IpAddr;
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime};
use tokio::process::Command;
use tokio::sync::watch;
use tokio::time::{Duration, timeout};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JobKind {
    Shell { command: String },
    HttpGet { url: String },
    Echo { text: String },
    Sleep { millis: u64 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerJob {
    pub id: String,
    pub created_at: String,
    pub retries: u8,
    #[serde(flatten)]
    pub kind: JobKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerResult {
    pub id: String,
    pub status: String,
    pub output: String,
    pub error: Option<String>,
    pub started_at: String,
    pub finished_at: String,
    pub duration_ms: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WorkerState {
    pub node_id: String,
    pub started_at: String,
    pub last_tick: Option<String>,
    pub processed: u64,
    pub failed: u64,
    pub last_error: Option<String>,
    pub pid: u32,
}

#[derive(Debug, Deserialize)]
struct InboundWorkerJob {
    id: Option<String>,
    created_at: Option<String>,
    retries: Option<u8>,
    #[serde(flatten)]
    kind: JobKind,
}

#[derive(Debug, Serialize)]
struct DeadLetterLine {
    received_at: String,
    code: String,
    error: String,
    raw: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct FileSignature {
    modified: SystemTime,
    len: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WaitDecision {
    Continue,
    Shutdown,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum HttpLaneMode {
    DryRun,
    Sandbox,
    Live,
}

pub fn job_from_json(raw: &str) -> Result<WorkerJob> {
    let incoming: InboundWorkerJob =
        serde_json::from_str(raw).context("invalid worker job JSON payload")?;
    Ok(WorkerJob {
        id: incoming.id.unwrap_or_else(generate_job_id),
        created_at: incoming.created_at.unwrap_or_else(now_rfc3339),
        retries: incoming.retries.unwrap_or(0),
        kind: incoming.kind,
    })
}

pub async fn run_once(cfg: &Config, job: WorkerJob, adapters: &AdapterHub) -> Result<WorkerResult> {
    let session = new_job_session_context(cfg, "worker");
    let result = execute_job_with_session(cfg, &job, &session).await;
    if matches!(job.kind, JobKind::HttpGet { .. }) {
        let code = result
            .error
            .as_deref()
            .map(error_code_from_message)
            .unwrap_or("http_get_ok");
        let attempt = job.retries.saturating_add(1);
        let _ = logutil::append_log(
            &cfg.logging,
            &format!(
                "lane=http_get mode={} job_id={} attempt={} result={} code={}",
                lane_mode_label(http_lane_mode(&cfg.worker.http_get_mode)),
                job.id,
                attempt,
                result.status,
                code
            ),
        );
    }
    append_outbox(&cfg.worker.outbox_path, &result)?;
    let _ = adapters
        .send_simple(
            "worker_once",
            &format!("job={} status={}", result.id, result.status),
        )
        .await;
    Ok(result)
}

pub async fn run_loop(cfg: Config, adapters: AdapterHub) -> Result<()> {
    run_loop_with_shutdown(cfg, adapters, None).await
}

pub async fn run_loop_with_shutdown(
    cfg: Config,
    adapters: AdapterHub,
    mut shutdown_rx: Option<watch::Receiver<bool>>,
) -> Result<()> {
    logutil::append_log(&cfg.logging, "worker loop starting")?;

    let mut state = read_state(&cfg.worker.state_path).unwrap_or_default();
    if state.started_at.is_empty() {
        state.started_at = now_rfc3339();
    }
    if state.node_id.is_empty() {
        state.node_id = cfg.node_id.clone();
    }
    state.pid = std::process::id();
    persist_state(&cfg.worker.state_path, &state)?;
    let batch_path = batch_path(&cfg.worker.inflight_path);

    let base_poll_secs = cfg.worker.poll_interval_seconds.max(1);
    let max_poll_secs = base_poll_secs.saturating_mul(2).clamp(3, 10);
    let mut current_poll_secs = base_poll_secs;
    let mut inbox_signature = file_signature(&cfg.worker.inbox_path);
    let mut force_initial_drain = true;
    let mut last_state_flush = Instant::now();
    let mut state_dirty = false;
    loop {
        if wait_for_shutdown_or_inbox_change(
            &mut shutdown_rx,
            current_poll_secs,
            &cfg.worker.inbox_path,
            inbox_signature,
        )
        .await
            == WaitDecision::Shutdown
        {
            logutil::append_log(&cfg.logging, "worker loop stopping on shutdown signal")?;
            break;
        }

        state.last_tick = Some(now_rfc3339());
        state_dirty = true;

        let mut jobs = read_batch(&batch_path).unwrap_or_default();
        if let Some(inflight) =
            read_inflight(&cfg.worker.inflight_path, &cfg.worker.dead_letter_path)?
        {
            jobs.push(inflight);
        }
        let new_sig = file_signature(&cfg.worker.inbox_path);
        let inbox_changed = force_initial_drain || new_sig != inbox_signature;
        if inbox_changed {
            jobs.extend(drain_inbox(
                &cfg.worker.inbox_path,
                &cfg.worker.dead_letter_path,
            )?);
            inbox_signature = file_signature(&cfg.worker.inbox_path);
            force_initial_drain = false;
        }

        if jobs.is_empty() {
            if state_dirty && last_state_flush.elapsed() >= Duration::from_secs(15) {
                persist_state(&cfg.worker.state_path, &state)?;
                state_dirty = false;
                last_state_flush = Instant::now();
            }
            current_poll_secs = (current_poll_secs.saturating_mul(2)).min(max_poll_secs);
            continue;
        }

        current_poll_secs = base_poll_secs;
        persist_batch(&batch_path, &jobs)?;

        for (idx, mut job) in jobs.into_iter().enumerate() {
            persist_inflight(&cfg.worker.inflight_path, &job)?;

            let session = new_job_session_context(&cfg, "worker");
            let result = execute_job_with_session(&cfg, &job, &session).await;
            append_outbox(&cfg.worker.outbox_path, &result)?;

            if result.status == "ok" {
                state.processed = state.processed.saturating_add(1);
                state.last_error = None;
            } else {
                state.failed = state.failed.saturating_add(1);
                state.last_error = result.error.clone();

                let failure_reason = result.error.as_deref().unwrap_or("unknown");
                let failure_code = error_code_from_message(failure_reason);
                let retryable = is_retryable_failure(&job.kind, failure_code);
                let attempt = job.retries.saturating_add(1);
                let mode = lane_mode_label(http_lane_mode(&cfg.worker.http_get_mode));
                if matches!(job.kind, JobKind::HttpGet { .. }) {
                    let _ = logutil::append_log(
                        &cfg.logging,
                        &format!(
                            "lane=http_get mode={} job_id={} attempt={} result=error code={} retryable={}",
                            mode, job.id, attempt, failure_code, retryable
                        ),
                    );
                }

                if retryable && job.retries < cfg.worker.max_retries {
                    job.retries = job.retries.saturating_add(1);
                    record_session_event(
                        &cfg,
                        &session,
                        SessionEvent::Retry {
                            job_id: Some(job.id.clone()),
                            attempt,
                            next_attempt: Some(job.retries.saturating_add(1)),
                            reason: failure_reason.to_string(),
                        },
                    );
                    submit_job(&cfg, &job)?;
                    logutil::append_log(
                        &cfg.logging,
                        &format!(
                            "job={} status=retry retry={} code={}",
                            job.id, job.retries, failure_code
                        ),
                    )?;
                } else if matches!(job.kind, JobKind::HttpGet { .. }) {
                    let raw_job = serde_json::to_string(&job).unwrap_or_else(|_| "{}".to_string());
                    append_dead_letter_line(
                        &cfg.worker.dead_letter_path,
                        failure_code,
                        &raw_job,
                        failure_reason,
                    )?;
                }
                logutil::append_log(
                    &cfg.logging,
                    &format!(
                        "job={} status=error duration_ms={} code={} error={}",
                        result.id, result.duration_ms, failure_code, failure_reason
                    ),
                )?;
            }

            let _ = adapters
                .send_simple(
                    "worker_result",
                    &format!("job={} status={}", result.id, result.status),
                )
                .await;
            clear_inflight(&cfg.worker.inflight_path)?;
            drop_processed_batch_prefix(&batch_path, idx + 1)?;
            state_dirty = true;
            if last_state_flush.elapsed() >= Duration::from_secs(2) {
                persist_state(&cfg.worker.state_path, &state)?;
                state_dirty = false;
                last_state_flush = Instant::now();
            }
        }
        clear_batch(&batch_path)?;
        if state_dirty {
            persist_state(&cfg.worker.state_path, &state)?;
            state_dirty = false;
            last_state_flush = Instant::now();
        }
    }

    if state_dirty {
        persist_state(&cfg.worker.state_path, &state)?;
    }

    Ok(())
}

pub async fn execute_task_spec(spec: &str, cfg: &Config) -> WorkerResult {
    let trimmed = spec.trim();
    let job = if let Some(command) = trimmed.strip_prefix("shell:") {
        WorkerJob {
            id: generate_job_id(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::Shell {
                command: command.trim().to_string(),
            },
        }
    } else if let Some(url) = trimmed.strip_prefix("http:") {
        WorkerJob {
            id: generate_job_id(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::HttpGet {
                url: url.trim().to_string(),
            },
        }
    } else if let Some(text) = trimmed.strip_prefix("echo:") {
        WorkerJob {
            id: generate_job_id(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::Echo {
                text: text.trim().to_string(),
            },
        }
    } else if let Some(raw_json) = trimmed.strip_prefix("json:") {
        match job_from_json(raw_json.trim()) {
            Ok(job) => job,
            Err(err) => {
                return WorkerResult {
                    id: generate_job_id(),
                    status: "error".to_string(),
                    output: String::new(),
                    error: Some(format!("invalid json task spec: {err}")),
                    started_at: now_rfc3339(),
                    finished_at: now_rfc3339(),
                    duration_ms: 0,
                };
            }
        }
    } else {
        WorkerJob {
            id: generate_job_id(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::Echo {
                text: trimmed.to_string(),
            },
        }
    };

    let session = new_job_session_context(cfg, "heartbeat");
    execute_job_with_session(cfg, &job, &session).await
}

pub fn submit_job(cfg: &Config, job: &WorkerJob) -> Result<()> {
    let depth = queue_depth(&cfg.worker.inbox_path)?;
    if depth >= cfg.worker.max_inbox_depth {
        return Err(anyhow!(
            "inbox queue is full (depth={} max_inbox_depth={})",
            depth,
            cfg.worker.max_inbox_depth
        ));
    }
    append_json_line(&cfg.worker.inbox_path, job)?;
    logutil::append_log(
        &cfg.logging,
        &format!(
            "job submitted id={} kind={}",
            job.id,
            job_kind_name(&job.kind)
        ),
    )?;
    Ok(())
}

pub fn read_state(path: &Path) -> Option<WorkerState> {
    read_state_checked(path).ok().flatten()
}

pub fn read_state_checked(path: &Path) -> Result<Option<WorkerState>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let parsed = serde_json::from_str(&raw)
        .with_context(|| format!("invalid worker state JSON {}", path.display()))?;
    Ok(Some(parsed))
}

pub fn queue_depth(path: &Path) -> Result<usize> {
    if !path.exists() {
        return Ok(0);
    }
    let content =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .count())
}

async fn execute_job_with_session(
    cfg: &Config,
    job: &WorkerJob,
    session: &SessionContext,
) -> WorkerResult {
    let started = Utc::now();
    let started_at = started.to_rfc3339();
    record_session_event(
        cfg,
        session,
        SessionEvent::Observation {
            message: "job_received".to_string(),
            job_id: Some(job.id.clone()),
            detail: Some(format!(
                "kind={} retries={}",
                job_kind_name(&job.kind),
                job.retries
            )),
        },
    );
    record_session_event(
        cfg,
        session,
        SessionEvent::FsmTransition {
            machine: "worker_job".to_string(),
            from_state: Some("queued".to_string()),
            to_state: "executing".to_string(),
            reason: format!("starting {}", job_kind_name(&job.kind)),
        },
    );

    let computed = match &job.kind {
        JobKind::Echo { text } => Ok((text.clone(), None)),
        JobKind::Sleep { millis } => {
            let bounded = (*millis).min(120_000);
            tokio::time::sleep(Duration::from_millis(bounded)).await;
            Ok((format!("slept for {}ms", bounded), None))
        }
        JobKind::HttpGet { url } => {
            if !cfg.worker.allow_http_get {
                record_session_event(
                    cfg,
                    session,
                    SessionEvent::PolicyDecision {
                        policy: "worker.allow_http_get".to_string(),
                        allowed: false,
                        reason: "http_get is disabled by config".to_string(),
                    },
                );
                Err(anyhow!(
                    "code=http_get_disabled http_get is disabled by config (worker.allow_http_get=false)"
                ))
            } else {
                let mode = http_lane_mode(&cfg.worker.http_get_mode);
                let attempt = job.retries.saturating_add(1);
                let validated_url = match validate_outbound_url(url) {
                    Ok(parsed) => parsed,
                    Err(err) => {
                        return session_error_result(
                            cfg,
                            session,
                            job,
                            started,
                            started_at,
                            format!("code=http_get_invalid_request invalid request: {}", err),
                        );
                    }
                };
                if matches!(mode, HttpLaneMode::Sandbox)
                    && !is_host_allowed_for_sandbox(
                        &cfg.worker.http_get_sandbox_hosts,
                        validated_url.host_str().unwrap_or_default(),
                    )
                {
                    record_session_event(
                        cfg,
                        session,
                        SessionEvent::PolicyDecision {
                            policy: "worker.http_get_sandbox_hosts".to_string(),
                            allowed: false,
                            reason: format!(
                                "host not allowed: {}",
                                validated_url.host_str().unwrap_or_default()
                            ),
                        },
                    );
                    return session_error_result(
                        cfg,
                        session,
                        job,
                        started,
                        started_at,
                        format!(
                            "code=http_get_sandbox_host_blocked host not in worker.http_get_sandbox_hosts: {}",
                            validated_url.host_str().unwrap_or_default()
                        ),
                    );
                }
                if matches!(mode, HttpLaneMode::DryRun) {
                    let _ = logutil::append_log(
                        &cfg.logging,
                        &format!(
                            "lane=http_get mode={} job_id={} attempt={} result=success code=dry_run",
                            lane_mode_label(mode),
                            job.id,
                            attempt
                        ),
                    );
                    record_session_event(
                        cfg,
                        session,
                        SessionEvent::AuditNote {
                            note: format!(
                                "http_get lane remained in dry_run for {}",
                                validated_url
                            ),
                        },
                    );
                    Ok((format!("dry-run: would request {}", validated_url), None))
                } else {
                    let client = reqwest::Client::builder()
                        .timeout(Duration::from_secs(
                            cfg.worker.command_timeout_seconds.max(1),
                        ))
                        .danger_accept_invalid_certs(cfg.worker.allow_insecure_https)
                        .build();
                    match client {
                        Ok(client) => match client.get(validated_url.clone()).send().await {
                            Ok(mut resp) => {
                                let status = resp.status();
                                if !status.is_success() {
                                    let code = status_error_code(status.as_u16());
                                    Err(anyhow!(
                                        "code={} unexpected HTTP status {} for {}",
                                        code,
                                        status.as_u16(),
                                        validated_url
                                    ))
                                } else {
                                    let text = match read_http_response_body(&mut resp).await {
                                        Ok(body) => body,
                                        Err(err) => {
                                            return session_error_result(
                                                cfg,
                                                session,
                                                job,
                                                started,
                                                started_at,
                                                err.to_string(),
                                            );
                                        }
                                    };
                                    if text.is_empty() {
                                        Err(anyhow!(
                                            "code=http_get_empty_response empty HTTP response body"
                                        ))
                                    } else {
                                        let _ = logutil::append_log(
                                            &cfg.logging,
                                            &format!(
                                                "lane=http_get mode={} job_id={} attempt={} result=success code=http_get_ok",
                                                lane_mode_label(mode),
                                                job.id,
                                                attempt
                                            ),
                                        );
                                        Ok((truncate(&text), None))
                                    }
                                }
                            }
                            Err(err) => Err(anyhow!(
                                "code=http_get_transport HTTP request failed: {err}"
                            )),
                        },
                        Err(err) => Err(anyhow!(
                            "code=http_get_client failed creating HTTP client: {err}"
                        )),
                    }
                }
            }
        }
        JobKind::Shell { command } => {
            if !cfg.worker.allow_shell_commands {
                record_session_event(
                    cfg,
                    session,
                    SessionEvent::PolicyDecision {
                        policy: "worker.allow_shell_commands".to_string(),
                        allowed: false,
                        reason: "shell jobs are disabled by config".to_string(),
                    },
                );
                Err(anyhow!(
                    "shell jobs are disabled by config (worker.allow_shell_commands=false)"
                ))
            } else {
                let run = timeout(
                    Duration::from_secs(cfg.worker.command_timeout_seconds.max(1)),
                    Command::new("sh")
                        .arg("-lc")
                        .arg(command)
                        .kill_on_drop(true)
                        .output(),
                )
                .await;

                match run {
                    Ok(Ok(output)) => {
                        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                        let combined = if stderr.trim().is_empty() {
                            stdout
                        } else if stdout.trim().is_empty() {
                            stderr.clone()
                        } else {
                            format!("{stdout}\n{stderr}")
                        };
                        if output.status.success() {
                            Ok((truncate(&combined), None))
                        } else {
                            Err(anyhow!(
                                "command exited with status {}: {}",
                                output.status,
                                truncate(&combined)
                            ))
                        }
                    }
                    Ok(Err(err)) => Err(anyhow!("failed spawning command: {err}")),
                    Err(_) => Err(anyhow!("command timed out")),
                }
            }
        }
    };

    let finished = Utc::now();
    let duration_ms = (finished - started).num_milliseconds();

    match computed {
        Ok((output, error)) => {
            record_session_event(
                cfg,
                session,
                SessionEvent::Output {
                    channel: "worker".to_string(),
                    summary: truncate(&output),
                    job_id: Some(job.id.clone()),
                },
            );
            record_session_event(
                cfg,
                session,
                SessionEvent::FsmTransition {
                    machine: "worker_job".to_string(),
                    from_state: Some("executing".to_string()),
                    to_state: "completed".to_string(),
                    reason: "job completed successfully".to_string(),
                },
            );
            WorkerResult {
                id: job.id.clone(),
                status: "ok".to_string(),
                output,
                error,
                started_at,
                finished_at: finished.to_rfc3339(),
                duration_ms,
            }
        }
        Err(err) => {
            let message = err.to_string();
            record_session_event(
                cfg,
                session,
                SessionEvent::Error {
                    code: Some(error_code_from_message(&message).to_string()),
                    message: message.clone(),
                },
            );
            record_session_event(
                cfg,
                session,
                SessionEvent::FsmTransition {
                    machine: "worker_job".to_string(),
                    from_state: Some("executing".to_string()),
                    to_state: "failed".to_string(),
                    reason: "job execution failed".to_string(),
                },
            );
            WorkerResult {
                id: job.id.clone(),
                status: "error".to_string(),
                output: String::new(),
                error: Some(message),
                started_at,
                finished_at: finished.to_rfc3339(),
                duration_ms,
            }
        }
    }
}

async fn read_http_response_body(resp: &mut reqwest::Response) -> Result<String> {
    const MAX_HTTP_BODY_BYTES: usize = 64 * 1024;
    let mut body = Vec::new();
    loop {
        match resp.chunk().await {
            Ok(Some(chunk)) => {
                if body.len() >= MAX_HTTP_BODY_BYTES {
                    break;
                }
                let remaining = MAX_HTTP_BODY_BYTES - body.len();
                let take = remaining.min(chunk.len());
                body.extend_from_slice(&chunk[..take]);
                if take < chunk.len() {
                    break;
                }
            }
            Ok(None) => break,
            Err(err) => {
                return Err(anyhow!(
                    "code=http_get_response_read failed reading HTTP response chunk: {err}"
                ));
            }
        }
    }
    Ok(String::from_utf8_lossy(&body).trim().to_string())
}

fn persist_state(path: &Path, state: &WorkerState) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(state).context("failed to serialize worker state")?;
    atomic_write(path, raw.as_bytes())
}

fn append_outbox(path: &Path, result: &WorkerResult) -> Result<()> {
    append_json_line(path, result)
}

fn drain_inbox(path: &Path, dead_letter_path: &Path) -> Result<Vec<WorkerJob>> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let drain_path = drain_snapshot_path(path);
    let mut jobs = parse_jobs_from_snapshot(&drain_path, dead_letter_path)?;

    if !path.exists() {
        fs::write(path, "").with_context(|| format!("failed to write {}", path.display()))?;
        return Ok(jobs);
    }

    // Atomically move active inbox to a snapshot file so concurrent submitters
    // can continue writing to a fresh inbox without losing jobs.
    if drain_path.exists() {
        fs::remove_file(&drain_path)
            .with_context(|| format!("failed to remove {}", drain_path.display()))?;
    }
    if fs::metadata(path).map(|meta| meta.len()).unwrap_or(0) == 0 {
        return Ok(jobs);
    }

    fs::rename(path, &drain_path).with_context(|| {
        format!(
            "failed to move inbox {} -> {}",
            path.display(),
            drain_path.display()
        )
    })?;
    fs::write(path, "").with_context(|| format!("failed to re-create {}", path.display()))?;

    jobs.extend(parse_jobs_from_snapshot(&drain_path, dead_letter_path)?);
    Ok(jobs)
}

fn parse_jobs_from_snapshot(path: &Path, dead_letter_path: &Path) -> Result<Vec<WorkerJob>> {
    if !path.exists() {
        return Ok(vec![]);
    }

    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut jobs = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match job_from_json(trimmed) {
            Ok(job) => jobs.push(job),
            Err(err) => {
                append_dead_letter_line(
                    dead_letter_path,
                    "invalid_job_json",
                    trimmed,
                    &err.to_string(),
                )?;
            }
        }
    }

    fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    Ok(jobs)
}

fn append_json_line<T: Serialize>(path: &Path, item: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open {}", path.display()))?;
    let line = serde_json::to_string(item).context("failed to serialize json line")?;
    writeln!(file, "{line}").with_context(|| format!("failed to append {}", path.display()))
}

fn persist_inflight(path: &Path, job: &WorkerJob) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let raw = serde_json::to_string_pretty(job).context("failed to serialize inflight job")?;
    atomic_write(path, raw.as_bytes())
}

fn read_inflight(path: &Path, dead_letter_path: &Path) -> Result<Option<WorkerJob>> {
    if !path.exists() {
        return Ok(None);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    match serde_json::from_str(&raw) {
        Ok(job) => Ok(Some(job)),
        Err(err) => {
            append_dead_letter_line(
                dead_letter_path,
                "invalid_inflight_json",
                &raw,
                &format!("invalid inflight JSON: {err}"),
            )?;
            clear_inflight(path)?;
            Ok(None)
        }
    }
}

fn clear_inflight(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn job_kind_name(kind: &JobKind) -> &'static str {
    match kind {
        JobKind::Shell { .. } => "shell",
        JobKind::HttpGet { .. } => "http_get",
        JobKind::Echo { .. } => "echo",
        JobKind::Sleep { .. } => "sleep",
    }
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

fn generate_job_id() -> String {
    format!("job-{}", Utc::now().timestamp_nanos_opt().unwrap_or(0))
}

fn new_job_session_context(cfg: &Config, domain: &str) -> SessionContext {
    sessions::runtime_context(&cfg.node_id, domain)
}

fn record_session_event(cfg: &Config, session: &SessionContext, event: SessionEvent) {
    if let Err(err) = sessions::append_event(cfg, session, event) {
        let _ = logutil::append_log(
            &cfg.logging,
            &format!(
                "session_event_error session_id={} error={}",
                session.session_id, err
            ),
        );
    }
}

fn session_error_result(
    cfg: &Config,
    session: &SessionContext,
    job: &WorkerJob,
    started: chrono::DateTime<Utc>,
    started_at: String,
    message: String,
) -> WorkerResult {
    let finished = Utc::now();
    let duration_ms = (finished - started).num_milliseconds();
    record_session_event(
        cfg,
        session,
        SessionEvent::Error {
            code: Some(error_code_from_message(&message).to_string()),
            message: message.clone(),
        },
    );
    record_session_event(
        cfg,
        session,
        SessionEvent::FsmTransition {
            machine: "worker_job".to_string(),
            from_state: Some("executing".to_string()),
            to_state: "failed".to_string(),
            reason: "job execution failed".to_string(),
        },
    );
    WorkerResult {
        id: job.id.clone(),
        status: "error".to_string(),
        output: String::new(),
        error: Some(message),
        started_at,
        finished_at: finished.to_rfc3339(),
        duration_ms,
    }
}

fn truncate(text: &str) -> String {
    const MAX: usize = 8192;
    if text.len() <= MAX {
        text.to_string()
    } else {
        format!("{}...[truncated]", &text[..MAX])
    }
}

fn file_signature(path: &Path) -> Option<FileSignature> {
    let meta = fs::metadata(path).ok()?;
    Some(FileSignature {
        modified: meta.modified().ok()?,
        len: meta.len(),
    })
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }
    let tmp_path = path.with_extension(format!(
        "{}.tmp-{}",
        path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("file"),
        std::process::id()
    ));
    fs::write(&tmp_path, bytes)
        .with_context(|| format!("failed to write temp file {}", tmp_path.display()))?;
    fs::rename(&tmp_path, path).with_context(|| {
        format!(
            "failed to atomically replace {} with {}",
            path.display(),
            tmp_path.display()
        )
    })
}

fn drain_snapshot_path(path: &Path) -> PathBuf {
    path.with_extension("drain")
}

fn append_dead_letter_line(path: &Path, code: &str, raw: &str, error: &str) -> Result<()> {
    let line = DeadLetterLine {
        received_at: now_rfc3339(),
        code: code.to_string(),
        error: error.to_string(),
        raw: truncate_dead_letter_raw(raw),
    };
    append_json_line(path, &line)
}

fn truncate_dead_letter_raw(raw: &str) -> String {
    const MAX: usize = 4096;
    if raw.len() <= MAX {
        raw.to_string()
    } else {
        format!("{}...[truncated]", &raw[..MAX])
    }
}

async fn wait_for_shutdown_or_inbox_change(
    shutdown_rx: &mut Option<watch::Receiver<bool>>,
    poll_secs: u64,
    inbox_path: &Path,
    known_signature: Option<FileSignature>,
) -> WaitDecision {
    let mut remaining = poll_secs.max(1);
    while remaining > 0 {
        let step_secs = remaining.min(1);
        if let Some(rx) = shutdown_rx.as_mut() {
            tokio::select! {
                _ = shutdown::wait_for_shutdown_signal() => return WaitDecision::Shutdown,
                changed = rx.changed() => {
                    if changed.is_ok() && *rx.borrow() {
                        return WaitDecision::Shutdown;
                    }
                }
                _ = tokio::time::sleep(Duration::from_secs(step_secs)) => {}
            }
        } else {
            tokio::select! {
                _ = shutdown::wait_for_shutdown_signal() => return WaitDecision::Shutdown,
                _ = tokio::time::sleep(Duration::from_secs(step_secs)) => {}
            }
        }

        if file_signature(inbox_path) != known_signature {
            return WaitDecision::Continue;
        }
        remaining = remaining.saturating_sub(step_secs);
    }
    WaitDecision::Continue
}

fn batch_path(inflight_path: &Path) -> PathBuf {
    inflight_path.with_extension("batch.json")
}

fn read_batch(path: &Path) -> Result<Vec<WorkerJob>> {
    if !path.exists() {
        return Ok(vec![]);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    serde_json::from_str::<Vec<WorkerJob>>(&raw)
        .with_context(|| format!("invalid batch JSON {}", path.display()))
}

fn persist_batch(path: &Path, jobs: &[WorkerJob]) -> Result<()> {
    let raw = serde_json::to_string_pretty(jobs).context("failed to serialize batch")?;
    atomic_write(path, raw.as_bytes())
}

fn drop_processed_batch_prefix(path: &Path, processed_count: usize) -> Result<()> {
    if !path.exists() {
        return Ok(());
    }
    let jobs = read_batch(path)?;
    if processed_count >= jobs.len() {
        clear_batch(path)
    } else {
        persist_batch(path, &jobs[processed_count..])
    }
}

fn clear_batch(path: &Path) -> Result<()> {
    if path.exists() {
        fs::remove_file(path).with_context(|| format!("failed to remove {}", path.display()))?;
    }
    Ok(())
}

fn http_lane_mode(mode: &str) -> HttpLaneMode {
    match mode.trim().to_ascii_lowercase().as_str() {
        "live" => HttpLaneMode::Live,
        "sandbox" => HttpLaneMode::Sandbox,
        _ => HttpLaneMode::DryRun,
    }
}

fn lane_mode_label(mode: HttpLaneMode) -> &'static str {
    match mode {
        HttpLaneMode::DryRun => "dry_run",
        HttpLaneMode::Sandbox => "sandbox",
        HttpLaneMode::Live => "live",
    }
}

fn error_code_from_message(error: &str) -> &str {
    error
        .strip_prefix("code=")
        .and_then(|rest| rest.split_whitespace().next())
        .unwrap_or("job_error_unknown")
}

fn status_error_code(status: u16) -> &'static str {
    if status == 408 || status == 429 {
        "http_get_status_retryable"
    } else if status >= 500 {
        "http_get_status_5xx"
    } else {
        "http_get_status_4xx"
    }
}

fn is_retryable_failure(kind: &JobKind, code: &str) -> bool {
    if !matches!(kind, JobKind::HttpGet { .. }) {
        return true;
    }
    matches!(
        code,
        "http_get_transport"
            | "http_get_response_read"
            | "http_get_status_retryable"
            | "http_get_status_5xx"
    )
}

fn is_host_allowed_for_sandbox(hosts: &[String], host: &str) -> bool {
    let normalized = host.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return false;
    }
    hosts.iter().any(|allowed| {
        normalized == *allowed
            || normalized
                .strip_suffix(allowed)
                .is_some_and(|prefix| prefix.ends_with('.'))
    })
}

fn validate_outbound_url(raw: &str) -> Result<reqwest::Url> {
    if raw.trim().is_empty() {
        return Err(anyhow!("URL is empty"));
    }
    let parsed = reqwest::Url::parse(raw).with_context(|| format!("invalid URL '{raw}'"))?;
    if parsed.scheme() != "https" {
        return Err(anyhow!("only https URLs are allowed"));
    }
    if let Some(host) = parsed.host_str() {
        if host.eq_ignore_ascii_case("localhost") {
            return Err(anyhow!("localhost is not allowed"));
        }
        if let Ok(ip) = host.parse::<IpAddr>() {
            let blocked = match ip {
                IpAddr::V4(ipv4) => {
                    ipv4.is_private()
                        || ipv4.is_loopback()
                        || ipv4.is_link_local()
                        || ipv4.is_multicast()
                        || ipv4.is_unspecified()
                }
                IpAddr::V6(ipv6) => {
                    ipv6.is_loopback()
                        || ipv6.is_unspecified()
                        || ipv6.is_multicast()
                        || ipv6.is_unique_local()
                        || ipv6.is_unicast_link_local()
                }
            };
            if blocked {
                return Err(anyhow!("private/loopback IPs are not allowed"));
            }
        }
    }
    Ok(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn test_config(tmp: &TempDir) -> Config {
        let workspace = tmp.path().join("workspace");
        Config {
            node_id: "test-node".to_string(),
            worker: crate::config::WorkerConfig {
                inbox_path: workspace.join("queue/inbox.ndjson"),
                outbox_path: workspace.join("queue/outbox.ndjson"),
                inflight_path: workspace.join("queue/inflight.json"),
                state_path: workspace.join("state/worker_state.json"),
                dead_letter_path: workspace.join("queue/dead-letter.ndjson"),
                poll_interval_seconds: 1,
                command_timeout_seconds: 2,
                concurrency: 1,
                max_retries: 1,
                max_inbox_depth: 10,
                allow_shell_commands: false,
                allow_http_get: false,
                allow_insecure_https: false,
                http_get_mode: "dry_run".to_string(),
                http_get_sandbox_hosts: vec![],
            },
            logging: crate::config::LoggingConfig {
                file: workspace.join("logs/quant-m.log"),
                max_bytes: 1_048_576,
                keep_files: 3,
            },
            ..Config::default()
        }
    }

    #[test]
    fn parse_job_json_defaults_id_and_created_at() {
        let raw = r#"{"kind":"echo","text":"hello"}"#;
        let job = job_from_json(raw).expect("job parse");
        assert!(!job.id.is_empty());
        assert!(!job.created_at.is_empty());
    }

    #[tokio::test]
    async fn execute_echo_task_spec() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let result = execute_task_spec("echo:hello worker", &cfg).await;
        assert_eq!(result.status, "ok");
        assert!(result.output.contains("hello worker"));
    }

    #[tokio::test]
    async fn http_get_dry_run_is_safe_and_successful() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        cfg.worker.allow_http_get = true;
        cfg.worker.http_get_mode = "dry_run".to_string();
        let result = execute_task_spec("http:https://example.com", &cfg).await;
        assert_eq!(result.status, "ok");
        assert!(result.output.contains("dry-run"));
    }

    #[tokio::test]
    async fn invalid_json_task_spec_is_rejected() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let result = execute_task_spec("json:{not-json", &cfg).await;
        assert_eq!(result.status, "error");
        assert!(
            result
                .error
                .as_deref()
                .unwrap_or_default()
                .contains("invalid json task spec")
        );
    }

    #[test]
    fn submit_and_count_queue_depth() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);

        let job = WorkerJob {
            id: "job-1".to_string(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::Echo {
                text: "ok".to_string(),
            },
        };
        submit_job(&cfg, &job).expect("submit");
        let depth = queue_depth(&cfg.worker.inbox_path).expect("depth");
        assert_eq!(depth, 1);
    }

    #[test]
    fn drain_inbox_routes_invalid_lines_to_dead_letter() {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = test_config(&tmp);
        let inbox_parent = cfg.worker.inbox_path.parent().expect("inbox parent");
        fs::create_dir_all(inbox_parent).expect("create inbox parent");
        fs::write(
            &cfg.worker.inbox_path,
            "{\"kind\":\"echo\",\"text\":\"ok\"}\nnot-json\n",
        )
        .expect("write inbox");

        let jobs =
            drain_inbox(&cfg.worker.inbox_path, &cfg.worker.dead_letter_path).expect("drain inbox");
        assert_eq!(jobs.len(), 1);
        assert!(cfg.worker.dead_letter_path.exists());
        let dead_depth = queue_depth(&cfg.worker.dead_letter_path).expect("dead letter depth");
        assert_eq!(dead_depth, 1);
    }

    #[test]
    fn submit_job_rejects_when_queue_is_full() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        cfg.worker.max_inbox_depth = 1;

        let job = WorkerJob {
            id: "job-1".to_string(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::Echo {
                text: "one".to_string(),
            },
        };
        submit_job(&cfg, &job).expect("first submit should pass");

        let second = WorkerJob {
            id: "job-2".to_string(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::Echo {
                text: "two".to_string(),
            },
        };
        let err = submit_job(&cfg, &second).expect_err("queue should reject second submit");
        assert!(err.to_string().contains("inbox queue is full"));
    }

    #[test]
    fn retryable_http_failures_are_classified() {
        assert!(is_retryable_failure(
            &JobKind::HttpGet {
                url: "https://example.com".to_string()
            },
            "http_get_status_5xx"
        ));
        assert!(is_retryable_failure(
            &JobKind::HttpGet {
                url: "https://example.com".to_string()
            },
            "http_get_transport"
        ));
        assert!(!is_retryable_failure(
            &JobKind::HttpGet {
                url: "https://example.com".to_string()
            },
            "http_get_status_4xx"
        ));
        assert!(!is_retryable_failure(
            &JobKind::HttpGet {
                url: "https://example.com".to_string()
            },
            "http_get_invalid_request"
        ));
    }

    #[tokio::test]
    async fn permanent_http_failure_routes_to_dead_letter() {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = test_config(&tmp);
        cfg.worker.allow_http_get = true;
        cfg.worker.http_get_mode = "sandbox".to_string();
        cfg.worker.http_get_sandbox_hosts = vec!["sandbox.allowed.example".to_string()];
        cfg.worker.max_retries = 1;

        let job = WorkerJob {
            id: "job-http-dead".to_string(),
            created_at: now_rfc3339(),
            retries: 0,
            kind: JobKind::HttpGet {
                url: "https://example.com".to_string(),
            },
        };
        submit_job(&cfg, &job).expect("submit");

        let adapters = AdapterHub::new(&cfg).expect("adapters");
        let (tx, rx) = watch::channel(false);
        let cfg_run = cfg.clone();
        let handle =
            tokio::spawn(async move { run_loop_with_shutdown(cfg_run, adapters, Some(rx)).await });

        let mut seen_dead_letter = false;
        for _ in 0..30 {
            if queue_depth(&cfg.worker.dead_letter_path).unwrap_or(0) > 0 {
                seen_dead_letter = true;
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        tx.send(true).expect("signal shutdown");
        handle.await.expect("join").expect("worker loop");

        let dead_depth = queue_depth(&cfg.worker.dead_letter_path).expect("dead-letter depth");
        assert!(seen_dead_letter);
        assert_eq!(dead_depth, 1);
    }
}
