use crate::config::{Config, ExternalChannel};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ChannelDescriptor {
    pub channel: ExternalChannel,
    pub label: &'static str,
    pub live_adapter: bool,
    pub configured: bool,
    pub notes: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChannelEventType {
    NotificationReceived,
    ApprovalRecorded,
    DenialRecorded,
    EscalationRequested,
    EvidenceAttached,
    CommandRejected,
    PolicyBlocked,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChannelIntentRecord {
    pub channel: ExternalChannel,
    pub event_type: ChannelEventType,
    pub raw_text: String,
    pub reason: String,
    pub evidence_only: bool,
    pub executes_runtime: bool,
    pub mutates_shared_state: bool,
    pub mutates_cost_ledger: bool,
    pub calls_provider: bool,
    pub performs_trading: bool,
    pub bypasses_policy: bool,
    pub bypasses_operator_approval: bool,
}

impl ChannelIntentRecord {
    pub fn operator_reply(&self) -> String {
        match self.event_type {
            ChannelEventType::CommandRejected | ChannelEventType::PolicyBlocked => {
                format!("channel intent rejected: {}", self.reason)
            }
            ChannelEventType::ApprovalRecorded => {
                "channel approval recorded as evidence only; no action executed".to_string()
            }
            ChannelEventType::DenialRecorded => {
                "channel denial recorded as evidence only; no action executed".to_string()
            }
            ChannelEventType::EscalationRequested => {
                "channel escalation recorded as evidence only; no action executed".to_string()
            }
            ChannelEventType::NotificationReceived | ChannelEventType::EvidenceAttached => {
                "channel message recorded as evidence only; no action executed".to_string()
            }
        }
    }
}

pub fn channel_label(channel: ExternalChannel) -> &'static str {
    match channel {
        ExternalChannel::Telegram => "telegram",
        ExternalChannel::Discord => "discord",
        ExternalChannel::Slack => "slack",
        ExternalChannel::Signal => "signal",
        ExternalChannel::Whatsapp => "whatsapp",
        ExternalChannel::Ichat => "ichat",
        ExternalChannel::Email => "email",
        ExternalChannel::None => "none",
    }
}

pub fn configured_channels(cfg: &Config) -> Vec<ChannelDescriptor> {
    let configured_policy = cfg.chat_channels.enabled;
    let allowed = &cfg.chat_channels.allowed_channels;
    all_channels()
        .into_iter()
        .map(|channel| ChannelDescriptor {
            channel,
            label: channel_label(channel),
            live_adapter: channel == ExternalChannel::Telegram && cfg.telegram.enabled,
            configured: configured_policy && allowed.contains(&channel),
            notes: channel_notes(channel),
        })
        .collect()
}

pub fn classify_channel_message(channel: ExternalChannel, text: &str) -> ChannelIntentRecord {
    let raw_text = text.trim().to_string();
    let normalized = raw_text.to_ascii_lowercase();
    let (event_type, reason) = if is_command_like(&normalized) {
        (
            ChannelEventType::CommandRejected,
            "channels are not execution authorities; use the Quant-M CLI for governed actions"
                .to_string(),
        )
    } else if looks_like_approval(&normalized) {
        (
            ChannelEventType::ApprovalRecorded,
            "approval language is evidence only and cannot bypass runtime approval gates"
                .to_string(),
        )
    } else if looks_like_denial(&normalized) {
        (
            ChannelEventType::DenialRecorded,
            "denial language is evidence only and cannot execute or mutate runtime state"
                .to_string(),
        )
    } else if looks_like_escalation(&normalized) {
        (
            ChannelEventType::EscalationRequested,
            "escalation language is evidence only and requires explicit operator follow-up"
                .to_string(),
        )
    } else if raw_text.is_empty() {
        (
            ChannelEventType::NotificationReceived,
            "empty channel message ignored as non-executing notification".to_string(),
        )
    } else {
        (
            ChannelEventType::EvidenceAttached,
            "channel message is non-executing evidence".to_string(),
        )
    };

    ChannelIntentRecord {
        channel,
        event_type,
        raw_text,
        reason,
        evidence_only: true,
        executes_runtime: false,
        mutates_shared_state: false,
        mutates_cost_ledger: false,
        calls_provider: false,
        performs_trading: false,
        bypasses_policy: false,
        bypasses_operator_approval: false,
    }
}

fn all_channels() -> Vec<ExternalChannel> {
    vec![
        ExternalChannel::Telegram,
        ExternalChannel::Discord,
        ExternalChannel::Slack,
        ExternalChannel::Signal,
        ExternalChannel::Whatsapp,
        ExternalChannel::Ichat,
        ExternalChannel::Email,
    ]
}

fn channel_notes(channel: ExternalChannel) -> &'static str {
    match channel {
        ExternalChannel::Telegram => "polling bot adapter; disabled unless telegram.enabled=true",
        ExternalChannel::Discord
        | ExternalChannel::Slack
        | ExternalChannel::Signal
        | ExternalChannel::Whatsapp
        | ExternalChannel::Ichat
        | ExternalChannel::Email => "configured channel surface only; no live adapter yet",
        ExternalChannel::None => "no external channel",
    }
}

