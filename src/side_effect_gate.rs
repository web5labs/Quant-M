use crate::fsm_core::{
    PolicyApprovalEvent, PolicyApprovalFsm, PolicyApprovalState, TransitionRecord,
    transition_record,
};
use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectKind {
    ShellCommand,
    ProviderCall,
    NetworkHttp,
    WebhookSend,
    TelegramSend,
    AdapterSend,
    FileWrite,
    StateMutation,
    WorkerExecution,
    TradingLikeAction,
    ReplaySideEffect,
    Unknown,
}

impl fmt::Display for SideEffectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::ShellCommand => "shell_command",
            Self::ProviderCall => "provider_call",
            Self::NetworkHttp => "network_http",
            Self::WebhookSend => "webhook_send",
            Self::TelegramSend => "telegram_send",
            Self::AdapterSend => "adapter_send",
            Self::FileWrite => "file_write",
            Self::StateMutation => "state_mutation",
            Self::WorkerExecution => "worker_execution",
            Self::TradingLikeAction => "trading_like_action",
            Self::ReplaySideEffect => "replay_side_effect",
            Self::Unknown => "unknown",
        })
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum SideEffectDecision {
    Allowed,
    Blocked,
    ApprovalPending,
    Denied,
    Unavailable,
    DryRunOnly,
    ReplaySkipped,
}

impl fmt::Display for SideEffectDecision {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Allowed => "allowed",
            Self::Blocked => "blocked",
            Self::ApprovalPending => "approval_pending",
            Self::Denied => "denied",
            Self::Unavailable => "unavailable",
            Self::DryRunOnly => "dry_run_only",
            Self::ReplaySkipped => "replay_skipped",
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SideEffectRequest {
    pub side_effect_kind: SideEffectKind,
    pub action_label: String,
    pub config_allowed: bool,
    pub policy_allowed: bool,
    pub approval_required: bool,
    pub dry_run: bool,
    pub replay: bool,
    pub session_id: Option<String>,
    pub evidence_ref: Option<String>,
    pub occurred_at: Option<String>,
}

impl SideEffectRequest {
    pub fn new(side_effect_kind: SideEffectKind, action_label: impl Into<String>) -> Self {
        Self {
            side_effect_kind,
            action_label: action_label.into(),
            config_allowed: false,
            policy_allowed: false,
            approval_required: false,
            dry_run: false,
            replay: false,
            session_id: None,
            evidence_ref: None,
            occurred_at: None,
        }
    }

    pub fn config_allowed(mut self, value: bool) -> Self {
        self.config_allowed = value;
        self
    }

    pub fn policy_allowed(mut self, value: bool) -> Self {
        self.policy_allowed = value;
        self
    }

    #[allow(dead_code)]
    pub fn approval_required(mut self, value: bool) -> Self {
        self.approval_required = value;
        self
    }

    pub fn dry_run(mut self, value: bool) -> Self {
        self.dry_run = value;
        self
    }

    #[allow(dead_code)]
    pub fn replay(mut self, value: bool) -> Self {
        self.replay = value;
        self
    }

    pub fn session_id(mut self, value: impl Into<String>) -> Self {
        self.session_id = Some(value.into());
        self
    }

    pub fn evidence_ref(mut self, value: impl Into<String>) -> Self {
        self.evidence_ref = Some(value.into());
        self
    }

