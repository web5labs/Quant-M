use crate::config::Config;
use crate::context_status::{self, ContextStatusReport};
use crate::fsm_core::{ContextGuardianState, ContextRecommendedAction};
use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PacketSize {
    Small,
    Medium,
    Large,
    Audit,
}

impl PacketSize {
    pub fn as_str(self) -> &'static str {
        match self {
            PacketSize::Small => "small",
            PacketSize::Medium => "medium",
            PacketSize::Large => "large",
            PacketSize::Audit => "audit",
        }
    }

    fn allowed_tiers(self) -> Vec<ContextTier> {
        match self {
            PacketSize::Small => vec![ContextTier::StateOnly, ContextTier::ContractOnly],
            PacketSize::Medium => vec![
                ContextTier::StateOnly,
                ContextTier::ContractOnly,
                ContextTier::SummaryOnly,
            ],
            PacketSize::Large => vec![
                ContextTier::StateOnly,
                ContextTier::ContractOnly,
                ContextTier::SummaryOnly,
                ContextTier::TargetedSourceSections,
            ],
            PacketSize::Audit => vec![
                ContextTier::StateOnly,
                ContextTier::ContractOnly,
                ContextTier::SummaryOnly,
                ContextTier::TargetedSourceSections,
                ContextTier::FullSourceContext,
            ],
        }
    }
}