fn is_command_like(normalized: &str) -> bool {
    let command_markers = [
        "quant-m ",
        "consensus",
        "dry-run",
        "replay",
        "state review",
        "cost summary",
        "run workflow",
        "worker once",
        "worker submit",
        "provider validate",
        "--live",
        "tool validate",
        "shell",
        "exec",
        "execute",
        "trade",
        "trading",
        "buy ",
        "sell ",
        "/ask",
        "/consensus",
        "/replay",
        "/state",
        "/cost",
        "/run",
        "/exec",
    ];
    command_markers
        .iter()
        .any(|marker| normalized.contains(marker))
}

fn looks_like_approval(normalized: &str) -> bool {
    normalized == "approve"
        || normalized == "/approve"
        || normalized.starts_with("approve ")
        || normalized.contains("approved")
}

fn looks_like_denial(normalized: &str) -> bool {
    normalized == "deny"
        || normalized == "/deny"
        || normalized.starts_with("deny ")
        || normalized.contains("denied")
}

fn looks_like_escalation(normalized: &str) -> bool {
    normalized == "escalate"
        || normalized == "/escalate"
        || normalized.starts_with("escalate ")
        || normalized.contains("needs escalation")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bootstrap;
    use crate::consensus;
    use crate::cost_ledger;
    use crate::shared_state::{HybridSharedStateStore, SharedStateStore};
    use std::fs;
    use tempfile::TempDir;

    #[allow(clippy::field_reassign_with_default)]
    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let mut cfg = Config::default();
        cfg.workspace_dir = tmp.path().join("workspace");
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        cfg.state_sql.sqlite_path = cfg.workspace_dir.join("state/shared-state.db");
        cfg.forex.redb_path = cfg.workspace_dir.join("state/forex.redb");
        cfg.llm.enabled = false;
        cfg.runtime.external_network_enabled = false;
        bootstrap::ensure_workspace(&cfg).expect("workspace");
        (tmp, cfg)
    }

    fn assert_no_channel_authority(record: &ChannelIntentRecord) {
        assert!(record.evidence_only);
        assert!(!record.executes_runtime);
        assert!(!record.mutates_shared_state);
        assert!(!record.mutates_cost_ledger);
        assert!(!record.calls_provider);
        assert!(!record.performs_trading);
        assert!(!record.bypasses_policy);
        assert!(!record.bypasses_operator_approval);
    }

    #[test]
    fn telegram_reports_live_adapter_only_when_enabled() {
        let mut cfg = Config::default();
        cfg.chat_channels.enabled = true;
        cfg.telegram.enabled = true;
        let items = configured_channels(&cfg);
        let telegram = items
            .iter()
            .find(|item| item.channel == ExternalChannel::Telegram)
            .expect("telegram channel");

        assert!(telegram.configured);
        assert!(telegram.live_adapter);
    }

    #[test]
    fn channel_command_like_text_is_rejected_as_non_executing_intent() {
        let record = classify_channel_message(
            ExternalChannel::Slack,
            "quant-m consensus --dry-run \"Should we adopt this API design?\"",
        );

        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert_no_channel_authority(&record);
    }

    #[test]
    fn telegram_like_input_cannot_invoke_consensus() {
        let (_tmp, cfg) = temp_cfg();
        let record =
            classify_channel_message(ExternalChannel::Telegram, "/consensus Should we adopt?");

        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert_no_channel_authority(&record);
        assert!(!cfg.workspace_dir.join("state/consensus").exists());
    }

    #[test]
    fn chat_like_input_cannot_invoke_consensus() {
        let (_tmp, cfg) = temp_cfg();
        let record = classify_channel_message(
            ExternalChannel::Discord,
            "consensus --dry-run Should we adopt?",
        );

        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert_no_channel_authority(&record);
        assert!(!cfg.workspace_dir.join("state/consensus").exists());
    }

    #[test]
    fn approval_text_creates_approval_evidence_only() {
        let record = classify_channel_message(
            ExternalChannel::Telegram,
            "approve for manual follow-up only",
        );

        assert_eq!(record.event_type, ChannelEventType::ApprovalRecorded);
        assert_no_channel_authority(&record);
    }

    #[test]
    fn denial_text_creates_denial_evidence_only() {
        let record = classify_channel_message(ExternalChannel::Signal, "deny this action");

        assert_eq!(record.event_type, ChannelEventType::DenialRecorded);
        assert_no_channel_authority(&record);
    }

    #[test]
    fn escalation_text_creates_escalation_evidence_only() {
        let record = classify_channel_message(ExternalChannel::Email, "escalate to operator");

        assert_eq!(record.event_type, ChannelEventType::EscalationRequested);
        assert_no_channel_authority(&record);
    }

    #[test]
    fn channel_path_cannot_write_consensus_state_or_cost_records_directly() {
        let (_tmp, cfg) = temp_cfg();
        let store = HybridSharedStateStore::from_config(&cfg);
        let before_state = store
            .list(Some(&crate::sessions::DomainId::new("domain:consensus")))
            .expect("list before");
        let ledger_path = cost_ledger::cost_ledger_path(&cfg);
        let before_ledger = fs::read(&ledger_path).unwrap_or_default();

        let record = classify_channel_message(
            ExternalChannel::Telegram,
            "quant-m consensus --dry-run \"Should we adopt this API design?\"",
        );

        let after_state = store
            .list(Some(&crate::sessions::DomainId::new("domain:consensus")))
            .expect("list after");
        let after_ledger = fs::read(&ledger_path).unwrap_or_default();
        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert_eq!(before_state, after_state);
        assert_eq!(before_ledger, after_ledger);
        assert_no_channel_authority(&record);
    }

    #[test]
    fn channel_path_cannot_call_provider_runtime_or_trigger_trading() {
        for text in [
            "/ask call the best provider",
            "provider validate openrouter --live",
            "execute trade buy EURUSD",
        ] {
            let record = classify_channel_message(ExternalChannel::Telegram, text);
            assert_no_channel_authority(&record);
            assert!(!record.calls_provider);
            assert!(!record.performs_trading);
        }
    }

    #[test]
    fn channel_path_cannot_bypass_policy_or_operator_approval() {
        let record = classify_channel_message(
            ExternalChannel::Telegram,
            "approve and execute run workflow workflow:mock-research-brief",
        );

        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert_no_channel_authority(&record);
    }

    #[test]
    fn channel_path_cannot_bypass_replay_validation_or_mutate_artifacts() {
        let (_tmp, cfg) = temp_cfg();
        let report = consensus::run_consensus_dry_run(&cfg, "Should we adopt this API design?")
            .expect("consensus");
        let session_dir = cfg.runtime.session_dir.join(&report.session_id);
        let report_path = session_dir.join("consensus-report.json");
        let evidence_path = session_dir.join("evidence-index.json");
        let ledger_path = cost_ledger::cost_ledger_path(&cfg);
        let before_report = fs::read(&report_path).expect("report before");
        let before_evidence = fs::read(&evidence_path).expect("evidence before");
        let before_ledger = fs::read(&ledger_path).expect("ledger before");

        let record = classify_channel_message(
            ExternalChannel::Telegram,
            &format!("replay {}", report.session_id),
        );

        assert_eq!(record.event_type, ChannelEventType::CommandRejected);
        assert_eq!(before_report, fs::read(&report_path).expect("report after"));
        assert_eq!(
            before_evidence,
            fs::read(&evidence_path).expect("evidence after")
        );
        assert_eq!(before_ledger, fs::read(&ledger_path).expect("ledger after"));
        assert_no_channel_authority(&record);
    }
}