    #[allow(dead_code)]
    pub fn occurred_at(mut self, value: impl Into<String>) -> Self {
        self.occurred_at = Some(value.into());
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SideEffectGateResult {
    pub decision: SideEffectDecision,
    pub reason: String,
    pub side_effect_kind: SideEffectKind,
    pub action_label: String,
    pub config_allowed: bool,
    pub policy_allowed: bool,
    pub approval_required: bool,
    pub dry_run: bool,
    pub replay: bool,
    pub session_id: Option<String>,
    pub policy_fsm_transition: Option<TransitionRecord>,
    pub policy_evidence_summary: String,
}

impl SideEffectGateResult {
    pub fn is_allowed(&self) -> bool {
        self.decision == SideEffectDecision::Allowed
    }

    #[allow(dead_code)]
    pub fn blocks_execution(&self) -> bool {
        !self.is_allowed()
    }

    pub fn audit_note(&self) -> String {
        format!(
            "side_effect_gate action={} kind={} decision={} reason={}",
            self.action_label, self.side_effect_kind, self.decision, self.reason
        )
    }
}

pub fn evaluate_side_effect(request: SideEffectRequest) -> SideEffectGateResult {
    let (decision, reason, from_state, event) = if request.replay {
        (
            SideEffectDecision::ReplaySkipped,
            "replay path does not execute side effects".to_string(),
            PolicyApprovalState::BlockedByPolicy,
            PolicyApprovalEvent::MarkReplaySkipped,
        )
    } else if request.dry_run {
        (
            SideEffectDecision::DryRunOnly,
            "dry-run path records intent without executing the side effect".to_string(),
            PolicyApprovalState::BlockedByPolicy,
            PolicyApprovalEvent::MarkReplaySkipped,
        )
    } else if !request.config_allowed {
        (
            unavailable_decision(request.side_effect_kind),
            "side effect is disabled or unavailable by configuration".to_string(),
            PolicyApprovalState::EvaluatingPolicy,
            PolicyApprovalEvent::PolicyBlocks,
        )
    } else if request.approval_required {
        (
            SideEffectDecision::ApprovalPending,
            "side effect requires explicit approval before execution".to_string(),
            PolicyApprovalState::EvaluatingPolicy,
            PolicyApprovalEvent::NeedsHumanApproval,
        )
    } else if !request.policy_allowed {
        (
            SideEffectDecision::Denied,
            "side effect is denied by policy".to_string(),
            PolicyApprovalState::EvaluatingPolicy,
            PolicyApprovalEvent::PolicyBlocks,
        )
    } else {
        (
            SideEffectDecision::Allowed,
            "side effect is allowed by config and policy".to_string(),
            PolicyApprovalState::EvaluatingPolicy,
            PolicyApprovalEvent::PolicyAllows,
        )
    };

    let transition = transition_record(
        &PolicyApprovalFsm,
        &from_state,
        &event,
        request
            .occurred_at
            .clone()
            .unwrap_or_else(|| "side_effect_gate".to_string()),
        request.evidence_ref.clone(),
    );
    let summary = format!(
        "policy_approval:{}:{}->{}",
        transition.event,
        transition.previous_state,
        transition
            .next_state
            .clone()
            .unwrap_or_else(|| "rejected".to_string())
    );

    SideEffectGateResult {
        decision,
        reason,
        side_effect_kind: request.side_effect_kind,
        action_label: request.action_label,
        config_allowed: request.config_allowed,
        policy_allowed: request.policy_allowed,
        approval_required: request.approval_required,
        dry_run: request.dry_run,
        replay: request.replay,
        session_id: request.session_id,
        policy_fsm_transition: Some(transition),
        policy_evidence_summary: summary,
    }
}

fn unavailable_decision(kind: SideEffectKind) -> SideEffectDecision {
    match kind {
        SideEffectKind::ProviderCall
        | SideEffectKind::WebhookSend
        | SideEffectKind::TelegramSend
        | SideEffectKind::NetworkHttp => SideEffectDecision::Unavailable,
        SideEffectKind::TradingLikeAction => SideEffectDecision::Denied,
        _ => SideEffectDecision::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_request_blocks_unknown_side_effects() {
        let result = evaluate_side_effect(SideEffectRequest::new(
            SideEffectKind::Unknown,
            "unknown.action",
        ));

        assert_eq!(result.decision, SideEffectDecision::Blocked);
        assert!(result.blocks_execution());
        assert_eq!(
            result.policy_fsm_transition.unwrap().next_state.unwrap(),
            "blocked_by_policy"
        );
    }

    #[test]
    fn allowed_requires_config_and_policy() {
        let result = evaluate_side_effect(
            SideEffectRequest::new(SideEffectKind::ShellCommand, "worker.shell")
                .config_allowed(true)
                .policy_allowed(true),
        );

        assert_eq!(result.decision, SideEffectDecision::Allowed);
        assert!(result.is_allowed());
        assert_eq!(
            result.policy_fsm_transition.unwrap().next_state.unwrap(),
            "execution_allowed"
        );
    }

    #[test]
    fn provider_call_is_unavailable_when_not_configured() {
        let result = evaluate_side_effect(SideEffectRequest::new(
            SideEffectKind::ProviderCall,
            "llm.ask",
        ));

        assert_eq!(result.decision, SideEffectDecision::Unavailable);
    }

    #[test]
    fn approval_required_never_executes() {
        let result = evaluate_side_effect(
            SideEffectRequest::new(SideEffectKind::WebhookSend, "adapter.webhook_send")
                .config_allowed(true)
                .policy_allowed(true)
                .approval_required(true),
        );

        assert_eq!(result.decision, SideEffectDecision::ApprovalPending);
        assert!(result.blocks_execution());
    }

    #[test]
    fn dry_run_and_replay_skip_execution() {
        let dry_run = evaluate_side_effect(
            SideEffectRequest::new(SideEffectKind::NetworkHttp, "worker.http_get").dry_run(true),
        );
        let replay = evaluate_side_effect(
            SideEffectRequest::new(SideEffectKind::ReplaySideEffect, "session.replay").replay(true),
        );

        assert_eq!(dry_run.decision, SideEffectDecision::DryRunOnly);
        assert_eq!(replay.decision, SideEffectDecision::ReplaySkipped);
        assert!(dry_run.blocks_execution());
        assert!(replay.blocks_execution());
    }

    #[test]
    fn trading_like_actions_deny_by_default() {
        let result = evaluate_side_effect(SideEffectRequest::new(
            SideEffectKind::TradingLikeAction,
            "trading.order",
        ));

        assert_eq!(result.decision, SideEffectDecision::Denied);
    }

    #[test]
    fn json_is_deterministic_for_fixed_request() {
        let request = SideEffectRequest::new(SideEffectKind::ProviderCall, "llm.ask")
            .config_allowed(true)
            .policy_allowed(false)
            .session_id("session-1")
            .evidence_ref("test")
            .occurred_at("2026-01-01T00:00:00Z");
        let first = serde_json::to_string(&evaluate_side_effect(request.clone())).unwrap();
        let second = serde_json::to_string(&evaluate_side_effect(request)).unwrap();

        assert_eq!(first, second);
        assert!(first.contains("\"decision\":\"denied\""));
        assert!(first.contains("\"side_effect_kind\":\"provider_call\""));
    }
}
