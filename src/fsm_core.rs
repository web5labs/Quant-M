use anyhow::{Result, anyhow};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

pub trait StateMachine {
    type State: Clone + Eq + fmt::Display;
    type Event: Clone + Eq + fmt::Display;

    fn machine_id(&self) -> &'static str;
    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State>;
    #[allow(dead_code)]
    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event>;
    #[allow(dead_code)]
    fn is_terminal(&self, state: &Self::State) -> bool;
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FsmError {
    pub machine_id: String,
    pub state: String,
    pub event: String,
    pub reason: String,
}

impl fmt::Display for FsmError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid transition for {}: state={} event={} reason={}",
            self.machine_id, self.state, self.event, self.reason
        )
    }
}

impl std::error::Error for FsmError {}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[allow(dead_code)]
pub struct TransitionRecord {
    pub machine_id: String,
    pub previous_state: String,
    pub event: String,
    pub next_state: Option<String>,
    pub accepted: bool,
    pub reason: String,
    pub occurred_at: String,
    pub evidence_ref: Option<String>,
}

#[allow(dead_code)]
pub fn transition_record<M>(
    machine: &M,
    previous_state: &M::State,
    event: &M::Event,
    occurred_at: impl Into<String>,
    evidence_ref: Option<String>,
) -> TransitionRecord
where
    M: StateMachine,
{
    match machine.transition(previous_state, event) {
        Ok(next_state) => TransitionRecord {
            machine_id: machine.machine_id().to_string(),
            previous_state: previous_state.to_string(),
            event: event.to_string(),
            next_state: Some(next_state.to_string()),
            accepted: true,
            reason: "transition accepted".to_string(),
            occurred_at: occurred_at.into(),
            evidence_ref,
        },
        Err(err) => TransitionRecord {
            machine_id: machine.machine_id().to_string(),
            previous_state: previous_state.to_string(),
            event: event.to_string(),
            next_state: None,
            accepted: false,
            reason: err.to_string(),
            occurred_at: occurred_at.into(),
            evidence_ref,
        },
    }
}

macro_rules! display_snake {
    ($ty:ty, {$($variant:path => $label:literal),+ $(,)?}) => {
        impl fmt::Display for $ty {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                let label = match self {
                    $($variant => $label,)+
                };
                f.write_str(label)
            }
        }
    };
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerJobState {
    Queued,
    Executing,
    Succeeded,
    Failed,
    Retrying,
    DeadLettered,
}

display_snake!(WorkerJobState, {
    WorkerJobState::Queued => "queued",
    WorkerJobState::Executing => "executing",
    WorkerJobState::Succeeded => "succeeded",
    WorkerJobState::Failed => "failed",
    WorkerJobState::Retrying => "retrying",
    WorkerJobState::DeadLettered => "dead_lettered",
});