impl fmt::Display for PacketSize {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for PacketSize {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "small" => Ok(Self::Small),
            "medium" => Ok(Self::Medium),
            "large" => Ok(Self::Large),
            "audit" => Ok(Self::Audit),
            other => Err(anyhow!(
                "unsupported context packet size '{other}'; expected small, medium, large, or audit"
            )),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ContextTier {
    StateOnly,
    ContractOnly,
    SummaryOnly,
    TargetedSourceSections,
    FullSourceContext,
}

impl ContextTier {
    pub fn as_str(self) -> &'static str {
        match self {
            ContextTier::StateOnly => "tier_0_state_only",
            ContextTier::ContractOnly => "tier_1_contract_only",
            ContextTier::SummaryOnly => "tier_2_summary_only",
            ContextTier::TargetedSourceSections => "tier_3_targeted_source_sections",
            ContextTier::FullSourceContext => "tier_4_full_source_context",
        }
    }
}

impl fmt::Display for ContextTier {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPacketRequest {
    pub fsm_state: String,
    pub size: PacketSize,
    pub task: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextItemReceipt {
    pub path: PathBuf,
    pub tier: ContextTier,
    pub included_as: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPacketReceipt {
    pub packet_id: String,
    pub created_at: String,
    pub current_fsm_state: String,
    pub packet_size: PacketSize,
    pub allowed_context_tiers: Vec<ContextTier>,
    pub included_context: Vec<ContextItemReceipt>,
    pub excluded_context: Vec<String>,
    pub estimated_token_size: usize,
    pub expected_output: Vec<String>,
    pub validation_commands: Vec<String>,
    pub stop_condition: String,
    pub context_state: String,
    pub guardian_state: ContextGuardianState,
    pub recommended_action: ContextRecommendedAction,
    pub blocked: bool,
    pub operator_review_required: bool,
    pub recommended_next_action: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ContextPacketResult {
    pub packet_id: String,
    pub output_dir: PathBuf,
    pub packet_path: PathBuf,
    pub receipt_path: PathBuf,
    pub receipt: ContextPacketReceipt,
}

pub fn generate_context_packet(
    cfg: &Config,
    request: ContextPacketRequest,
) -> Result<ContextPacketResult> {
    let status = context_status::context_status(cfg)?;
    let packet_id = format!("context-packet-{}", Utc::now().timestamp_micros());
    let output_dir = cfg
        .workspace_dir
        .join("state")
        .join("context-packets")
        .join(&packet_id);
    fs::create_dir_all(&output_dir)
        .with_context(|| format!("failed to create {}", output_dir.display()))?;

    let included_context = included_context_for_size(cfg, request.size, &status);
    let excluded_context = excluded_context_for_size(request.size);
    let expected_output = expected_output_for_size(request.size);
    let validation_commands = validation_commands_for_size(request.size);
    let allowed_context_tiers = request.size.allowed_tiers();

    let mut receipt = ContextPacketReceipt {
        packet_id: packet_id.clone(),
        created_at: Utc::now().to_rfc3339(),
        current_fsm_state: request.fsm_state,
        packet_size: request.size,
        allowed_context_tiers,
        included_context,
        excluded_context,
        estimated_token_size: 0,
        expected_output,
        validation_commands,
        stop_condition: "Stop after producing the requested output and recording validation evidence; do not widen context or mutate canonical truth.".to_string(),
        context_state: format!("{:?}", status.context_state).to_ascii_lowercase(),
        guardian_state: status.guardian_state,
        recommended_action: status.recommended_action,
        blocked: status.blocked,
        operator_review_required: status.operator_review_required,
        recommended_next_action: status.recommended_next_action.clone(),
    };

    let packet = render_packet(&receipt, request.task.as_deref(), &status);
    receipt.estimated_token_size = estimate_tokens(&packet);

    let packet_path = output_dir.join("packet.md");
    let receipt_path = output_dir.join("receipt.json");
    fs::write(&packet_path, packet)
        .with_context(|| format!("failed to write {}", packet_path.display()))?;
    fs::write(&receipt_path, serde_json::to_string_pretty(&receipt)?)
        .with_context(|| format!("failed to write {}", receipt_path.display()))?;

    Ok(ContextPacketResult {
        packet_id,
        output_dir,
        packet_path,
        receipt_path,
        receipt,
    })
}

fn included_context_for_size(
    cfg: &Config,
    size: PacketSize,
    status: &ContextStatusReport,
) -> Vec<ContextItemReceipt> {
    let mut items = vec![
        ContextItemReceipt {
            path: PathBuf::from("docs/codex/execution-plan.md"),
            tier: ContextTier::ContractOnly,
            included_as: "contract_summary_reference".to_string(),
            reason: "Current objective, FSM state, validation commands, and stop conditions are required for every packet.".to_string(),
        },
        ContextItemReceipt {
            path: PathBuf::from("docs/fsm/product-state-machines.md"),
            tier: ContextTier::StateOnly,
            included_as: "state_boundary_reference".to_string(),
            reason: "Runtime state boundaries prevent packet work from skipping policy, replay, or approval gates.".to_string(),
        },
    ];

    if matches!(
        size,
        PacketSize::Medium | PacketSize::Large | PacketSize::Audit
    ) {
        items.push(ContextItemReceipt {
            path: PathBuf::from("docs/wiki/MANIFEST.md"),
            tier: ContextTier::SummaryOnly,
            included_as: "context_router_summary".to_string(),
            reason: "The manifest routes agents to summaries before full wiki or source context."
                .to_string(),
        });
        if let Some(path) = status.latest_compact_packet_path.as_ref() {
            items.push(ContextItemReceipt {
                path: display_path(cfg, path),
                tier: ContextTier::SummaryOnly,
                included_as: "compact_truth_packet_summary".to_string(),
                reason: "Latest compact packet summarizes session evidence without rereading full session history.".to_string(),
            });
        }
    }

    if matches!(size, PacketSize::Large | PacketSize::Audit) {
        items.push(ContextItemReceipt {
            path: PathBuf::from("docs/codex/context-firewall.md"),
            tier: ContextTier::TargetedSourceSections,
            included_as: "packet_rules_source".to_string(),
            reason: "Large packets may include targeted source sections for packet rules and acceptance gates.".to_string(),
        });
    }

    if matches!(size, PacketSize::Audit) {
        items.push(ContextItemReceipt {
            path: PathBuf::from("docs/project-spec.md"),
            tier: ContextTier::FullSourceContext,
            included_as: "audit_source_context".to_string(),
            reason: "Audit packets may inspect full project intent when reconstructing or reviewing contract drift.".to_string(),
        });
    }

    items
}

fn excluded_context_for_size(size: PacketSize) -> Vec<String> {
    match size {
        PacketSize::Small => vec![
            "Full wiki files excluded; use manifest or split the task if more context is needed.".to_string(),
            "Full source files excluded; this packet is for state and contract only.".to_string(),
            "Conversation history excluded; use durable session or compact evidence only.".to_string(),
        ],
        PacketSize::Medium => vec![
            "Full wiki files excluded; summaries are enough for this packet.".to_string(),
            "Full source files excluded; request a large packet for targeted source sections.".to_string(),
            "Conversation history excluded; use durable session or compact evidence only.".to_string(),
        ],
        PacketSize::Large => vec![
            "Whole-project scans excluded; targeted source sections only.".to_string(),
            "Tier 4 full source context excluded; use audit packet only for audits, migrations, or reconstruction.".to_string(),
            "Conversation history excluded; use durable session or compact evidence only.".to_string(),
        ],
        PacketSize::Audit => vec![
            "Unbounded agent browsing excluded; broad context must still be justified in this receipt.".to_string(),
            "Implementation authority excluded unless a smaller approved slice follows this audit.".to_string(),
        ],
    }
}

fn expected_output_for_size(size: PacketSize) -> Vec<String> {
    let mut output = vec![
        "what changed or what was inspected".to_string(),
        "files touched or evidence read".to_string(),
        "validation run".to_string(),
        "risks remaining".to_string(),
        "next recommended state".to_string(),
    ];
    if matches!(size, PacketSize::Audit) {
        output.push("audit findings ordered by severity".to_string());
    }
    output
}

fn validation_commands_for_size(size: PacketSize) -> Vec<String> {
    let mut commands = vec!["cargo test context_firewall".to_string()];
    if !matches!(size, PacketSize::Small) {
        commands.push("cargo run -- context-status --json".to_string());
    }
    commands
}

fn render_packet(
    receipt: &ContextPacketReceipt,
    task: Option<&str>,
    status: &ContextStatusReport,
) -> String {
    format!(
        "# Context Packet\n\n\
         ## State\n\n\
         - packet_id: {}\n\
         - current_fsm_state: {}\n\
         - packet_size: {}\n\
         - context_state: {}\n\
         - guardian_state: {}\n\
         - recommended_action: {}\n\
         - blocked: {}\n\
         - latest_session_id: {}\n\
         - recommended_next_action: {}\n\n\
         ## Task\n\n{}\n\n\
         ## Allowed Context\n\n{}\n\n\
         ## Excluded Context\n\n{}\n\n\
         ## Expected Output\n\n{}\n\n\
         ## Validation\n\n{}\n\n\
         ## Stop Condition\n\n{}\n",
        receipt.packet_id,
        receipt.current_fsm_state,
        receipt.packet_size,
        receipt.context_state,
        receipt.guardian_state,
        receipt.recommended_action,
        receipt.blocked,
        status.latest_session_id.as_deref().unwrap_or("none"),
        receipt.recommended_next_action,
        task.unwrap_or(
            "Use this packet to perform the smallest safe next action for the current state."
        ),
        render_included(&receipt.included_context),
        render_bullets(&receipt.excluded_context),
        render_bullets(&receipt.expected_output),
        render_bullets(&receipt.validation_commands),
        receipt.stop_condition,
    )
}

fn render_included(items: &[ContextItemReceipt]) -> String {
    items
        .iter()
        .map(|item| {
            format!(
                "- `{}` ({}) as {}: {}",
                item.path.display(),
                item.tier,
                item.included_as,
                item.reason
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn render_bullets(items: &[String]) -> String {
    if items.is_empty() {
        return "- none".to_string();
    }
    items
        .iter()
        .map(|item| format!("- {item}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn estimate_tokens(text: &str) -> usize {
    text.split_whitespace().count()
}

fn display_path(cfg: &Config, path: &Path) -> PathBuf {
    path.strip_prefix(&cfg.workspace_dir)
        .map(Path::to_path_buf)
        .unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn temp_cfg() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let workspace_dir = tmp.path().join("workspace");
        let mut cfg = Config {
            workspace_dir: workspace_dir.clone(),
            ..Config::default()
        };
        cfg.runtime.session_dir = cfg.workspace_dir.join("state/sessions");
        fs::create_dir_all(&cfg.workspace_dir).expect("workspace");
        for file in ["QUANTM.md", "POLICY.md", "SHIPPABLE.md", "AGENTS.md"] {
            fs::write(cfg.workspace_dir.join(file), format!("# {file}\n")).expect("truth file");
        }
        (tmp, cfg)
    }

    #[test]
    fn small_packet_writes_packet_and_receipt() {
        let (_tmp, cfg) = temp_cfg();

        let result = generate_context_packet(
            &cfg,
            ContextPacketRequest {
                fsm_state: "QUESTION_TO_WORKER_PROPOSAL_01_VALIDATED".to_string(),
                size: PacketSize::Small,
                task: Some("Create the next safe packet.".to_string()),
            },
        )
        .expect("packet");

        assert!(result.packet_path.exists());
        assert!(result.receipt_path.exists());
        assert_eq!(result.receipt.packet_size, PacketSize::Small);
        assert_eq!(
            result.receipt.guardian_state,
            ContextGuardianState::NoSession
        );
        assert_eq!(
            result.receipt.recommended_action,
            ContextRecommendedAction::Observe
        );
        assert!(!result.receipt.blocked);
        assert!(result.receipt.estimated_token_size > 0);
        assert!(
            result
                .receipt
                .allowed_context_tiers
                .contains(&ContextTier::StateOnly)
        );
        assert!(
            !result
                .receipt
                .allowed_context_tiers
                .contains(&ContextTier::FullSourceContext)
        );
    }

    #[test]
    fn audit_packet_is_the_only_packet_with_full_source_tier() {
        let (_tmp, cfg) = temp_cfg();

        let result = generate_context_packet(
            &cfg,
            ContextPacketRequest {
                fsm_state: "AUDIT_CONTEXT".to_string(),
                size: PacketSize::Audit,
                task: None,
            },
        )
        .expect("packet");

        assert!(
            result
                .receipt
                .allowed_context_tiers
                .contains(&ContextTier::FullSourceContext)
        );
        assert!(
            result
                .receipt
                .included_context
                .iter()
                .any(|item| item.path == PathBuf::from("docs/project-spec.md"))
        );
    }

    #[test]
    fn packet_size_parser_rejects_unknown_sizes() {
        let err = "huge".parse::<PacketSize>().expect_err("bad size");
        assert!(err.to_string().contains("unsupported context packet size"));
    }
}
