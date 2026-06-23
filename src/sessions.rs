use crate::config::Config;
use crate::fsm_core::SessionLifecycleState;
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};

static ID_COUNTER: AtomicU64 = AtomicU64::new(1);

macro_rules! typed_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[serde(transparent)]
        pub struct $name(String);

        #[allow(dead_code)]
        impl $name {
            pub fn new_generated() -> Self {
                Self(generate_id($prefix))
            }

            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }

        impl FromStr for $name {
            type Err = anyhow::Error;

            fn from_str(value: &str) -> Result<Self> {
                let trimmed = value.trim();
                if trimmed.is_empty() {
                    return Err(anyhow!(concat!(stringify!($name), " is empty")));
                }
                Ok(Self(trimmed.to_string()))
            }
        }
    };
}

typed_id!(SessionId, "session");
typed_id!(RunId, "run");
typed_id!(AgentId, "agent");
typed_id!(StepId, "step");
typed_id!(DomainId, "domain");

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionContext {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub domain_id: DomainId,
}

impl SessionContext {
    pub fn new(agent_id: AgentId, domain_id: DomainId) -> Self {
        Self {
            session_id: SessionId::new_generated(),
            run_id: RunId::new_generated(),
            agent_id,
            domain_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum SessionEvent {
    Observation {
        message: String,
        job_id: Option<String>,
        detail: Option<String>,
    },
    SkillCall {
        skill_name: String,
        input_preview: String,
        command_preview: Option<String>,
        status: String,
    },
    PolicyDecision {
        policy: String,
        allowed: bool,
        reason: String,
    },
    FsmTransition {
        machine: String,
        from_state: Option<String>,
        to_state: String,
        reason: String,
    },
    Error {
        code: Option<String>,
        message: String,
    },
    Retry {
        job_id: Option<String>,
        attempt: u8,
        next_attempt: Option<u8>,
        reason: String,
    },
    Output {
        channel: String,
        summary: String,
        job_id: Option<String>,
    },
    OperatorDecision {
        record: OperatorDecisionRecord,
    },
    AuditNote {
        note: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionLogEntry {
    pub sequence: u64,
    pub session_id: SessionId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub step_id: StepId,
    pub domain_id: DomainId,
    pub occurred_at: String,
    pub event: SessionEvent,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionSummary {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub agent_id: AgentId,
    pub domain_id: DomainId,
    pub started_at: String,
    pub last_event_at: String,
    pub event_count: usize,
    pub output_count: usize,
    pub error_count: usize,
    pub retry_count: usize,
    pub final_status: String,
    pub typed_final_state: SessionLifecycleState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionDetail {
    pub summary: SessionSummary,
    pub events: Vec<SessionLogEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionReplayState {
    pub current_fsm_state: Option<String>,
    pub typed_final_state: SessionLifecycleState,
    pub observations: usize,
    pub skill_calls: usize,
    pub policy_decisions: usize,
    pub policy_denials: usize,
    pub errors: usize,
    pub retries: usize,
    pub outputs: usize,
    pub operator_decisions: usize,
    pub audit_notes: usize,
    pub last_output: Option<String>,
    pub last_error: Option<String>,
    pub last_skill: Option<String>,
    pub last_operator_decision: Option<OperatorDecision>,
    pub final_status: String,
    pub side_effects_replayed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SessionReplay {
    pub summary: SessionSummary,
    pub state: SessionReplayState,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum OperatorDecision {
    Approved,
    Denied,
    NeedsMoreInfo,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OperatorDecisionRecord {
    pub session_id: SessionId,
    pub run_id: RunId,
    pub step_id: StepId,
    pub domain_id: DomainId,
    pub decision: OperatorDecision,
    pub reason: String,
    pub decided_at: String,
    pub decided_by: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ResumeStatus {
    Complete,
    FailedInspectable,
    WaitingForInput,
    WaitingForPolicyApproval,
    ReplayOnly,
    UnsafeToResume,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ResumePlan {
    pub session_id: SessionId,
    pub last_run_id: RunId,
    pub last_step_id: StepId,
    pub domain_id: DomainId,
    pub status: ResumeStatus,
    pub safe_to_resume: bool,
    pub blocked_reasons: Vec<String>,
    pub required_operator_actions: Vec<String>,
    pub proposed_next_step: Option<String>,
}

pub fn runtime_context(node_id: &str, domain: &str) -> SessionContext {
    SessionContext::new(
        AgentId::new(format!("agent:{node_id}")),
        DomainId::new(format!("domain:{domain}")),
    )
}

pub fn append_event(cfg: &Config, context: &SessionContext, event: SessionEvent) -> Result<()> {
    let path = session_log_path(cfg, &context.session_id);
    let sequence = existing_event_count(&path)?.saturating_add(1);
    let entry = SessionLogEntry {
        sequence,
        session_id: context.session_id.clone(),
        run_id: context.run_id.clone(),
        agent_id: context.agent_id.clone(),
        step_id: StepId::new(format!("step-{sequence:06}")),
        domain_id: context.domain_id.clone(),
        occurred_at: now_rfc3339(),
        event,
    };
    append_json_line(&path, &entry)
}

pub fn list_sessions(cfg: &Config) -> Result<Vec<SessionSummary>> {
    let dir = session_dir(cfg);
    if !dir.exists() {
        return Ok(vec![]);
    }

    let mut sessions = Vec::new();
    for entry in fs::read_dir(&dir).with_context(|| format!("failed to read {}", dir.display()))? {
        let entry = entry.context("failed to read session directory entry")?;
        let path = entry.path();
        if !is_session_log_path(&path) {
            continue;
        }
        let events = read_session_entries_from_path(&path)?;
        if events.is_empty() {
            continue;
        }
        sessions.push(build_summary(&events)?);
    }

    sessions.sort_by(|a, b| {
        b.last_event_at
            .cmp(&a.last_event_at)
            .then_with(|| a.session_id.as_str().cmp(b.session_id.as_str()))
    });
    Ok(sessions)
}

pub fn show_session(cfg: &Config, session_id: &SessionId) -> Result<SessionDetail> {
    let events = read_session_entries(cfg, session_id)?;
    if events.is_empty() {
        return Err(anyhow!("session '{}' has no events", session_id));
    }
    Ok(SessionDetail {
        summary: build_summary(&events)?,
        events,
    })
}

pub fn replay_session(cfg: &Config, session_id: &SessionId) -> Result<SessionReplay> {
    let events = read_session_entries(cfg, session_id)?;
    if events.is_empty() {
        return Err(anyhow!("session '{}' has no events", session_id));
    }
    Ok(SessionReplay {
        summary: build_summary(&events)?,
        state: replay_entries(&events),
    })
}

pub fn resume_plan_session(cfg: &Config, session_id: &SessionId) -> Result<ResumePlan> {
    let events = read_session_entries(cfg, session_id)?;
    build_resume_plan(&events)
}

pub(crate) fn replay_session_from_entries_for_compaction(
    events: &[SessionLogEntry],
) -> Result<SessionReplay> {
    Ok(SessionReplay {
        summary: build_summary(events)?,
        state: replay_entries(events),
    })
}

pub(crate) fn resume_plan_from_entries_for_compaction(
    events: &[SessionLogEntry],
) -> Result<ResumePlan> {
    build_resume_plan(events)
}

pub fn record_operator_decision(
    cfg: &Config,
    session_id: &SessionId,
    decision: OperatorDecision,
    reason: &str,
    decided_by: &str,
) -> Result<OperatorDecisionRecord> {
    let trimmed_reason = reason.trim();
    if trimmed_reason.is_empty() {
        return Err(anyhow!("operator decision reason is empty"));
    }
    let trimmed_decided_by = decided_by.trim();
    if trimmed_decided_by.is_empty() {
        return Err(anyhow!("operator decision decided_by is empty"));
    }

    let events = read_session_entries(cfg, session_id)?;
    let last = events
        .last()
        .ok_or_else(|| anyhow!("session '{}' has no events", session_id))?;
    let record = OperatorDecisionRecord {
        session_id: last.session_id.clone(),
        run_id: last.run_id.clone(),
        step_id: last.step_id.clone(),
        domain_id: last.domain_id.clone(),
        decision,
        reason: trimmed_reason.to_string(),
        decided_at: now_rfc3339(),
        decided_by: trimmed_decided_by.to_string(),
    };
    let context = SessionContext {
        session_id: last.session_id.clone(),
        run_id: last.run_id.clone(),
        agent_id: last.agent_id.clone(),
        domain_id: last.domain_id.clone(),
    };
    append_event(
        cfg,
        &context,
        SessionEvent::OperatorDecision {
            record: record.clone(),
        },
    )?;
    Ok(record)
}

fn build_summary(events: &[SessionLogEntry]) -> Result<SessionSummary> {
    let first = events
        .first()
        .ok_or_else(|| anyhow!("session event list is empty"))?;
    let last = events
        .last()
        .ok_or_else(|| anyhow!("session event list is empty"))?;
    let state = replay_entries(events);
    Ok(SessionSummary {
        session_id: first.session_id.clone(),
        run_id: first.run_id.clone(),
        agent_id: first.agent_id.clone(),
        domain_id: first.domain_id.clone(),
        started_at: first.occurred_at.clone(),
        last_event_at: last.occurred_at.clone(),
        event_count: events.len(),
        output_count: state.outputs,
        error_count: state.errors,
        retry_count: state.retries,
        final_status: state.final_status,
        typed_final_state: state.typed_final_state,
    })
}

fn build_resume_plan(events: &[SessionLogEntry]) -> Result<ResumePlan> {
    let first = events
        .first()
        .ok_or_else(|| anyhow!("session event list is empty"))?;
    let last = events
        .last()
        .ok_or_else(|| anyhow!("session event list is empty"))?;
    let analysis = analyze_resume(entries_for_resume(events));
    let (status, blocked_reasons, required_operator_actions, proposed_next_step) =
        finalize_resume_plan(&analysis, last);

    Ok(ResumePlan {
        session_id: first.session_id.clone(),
        last_run_id: last.run_id.clone(),
        last_step_id: last.step_id.clone(),
        domain_id: last.domain_id.clone(),
        status,
        safe_to_resume: false,
        blocked_reasons,
        required_operator_actions,
        proposed_next_step,
    })
}

fn replay_entries(events: &[SessionLogEntry]) -> SessionReplayState {
    let mut state = SessionReplayState {
        current_fsm_state: None,
        typed_final_state: SessionLifecycleState::Created,
        observations: 0,
        skill_calls: 0,
        policy_decisions: 0,
        policy_denials: 0,
        errors: 0,
        retries: 0,
        outputs: 0,
        operator_decisions: 0,
        audit_notes: 0,
        last_output: None,
        last_error: None,
        last_skill: None,
        last_operator_decision: None,
        final_status: "unknown".to_string(),
        side_effects_replayed: false,
    };

    for entry in events {
        match &entry.event {
            SessionEvent::Observation { .. } => {
                state.observations = state.observations.saturating_add(1);
            }
            SessionEvent::SkillCall {
                skill_name, status, ..
            } => {
                state.skill_calls = state.skill_calls.saturating_add(1);
                state.last_skill = Some(skill_name.clone());
                state.final_status = status.clone();
            }
            SessionEvent::PolicyDecision { allowed, .. } => {
                state.policy_decisions = state.policy_decisions.saturating_add(1);
                if !allowed {
                    state.policy_denials = state.policy_denials.saturating_add(1);
                    state.final_status = "blocked".to_string();
                }
            }
            SessionEvent::FsmTransition { to_state, .. } => {
                state.current_fsm_state = Some(to_state.clone());
                state.final_status = to_state.clone();
            }
            SessionEvent::Error { message, .. } => {
                state.errors = state.errors.saturating_add(1);
                state.last_error = Some(message.clone());
                state.final_status = "error".to_string();
            }
            SessionEvent::Retry { .. } => {
                state.retries = state.retries.saturating_add(1);
                state.final_status = "retrying".to_string();
            }
            SessionEvent::Output { summary, .. } => {
                state.outputs = state.outputs.saturating_add(1);
                state.last_output = Some(summary.clone());
                state.final_status = "ok".to_string();
            }
            SessionEvent::OperatorDecision { record } => {
                state.operator_decisions = state.operator_decisions.saturating_add(1);
                state.last_operator_decision = Some(record.decision.clone());
                state.final_status = match record.decision {
                    OperatorDecision::Approved => "operator_approved".to_string(),
                    OperatorDecision::Denied => "operator_denied".to_string(),
                    OperatorDecision::NeedsMoreInfo => "operator_needs_more_info".to_string(),
                };
            }
            SessionEvent::AuditNote { .. } => {
                state.audit_notes = state.audit_notes.saturating_add(1);
            }
        }
    }

    state.typed_final_state = typed_session_state_from_replay(&state);
    state
}

fn typed_session_state_from_replay(state: &SessionReplayState) -> SessionLifecycleState {
    if let Some(current) = &state.current_fsm_state
        && let Ok(parsed) = current.parse::<SessionLifecycleState>()
    {
        return parsed;
    }
    if state.policy_denials > 0 && state.errors == 0 {
        return SessionLifecycleState::WaitingForApproval;
    }
    if state.errors > 0 {
        return SessionLifecycleState::Failed;
    }
    if state.final_status.to_ascii_lowercase().contains("replay")
        || state
            .last_output
            .as_deref()
            .is_some_and(|value| value.to_ascii_lowercase().contains("dry-run"))
    {
        return SessionLifecycleState::ReplayOnly;
    }
    state
        .final_status
        .parse::<SessionLifecycleState>()
        .unwrap_or(SessionLifecycleState::Recording)
}

#[cfg(feature = "fuzzing_hooks")]
#[allow(dead_code)]
pub fn parse_and_replay_event_for_fuzz(raw: &str) -> Result<SessionReplayState> {
    let event = serde_json::from_str::<SessionEvent>(raw)?;
    if let SessionEvent::OperatorDecision { record } = &event {
        if record.reason.trim().is_empty() {
            return Err(anyhow!("operator decision reason is empty"));
        }
        if record.decided_by.trim().is_empty() {
            return Err(anyhow!("operator decision decided_by is empty"));
        }
        let _ = SessionId::from_str(record.session_id.as_str())?;
        let _ = RunId::from_str(record.run_id.as_str())?;
        let _ = StepId::from_str(record.step_id.as_str())?;
        let _ = DomainId::from_str(record.domain_id.as_str())?;
    }

    let entries = vec![SessionLogEntry {
        sequence: 1,
        session_id: SessionId::new("session:fuzz"),
        run_id: RunId::new("run:fuzz"),
        agent_id: AgentId::new("agent:fuzz"),
        step_id: StepId::new("step:fuzz"),
        domain_id: DomainId::new("domain:fuzz"),
        occurred_at: now_rfc3339(),
        event,
    }];
    let _ = build_summary(&entries)?;
    Ok(replay_entries(&entries))
}

#[derive(Debug, Clone, Default)]
struct ResumeAnalysis {
    current_fsm_state: Option<String>,
    last_observation_hint: Option<String>,
    last_skill_name: Option<String>,
    last_skill_status: Option<String>,
    last_output: Option<String>,
    last_error: Option<String>,
    last_complete_seq: Option<u64>,
    last_waiting_input_seq: Option<u64>,
    last_waiting_input_reason: Option<String>,
    last_policy_block_seq: Option<u64>,
    last_policy_name: Option<String>,
    last_policy_reason: Option<String>,
    last_policy_allow_seq: Option<u64>,
    last_failed_seq: Option<u64>,
    last_failed_reason: Option<String>,
    last_replay_only_seq: Option<u64>,
    last_replay_only_reason: Option<String>,
    last_interrupted_seq: Option<u64>,
    last_interrupted_reason: Option<String>,
    last_operator_decision_seq: Option<u64>,
    last_operator_decision: Option<OperatorDecision>,
    last_operator_reason: Option<String>,
}

fn entries_for_resume(events: &[SessionLogEntry]) -> &[SessionLogEntry] {
    events
}

fn analyze_resume(events: &[SessionLogEntry]) -> ResumeAnalysis {
    let mut analysis = ResumeAnalysis::default();

    for entry in events {
        match &entry.event {
            SessionEvent::Observation {
                message, detail, ..
            } => {
                let hint = detail
                    .as_ref()
                    .map(|value| format!("{message}: {value}"))
                    .unwrap_or_else(|| message.clone());
                analysis.last_observation_hint = Some(hint.clone());
                if let Some(waiting_reason) = detect_waiting_input_hint(&hint) {
                    analysis.last_waiting_input_seq = Some(entry.sequence);
                    analysis.last_waiting_input_reason = Some(waiting_reason);
                }
                if let Some(replay_reason) = detect_replay_only_hint(&hint) {
                    analysis.last_replay_only_seq = Some(entry.sequence);
                    analysis.last_replay_only_reason = Some(replay_reason);
                }
            }
            SessionEvent::SkillCall {
                skill_name, status, ..
            } => {
                analysis.last_skill_name = Some(skill_name.clone());
                analysis.last_skill_status = Some(status.clone());
                if is_pending_state(status) {
                    analysis.last_interrupted_seq = Some(entry.sequence);
                    analysis.last_interrupted_reason = Some(format!(
                        "last skill '{}' remained in non-terminal status '{}'",
                        skill_name, status
                    ));
                }
                if let Some(replay_reason) = detect_replay_only_hint(status) {
                    analysis.last_replay_only_seq = Some(entry.sequence);
                    analysis.last_replay_only_reason = Some(replay_reason);
                }
            }
            SessionEvent::PolicyDecision {
                policy,
                allowed,
                reason,
            } => {
                if *allowed {
                    analysis.last_policy_allow_seq = Some(entry.sequence);
                } else {
                    analysis.last_policy_block_seq = Some(entry.sequence);
                    analysis.last_policy_name = Some(policy.clone());
                    analysis.last_policy_reason = Some(reason.clone());
                }
            }
            SessionEvent::FsmTransition {
                to_state, reason, ..
            } => {
                analysis.current_fsm_state = Some(to_state.clone());
                if is_complete_state(to_state) {
                    analysis.last_complete_seq = Some(entry.sequence);
                } else if is_failed_state(to_state) {
                    analysis.last_failed_seq = Some(entry.sequence);
                    analysis.last_failed_reason =
                        Some(format!("fsm ended in '{}' because {}", to_state, reason));
                } else if is_waiting_input_state(to_state) {
                    analysis.last_waiting_input_seq = Some(entry.sequence);
                    analysis.last_waiting_input_reason =
                        Some(format!("fsm paused in '{}'", to_state));
                } else if is_pending_state(to_state) {
                    analysis.last_interrupted_seq = Some(entry.sequence);
                    analysis.last_interrupted_reason =
                        Some(format!("fsm stopped in non-terminal state '{}'", to_state));
                }
            }
            SessionEvent::Error { message, .. } => {
                analysis.last_error = Some(message.clone());
                analysis.last_failed_seq = Some(entry.sequence);
                analysis.last_failed_reason = Some(message.clone());
            }
            SessionEvent::Retry {
                attempt,
                next_attempt,
                reason,
                ..
            } => {
                analysis.last_interrupted_seq = Some(entry.sequence);
                analysis.last_interrupted_reason = Some(match next_attempt {
                    Some(next) => format!(
                        "retry {} scheduled after attempt {} because {}",
                        next, attempt, reason
                    ),
                    None => format!("retry attempt {} recorded because {}", attempt, reason),
                });
            }
            SessionEvent::Output { summary, .. } => {
                analysis.last_output = Some(summary.clone());
                analysis.last_complete_seq = Some(entry.sequence);
            }
            SessionEvent::OperatorDecision { record } => {
                analysis.last_operator_decision_seq = Some(entry.sequence);
                analysis.last_operator_decision = Some(record.decision.clone());
                analysis.last_operator_reason = Some(record.reason.clone());
            }
            SessionEvent::AuditNote { note } => {
                if let Some(waiting_reason) = detect_waiting_input_hint(note) {
                    analysis.last_waiting_input_seq = Some(entry.sequence);
                    analysis.last_waiting_input_reason = Some(waiting_reason);
                }
                if let Some(replay_reason) = detect_replay_only_hint(note) {
                    analysis.last_replay_only_seq = Some(entry.sequence);
                    analysis.last_replay_only_reason = Some(replay_reason);
                }
            }
        }
    }

    analysis
}

fn finalize_resume_plan(
    analysis: &ResumeAnalysis,
    last: &SessionLogEntry,
) -> (ResumeStatus, Vec<String>, Vec<String>, Option<String>) {
    let complete_seq = analysis.last_complete_seq.unwrap_or(0);
    let waiting_input_active = analysis
        .last_waiting_input_seq
        .is_some_and(|seq| seq > complete_seq);
    let policy_block_active = analysis
        .last_policy_block_seq
        .is_some_and(|seq| seq > complete_seq && seq > analysis.last_policy_allow_seq.unwrap_or(0));
    let failure_active = analysis
        .last_failed_seq
        .is_some_and(|seq| seq > complete_seq);
    let replay_only_active = analysis.last_replay_only_seq.is_some_and(|seq| {
        seq > complete_seq
            && seq > analysis.last_failed_seq.unwrap_or(0)
            && seq > analysis.last_policy_block_seq.unwrap_or(0)
            && seq > analysis.last_waiting_input_seq.unwrap_or(0)
    });
    let decision_after_policy_block = analysis
        .last_operator_decision_seq
        .is_some_and(|seq| seq > analysis.last_policy_block_seq.unwrap_or(0));
    if complete_seq > 0 && !waiting_input_active && !policy_block_active && !failure_active {
        return (ResumeStatus::Complete, vec![], vec![], None);
    }

    if policy_block_active && decision_after_policy_block {
        return finalize_operator_resolution(analysis, last, replay_only_active, failure_active);
    }

    if waiting_input_active {
        let reason = analysis
            .last_waiting_input_reason
            .clone()
            .unwrap_or_else(|| "session is waiting for operator input".to_string());
        let actions = vec![
            "Provide the missing operator or user input outside the replay path.".to_string(),
            "Start a fresh run manually after the required input is available.".to_string(),
        ];
        let next_step = Some(format!(
            "collect the missing input for {} before launching another run",
            last.domain_id
        ));
        return (
            ResumeStatus::WaitingForInput,
            vec![reason],
            actions,
            next_step,
        );
    }

    if policy_block_active {
        let policy = analysis
            .last_policy_name
            .clone()
            .unwrap_or_else(|| "unknown policy".to_string());
        let reason = analysis
            .last_policy_reason
            .clone()
            .unwrap_or_else(|| "policy denied the last action".to_string());
        let actions = vec![
            format!("Review '{}' and decide whether the blocked action should ever be allowed.", policy),
            "If approval is intentional, change config or policy explicitly before starting a new run.".to_string(),
        ];
        let next_step = Some(format!(
            "review policy '{}' and then intentionally launch a fresh run if approval is justified",
            policy
        ));
        return (
            ResumeStatus::WaitingForPolicyApproval,
            vec![format!("{}: {}", policy, reason)],
            actions,
            next_step,
        );
    }

    if failure_active {
        let reason = analysis
            .last_failed_reason
            .clone()
            .or_else(|| analysis.last_error.clone())
            .unwrap_or_else(|| "session ended in a failed state".to_string());
        let actions = vec![
            "Inspect the persisted session events and error details before retrying.".to_string(),
            "Fix the root cause and launch a new run manually instead of resuming in place."
                .to_string(),
        ];
        let next_step = Some(format!(
            "inspect failed step {} and prepare a clean rerun once the root cause is fixed",
            last.step_id
        ));
        return (
            ResumeStatus::FailedInspectable,
            vec![reason],
            actions,
            next_step,
        );
    }

    if replay_only_active {
        let reason = analysis
            .last_replay_only_reason
            .clone()
            .unwrap_or_else(|| "session only captured replay-safe evidence".to_string());
        let actions = vec![
            "Use this session for audit and evidence only.".to_string(),
            "Start a separate live-capable run manually if real execution is still desired."
                .to_string(),
        ];
        let next_step = Some(
            "treat the session as replay-only evidence and create a fresh run for future work"
                .to_string(),
        );
        return (ResumeStatus::ReplayOnly, vec![reason], actions, next_step);
    }

    let interrupted_reason =
        analysis
            .last_interrupted_reason
            .clone()
            .or_else(|| {
                analysis.current_fsm_state.as_ref().map(|state| {
                    format!("session ended while the runtime still reported '{}'", state)
                })
            })
            .or_else(|| {
                analysis.last_skill_status.as_ref().map(|status| {
                    format!("session ended with non-terminal skill status '{}'", status)
                })
            })
            .or_else(|| analysis.last_observation_hint.clone())
            .unwrap_or_else(|| "session ended without a terminal outcome".to_string());
    let actions = vec![
        "Inspect the session timeline before deciding whether a follow-up run is appropriate."
            .to_string(),
        "Launch a fresh run manually if you want to continue from this boundary.".to_string(),
    ];
    let next_step = Some(match analysis.last_skill_name.as_deref() {
        Some(skill_name) => format!(
            "review interrupted skill '{}' at step {} and decide whether to relaunch it in a new run",
            skill_name, last.step_id
        ),
        None => format!(
            "review step {} in domain {} and decide whether to create a fresh follow-up run",
            last.step_id, last.domain_id
        ),
    });
    (
        ResumeStatus::UnsafeToResume,
        vec![interrupted_reason],
        actions,
        next_step,
    )
}

fn finalize_operator_resolution(
    analysis: &ResumeAnalysis,
    last: &SessionLogEntry,
    replay_only_active: bool,
    failure_active: bool,
) -> (ResumeStatus, Vec<String>, Vec<String>, Option<String>) {
    let decision = analysis
        .last_operator_decision
        .clone()
        .unwrap_or(OperatorDecision::NeedsMoreInfo);
    let operator_reason = analysis
        .last_operator_reason
        .clone()
        .unwrap_or_else(|| "operator decision recorded without a reason".to_string());
    let policy = analysis
        .last_policy_name
        .clone()
        .unwrap_or_else(|| "unknown policy".to_string());
    let blocked_reason = analysis
        .last_policy_reason
        .clone()
        .unwrap_or_else(|| "policy denied the last action".to_string());

    match decision {
        OperatorDecision::Approved => {
            if replay_only_active {
                let actions = vec![
                    "Keep this session as audit evidence only; approval does not execute it."
                        .to_string(),
                    "Create a fresh run manually if you want to continue beyond the replay-only boundary."
                        .to_string(),
                ];
                let next_step = Some(format!(
                    "operator approved '{}' for review, but the session remains replay-only; launch a fresh run manually if needed",
                    policy
                ));
                return (
                    ResumeStatus::ReplayOnly,
                    vec![format!(
                        "operator approved '{}' but the session is still replay-only: {}",
                        policy, operator_reason
                    )],
                    actions,
                    next_step,
                );
            }

            let actions = vec![
                "Collect any remaining input or config changes outside the replay path."
                    .to_string(),
                "Start a fresh run manually when you are ready; approval is not execution."
                    .to_string(),
            ];
            let next_step = Some(format!(
                "operator approved '{}' with reason '{}'; prepare any missing input and launch a fresh run manually",
                policy, operator_reason
            ));
            (
                ResumeStatus::WaitingForInput,
                vec![format!(
                    "operator approved resolution for '{}': {}",
                    policy, operator_reason
                )],
                actions,
                next_step,
            )
        }
        OperatorDecision::Denied => {
            let actions = vec![
                "Treat the blocked action as denied and do not attempt in-place continuation."
                    .to_string(),
                "Inspect the session and redesign the workflow before starting any new run."
                    .to_string(),
            ];
            let next_step = Some(format!(
                "operator denied '{}' with reason '{}'; inspect step {} before any future rerun",
                policy, operator_reason, last.step_id
            ));
            if failure_active {
                (
                    ResumeStatus::FailedInspectable,
                    vec![format!("operator denied '{}': {}", policy, operator_reason)],
                    actions,
                    next_step,
                )
            } else {
                (
                    ResumeStatus::UnsafeToResume,
                    vec![format!(
                        "operator denied '{}' after block '{}': {}",
                        policy, blocked_reason, operator_reason
                    )],
                    actions,
                    next_step,
                )
            }
        }
        OperatorDecision::NeedsMoreInfo => {
            let actions = vec![
                format!(
                    "Gather the missing context for '{}' before making another approval decision.",
                    policy
                ),
                "Keep the session gated and do not execute follow-up work automatically."
                    .to_string(),
            ];
            let next_step = Some(format!(
                "collect the missing information for '{}' and then record a new operator decision",
                policy
            ));
            (
                ResumeStatus::WaitingForPolicyApproval,
                vec![format!(
                    "operator requested more information for '{}': {}",
                    policy, operator_reason
                )],
                actions,
                next_step,
            )
        }
    }
}

fn detect_waiting_input_hint(value: &str) -> Option<String> {
    let normalized = value.to_ascii_lowercase();
    if normalized.contains("waiting_for_input")
        || normalized.contains("waiting for input")
        || normalized.contains("awaiting input")
        || normalized.contains("operator_input_required")
        || normalized.contains("user_input_required")
    {
        Some(value.to_string())
    } else {
        None
    }
}

fn detect_replay_only_hint(value: &str) -> Option<String> {
    let normalized = value.to_ascii_lowercase();
    if normalized.contains("dry_run")
        || normalized.contains("dry-run")
        || normalized.contains("replay_only")
        || normalized.contains("analysis only")
        || normalized.contains("would have executed")
        || normalized.contains("would request")
    {
        Some(value.to_string())
    } else {
        None
    }
}

fn is_complete_state(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "completed" | "complete" | "ok" | "success" | "succeeded" | "done"
    )
}

fn is_failed_state(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "failed" | "error" | "aborted" | "fatal"
    )
}

fn is_waiting_input_state(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "waiting_for_input" | "waiting for input" | "awaiting_input"
    )
}

fn is_pending_state(value: &str) -> bool {
    matches!(
        value.trim().to_ascii_lowercase().as_str(),
        "queued" | "executing" | "running" | "retrying" | "pending" | "planned"
    )
}

fn read_session_entries(cfg: &Config, session_id: &SessionId) -> Result<Vec<SessionLogEntry>> {
    let path = session_log_path(cfg, session_id);
    if !path.exists() {
        return Err(anyhow!("session '{}' not found", session_id));
    }
    read_session_entries_from_path(&path)
}

fn read_session_entries_from_path(path: &Path) -> Result<Vec<SessionLogEntry>> {
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    let mut entries = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let entry = serde_json::from_str::<SessionLogEntry>(trimmed)
            .with_context(|| format!("invalid session event JSON in {}", path.display()))?;
        entries.push(entry);
    }
    Ok(entries)
}

fn session_dir(cfg: &Config) -> PathBuf {
    cfg.runtime.session_dir.clone()
}

fn session_log_path(cfg: &Config, session_id: &SessionId) -> PathBuf {
    session_dir(cfg).join(format!("{}.ndjson", session_id.as_str()))
}

fn existing_event_count(path: &Path) -> Result<u64> {
    if !path.exists() {
        return Ok(0);
    }
    let raw =
        fs::read_to_string(path).with_context(|| format!("failed to read {}", path.display()))?;
    Ok(raw.lines().filter(|line| !line.trim().is_empty()).count() as u64)
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
    let line = serde_json::to_string(item).context("failed to serialize session event")?;
    writeln!(file, "{line}").with_context(|| format!("failed to append {}", path.display()))
}

fn is_session_log_path(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|value| value.eq_ignore_ascii_case("ndjson"))
}

fn generate_id(prefix: &str) -> String {
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}-{}-{counter}", Utc::now().timestamp_micros())
}

fn now_rfc3339() -> String {
    Utc::now().to_rfc3339()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        (tmp, cfg)
    }

    #[test]
    fn events_persist_in_order() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-a", "worker");

        append_event(
            &cfg,
            &context,
            SessionEvent::Observation {
                message: "job accepted".to_string(),
                job_id: Some("job-1".to_string()),
                detail: Some("echo".to_string()),
            },
        )
        .expect("append observation");
        append_event(
            &cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "ok".to_string(),
                job_id: Some("job-1".to_string()),
            },
        )
        .expect("append output");
        append_event(
            &cfg,
            &context,
            SessionEvent::AuditNote {
                note: "done".to_string(),
            },
        )
        .expect("append note");

        let detail = show_session(&cfg, &context.session_id).expect("show session");
        assert_eq!(detail.events.len(), 3);
        assert_eq!(detail.events[0].sequence, 1);
        assert_eq!(detail.events[1].sequence, 2);
        assert_eq!(detail.events[2].sequence, 3);
        assert!(matches!(
            detail.events[0].event,
            SessionEvent::Observation { .. }
        ));
        assert!(matches!(
            detail.events[1].event,
            SessionEvent::Output { .. }
        ));
        assert!(matches!(
            detail.events[2].event,
            SessionEvent::AuditNote { .. }
        ));
    }

    #[test]
    fn replay_is_deterministic() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-b", "skills");

        append_event(
            &cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "summarize".to_string(),
                input_preview: "hello".to_string(),
                command_preview: Some("echo hello".to_string()),
                status: "running".to_string(),
            },
        )
        .expect("append skill call");
        append_event(
            &cfg,
            &context,
            SessionEvent::Output {
                channel: "skills".to_string(),
                summary: "summary result".to_string(),
                job_id: None,
            },
        )
        .expect("append output");

        let replay_a = replay_session(&cfg, &context.session_id).expect("replay a");
        let replay_b = replay_session(&cfg, &context.session_id).expect("replay b");
        assert_eq!(replay_a, replay_b);
    }

    #[test]
    fn failed_sessions_can_be_inspected() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-c", "worker");

        append_event(
            &cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled".to_string(),
            },
        )
        .expect("append policy");
        append_event(
            &cfg,
            &context,
            SessionEvent::Error {
                code: Some("shell_disabled".to_string()),
                message: "shell jobs are disabled".to_string(),
            },
        )
        .expect("append error");

        let listed = list_sessions(&cfg).expect("list sessions");
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].final_status, "error");
        assert_eq!(listed[0].error_count, 1);

        let detail = show_session(&cfg, &context.session_id).expect("show session");
        assert_eq!(detail.summary.error_count, 1);
        assert_eq!(detail.events.len(), 2);
    }

    #[test]
    fn replay_does_not_execute_side_effects() {
        let (tmp, cfg) = temp_cfg();
        let context = runtime_context("node-d", "skills");
        let marker = tmp.path().join("marker.txt");

        append_event(
            &cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "dangerous".to_string(),
                input_preview: "touch marker".to_string(),
                command_preview: Some(format!("touch {}", marker.display())),
                status: "planned".to_string(),
            },
        )
        .expect("append skill call");
        append_event(
            &cfg,
            &context,
            SessionEvent::AuditNote {
                note: "would have executed an external command in live mode".to_string(),
            },
        )
        .expect("append note");

        let replay = replay_session(&cfg, &context.session_id).expect("replay session");
        assert!(!marker.exists());
        assert!(!replay.state.side_effects_replayed);
    }

    fn append_policy_blocked_session(cfg: &Config, context: &SessionContext) {
        append_event(
            cfg,
            context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell jobs are disabled by config".to_string(),
            },
        )
        .expect("append policy");
        append_event(
            cfg,
            context,
            SessionEvent::Error {
                code: Some("shell_disabled".to_string()),
                message: "shell jobs are disabled by config".to_string(),
            },
        )
        .expect("append error");
    }

    #[test]
    fn complete_sessions_are_not_resumed() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-e", "worker");

        append_event(
            &cfg,
            &context,
            SessionEvent::Observation {
                message: "job_received".to_string(),
                job_id: Some("job-2".to_string()),
                detail: Some("kind=echo".to_string()),
            },
        )
        .expect("append observation");
        append_event(
            &cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "ok".to_string(),
                job_id: Some("job-2".to_string()),
            },
        )
        .expect("append output");
        append_event(
            &cfg,
            &context,
            SessionEvent::FsmTransition {
                machine: "worker_job".to_string(),
                from_state: Some("executing".to_string()),
                to_state: "completed".to_string(),
                reason: "job completed successfully".to_string(),
            },
        )
        .expect("append fsm");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::Complete);
        assert!(!plan.safe_to_resume);
        assert!(plan.blocked_reasons.is_empty());
        assert!(plan.proposed_next_step.is_none());
    }

    #[test]
    fn failed_sessions_produce_inspectable_blocked_reasons() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-f", "worker");

        append_event(
            &cfg,
            &context,
            SessionEvent::Error {
                code: Some("http_get_transport".to_string()),
                message: "HTTP request failed before response".to_string(),
            },
        )
        .expect("append error");
        append_event(
            &cfg,
            &context,
            SessionEvent::FsmTransition {
                machine: "worker_job".to_string(),
                from_state: Some("executing".to_string()),
                to_state: "failed".to_string(),
                reason: "job execution failed".to_string(),
            },
        )
        .expect("append failed transition");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::FailedInspectable);
        assert!(!plan.safe_to_resume);
        assert!(
            plan.blocked_reasons
                .iter()
                .any(|reason| reason.contains("failed"))
        );
        assert!(!plan.required_operator_actions.is_empty());
    }

    #[test]
    fn policy_blocked_sessions_require_operator_action() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-g", "skills");

        append_event(
            &cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "skills.allow_shell_commands".to_string(),
                allowed: false,
                reason: "skill shell execution is disabled".to_string(),
            },
        )
        .expect("append policy");
        append_event(
            &cfg,
            &context,
            SessionEvent::Error {
                code: Some("skills_shell_disabled".to_string()),
                message: "skill shell execution is disabled".to_string(),
            },
        )
        .expect("append error");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::WaitingForPolicyApproval);
        assert!(!plan.safe_to_resume);
        assert!(
            plan.blocked_reasons
                .iter()
                .any(|reason| reason.contains("skills.allow_shell_commands"))
        );
        assert!(
            plan.required_operator_actions
                .iter()
                .any(|action| action.contains("Review"))
        );
    }

    #[test]
    fn replay_only_resume_plans_do_not_execute_side_effects() {
        let (tmp, cfg) = temp_cfg();
        let context = runtime_context("node-h", "skills");
        let marker = tmp.path().join("resume-marker.txt");

        append_event(
            &cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "dangerous".to_string(),
                input_preview: "touch marker".to_string(),
                command_preview: Some(format!("touch {}", marker.display())),
                status: "planned".to_string(),
            },
        )
        .expect("append skill call");
        append_event(
            &cfg,
            &context,
            SessionEvent::AuditNote {
                note: "would have executed an external command in live mode".to_string(),
            },
        )
        .expect("append note");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::ReplayOnly);
        assert!(!marker.exists());
        assert!(!plan.safe_to_resume);
    }

    #[test]
    fn interrupted_sessions_produce_gated_next_step() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-i", "worker");

        append_event(
            &cfg,
            &context,
            SessionEvent::FsmTransition {
                machine: "worker_job".to_string(),
                from_state: Some("queued".to_string()),
                to_state: "executing".to_string(),
                reason: "starting shell".to_string(),
            },
        )
        .expect("append executing transition");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::UnsafeToResume);
        assert!(!plan.safe_to_resume);
        assert!(
            plan.blocked_reasons
                .iter()
                .any(|reason| reason.contains("non-terminal"))
        );
        assert!(plan.proposed_next_step.is_some());
    }

    #[test]
    fn approvals_are_persisted() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-j", "worker");
        append_policy_blocked_session(&cfg, &context);

        let record = record_operator_decision(
            &cfg,
            &context.session_id,
            OperatorDecision::Approved,
            "reviewed by operator",
            "operator:tester",
        )
        .expect("record decision");

        let detail = show_session(&cfg, &context.session_id).expect("show session");
        let last = detail.events.last().expect("last event");
        assert_eq!(record.session_id, context.session_id);
        assert!(matches!(
            &last.event,
            SessionEvent::OperatorDecision { record }
                if record.decision == OperatorDecision::Approved
                    && record.reason == "reviewed by operator"
                    && record.decided_by == "operator:tester"
        ));
    }

    #[test]
    fn denial_blocks_resume() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-k", "worker");
        append_policy_blocked_session(&cfg, &context);

        record_operator_decision(
            &cfg,
            &context.session_id,
            OperatorDecision::Denied,
            "do not allow shell execution",
            "operator:tester",
        )
        .expect("record denial");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::FailedInspectable);
        assert!(!plan.safe_to_resume);
        assert!(
            plan.blocked_reasons
                .iter()
                .any(|reason| reason.contains("operator denied"))
        );
    }

    #[test]
    fn needs_info_keeps_session_gated() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-l", "worker");
        append_policy_blocked_session(&cfg, &context);

        record_operator_decision(
            &cfg,
            &context.session_id,
            OperatorDecision::NeedsMoreInfo,
            "need desk context first",
            "operator:tester",
        )
        .expect("record needs info");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::WaitingForPolicyApproval);
        assert!(!plan.safe_to_resume);
        assert!(
            plan.required_operator_actions
                .iter()
                .any(|action| action.contains("missing context"))
        );
    }

    #[test]
    fn approval_never_triggers_side_effects() {
        let (tmp, cfg) = temp_cfg();
        let context = runtime_context("node-m", "skills");
        let marker = tmp.path().join("approval-marker.txt");

        append_event(
            &cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "dangerous".to_string(),
                input_preview: "touch marker".to_string(),
                command_preview: Some(format!("touch {}", marker.display())),
                status: "planned".to_string(),
            },
        )
        .expect("append skill");
        append_event(
            &cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "skills.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled".to_string(),
            },
        )
        .expect("append policy");
        append_event(
            &cfg,
            &context,
            SessionEvent::AuditNote {
                note: "would have executed an external command in live mode".to_string(),
            },
        )
        .expect("append note");

        record_operator_decision(
            &cfg,
            &context.session_id,
            OperatorDecision::Approved,
            "approved for manual follow-up only",
            "operator:tester",
        )
        .expect("record approval");

        let plan = resume_plan_session(&cfg, &context.session_id).expect("resume plan");
        assert_eq!(plan.status, ResumeStatus::ReplayOnly);
        assert!(!marker.exists());
        assert!(!plan.safe_to_resume);
    }

    #[test]
    fn replay_includes_operator_decision_events_deterministically() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-n", "worker");
        append_policy_blocked_session(&cfg, &context);
        record_operator_decision(
            &cfg,
            &context.session_id,
            OperatorDecision::NeedsMoreInfo,
            "need compliance review",
            "operator:tester",
        )
        .expect("record decision");

        let replay_a = replay_session(&cfg, &context.session_id).expect("replay a");
        let replay_b = replay_session(&cfg, &context.session_id).expect("replay b");
        assert_eq!(replay_a, replay_b);
        assert_eq!(replay_a.state.operator_decisions, 1);
        assert_eq!(
            replay_a.state.last_operator_decision,
            Some(OperatorDecision::NeedsMoreInfo)
        );
    }

    #[test]
    fn replay_computes_typed_completed_state_from_legacy_completed_transition() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-o", "worker");
        append_event(
            &cfg,
            &context,
            SessionEvent::FsmTransition {
                machine: "worker_job".to_string(),
                from_state: Some("executing".to_string()),
                to_state: "completed".to_string(),
                reason: "legacy artifact".to_string(),
            },
        )
        .expect("append transition");

        let replay = replay_session(&cfg, &context.session_id).expect("replay");
        assert_eq!(
            replay.state.typed_final_state,
            SessionLifecycleState::Completed
        );
        assert_eq!(
            replay.summary.typed_final_state,
            SessionLifecycleState::Completed
        );
    }

    #[test]
    fn policy_block_replays_as_waiting_for_approval() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-p", "worker");
        append_event(
            &cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled".to_string(),
            },
        )
        .expect("append policy");

        let replay = replay_session(&cfg, &context.session_id).expect("replay");
        assert_eq!(
            replay.state.typed_final_state,
            SessionLifecycleState::WaitingForApproval
        );
    }
}
