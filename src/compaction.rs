use crate::config::Config;
use crate::sessions::{self, SessionEvent, SessionId, SessionLogEntry};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

const MISSING_VALIDATION: &str = "No validation evidence found.";
const MISSING_CHANGED_FILES: &str = "No changed-file evidence found.";
const MISSING_SHIPPABLE: &str = "No shippable definition found.";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactPacket {
    pub session_id: String,
    pub created_at: String,
    pub source_event_count: usize,
    pub goal: String,
    pub current_state: String,
    pub decisions: Vec<String>,
    pub files_changed: Vec<String>,
    pub commands_observed: Vec<String>,
    pub evidence_refs: Vec<EvidenceRef>,
    pub policy_blocks: Vec<String>,
    pub open_risks: Vec<String>,
    pub next_safe_actions: Vec<String>,
    pub blocked_actions: Vec<String>,
    pub definition_of_shippable: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceRef {
    pub sequence: u64,
    pub step_id: String,
    pub occurred_at: String,
    pub event_kind: String,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CompactArtifacts {
    pub output_dir: PathBuf,
    pub compact_md: PathBuf,
    pub compact_json: PathBuf,
    pub evidence_index_json: PathBuf,
    pub next_action_md: PathBuf,
    pub risks_md: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CompactResult {
    pub session_id: String,
    pub created_at: String,
    pub source_event_count: usize,
    pub artifacts: CompactArtifacts,
}

pub fn compact_session(cfg: &Config, session_id: &SessionId) -> Result<CompactResult> {
    compact_session_at(cfg, session_id, Utc::now().to_rfc3339())
}

fn compact_session_at(
    cfg: &Config,
    session_id: &SessionId,
    created_at: String,
) -> Result<CompactResult> {
    let detail = sessions::show_session(cfg, session_id)?;
    let packet = build_packet(&detail.events, created_at)?;
    write_artifacts(cfg, &packet)
}

fn build_packet(events: &[SessionLogEntry], created_at: String) -> Result<CompactPacket> {
    let first = events
        .first()
        .ok_or_else(|| anyhow!("cannot compact an empty session"))?;
    let replay = sessions::replay_session_from_entries_for_compaction(events)?;
    let resume_plan = sessions::resume_plan_from_entries_for_compaction(events)?;
    let evidence_refs = build_evidence_refs(events);
    let policy_blocks = collect_policy_blocks(events);
    let commands_observed = collect_commands(events);
    let files_changed = collect_changed_files(events);
    let open_risks = collect_open_risks(events, &policy_blocks, &files_changed, &commands_observed);
    let next_safe_actions = collect_next_safe_actions(&resume_plan);
    let blocked_actions = collect_blocked_actions(&policy_blocks, &open_risks);
    let decisions = collect_decisions(events);
    let goal = collect_goal(events);
    let definition_of_shippable = collect_definition_of_shippable(events);

    Ok(CompactPacket {
        session_id: first.session_id.to_string(),
        created_at,
        source_event_count: events.len(),
        goal,
        current_state: replay.state.final_status,
        decisions,
        files_changed,
        commands_observed,
        evidence_refs,
        policy_blocks,
        open_risks,
        next_safe_actions,
        blocked_actions,
        definition_of_shippable,
        confidence: confidence_for(events),
    })
}

fn write_artifacts(cfg: &Config, packet: &CompactPacket) -> Result<CompactResult> {
    let output_dir = cfg
        .workspace_dir
        .join("state")
        .join("compacted")
        .join(&packet.session_id);
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let artifacts = CompactArtifacts {
        compact_md: output_dir.join("compact.md"),
        compact_json: output_dir.join("compact.json"),
        evidence_index_json: output_dir.join("evidence-index.json"),
        next_action_md: output_dir.join("next-action.md"),
        risks_md: output_dir.join("risks.md"),
        output_dir,
    };

    write_text(&artifacts.compact_md, &render_compact_markdown(packet))?;
    write_text(
        &artifacts.compact_json,
        &format!("{}\n", serde_json::to_string_pretty(packet)?),
    )?;
    write_text(
        &artifacts.evidence_index_json,
        &format!("{}\n", serde_json::to_string_pretty(&packet.evidence_refs)?),
    )?;
    write_text(&artifacts.next_action_md, &render_next_actions(packet))?;
    write_text(&artifacts.risks_md, &render_risks(packet))?;

    Ok(CompactResult {
        session_id: packet.session_id.clone(),
        created_at: packet.created_at.clone(),
        source_event_count: packet.source_event_count,
        artifacts,
    })
}

fn write_text(path: &PathBuf, content: &str) -> Result<()> {
    fs::write(path, content).with_context(|| format!("failed to write {}", path.display()))
}

fn build_evidence_refs(events: &[SessionLogEntry]) -> Vec<EvidenceRef> {
    events
        .iter()
        .map(|entry| EvidenceRef {
            sequence: entry.sequence,
            step_id: entry.step_id.to_string(),
            occurred_at: entry.occurred_at.clone(),
            event_kind: event_kind(&entry.event).to_string(),
            summary: event_summary(&entry.event),
        })
        .collect()
}

fn collect_goal(events: &[SessionLogEntry]) -> String {
    events
        .iter()
        .find_map(|entry| match &entry.event {
            SessionEvent::Observation {
                message, detail, ..
            } => {
                let normalized = format!("{} {}", message, detail.clone().unwrap_or_default())
                    .to_ascii_lowercase();
                if normalized.contains("goal") {
                    Some(join_message_detail(message, detail.as_deref()))
                } else {
                    None
                }
            }
            _ => None,
        })
        .or_else(|| {
            events.iter().find_map(|entry| match &entry.event {
                SessionEvent::Observation {
                    message, detail, ..
                } => Some(join_message_detail(message, detail.as_deref())),
                _ => None,
            })
        })
        .unwrap_or_else(|| "No explicit goal found.".to_string())
}

fn collect_decisions(events: &[SessionLogEntry]) -> Vec<String> {
    let mut decisions = Vec::new();
    for entry in events {
        match &entry.event {
            SessionEvent::PolicyDecision {
                policy,
                allowed,
                reason,
            } => decisions.push(format!(
                "step {}: policy '{}' allowed={} because {}",
                entry.step_id, policy, allowed, reason
            )),
            SessionEvent::OperatorDecision { record } => decisions.push(format!(
                "step {}: operator decision {:?} because {}",
                entry.step_id, record.decision, record.reason
            )),
            SessionEvent::FsmTransition {
                machine,
                from_state,
                to_state,
                reason,
            } => decisions.push(format!(
                "step {}: fsm '{}' {:?} -> '{}' because {}",
                entry.step_id, machine, from_state, to_state, reason
            )),
            _ => {}
        }
    }
    if decisions.is_empty() {
        decisions.push("No explicit decisions found.".to_string());
    }
    decisions
}

fn collect_changed_files(events: &[SessionLogEntry]) -> Vec<String> {
    let mut files = Vec::new();
    for entry in events {
        let text = event_search_text(&entry.event);
        for token in text.split_whitespace() {
            let trimmed = token.trim_matches(|c: char| {
                matches!(c, ',' | ';' | ':' | '"' | '\'' | ')' | '(' | '[' | ']')
            });
            if looks_like_file_path(trimmed) && !files.iter().any(|existing| existing == trimmed) {
                files.push(trimmed.to_string());
            }
        }
    }
    if files.is_empty() {
        files.push(MISSING_CHANGED_FILES.to_string());
    }
    files
}

fn collect_commands(events: &[SessionLogEntry]) -> Vec<String> {
    let mut commands = Vec::new();
    for entry in events {
        if let SessionEvent::SkillCall {
            command_preview: Some(command),
            ..
        } = &entry.event
        {
            commands.push(format!("step {}: {}", entry.step_id, command));
        }
    }
    if commands.is_empty() {
        commands.push(MISSING_VALIDATION.to_string());
    }
    commands
}

fn collect_policy_blocks(events: &[SessionLogEntry]) -> Vec<String> {
    let mut blocks = Vec::new();
    for entry in events {
        if let SessionEvent::PolicyDecision {
            policy,
            allowed: false,
            reason,
        } = &entry.event
        {
            blocks.push(format!(
                "step {}: {} blocked: {}",
                entry.step_id, policy, reason
            ));
        }
    }
    if blocks.is_empty() {
        blocks.push("No policy blocks found.".to_string());
    }
    blocks
}

fn collect_open_risks(
    events: &[SessionLogEntry],
    policy_blocks: &[String],
    files_changed: &[String],
    commands_observed: &[String],
) -> Vec<String> {
    let mut risks = Vec::new();
    for entry in events {
        match &entry.event {
            SessionEvent::Error { message, .. } => {
                risks.push(format!(
                    "step {}: error remains: {}",
                    entry.step_id, message
                ));
            }
            SessionEvent::Retry { reason, .. } => {
                risks.push(format!(
                    "step {}: retry recorded: {}",
                    entry.step_id, reason
                ));
            }
            _ => {}
        }
    }
    if files_changed
        .iter()
        .any(|value| value == MISSING_CHANGED_FILES)
    {
        risks.push(MISSING_CHANGED_FILES.to_string());
    }
    if commands_observed
        .iter()
        .any(|value| value == MISSING_VALIDATION)
    {
        risks.push(MISSING_VALIDATION.to_string());
    }
    if policy_blocks
        .iter()
        .any(|value| value != "No policy blocks found.")
    {
        risks.push("One or more policy blocks remain in the session evidence.".to_string());
    }
    if risks.is_empty() {
        risks.push("No unresolved risks found in session evidence.".to_string());
    }
    risks
}

fn collect_next_safe_actions(resume_plan: &sessions::ResumePlan) -> Vec<String> {
    if let Some(next_step) = &resume_plan.proposed_next_step {
        vec![next_step.clone()]
    } else if resume_plan.status == sessions::ResumeStatus::Complete {
        vec!["Review compact artifacts and keep session evidence for handoff.".to_string()]
    } else {
        vec!["Session is not safe to continue without operator review.".to_string()]
    }
}

fn collect_blocked_actions(policy_blocks: &[String], open_risks: &[String]) -> Vec<String> {
    let mut blocked = Vec::new();
    if policy_blocks
        .iter()
        .any(|value| value != "No policy blocks found.")
    {
        blocked.push(
            "Do not retry policy-blocked actions without explicit policy/config review."
                .to_string(),
        );
    }
    if open_risks.iter().any(|value| value == MISSING_VALIDATION) {
        blocked
            .push("Do not claim validation success until validation evidence exists.".to_string());
    }
    if open_risks
        .iter()
        .any(|value| value == MISSING_CHANGED_FILES)
    {
        blocked.push("Do not claim changed-file evidence until file evidence exists.".to_string());
    }
    if blocked.is_empty() {
        blocked.push("No blocked actions found in session evidence.".to_string());
    }
    blocked
}

fn collect_definition_of_shippable(events: &[SessionLogEntry]) -> String {
    events
        .iter()
        .find_map(|entry| {
            let text = event_search_text(&entry.event);
            if text.to_ascii_lowercase().contains("shippable") {
                Some(text)
            } else {
                None
            }
        })
        .unwrap_or_else(|| MISSING_SHIPPABLE.to_string())
}

fn confidence_for(events: &[SessionLogEntry]) -> f64 {
    let mut score: f64 = 0.4;
    if events
        .iter()
        .any(|entry| matches!(entry.event, SessionEvent::Output { .. }))
    {
        score += 0.2;
    }
    if events
        .iter()
        .any(|entry| matches!(entry.event, SessionEvent::PolicyDecision { .. }))
    {
        score += 0.1;
    }
    if events
        .iter()
        .any(|entry| matches!(entry.event, SessionEvent::FsmTransition { .. }))
    {
        score += 0.1;
    }
    if events
        .iter()
        .any(|entry| matches!(entry.event, SessionEvent::Error { .. }))
    {
        score -= 0.1;
    }
    score.clamp(0.0, 1.0)
}

fn event_kind(event: &SessionEvent) -> &'static str {
    match event {
        SessionEvent::Observation { .. } => "observation",
        SessionEvent::SkillCall { .. } => "skill_call",
        SessionEvent::PolicyDecision { .. } => "policy_decision",
        SessionEvent::FsmTransition { .. } => "fsm_transition",
        SessionEvent::Error { .. } => "error",
        SessionEvent::Retry { .. } => "retry",
        SessionEvent::Output { .. } => "output",
        SessionEvent::OperatorDecision { .. } => "operator_decision",
        SessionEvent::AuditNote { .. } => "audit_note",
    }
}

fn event_summary(event: &SessionEvent) -> String {
    match event {
        SessionEvent::Observation {
            message, detail, ..
        } => join_message_detail(message, detail.as_deref()),
        SessionEvent::SkillCall {
            skill_name,
            input_preview,
            command_preview,
            status,
        } => format!(
            "skill={} status={} input={} command={}",
            skill_name,
            status,
            input_preview,
            command_preview
                .as_deref()
                .unwrap_or("No command preview recorded.")
        ),
        SessionEvent::PolicyDecision {
            policy,
            allowed,
            reason,
        } => format!("policy={} allowed={} reason={}", policy, allowed, reason),
        SessionEvent::FsmTransition {
            machine,
            from_state,
            to_state,
            reason,
        } => format!(
            "machine={} from={:?} to={} reason={}",
            machine, from_state, to_state, reason
        ),
        SessionEvent::Error { code, message } => {
            format!("code={:?} message={}", code, message)
        }
        SessionEvent::Retry {
            attempt,
            next_attempt,
            reason,
            ..
        } => format!(
            "attempt={} next_attempt={:?} reason={}",
            attempt, next_attempt, reason
        ),
        SessionEvent::Output {
            channel, summary, ..
        } => format!("channel={} summary={}", channel, summary),
        SessionEvent::OperatorDecision { record } => format!(
            "decision={:?} reason={} decided_by={}",
            record.decision, record.reason, record.decided_by
        ),
        SessionEvent::AuditNote { note } => note.clone(),
    }
}

fn event_search_text(event: &SessionEvent) -> String {
    event_summary(event)
}

fn join_message_detail(message: &str, detail: Option<&str>) -> String {
    match detail {
        Some(detail) if !detail.trim().is_empty() => format!("{}: {}", message, detail),
        _ => message.to_string(),
    }
}

fn looks_like_file_path(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    (value.contains('/') || value.contains('\\'))
        && (lower.ends_with(".rs")
            || lower.ends_with(".md")
            || lower.ends_with(".json")
            || lower.ends_with(".toml")
            || lower.ends_with(".txt")
            || lower.ends_with(".yaml")
            || lower.ends_with(".yml"))
}

fn render_compact_markdown(packet: &CompactPacket) -> String {
    format!(
        "# Compact Truth Packet\n\n\
## Goal\n\n{}\n\n\
## Current State\n\n{}\n\n\
## Decisions Made\n\n{}\n\n\
## Evidence\n\n{}\n\n\
## Files Changed\n\n{}\n\n\
## Commands / Validation\n\n{}\n\n\
## Policy Blocks\n\n{}\n\n\
## Open Risks\n\n{}\n\n\
## Next Safe Action\n\n{}\n\n\
## Definition of Shippable\n\n{}\n",
        packet.goal,
        packet.current_state,
        render_list(&packet.decisions),
        render_evidence_refs(&packet.evidence_refs),
        render_list(&packet.files_changed),
        render_list(&packet.commands_observed),
        render_list(&packet.policy_blocks),
        render_list(&packet.open_risks),
        render_list(&packet.next_safe_actions),
        packet.definition_of_shippable,
    )
}

fn render_next_actions(packet: &CompactPacket) -> String {
    format!(
        "# Next Safe Action\n\n{}\n\n# Blocked Actions\n\n{}\n",
        render_list(&packet.next_safe_actions),
        render_list(&packet.blocked_actions)
    )
}

fn render_risks(packet: &CompactPacket) -> String {
    format!("# Risks\n\n{}\n", render_list(&packet.open_risks))
}

fn render_list(items: &[String]) -> String {
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_evidence_refs(refs: &[EvidenceRef]) -> String {
    refs.iter()
        .map(|item| {
            format!(
                "- seq={} step={} kind={} at={} :: {}",
                item.sequence, item.step_id, item.event_kind, item.occurred_at, item.summary
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::sessions::{SessionContext, append_event, runtime_context};
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

    fn fixture_session(cfg: &Config) -> SessionContext {
        let context = runtime_context("node-compact", "worker");
        append_event(
            cfg,
            &context,
            SessionEvent::Observation {
                message: "goal".to_string(),
                job_id: Some("job-compact".to_string()),
                detail: Some("prove compaction for quantm/src/main.rs".to_string()),
            },
        )
        .expect("append goal");
        append_event(
            cfg,
            &context,
            SessionEvent::SkillCall {
                skill_name: "validation".to_string(),
                input_preview: "run tests".to_string(),
                command_preview: Some("cargo test compaction".to_string()),
                status: "ok".to_string(),
            },
        )
        .expect("append skill");
        append_event(
            cfg,
            &context,
            SessionEvent::PolicyDecision {
                policy: "worker.allow_shell_commands".to_string(),
                allowed: false,
                reason: "shell disabled".to_string(),
            },
        )
        .expect("append policy");
        append_event(
            cfg,
            &context,
            SessionEvent::Output {
                channel: "worker".to_string(),
                summary: "validated compact artifacts".to_string(),
                job_id: Some("job-compact".to_string()),
            },
        )
        .expect("append output");
        context
    }

    #[test]
    fn compact_output_is_deterministic_for_same_input() {
        let (_tmp, cfg) = temp_cfg();
        let context = fixture_session(&cfg);
        let created_at = "2026-06-13T00:00:00Z".to_string();

        let first = build_packet(
            &sessions::show_session(&cfg, &context.session_id)
                .unwrap()
                .events,
            created_at.clone(),
        )
        .expect("first packet");
        let second = build_packet(
            &sessions::show_session(&cfg, &context.session_id)
                .unwrap()
                .events,
            created_at,
        )
        .expect("second packet");

        assert_eq!(first, second);
    }

    #[test]
    fn missing_evidence_is_reported_honestly() {
        let (_tmp, cfg) = temp_cfg();
        let context = runtime_context("node-compact-missing", "worker");
        append_event(
            &cfg,
            &context,
            SessionEvent::Observation {
                message: "goal".to_string(),
                job_id: None,
                detail: Some("inspect only".to_string()),
            },
        )
        .expect("append observation");

        let packet = build_packet(
            &sessions::show_session(&cfg, &context.session_id)
                .unwrap()
                .events,
            "2026-06-13T00:00:00Z".to_string(),
        )
        .expect("packet");

        assert!(
            packet
                .commands_observed
                .contains(&MISSING_VALIDATION.to_string())
        );
        assert!(
            packet
                .files_changed
                .contains(&MISSING_CHANGED_FILES.to_string())
        );
        assert_eq!(packet.definition_of_shippable, MISSING_SHIPPABLE);
    }

    #[test]
    fn policy_blocks_survive_compaction() {
        let (_tmp, cfg) = temp_cfg();
        let context = fixture_session(&cfg);
        let packet = build_packet(
            &sessions::show_session(&cfg, &context.session_id)
                .unwrap()
                .events,
            "2026-06-13T00:00:00Z".to_string(),
        )
        .expect("packet");

        assert!(
            packet
                .policy_blocks
                .iter()
                .any(|block| block.contains("worker.allow_shell_commands"))
        );
        assert!(
            packet
                .blocked_actions
                .iter()
                .any(|action| action.contains("policy-blocked"))
        );
    }

    #[test]
    fn compact_files_are_created_in_expected_directory() {
        let (_tmp, cfg) = temp_cfg();
        let context = fixture_session(&cfg);

        let result = compact_session_at(
            &cfg,
            &context.session_id,
            "2026-06-13T00:00:00Z".to_string(),
        )
        .expect("compact");

        assert!(result.artifacts.compact_md.exists());
        assert!(result.artifacts.compact_json.exists());
        assert!(result.artifacts.evidence_index_json.exists());
        assert!(result.artifacts.next_action_md.exists());
        assert!(result.artifacts.risks_md.exists());
        assert!(
            result
                .artifacts
                .output_dir
                .ends_with(context.session_id.as_str())
        );
    }

    #[test]
    fn missing_session_fails_safely() {
        let (_tmp, cfg) = temp_cfg();
        let missing = SessionId::new("session-missing");
        let error = compact_session_at(&cfg, &missing, "2026-06-13T00:00:00Z".to_string())
            .expect_err("missing session should fail");

        assert!(error.to_string().contains("not found"));
    }
}