impl FromStr for WorkerJobState {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "queued" => Ok(Self::Queued),
            "executing" | "running" => Ok(Self::Executing),
            "succeeded" | "completed" | "complete" | "ok" | "success" => Ok(Self::Succeeded),
            "failed" | "error" => Ok(Self::Failed),
            "retrying" | "retry" => Ok(Self::Retrying),
            "dead_lettered" | "dead-lettered" | "dead_letter" | "dead-letter" => {
                Ok(Self::DeadLettered)
            }
            other => Err(anyhow!("unsupported worker job state '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerJobEvent {
    Start,
    Complete,
    Fail,
    Retry,
    ExhaustRetries,
    DeadLetter,
    ResetForRetry,
}

display_snake!(WorkerJobEvent, {
    WorkerJobEvent::Start => "start",
    WorkerJobEvent::Complete => "complete",
    WorkerJobEvent::Fail => "fail",
    WorkerJobEvent::Retry => "retry",
    WorkerJobEvent::ExhaustRetries => "exhaust_retries",
    WorkerJobEvent::DeadLetter => "dead_letter",
    WorkerJobEvent::ResetForRetry => "reset_for_retry",
});

pub struct WorkerJobFsm;

impl StateMachine for WorkerJobFsm {
    type State = WorkerJobState;
    type Event = WorkerJobEvent;

    fn machine_id(&self) -> &'static str {
        "worker_job"
    }

    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State> {
        use WorkerJobEvent::*;
        use WorkerJobState::*;
        match (current, event) {
            (Queued, Start) | (Retrying, Start) | (Retrying, ResetForRetry) => Ok(Executing),
            (Executing, Complete) => Ok(Succeeded),
            (Executing, Fail) => Ok(Failed),
            (Failed, Retry) => Ok(Retrying),
            (Failed, DeadLetter) | (Failed, ExhaustRetries) => Ok(DeadLettered),
            (Succeeded, _) | (DeadLettered, _) => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "terminal state rejects further events",
            )),
            _ => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "event is not allowed from current state",
            )),
        }
    }

    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event> {
        use WorkerJobEvent::*;
        use WorkerJobState::*;
        match current {
            Queued => vec![Start],
            Executing => vec![Complete, Fail],
            Failed => vec![Retry, ExhaustRetries, DeadLetter],
            Retrying => vec![Start, ResetForRetry],
            Succeeded | DeadLettered => vec![],
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            WorkerJobState::Succeeded | WorkerJobState::DeadLettered
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCursorState {
    Declared,
    Ready,
    StepRunning,
    StepSucceeded,
    StepFailed,
    Blocked,
    Completed,
    ReplaySkipped,
}

display_snake!(WorkflowCursorState, {
    WorkflowCursorState::Declared => "declared",
    WorkflowCursorState::Ready => "ready",
    WorkflowCursorState::StepRunning => "step_running",
    WorkflowCursorState::StepSucceeded => "step_succeeded",
    WorkflowCursorState::StepFailed => "step_failed",
    WorkflowCursorState::Blocked => "blocked",
    WorkflowCursorState::Completed => "completed",
    WorkflowCursorState::ReplaySkipped => "replay_skipped",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkflowCursorEvent {
    Prepare,
    StartStep,
    CompleteStep,
    FailStep,
    Block,
    CompleteWorkflow,
    SkipReplaySideEffect,
}

display_snake!(WorkflowCursorEvent, {
    WorkflowCursorEvent::Prepare => "prepare",
    WorkflowCursorEvent::StartStep => "start_step",
    WorkflowCursorEvent::CompleteStep => "complete_step",
    WorkflowCursorEvent::FailStep => "fail_step",
    WorkflowCursorEvent::Block => "block",
    WorkflowCursorEvent::CompleteWorkflow => "complete_workflow",
    WorkflowCursorEvent::SkipReplaySideEffect => "skip_replay_side_effect",
});

pub struct WorkflowCursorFsm;

impl StateMachine for WorkflowCursorFsm {
    type State = WorkflowCursorState;
    type Event = WorkflowCursorEvent;

    fn machine_id(&self) -> &'static str {
        "workflow_cursor"
    }

    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State> {
        use WorkflowCursorEvent::*;
        use WorkflowCursorState::*;
        match (current, event) {
            (Declared, Prepare) => Ok(Ready),
            (Ready, StartStep) | (StepSucceeded, StartStep) => Ok(StepRunning),
            (StepRunning, CompleteStep) => Ok(StepSucceeded),
            (StepRunning, FailStep) => Ok(StepFailed),
            (Ready, Block) | (StepRunning, Block) | (StepFailed, Block) => Ok(Blocked),
            (StepSucceeded, CompleteWorkflow) => Ok(Completed),
            (Ready, SkipReplaySideEffect) => Ok(ReplaySkipped),
            (Completed, _) | (Blocked, _) | (ReplaySkipped, _) => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "terminal workflow cursor state rejects further events",
            )),
            _ => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "event is not allowed from current workflow cursor state",
            )),
        }
    }

    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event> {
        use WorkflowCursorEvent::*;
        use WorkflowCursorState::*;
        match current {
            Declared => vec![Prepare],
            Ready => vec![StartStep, Block, SkipReplaySideEffect],
            StepRunning => vec![CompleteStep, FailStep, Block],
            StepSucceeded => vec![StartStep, CompleteWorkflow],
            StepFailed => vec![Block],
            Completed | Blocked | ReplaySkipped => vec![],
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            WorkflowCursorState::Completed
                | WorkflowCursorState::Blocked
                | WorkflowCursorState::ReplaySkipped
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SessionLifecycleState {
    Created,
    Recording,
    WaitingForApproval,
    Completed,
    Failed,
    Compacted,
    ReplayOnly,
    ResumePlanReady,
}

display_snake!(SessionLifecycleState, {
    SessionLifecycleState::Created => "created",
    SessionLifecycleState::Recording => "recording",
    SessionLifecycleState::WaitingForApproval => "waiting_for_approval",
    SessionLifecycleState::Completed => "completed",
    SessionLifecycleState::Failed => "failed",
    SessionLifecycleState::Compacted => "compacted",
    SessionLifecycleState::ReplayOnly => "replay_only",
    SessionLifecycleState::ResumePlanReady => "resume_plan_ready",
});

impl FromStr for SessionLifecycleState {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "created" => Ok(Self::Created),
            "recording" | "running" | "queued" | "executing" | "retrying" => Ok(Self::Recording),
            "waiting_for_approval"
            | "approval_pending"
            | "operator_needs_more_info"
            | "blocked" => Ok(Self::WaitingForApproval),
            "completed" | "complete" | "succeeded" | "success" | "ok" | "done" => {
                Ok(Self::Completed)
            }
            "failed" | "error" | "fatal" | "aborted" | "operator_denied" => Ok(Self::Failed),
            "compacted" | "compact_fresh" => Ok(Self::Compacted),
            "replay_only" | "dry_run" | "dry-run" => Ok(Self::ReplayOnly),
            "resume_plan_ready" => Ok(Self::ResumePlanReady),
            other => Err(anyhow!("unsupported session lifecycle state '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum SessionLifecycleEvent {
    StartRecording,
    NeedApproval,
    Approve,
    Complete,
    Fail,
    Compact,
    MarkReplayOnly,
    PrepareResumePlan,
}

display_snake!(SessionLifecycleEvent, {
    SessionLifecycleEvent::StartRecording => "start_recording",
    SessionLifecycleEvent::NeedApproval => "need_approval",
    SessionLifecycleEvent::Approve => "approve",
    SessionLifecycleEvent::Complete => "complete",
    SessionLifecycleEvent::Fail => "fail",
    SessionLifecycleEvent::Compact => "compact",
    SessionLifecycleEvent::MarkReplayOnly => "mark_replay_only",
    SessionLifecycleEvent::PrepareResumePlan => "prepare_resume_plan",
});

#[allow(dead_code)]
pub struct SessionLifecycleFsm;

impl StateMachine for SessionLifecycleFsm {
    type State = SessionLifecycleState;
    type Event = SessionLifecycleEvent;

    fn machine_id(&self) -> &'static str {
        "session_lifecycle"
    }

    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State> {
        use SessionLifecycleEvent::*;
        use SessionLifecycleState::*;
        match (current, event) {
            (Created, StartRecording) => Ok(Recording),
            (Created, MarkReplayOnly) | (Recording, MarkReplayOnly) => Ok(ReplayOnly),
            (Recording, NeedApproval) => Ok(WaitingForApproval),
            (WaitingForApproval, Approve) => Ok(Recording),
            (Recording, Complete) | (WaitingForApproval, Complete) => Ok(Completed),
            (Recording, Fail) | (WaitingForApproval, Fail) | (ReplayOnly, Fail) => Ok(Failed),
            (Completed, Compact) => Ok(Compacted),
            (Completed, PrepareResumePlan)
            | (Failed, PrepareResumePlan)
            | (ReplayOnly, PrepareResumePlan) => Ok(ResumePlanReady),
            (Completed, _) | (Failed, _) | (Compacted, _) | (ResumePlanReady, _) => {
                Err(invalid_transition(
                    self.machine_id(),
                    current,
                    event,
                    "terminal session state rejects this event",
                ))
            }
            _ => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "event is not allowed from current session state",
            )),
        }
    }

    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event> {
        use SessionLifecycleEvent::*;
        use SessionLifecycleState::*;
        match current {
            Created => vec![StartRecording, MarkReplayOnly],
            Recording => vec![NeedApproval, Complete, Fail, MarkReplayOnly],
            WaitingForApproval => vec![Approve, Complete, Fail],
            Completed => vec![Compact, PrepareResumePlan],
            Failed | ReplayOnly => vec![PrepareResumePlan],
            Compacted | ResumePlanReady => vec![],
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            SessionLifecycleState::Compacted | SessionLifecycleState::ResumePlanReady
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum PolicyApprovalState {
    Requested,
    EvaluatingPolicy,
    BlockedByPolicy,
    ApprovalPending,
    Approved,
    Denied,
    ExecutionAllowed,
    Executed,
    ReplaySkipped,
}

display_snake!(PolicyApprovalState, {
    PolicyApprovalState::Requested => "requested",
    PolicyApprovalState::EvaluatingPolicy => "evaluating_policy",
    PolicyApprovalState::BlockedByPolicy => "blocked_by_policy",
    PolicyApprovalState::ApprovalPending => "approval_pending",
    PolicyApprovalState::Approved => "approved",
    PolicyApprovalState::Denied => "denied",
    PolicyApprovalState::ExecutionAllowed => "execution_allowed",
    PolicyApprovalState::Executed => "executed",
    PolicyApprovalState::ReplaySkipped => "replay_skipped",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
#[allow(dead_code)]
pub enum PolicyApprovalEvent {
    Request,
    PolicyAllows,
    PolicyBlocks,
    NeedsHumanApproval,
    HumanApproves,
    HumanDenies,
    Execute,
    MarkReplaySkipped,
}

display_snake!(PolicyApprovalEvent, {
    PolicyApprovalEvent::Request => "request",
    PolicyApprovalEvent::PolicyAllows => "policy_allows",
    PolicyApprovalEvent::PolicyBlocks => "policy_blocks",
    PolicyApprovalEvent::NeedsHumanApproval => "needs_human_approval",
    PolicyApprovalEvent::HumanApproves => "human_approves",
    PolicyApprovalEvent::HumanDenies => "human_denies",
    PolicyApprovalEvent::Execute => "execute",
    PolicyApprovalEvent::MarkReplaySkipped => "mark_replay_skipped",
});

#[allow(dead_code)]
pub struct PolicyApprovalFsm;

impl StateMachine for PolicyApprovalFsm {
    type State = PolicyApprovalState;
    type Event = PolicyApprovalEvent;

    fn machine_id(&self) -> &'static str {
        "policy_approval"
    }

    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State> {
        use PolicyApprovalEvent::*;
        use PolicyApprovalState::*;
        match (current, event) {
            (Requested, Request) => Ok(EvaluatingPolicy),
            (EvaluatingPolicy, PolicyAllows) => Ok(ExecutionAllowed),
            (EvaluatingPolicy, PolicyBlocks) => Ok(BlockedByPolicy),
            (EvaluatingPolicy, NeedsHumanApproval) => Ok(ApprovalPending),
            (ApprovalPending, HumanApproves) => Ok(Approved),
            (ApprovalPending, HumanDenies) => Ok(Denied),
            (Approved, Execute) | (ExecutionAllowed, Execute) => Ok(Executed),
            (BlockedByPolicy, MarkReplaySkipped)
            | (ApprovalPending, MarkReplaySkipped)
            | (Denied, MarkReplaySkipped)
            | (Executed, MarkReplaySkipped) => Ok(ReplaySkipped),
            (BlockedByPolicy, Execute) | (ApprovalPending, Execute) | (Denied, Execute) => {
                Err(invalid_transition(
                    self.machine_id(),
                    current,
                    event,
                    "blocked, pending, and denied actions cannot execute",
                ))
            }
            (Executed, _) | (ReplaySkipped, _) => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "terminal policy state rejects further events",
            )),
            _ => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "event is not allowed from current policy state",
            )),
        }
    }

    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event> {
        use PolicyApprovalEvent::*;
        use PolicyApprovalState::*;
        match current {
            Requested => vec![Request],
            EvaluatingPolicy => vec![PolicyAllows, PolicyBlocks, NeedsHumanApproval],
            ApprovalPending => vec![HumanApproves, HumanDenies, MarkReplaySkipped],
            Approved | ExecutionAllowed => vec![Execute],
            BlockedByPolicy | Denied => vec![MarkReplaySkipped],
            Executed => vec![MarkReplaySkipped],
            ReplaySkipped => vec![],
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            PolicyApprovalState::Executed | PolicyApprovalState::ReplaySkipped
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillExecutionState {
    Declared,
    Loaded,
    PolicyChecked,
    Blocked,
    Ready,
    Running,
    Succeeded,
    Failed,
}

display_snake!(SkillExecutionState, {
    SkillExecutionState::Declared => "declared",
    SkillExecutionState::Loaded => "loaded",
    SkillExecutionState::PolicyChecked => "policy_checked",
    SkillExecutionState::Blocked => "blocked",
    SkillExecutionState::Ready => "ready",
    SkillExecutionState::Running => "running",
    SkillExecutionState::Succeeded => "succeeded",
    SkillExecutionState::Failed => "failed",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SkillExecutionEvent {
    Load,
    CheckPolicy,
    PolicyAllows,
    PolicyBlocks,
    Start,
    Complete,
    Fail,
}

display_snake!(SkillExecutionEvent, {
    SkillExecutionEvent::Load => "load",
    SkillExecutionEvent::CheckPolicy => "check_policy",
    SkillExecutionEvent::PolicyAllows => "policy_allows",
    SkillExecutionEvent::PolicyBlocks => "policy_blocks",
    SkillExecutionEvent::Start => "start",
    SkillExecutionEvent::Complete => "complete",
    SkillExecutionEvent::Fail => "fail",
});

pub struct SkillExecutionFsm;

impl StateMachine for SkillExecutionFsm {
    type State = SkillExecutionState;
    type Event = SkillExecutionEvent;

    fn machine_id(&self) -> &'static str {
        "skill_execution"
    }

    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State> {
        use SkillExecutionEvent::*;
        use SkillExecutionState::*;
        match (current, event) {
            (Declared, Load) => Ok(Loaded),
            (Loaded, CheckPolicy) => Ok(PolicyChecked),
            (PolicyChecked, PolicyAllows) => Ok(Ready),
            (PolicyChecked, PolicyBlocks) => Ok(Blocked),
            (Ready, Start) => Ok(Running),
            (Running, Complete) => Ok(Succeeded),
            (Running, Fail) => Ok(Failed),
            (Blocked, _) | (Succeeded, _) | (Failed, _) => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "terminal skill execution state rejects further events",
            )),
            _ => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "event is not allowed from current skill execution state",
            )),
        }
    }

    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event> {
        use SkillExecutionEvent::*;
        use SkillExecutionState::*;
        match current {
            Declared => vec![Load],
            Loaded => vec![CheckPolicy],
            PolicyChecked => vec![PolicyAllows, PolicyBlocks],
            Ready => vec![Start],
            Running => vec![Complete, Fail],
            Blocked | Succeeded | Failed => vec![],
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            SkillExecutionState::Blocked
                | SkillExecutionState::Succeeded
                | SkillExecutionState::Failed
        )
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextGuardianState {
    NoSession,
    Observing,
    Healthy,
    NearLimit,
    NeedsCompact,
    CompactFresh,
    CompactStale,
    HandoffReady,
    OperatorReviewRequired,
    Blocked,
}

display_snake!(ContextGuardianState, {
    ContextGuardianState::NoSession => "no_session",
    ContextGuardianState::Observing => "observing",
    ContextGuardianState::Healthy => "healthy",
    ContextGuardianState::NearLimit => "near_limit",
    ContextGuardianState::NeedsCompact => "needs_compact",
    ContextGuardianState::CompactFresh => "compact_fresh",
    ContextGuardianState::CompactStale => "compact_stale",
    ContextGuardianState::HandoffReady => "handoff_ready",
    ContextGuardianState::OperatorReviewRequired => "operator_review_required",
    ContextGuardianState::Blocked => "blocked",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextGuardianEvent {
    SessionStarted,
    ContextMeasured,
    WithinBudget,
    NearBudgetLimit,
    OverBudgetLimit,
    CompactCreated,
    CompactExpired,
    HandoffCreated,
    ReviewRequested,
    ReviewCompleted,
    Block,
}

display_snake!(ContextGuardianEvent, {
    ContextGuardianEvent::SessionStarted => "session_started",
    ContextGuardianEvent::ContextMeasured => "context_measured",
    ContextGuardianEvent::WithinBudget => "within_budget",
    ContextGuardianEvent::NearBudgetLimit => "near_budget_limit",
    ContextGuardianEvent::OverBudgetLimit => "over_budget_limit",
    ContextGuardianEvent::CompactCreated => "compact_created",
    ContextGuardianEvent::CompactExpired => "compact_expired",
    ContextGuardianEvent::HandoffCreated => "handoff_created",
    ContextGuardianEvent::ReviewRequested => "review_requested",
    ContextGuardianEvent::ReviewCompleted => "review_completed",
    ContextGuardianEvent::Block => "block",
});

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextRecommendedAction {
    Observe,
    Continue,
    Compact,
    RefreshCompact,
    CreateHandoff,
    RequestOperatorReview,
    BlockContinuation,
}

display_snake!(ContextRecommendedAction, {
    ContextRecommendedAction::Observe => "observe",
    ContextRecommendedAction::Continue => "continue",
    ContextRecommendedAction::Compact => "compact",
    ContextRecommendedAction::RefreshCompact => "refresh_compact",
    ContextRecommendedAction::CreateHandoff => "create_handoff",
    ContextRecommendedAction::RequestOperatorReview => "request_operator_review",
    ContextRecommendedAction::BlockContinuation => "block_continuation",
});

pub struct ContextGuardianFsm;

impl StateMachine for ContextGuardianFsm {
    type State = ContextGuardianState;
    type Event = ContextGuardianEvent;

    fn machine_id(&self) -> &'static str {
        "context_guardian"
    }

    fn transition(&self, current: &Self::State, event: &Self::Event) -> Result<Self::State> {
        use ContextGuardianEvent::*;
        use ContextGuardianState::*;
        match (current, event) {
            (NoSession, SessionStarted) => Ok(Observing),
            (Observing, WithinBudget) => Ok(Healthy),
            (Observing, NearBudgetLimit) => Ok(NearLimit),
            (Observing, ContextMeasured) => Ok(NeedsCompact),
            (Observing, CompactCreated) => Ok(CompactFresh),
            (Observing, ReviewRequested) => Ok(OperatorReviewRequired),
            (Observing, OverBudgetLimit) | (Observing, Block) => Ok(Blocked),
            (Healthy, NearBudgetLimit) => Ok(NearLimit),
            (Healthy, CompactCreated)
            | (NeedsCompact, CompactCreated)
            | (NearLimit, CompactCreated) => Ok(CompactFresh),
            (NearLimit, OverBudgetLimit) => Ok(NeedsCompact),
            (NearLimit, ReviewRequested)
            | (NeedsCompact, ReviewRequested)
            | (CompactStale, ReviewRequested) => Ok(OperatorReviewRequired),
            (NeedsCompact, OverBudgetLimit) | (CompactStale, OverBudgetLimit) => Ok(Blocked),
            (CompactFresh, CompactExpired) => Ok(CompactStale),
            (CompactFresh, HandoffCreated) => Ok(HandoffReady),
            (CompactStale, CompactCreated) => Ok(CompactFresh),
            (OperatorReviewRequired, ReviewCompleted) => Ok(Observing),
            (OperatorReviewRequired, Block) => Ok(Blocked),
            (Blocked, _) => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "blocked context evaluation is terminal",
            )),
            _ => Err(invalid_transition(
                self.machine_id(),
                current,
                event,
                "event is not allowed from current context guardian state",
            )),
        }
    }

    fn allowed_events(&self, current: &Self::State) -> Vec<Self::Event> {
        use ContextGuardianEvent::*;
        use ContextGuardianState::*;
        match current {
            NoSession => vec![SessionStarted],
            Observing => vec![
                WithinBudget,
                NearBudgetLimit,
                ContextMeasured,
                CompactCreated,
                ReviewRequested,
                OverBudgetLimit,
                Block,
            ],
            Healthy => vec![NearBudgetLimit, CompactCreated],
            NearLimit => vec![CompactCreated, OverBudgetLimit, ReviewRequested],
            NeedsCompact => vec![CompactCreated, OverBudgetLimit, ReviewRequested],
            CompactFresh => vec![CompactExpired, HandoffCreated],
            CompactStale => vec![CompactCreated, OverBudgetLimit, ReviewRequested],
            OperatorReviewRequired => vec![ReviewCompleted, Block],
            HandoffReady | Blocked => vec![],
        }
    }

    fn is_terminal(&self, state: &Self::State) -> bool {
        matches!(
            state,
            ContextGuardianState::HandoffReady | ContextGuardianState::Blocked
        )
    }
}

fn invalid_transition<S, E>(machine_id: &str, state: &S, event: &E, reason: &str) -> anyhow::Error
where
    S: fmt::Display,
    E: fmt::Display,
{
    anyhow!(FsmError {
        machine_id: machine_id.to_string(),
        state: state.to_string(),
        event: event.to_string(),
        reason: reason.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worker_job_fsm_accepts_valid_lifecycle() {
        let fsm = WorkerJobFsm;
        let executing = fsm
            .transition(&WorkerJobState::Queued, &WorkerJobEvent::Start)
            .expect("start");
        assert_eq!(executing, WorkerJobState::Executing);
        let succeeded = fsm
            .transition(&executing, &WorkerJobEvent::Complete)
            .expect("complete");
        assert_eq!(succeeded, WorkerJobState::Succeeded);
        assert!(fsm.is_terminal(&succeeded));
    }

    #[test]
    fn worker_job_fsm_rejects_invalid_and_terminal_transitions() {
        let fsm = WorkerJobFsm;
        assert!(
            fsm.transition(&WorkerJobState::Queued, &WorkerJobEvent::Complete)
                .is_err()
        );
        assert!(
            fsm.transition(&WorkerJobState::Succeeded, &WorkerJobEvent::Start)
                .is_err()
        );
    }

    #[test]
    fn worker_job_fsm_models_retry_exhaustion() {
        let fsm = WorkerJobFsm;
        let retrying = fsm
            .transition(&WorkerJobState::Failed, &WorkerJobEvent::Retry)
            .expect("retry");
        assert_eq!(retrying, WorkerJobState::Retrying);
        let dead = fsm
            .transition(&WorkerJobState::Failed, &WorkerJobEvent::ExhaustRetries)
            .expect("dead letter");
        assert_eq!(dead, WorkerJobState::DeadLettered);
    }

    #[test]
    fn legacy_worker_completed_state_parses_to_succeeded() {
        assert_eq!(
            "completed".parse::<WorkerJobState>().expect("parse"),
            WorkerJobState::Succeeded
        );
    }

    #[test]
    fn workflow_cursor_fsm_accepts_valid_lifecycle() {
        let fsm = WorkflowCursorFsm;
        let ready = fsm
            .transition(
                &WorkflowCursorState::Declared,
                &WorkflowCursorEvent::Prepare,
            )
            .expect("prepare");
        assert_eq!(ready, WorkflowCursorState::Ready);
        let running = fsm
            .transition(&ready, &WorkflowCursorEvent::StartStep)
            .expect("start step");
        assert_eq!(running, WorkflowCursorState::StepRunning);
        let succeeded = fsm
            .transition(&running, &WorkflowCursorEvent::CompleteStep)
            .expect("complete step");
        assert_eq!(succeeded, WorkflowCursorState::StepSucceeded);
        let completed = fsm
            .transition(&succeeded, &WorkflowCursorEvent::CompleteWorkflow)
            .expect("complete workflow");
        assert_eq!(completed, WorkflowCursorState::Completed);
    }

    #[test]
    fn workflow_cursor_fsm_rejects_invalid_and_terminal_transitions() {
        let fsm = WorkflowCursorFsm;
        assert!(
            fsm.transition(
                &WorkflowCursorState::Ready,
                &WorkflowCursorEvent::CompleteWorkflow,
            )
            .is_err()
        );
        assert!(
            fsm.transition(
                &WorkflowCursorState::StepRunning,
                &WorkflowCursorEvent::StartStep,
            )
            .is_err()
        );
        assert!(
            fsm.transition(
                &WorkflowCursorState::Completed,
                &WorkflowCursorEvent::StartStep,
            )
            .is_err()
        );
    }

    #[test]
    fn workflow_cursor_failed_step_must_block_current_run() {
        let fsm = WorkflowCursorFsm;
        let failed = fsm
            .transition(
                &WorkflowCursorState::StepRunning,
                &WorkflowCursorEvent::FailStep,
            )
            .expect("fail step");
        assert_eq!(failed, WorkflowCursorState::StepFailed);
        assert!(
            fsm.transition(&failed, &WorkflowCursorEvent::StartStep)
                .is_err()
        );
        let blocked = fsm
            .transition(&failed, &WorkflowCursorEvent::Block)
            .expect("block");
        assert_eq!(blocked, WorkflowCursorState::Blocked);
    }

    #[test]
    fn session_lifecycle_compatibility_parses_old_strings() {
        assert_eq!(
            "ok".parse::<SessionLifecycleState>().expect("ok"),
            SessionLifecycleState::Completed
        );
        assert_eq!(
            "operator_denied"
                .parse::<SessionLifecycleState>()
                .expect("denied"),
            SessionLifecycleState::Failed
        );
    }

    #[test]
    fn policy_fsm_blocks_denied_and_pending_execution() {
        let fsm = PolicyApprovalFsm;
        assert!(
            fsm.transition(
                &PolicyApprovalState::BlockedByPolicy,
                &PolicyApprovalEvent::Execute,
            )
            .is_err()
        );
        assert!(
            fsm.transition(&PolicyApprovalState::Denied, &PolicyApprovalEvent::Execute,)
                .is_err()
        );
        assert!(
            fsm.transition(
                &PolicyApprovalState::ApprovalPending,
                &PolicyApprovalEvent::Execute,
            )
            .is_err()
        );
    }

    #[test]
    fn policy_fsm_replay_skips_side_effects() {
        let fsm = PolicyApprovalFsm;
        let skipped = fsm
            .transition(
                &PolicyApprovalState::BlockedByPolicy,
                &PolicyApprovalEvent::MarkReplaySkipped,
            )
            .expect("skip");
        assert_eq!(skipped, PolicyApprovalState::ReplaySkipped);
        assert!(fsm.is_terminal(&skipped));
    }

    #[test]
    fn transition_records_are_deterministic_and_report_rejections() {
        let fsm = WorkerJobFsm;
        let accepted = transition_record(
            &fsm,
            &WorkerJobState::Queued,
            &WorkerJobEvent::Start,
            "2026-06-18T00:00:00Z",
            Some("session:test#1".to_string()),
        );
        assert!(accepted.accepted);
        assert_eq!(accepted.next_state.as_deref(), Some("executing"));

        let rejected = transition_record(
            &fsm,
            &WorkerJobState::Succeeded,
            &WorkerJobEvent::Start,
            "2026-06-18T00:00:00Z",
            None,
        );
        assert!(!rejected.accepted);
        assert_eq!(rejected.next_state, None);
    }

    #[test]
    fn skill_execution_fsm_accepts_valid_lifecycle() {
        let fsm = SkillExecutionFsm;
        let loaded = fsm
            .transition(&SkillExecutionState::Declared, &SkillExecutionEvent::Load)
            .expect("load");
        let checked = fsm
            .transition(&loaded, &SkillExecutionEvent::CheckPolicy)
            .expect("check policy");
        let ready = fsm
            .transition(&checked, &SkillExecutionEvent::PolicyAllows)
            .expect("ready");
        let running = fsm
            .transition(&ready, &SkillExecutionEvent::Start)
            .expect("running");
        let succeeded = fsm
            .transition(&running, &SkillExecutionEvent::Complete)
            .expect("complete");
        assert_eq!(succeeded, SkillExecutionState::Succeeded);
        assert!(fsm.is_terminal(&succeeded));
    }

    #[test]
    fn skill_execution_fsm_blocks_and_rejects_invalid_jumps() {
        let fsm = SkillExecutionFsm;
        assert!(
            fsm.transition(&SkillExecutionState::Declared, &SkillExecutionEvent::Start,)
                .is_err()
        );
        let blocked = fsm
            .transition(
                &SkillExecutionState::PolicyChecked,
                &SkillExecutionEvent::PolicyBlocks,
            )
            .expect("blocked");
        assert_eq!(blocked, SkillExecutionState::Blocked);
        assert!(
            fsm.transition(&blocked, &SkillExecutionEvent::Start)
                .is_err()
        );
    }

    #[test]
    fn skill_execution_succeeded_and_failed_are_terminal() {
        let fsm = SkillExecutionFsm;
        assert!(
            fsm.transition(&SkillExecutionState::Succeeded, &SkillExecutionEvent::Start,)
                .is_err()
        );
        assert!(
            fsm.transition(&SkillExecutionState::Failed, &SkillExecutionEvent::Start)
                .is_err()
        );
    }

    #[test]
    fn context_guardian_fsm_accepts_valid_paths() {
        let fsm = ContextGuardianFsm;
        assert_eq!(
            fsm.transition(
                &ContextGuardianState::NoSession,
                &ContextGuardianEvent::SessionStarted,
            )
            .expect("session started"),
            ContextGuardianState::Observing
        );
        assert_eq!(
            fsm.transition(
                &ContextGuardianState::Observing,
                &ContextGuardianEvent::NearBudgetLimit,
            )
            .expect("near limit"),
            ContextGuardianState::NearLimit
        );
        assert_eq!(
            fsm.transition(
                &ContextGuardianState::CompactFresh,
                &ContextGuardianEvent::HandoffCreated,
            )
            .expect("handoff"),
            ContextGuardianState::HandoffReady
        );
    }

    #[test]
    fn context_guardian_fsm_rejects_invalid_and_blocked_transitions() {
        let fsm = ContextGuardianFsm;
        assert!(
            fsm.transition(
                &ContextGuardianState::NoSession,
                &ContextGuardianEvent::WithinBudget,
            )
            .is_err()
        );
        assert!(
            fsm.transition(
                &ContextGuardianState::Blocked,
                &ContextGuardianEvent::ReviewCompleted,
            )
            .is_err()
        );
    }
}
