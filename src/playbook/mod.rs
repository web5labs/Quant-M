use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use ring::digest::{SHA256, digest};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

use crate::config::Config;
use crate::desk_registry::DeskId;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookAuthority {
    Observe,
    Analyze,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct KnowledgePackRef {
    pub pack_id: String,
    pub version: String,
    pub hash: String,
    pub required: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedStateRequirement {
    pub source_id: String,
    pub required: bool,
    pub max_age_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ModelTaskKind {
    SummarizeEvidence,
    ExtractFacts,
    DetectContradictions,
    LabelObservation,
    SuggestSharedStateUpdate,
}

impl std::str::FromStr for ModelTaskKind {
    type Err = anyhow::Error;

    fn from_str(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "summarize-evidence" | "summarize_evidence" => Ok(Self::SummarizeEvidence),
            "extract-facts" | "extract_facts" => Ok(Self::ExtractFacts),
            "detect-contradictions" | "detect_contradictions" => Ok(Self::DetectContradictions),
            "label-observation" | "label_observation" => Ok(Self::LabelObservation),
            "suggest-shared-state-update" | "suggest_shared_state_update" => {
                Ok(Self::SuggestSharedStateUpdate)
            }
            other => Err(anyhow!("unsupported model task kind '{other}'")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ForbiddenOutput {
    Trade,
    Bet,
    Execute,
    Approve,
    CanonicalWrite,
    ProviderCredentialRequest,
    IgnoreRisk,
    IgnoreTiming,
    BypassPolicy,
    ProfitGuarantee,
    NetEdgeClaim,
    ArbitrageInstruction,
    PositionSizingInstruction,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CanonicalHash {
    pub algorithm: String,
    pub value: String,
    pub canonicalization_version: String,
}

impl CanonicalHash {
    pub fn sha256(value: String) -> Self {
        Self {
            algorithm: "sha256".to_string(),
            value,
            canonicalization_version: "canonical_json_v1".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PlaybookValidationStatus {
    Valid,
    RejectedForbiddenAuthority,
    RejectedExecutionLanguage,
    RejectedBettingLanguage,
    RejectedTradingLanguage,
    RejectedProviderCredentialLanguage,
    RejectedAmbiguousStrategyLanguage,
    RejectedMissingForbiddenOutputs,
    RejectedMissingKnowledgePack,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DeskPlaybook {
    pub playbook_id: String,
    pub version: String,
    pub desk_id: DeskId,
    pub role_id: String,
    pub authority: PlaybookAuthority,
    pub knowledge_pack_refs: Vec<KnowledgePackRef>,
    pub shared_state_requirements: Vec<SharedStateRequirement>,
    pub allowed_evidence_kinds: Vec<String>,
    pub allowed_model_tasks: Vec<ModelTaskKind>,
    pub forbidden_outputs: Vec<ForbiddenOutput>,
    pub output_schema_id: String,
    pub timing_policy_ref: Option<String>,
    pub risk_profile_ref: Option<String>,
    pub created_at: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaybookBundle {
    pub bundle_id: String,
    pub created_at: String,
    pub playbook: DeskPlaybook,
    pub bundle_hash: String,
    pub replay_safe: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SharedStateSnapshotRef {
    pub snapshot_id: String,
    pub snapshot_hash: String,
    pub created_at: String,
    pub source_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct PlaybookPaths {
    pub root: PathBuf,
    pub desks: PathBuf,
    pub roles: PathBuf,
    pub bundles: PathBuf,
    pub handoffs: PathBuf,
    pub snapshots: PathBuf,
    pub update_proposals: PathBuf,
}

impl PlaybookPaths {
    pub fn new(cfg: &Config) -> Self {
        let root = cfg.workspace_dir.join("playbooks");
        Self {
            desks: root.join("desks"),
            roles: root.join("roles"),
            bundles: root.join("bundles"),
            handoffs: root.join("handoffs"),
            snapshots: cfg.workspace_dir.join("state/shared/snapshots"),
            update_proposals: cfg.workspace_dir.join("state/shared/update-proposals"),
            root,
        }
    }

    pub fn ensure(&self) -> Result<()> {
        for path in [
            &self.root,
            &self.desks,
            &self.roles,
            &self.bundles,
            &self.handoffs,
            &self.snapshots,
            &self.update_proposals,
        ] {
            fs::create_dir_all(path)?;
        }
        Ok(())
    }
}

pub fn default_playbooks() -> Vec<DeskPlaybook> {
    vec![
        playbook(
            "stablecoin_peg_watcher",
            "crypto",
            "stablecoin_peg_watcher",
            vec![
                "stablecoin_peg_watch",
                "venue_risk_language",
                "fee_slippage_language",
            ],
            vec![
                "stablecoin_peg_deviation_observation",
                "evidence_freshness_observation",
            ],
            vec![
                ModelTaskKind::SummarizeEvidence,
                ModelTaskKind::DetectContradictions,
                ModelTaskKind::SuggestSharedStateUpdate,
            ],
        ),
        playbook(
            "forex_calendar_watcher",
            "forex",
            "forex_calendar_watcher",
            vec![
                "positive_carry_language",
                "rollover_language",
                "macro_event_windows",
            ],
            vec![
                "forex_calendar_timing_observation",
                "evidence_freshness_observation",
            ],
            vec![
                ModelTaskKind::SummarizeEvidence,
                ModelTaskKind::ExtractFacts,
                ModelTaskKind::DetectContradictions,
                ModelTaskKind::SuggestSharedStateUpdate,
            ],
        ),
        playbook(
            "sports_scout",
            "sports",
            "sports_scout",
            vec![
                "major_event_scouting",
                "injury_language",
                "line_movement_language",
            ],
            vec![
                "sports_event_slate_observation",
                "evidence_freshness_observation",
            ],
            vec![
                ModelTaskKind::SummarizeEvidence,
                ModelTaskKind::ExtractFacts,
                ModelTaskKind::DetectContradictions,
                ModelTaskKind::SuggestSharedStateUpdate,
            ],
        ),
        playbook(
            "bitcoin_dca_monitor",
            "crypto",
            "bitcoin_dca_monitor",
            vec!["bitcoin_dca", "accumulation_language", "drawdown_zones"],
            vec![
                "bitcoin_dca_schedule_observation",
                "evidence_freshness_observation",
            ],
            vec![
                ModelTaskKind::SummarizeEvidence,
                ModelTaskKind::DetectContradictions,
                ModelTaskKind::SuggestSharedStateUpdate,
            ],
        ),
        playbook(
            "stock_index_session_watcher",
            "stocks_options",
            "stock_index_session_watcher",
            vec!["index_session_language", "macro_event_windows"],
            vec![
                "index_session_observation",
                "evidence_freshness_observation",
            ],
            vec![
                ModelTaskKind::SummarizeEvidence,
                ModelTaskKind::ExtractFacts,
                ModelTaskKind::DetectContradictions,
            ],
        ),
    ]
}

pub fn list_playbooks(_cfg: &Config) -> Result<Vec<DeskPlaybook>> {
    Ok(default_playbooks())
}

pub fn load_playbook(cfg: &Config, desk: &str, role: &str) -> Result<DeskPlaybook> {
    list_playbooks(cfg)?
        .into_iter()
        .find(|playbook| playbook.desk_id.as_str() == desk && playbook.role_id == role)
        .ok_or_else(|| anyhow!("playbook not found for desk '{desk}' role '{role}'"))
}

pub fn load_playbook_by_id(cfg: &Config, playbook_id: &str) -> Result<DeskPlaybook> {
    list_playbooks(cfg)?
        .into_iter()
        .find(|playbook| playbook.playbook_id == playbook_id)
        .ok_or_else(|| anyhow!("playbook '{playbook_id}' not found"))
}

pub fn validate_playbook(playbook: &DeskPlaybook) -> Result<()> {
    let status = validate_playbook_status(playbook)?;
    if status != PlaybookValidationStatus::Valid {
        return Err(anyhow!("playbook rejected: {status:?}"));
    }
    Ok(())
}

pub fn validate_playbook_status(playbook: &DeskPlaybook) -> Result<PlaybookValidationStatus> {
    if playbook.playbook_id.trim().is_empty() {
        return Ok(PlaybookValidationStatus::RejectedForbiddenAuthority);
    }
    if !matches!(
        playbook.authority,
        PlaybookAuthority::Observe | PlaybookAuthority::Analyze
    ) {
        return Ok(PlaybookValidationStatus::RejectedForbiddenAuthority);
    }
    if playbook.knowledge_pack_refs.iter().any(|pack| {
        pack.required && (pack.pack_id.trim().is_empty() || pack.hash.trim().is_empty())
    }) {
        return Ok(PlaybookValidationStatus::RejectedMissingKnowledgePack);
    }
    if playbook.forbidden_outputs.is_empty() {
        return Ok(PlaybookValidationStatus::RejectedMissingForbiddenOutputs);
    }
    if !playbook.forbidden_outputs.contains(&ForbiddenOutput::Trade)
        || !playbook.forbidden_outputs.contains(&ForbiddenOutput::Bet)
        || !playbook
            .forbidden_outputs
            .contains(&ForbiddenOutput::CanonicalWrite)
    {
        return Ok(PlaybookValidationStatus::RejectedMissingForbiddenOutputs);
    }
    if let Some(status) =
        rejected_language_status(&canonical_json(&playbook_language_surface(playbook))?)
    {
        return Ok(status);
    }
    if stable_hash(&canonical_playbook_json(playbook)?) != playbook.hash {
        return Err(anyhow!("playbook hash mismatch"));
    }
    Ok(PlaybookValidationStatus::Valid)
}

pub fn bundle_playbook(cfg: &Config, desk: &str, role: &str) -> Result<PlaybookBundle> {
    let paths = PlaybookPaths::new(cfg);
    paths.ensure()?;
    let playbook = load_playbook(cfg, desk, role)?;
    validate_playbook(&playbook)?;
    let bundle_hash = canonical_hash(&playbook)?.value;
    let bundle = PlaybookBundle {
        bundle_id: format!("{}-bundle", playbook.playbook_id),
        created_at: Utc::now().to_rfc3339(),
        playbook,
        bundle_hash,
        replay_safe: true,
    };
    fs::write(
        paths
            .bundles
            .join(format!("{}.bundle.json", bundle.playbook.playbook_id)),
        serde_json::to_string_pretty(&bundle)?,
    )?;
    Ok(bundle)
}

pub fn latest_snapshot(cfg: &Config) -> Result<SharedStateSnapshotRef> {
    let paths = PlaybookPaths::new(cfg);
    paths.ensure()?;
    let snapshot = SharedStateSnapshotRef {
        snapshot_id: "latest".to_string(),
        snapshot_hash: stable_hash("quantm-shared-state-latest-empty-snapshot-v1"),
        created_at: Utc::now().to_rfc3339(),
        source_refs: vec!["shared-state-local".to_string()],
    };
    fs::write(
        paths.snapshots.join("latest.json"),
        serde_json::to_string_pretty(&snapshot)?,
    )?;
    Ok(snapshot)
}

pub fn stable_hash(value: &str) -> String {
    hex_lower(digest(&SHA256, value.as_bytes()).as_ref())
}

pub fn canonical_hash(value: &impl Serialize) -> Result<CanonicalHash> {
    Ok(CanonicalHash::sha256(stable_hash(&canonical_json(value)?)))
}

pub fn canonical_json(value: &impl Serialize) -> Result<String> {
    let value = serde_json::to_value(value).context("serialize value for canonical json")?;
    let canonical = canonicalize_value(value);
    serde_json::to_string(&canonical).context("serialize canonical json")
}

fn canonicalize_value(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Array(values) => {
            serde_json::Value::Array(values.into_iter().map(canonicalize_value).collect())
        }
        serde_json::Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut sorted = serde_json::Map::new();
            for key in keys {
                if let Some(value) = map.get(&key) {
                    sorted.insert(key, canonicalize_value(value.clone()));
                }
            }
            serde_json::Value::Object(sorted)
        }
        other => other,
    }
}

fn playbook(
    playbook_id: &str,
    desk: &str,
    role: &str,
    packs: Vec<&str>,
    evidence_kinds: Vec<&str>,
    tasks: Vec<ModelTaskKind>,
) -> DeskPlaybook {
    let mut playbook = DeskPlaybook {
        playbook_id: playbook_id.to_string(),
        version: "1.0.0".to_string(),
        desk_id: DeskId::new(desk),
        role_id: role.to_string(),
        authority: PlaybookAuthority::Observe,
        knowledge_pack_refs: packs
            .into_iter()
            .map(|pack_id| KnowledgePackRef {
                pack_id: pack_id.to_string(),
                version: "1.0.0".to_string(),
                hash: stable_hash(&format!("knowledge-pack:{pack_id}:1.0.0")),
                required: true,
            })
            .collect(),
        shared_state_requirements: vec![SharedStateRequirement {
            source_id: "shared-state-latest".to_string(),
            required: true,
            max_age_seconds: 3600,
        }],
        allowed_evidence_kinds: evidence_kinds.into_iter().map(str::to_string).collect(),
        allowed_model_tasks: tasks,
        forbidden_outputs: all_forbidden_outputs(),
        output_schema_id: "quantm.model_output.v1".to_string(),
        timing_policy_ref: Some(format!("{desk}:{role}")),
        risk_profile_ref: None,
        created_at: "2026-01-01T00:00:00Z".to_string(),
        hash: String::new(),
    };
    playbook.hash = stable_hash(&canonical_playbook_json(&playbook).expect("static playbook json"));
    playbook
}

fn all_forbidden_outputs() -> Vec<ForbiddenOutput> {
    vec![
        ForbiddenOutput::Trade,
        ForbiddenOutput::Bet,
        ForbiddenOutput::Execute,
        ForbiddenOutput::Approve,
        ForbiddenOutput::CanonicalWrite,
        ForbiddenOutput::ProviderCredentialRequest,
        ForbiddenOutput::IgnoreRisk,
        ForbiddenOutput::IgnoreTiming,
        ForbiddenOutput::BypassPolicy,
        ForbiddenOutput::ProfitGuarantee,
        ForbiddenOutput::NetEdgeClaim,
        ForbiddenOutput::ArbitrageInstruction,
        ForbiddenOutput::PositionSizingInstruction,
    ]
}

fn canonical_playbook_json(playbook: &DeskPlaybook) -> Result<String> {
    let mut clone = playbook.clone();
    clone.hash.clear();
    canonical_json(&clone).context("serialize canonical playbook")
}

#[derive(Serialize)]
struct PlaybookLanguageSurface<'a> {
    playbook_id: &'a str,
    role_id: &'a str,
    knowledge_pack_refs: &'a [KnowledgePackRef],
    shared_state_requirements: &'a [SharedStateRequirement],
    allowed_evidence_kinds: &'a [String],
    allowed_model_tasks: &'a [ModelTaskKind],
    output_schema_id: &'a str,
    timing_policy_ref: &'a Option<String>,
    risk_profile_ref: &'a Option<String>,
}

fn playbook_language_surface(playbook: &DeskPlaybook) -> PlaybookLanguageSurface<'_> {
    PlaybookLanguageSurface {
        playbook_id: &playbook.playbook_id,
        role_id: &playbook.role_id,
        knowledge_pack_refs: &playbook.knowledge_pack_refs,
        shared_state_requirements: &playbook.shared_state_requirements,
        allowed_evidence_kinds: &playbook.allowed_evidence_kinds,
        allowed_model_tasks: &playbook.allowed_model_tasks,
        output_schema_id: &playbook.output_schema_id,
        timing_policy_ref: &playbook.timing_policy_ref,
        risk_profile_ref: &playbook.risk_profile_ref,
    }
}

fn rejected_language_status(raw: &str) -> Option<PlaybookValidationStatus> {
    let lower = raw.to_ascii_lowercase();
    let forbidden: &[(PlaybookValidationStatus, &[&str])] = &[
        (
            PlaybookValidationStatus::RejectedTradingLanguage,
            &[
                "trade now",
                "recommend entry",
                "recommend exit",
                "increase lot",
                "allocation should be increased",
                "all in",
                "martingale",
                "recover losses",
            ],
        ),
        (
            PlaybookValidationStatus::RejectedBettingLanguage,
            &[
                "place bet",
                "sure win",
                "guaranteed pick",
                "lock pick",
                "bet now",
                "recover losses",
                "all in",
                "martingale",
            ],
        ),
        (
            PlaybookValidationStatus::RejectedExecutionLanguage,
            &[
                "execute",
                "approve proposal",
                "canonical write",
                "withdraw funds",
                "transfer funds",
            ],
        ),
        (
            PlaybookValidationStatus::RejectedProviderCredentialLanguage,
            &[
                "provider credential",
                "api key",
                "secret key",
                "provider token",
            ],
        ),
        (
            PlaybookValidationStatus::RejectedAmbiguousStrategyLanguage,
            &[
                "ignore risk",
                "ignore timing",
                "bypass policy",
                "guaranteed profit",
                "profitable opportunities",
                "best entries",
            ],
        ),
    ];
    for (status, phrases) in forbidden {
        if phrases.iter().any(|phrase| lower.contains(phrase)) {
            return Some(status.clone());
        }
    }
    None
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

pub fn write_json(path: &Path, value: &impl Serialize) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_string_pretty(value)?)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use tempfile::TempDir;

    fn test_config() -> (TempDir, Config) {
        let tmp = TempDir::new().expect("tempdir");
        let cfg = Config {
            workspace_dir: tmp.path().join("workspace"),
            ..Config::default()
        };
        (tmp, cfg)
    }

    #[test]
    fn playbook_loads_valid_stablecoin_role() {
        let (_tmp, cfg) = test_config();
        let playbook = load_playbook(&cfg, "crypto", "stablecoin_peg_watcher").expect("playbook");
        validate_playbook(&playbook).expect("valid");
        assert_eq!(playbook.playbook_id, "stablecoin_peg_watcher");
        assert!(playbook.forbidden_outputs.contains(&ForbiddenOutput::Trade));
    }

    #[test]
    fn playbook_rejects_missing_knowledge_pack() {
        let (_tmp, cfg) = test_config();
        let mut playbook =
            load_playbook(&cfg, "crypto", "stablecoin_peg_watcher").expect("playbook");
        playbook.knowledge_pack_refs[0].hash.clear();
        playbook.hash = stable_hash(&canonical_playbook_json(&playbook).expect("json"));
        let status = validate_playbook_status(&playbook).expect("status");
        assert_eq!(
            status,
            PlaybookValidationStatus::RejectedMissingKnowledgePack
        );
    }

    #[test]
    fn playbook_bundle_hash_is_stable() {
        let (_tmp, cfg) = test_config();
        let first = bundle_playbook(&cfg, "crypto", "stablecoin_peg_watcher").expect("bundle");
        let second = bundle_playbook(&cfg, "crypto", "stablecoin_peg_watcher").expect("bundle");
        assert_eq!(first.bundle_hash, second.bundle_hash);
    }

    #[test]
    fn canonical_hash_is_stable_across_field_order() {
        let left = serde_json::json!({"b": 2, "a": {"d": 4, "c": 3}});
        let right = serde_json::json!({"a": {"c": 3, "d": 4}, "b": 2});
        assert_eq!(
            canonical_hash(&left).expect("left").value,
            canonical_hash(&right).expect("right").value
        );
    }

    #[test]
    fn playbook_rejects_trading_language() {
        let (_tmp, cfg) = test_config();
        let mut playbook =
            load_playbook(&cfg, "crypto", "stablecoin_peg_watcher").expect("playbook");
        playbook
            .allowed_evidence_kinds
            .push("trade now".to_string());
        playbook.hash = stable_hash(&canonical_playbook_json(&playbook).expect("json"));
        let status = validate_playbook_status(&playbook).expect("status");
        assert_eq!(status, PlaybookValidationStatus::RejectedTradingLanguage);
    }

    #[test]
    fn playbook_rejects_provider_credential_language() {
        let (_tmp, cfg) = test_config();
        let mut playbook =
            load_playbook(&cfg, "crypto", "stablecoin_peg_watcher").expect("playbook");
        playbook.allowed_evidence_kinds.push("api key".to_string());
        playbook.hash = stable_hash(&canonical_playbook_json(&playbook).expect("json"));
        let status = validate_playbook_status(&playbook).expect("status");
        assert_eq!(
            status,
            PlaybookValidationStatus::RejectedProviderCredentialLanguage
        );
    }
}
