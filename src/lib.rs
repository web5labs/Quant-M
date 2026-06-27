#[cfg(not(feature = "child-min"))]
pub mod adapters;
#[cfg(not(feature = "child-min"))]
pub mod agent_shell;
#[cfg(not(feature = "child-min"))]
pub mod boil;
#[cfg(not(feature = "child-min"))]
pub mod bootstrap;
#[cfg(not(feature = "child-min"))]
pub mod capabilities;
#[cfg(not(feature = "child-min"))]
pub mod channels;
#[cfg(not(feature = "child-min"))]
pub mod cluster;
#[cfg(not(feature = "child-min"))]
pub mod cluster_boundary;
#[cfg(not(feature = "child-min"))]
pub mod compaction;
#[cfg(not(feature = "child-min"))]
pub mod compute;
#[cfg(not(feature = "child-min"))]
pub mod config;
#[cfg(not(feature = "child-min"))]
pub mod consensus;
#[cfg(not(feature = "child-min"))]
pub mod context_decay;
#[cfg(not(feature = "child-min"))]
pub mod context_firewall;
#[cfg(not(feature = "child-min"))]
pub mod context_guardian;
#[cfg(not(feature = "child-min"))]
pub mod context_status;
#[cfg(not(feature = "child-min"))]
pub mod cost_ledger;
#[cfg(not(feature = "child-min"))]
pub mod daemon;
#[cfg(not(feature = "child-min"))]
pub mod demo_flow;
#[cfg(not(feature = "child-min"))]
pub mod desk_registry;
#[cfg(not(feature = "child-min"))]
pub mod device;
pub mod device_telemetry;
#[cfg(not(feature = "child-min"))]
pub mod domain;
#[cfg(not(feature = "child-min"))]
pub mod execution_runtime;
#[cfg(not(feature = "child-min"))]
pub mod forex;
#[cfg(not(feature = "child-min"))]
pub mod fsm_authority;
#[cfg(not(feature = "child-min"))]
pub mod fsm_core;
#[cfg(not(feature = "child-min"))]
pub mod fsm_registry;
#[cfg(not(feature = "child-min"))]
pub mod heartbeat;
#[cfg(not(feature = "child-min"))]
pub mod llm;
#[cfg(not(feature = "child-min"))]
pub mod logutil;
#[cfg(not(feature = "child-min"))]
pub mod loop_dry_run;
#[cfg(not(feature = "child-min"))]
pub mod memory;
#[cfg(not(feature = "child-min"))]
pub mod model_router;
#[cfg(not(feature = "child-min"))]
pub mod numeric_hotpath;
#[cfg(not(feature = "child-min"))]
pub mod pairing;
#[cfg(not(feature = "child-min"))]
pub mod playbook;
#[cfg(not(feature = "child-min"))]
pub mod policy_registry;
#[cfg(not(feature = "child-min"))]
pub mod question;
#[cfg(not(feature = "child-min"))]
pub mod scheduler_registry;
#[cfg(not(feature = "child-min"))]
pub mod sessions;
#[cfg(not(feature = "child-min"))]
pub mod shared_state;
#[cfg(not(feature = "child-min"))]
pub mod shutdown;
#[cfg(not(feature = "child-min"))]
pub mod side_effect_gate;
#[cfg(not(feature = "child-min"))]
pub mod skill_registry;
#[cfg(not(feature = "child-min"))]
pub mod skills;
#[cfg(not(feature = "child-min"))]
pub mod state_review;
#[cfg(not(feature = "child-min"))]
pub mod state_sql;
#[cfg(not(feature = "child-min"))]
pub mod strategist;
#[cfg(not(feature = "child-min"))]
pub mod telegram;
#[cfg(not(feature = "child-min"))]
pub mod timing;
#[cfg(not(feature = "child-min"))]
pub mod truth_files;
#[cfg(not(feature = "child-min"))]
pub mod tui_shell;
#[cfg(not(feature = "child-min"))]
pub mod worker;
#[cfg(not(feature = "child-min"))]
pub mod worker_proposals;
#[cfg(not(feature = "child-min"))]
pub mod workflow_registry;

#[cfg(not(feature = "child-min"))]
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
