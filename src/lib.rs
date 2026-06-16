pub mod adapters;
pub mod agent_shell;
pub mod bootstrap;
pub mod channels;
pub mod cluster_boundary;
pub mod compaction;
pub mod config;
pub mod consensus;
pub mod context_decay;
pub mod context_firewall;
pub mod context_status;
pub mod cost_ledger;
pub mod daemon;
pub mod desk_registry;
pub mod domain;
pub mod execution_runtime;
pub mod forex;
pub mod fsm_registry;
pub mod heartbeat;
pub mod llm;
pub mod logutil;
pub mod loop_dry_run;
pub mod memory;
pub mod policy_registry;
pub mod question;
pub mod scheduler_registry;
pub mod sessions;
pub mod shared_state;
pub mod shutdown;
pub mod skill_registry;
pub mod skills;
pub mod state_sql;
pub mod strategist;
pub mod telegram;
pub mod truth_files;
pub mod tui_shell;
pub mod worker;
pub mod worker_proposals;
pub mod workflow_registry;

pub mod ingest {
    use anyhow::{Result, anyhow};

    use crate::worker::{self, WorkerJob};

    pub fn parse_worker_job_json(raw: &str) -> Result<WorkerJob> {
        worker::job_from_json(raw)
    }

    pub fn parse_worker_jobs_ndjson(raw: &str, max_lines: usize) -> Result<Vec<WorkerJob>> {
        let mut out = Vec::new();
        for line in raw.lines().take(max_lines.max(1)) {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            out.push(worker::job_from_json(trimmed)?);
        }
        Ok(out)
    }

    pub fn validate_worker_job_minimal(job: &WorkerJob) -> Result<()> {
        if job.id.trim().is_empty() {
            return Err(anyhow!("worker job id is empty"));
        }
        if job.created_at.trim().is_empty() {
            return Err(anyhow!("worker job created_at is empty"));
        }
        Ok(())
    }
}
