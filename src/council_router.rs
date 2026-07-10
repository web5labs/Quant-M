use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const INPUT_SCHEMA: &str = "quant-m/council-shadow-input/v1";
pub const POLICY_SCHEMA: &str = "quant-m/council-policy/v1";

#[derive(Debug, Error)]
pub enum CouncilRouterError {
    #[error("failed to read {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to write {path}: {source}")]
    Write {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("invalid JSON at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("unsupported input schema {0}")]
    UnsupportedInputSchema(String),
    #[error("invalid policy: {0}")]
    InvalidPolicy(String),
    #[error("route {0:?} is missing from policy")]
    MissingRoute(RouteClass),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum RouteClass {
    LowRisk,
    Standard,
    EvidenceCritical,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum MetadataConfidence {
    Verified,
    Curated,
    Inferred,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ChairmanPolicy {
    Rare,
    Conditional,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct RoutePolicy {
    pub initial_workers: u16,
    pub maximum_workers: u16,
    pub initial_auditors: u16,
    pub maximum_auditors: u16,
    pub allow_representative_return_without_audit: bool,
    pub require_independent_critic: bool,
    pub require_evidence_support: bool,
    pub minimum_independent_lineages: u8,
    pub minimum_claim_coverage_bps: u16,
    pub chairman_policy: ChairmanPolicy,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CouncilPolicy {
    pub schema_version: String,
    pub policy_id: String,
    pub routes: BTreeMap<RouteClass, RoutePolicy>,
}

impl CouncilPolicy {
    pub fn validate(&self) -> Result<(), CouncilRouterError> {
        if self.schema_version != POLICY_SCHEMA {
            return Err(CouncilRouterError::InvalidPolicy(format!(
                "unsupported schema {}",
                self.schema_version
            )));
        }
        for route in [
            RouteClass::LowRisk,
            RouteClass::Standard,
            RouteClass::EvidenceCritical,
        ] {
            let policy = self
                .routes
                .get(&route)
                .ok_or(CouncilRouterError::MissingRoute(route))?;
            if policy.initial_workers == 0
                || policy.initial_workers > policy.maximum_workers
                || policy.initial_auditors > policy.maximum_auditors
                || policy.minimum_claim_coverage_bps > 10_000
            {
                return Err(CouncilRouterError::InvalidPolicy(format!(
                    "invalid limits for route {route:?}"
                )));
            }
        }
        Ok(())
    }
}

pub fn default_policy() -> CouncilPolicy {
    CouncilPolicy {
        schema_version: POLICY_SCHEMA.to_string(),
        policy_id: "adaptive-council-shadow-2026-07".to_string(),
        routes: BTreeMap::from([
            (
                RouteClass::LowRisk,
                RoutePolicy {
                    initial_workers: 2,
                    maximum_workers: 3,
                    initial_auditors: 1,
                    maximum_auditors: 3,
                    allow_representative_return_without_audit: false,
                    require_independent_critic: true,
                    require_evidence_support: false,
                    minimum_independent_lineages: 2,
                    minimum_claim_coverage_bps: 9_000,
                    chairman_policy: ChairmanPolicy::Rare,
                },
            ),
            (
                RouteClass::Standard,
                RoutePolicy {
                    initial_workers: 3,
                    maximum_workers: 4,
                    initial_auditors: 1,
                    maximum_auditors: 3,
                    allow_representative_return_without_audit: false,
                    require_independent_critic: true,
                    require_evidence_support: false,
                    minimum_independent_lineages: 2,
                    minimum_claim_coverage_bps: 9_000,
                    chairman_policy: ChairmanPolicy::Conditional,
                },
            ),
            (
                RouteClass::EvidenceCritical,
                RoutePolicy {
                    initial_workers: 3,
                    maximum_workers: 4,
                    initial_auditors: 3,
                    maximum_auditors: 3,
                    allow_representative_return_without_audit: false,
                    require_independent_critic: true,
                    require_evidence_support: true,
                    minimum_independent_lineages: 3,
                    minimum_claim_coverage_bps: 10_000,
                    chairman_policy: ChairmanPolicy::Conditional,
                },
            ),
        ]),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WireCandidate {
    pub candidate_id: String,
    pub answer: String,
    #[serde(default = "default_true")]
    pub complete: bool,
    #[serde(default = "default_true")]
    pub format_compliant: bool,
    #[serde(default = "default_true")]
    pub language_compliant: bool,
    #[serde(default)]
    pub unsupported_material_claims: u16,
    #[serde(default)]
    pub material_conflicts: Vec<String>,
    #[serde(default)]
    pub analyzed_claim_ids: BTreeSet<String>,
    #[serde(default)]
    pub lineage_group: Option<String>,
    #[serde(default)]
    pub lineage_confidence: MetadataConfidence,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    answer: String,
    format_compliant: bool,
    language_compliant: bool,
    unsupported_material_claims: u16,
    material_conflicts: Vec<String>,
    analyzed_claim_ids: BTreeSet<String>,
    lineage_group: Option<String>,
    lineage_confidence: MetadataConfidence,
}

impl TryFrom<WireCandidate> for Candidate {
    type Error = String;

    fn try_from(value: WireCandidate) -> Result<Self, Self::Error> {
        let id = value.candidate_id.trim();
        if id.is_empty() {
            return Err("candidate id is empty".to_string());
        }
        let answer = value.answer.trim();
        if answer.is_empty() {
            return Err(format!("candidate {id} answer is empty"));
        }
        if answer.len() > 1_000_000 {
            return Err(format!("candidate {id} answer exceeds 1,000,000 bytes"));
        }
        if !value.complete {
            return Err(format!("candidate {id} is incomplete or truncated"));
        }
        Ok(Self {
            id: id.to_string(),
            answer: answer.to_string(),
            format_compliant: value.format_compliant,
            language_compliant: value.language_compliant,
            unsupported_material_claims: value.unsupported_material_claims,
            material_conflicts: value.material_conflicts,
            analyzed_claim_ids: value.analyzed_claim_ids,
            lineage_group: value.lineage_group.and_then(non_empty),
            lineage_confidence: value.lineage_confidence,
        })
    }
}

fn non_empty(value: String) -> Option<String> {
    let value = value.trim();
    (!value.is_empty()).then(|| value.to_string())
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuditBallot {
    pub ballot_id: String,
    pub auditor_seat_id: String,
    pub ranking: Vec<String>,
    pub winner: String,
    pub main_objection: String,
    #[serde(default)]
    pub must_fix: Vec<String>,
    #[serde(default)]
    pub material_conflicts: Vec<String>,
    pub valid: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AggregateUsage {
    #[serde(default)]
    pub worker_calls: u16,
    #[serde(default)]
    pub auditor_calls: u16,
    #[serde(default)]
    pub editor_calls: u16,
    #[serde(default)]
    pub embedding_calls: u16,
    #[serde(default)]
    pub prompt_tokens: u64,
    #[serde(default)]
    pub completion_tokens: u64,
    #[serde(default)]
    pub estimated_cost_micros: Option<u64>,
    #[serde(default)]
    pub provider_reported_cost_micros: Option<u64>,
    #[serde(default)]
    pub latency_ms: u64,
    #[serde(default)]
    pub retry_count: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CouncilShadowInput {
    pub schema_version: String,
    pub request_id: String,
    pub route: RouteClass,
    pub candidates: Vec<WireCandidate>,
    #[serde(default)]
    pub deterministic_winner: Option<String>,
    #[serde(default)]
    pub semantic_agreement_bps: Option<u16>,
    #[serde(default)]
    pub embedding_available: bool,
    #[serde(default)]
    pub evidence_required: bool,
    #[serde(default)]
    pub evidence_available: bool,
    #[serde(default)]
    pub freshness_checked: bool,
    #[serde(default)]
    pub accepted_claim_ids: BTreeSet<String>,
    #[serde(default)]
    pub ballots: Vec<AuditBallot>,
    #[serde(default)]
    pub citation_reconciliation_required: bool,
    #[serde(default)]
    pub user_requested_synthesis: bool,
    #[serde(default)]
    pub usage: AggregateUsage,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CouncilAction {
    ExpandWorkerPanel,
    RunBlindCritic,
    ExpandAuditQuorum,
    RepresentativeReturn,
    RunConstrainedEditor,
    RunChairman,
    Abstain,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TrustClassification {
    HighReviewedConsensus,
    ModerateReviewedConsensus,
    LowOrContestedConsensus,
    EvidenceInsufficient,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TrustEvidence {
    pub classification: TrustClassification,
    pub semantic_agreement_bps: Option<u16>,
    pub evidence_support_bps: Option<u16>,
    pub reviewer_consensus_bps: Option<u16>,
    pub accepted_claim_coverage_bps: u16,
    pub independent_lineage_groups: u8,
    pub material_conflicts: u16,
    pub unsupported_material_claims: u16,
    pub freshness_checked: bool,
    pub final_answer_revalidated: bool,
    pub chairman_used: bool,
    pub embedding_available: bool,
    pub policy_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CandidateAssessment {
    pub candidate_id: String,
    pub structurally_eligible: bool,
    pub direct_return_eligible: bool,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BallotAssessment {
    pub ballot_id: String,
    pub valid: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BordaResult {
    pub winner: Option<String>,
    pub tie: bool,
    pub rank_sums: BTreeMap<String, u32>,
    pub top_pick_share_bps: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StagePlan {
    pub stage1_mode: String,
    pub stage2_mode: String,
    pub stage3_mode: String,
    pub stage3_skipped: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CouncilDecisionRecord {
    pub schema_version: String,
    pub request_id: String,
    pub policy_version: String,
    pub route: RouteClass,
    pub action: CouncilAction,
    pub selected_candidate: Option<String>,
    pub reasons: Vec<String>,
    pub candidates: Vec<CandidateAssessment>,
    pub ballots: Vec<BallotAssessment>,
    pub borda: Option<BordaResult>,
    pub trust: TrustEvidence,
    pub stage_plan: StagePlan,
    pub candidate_hashes: BTreeMap<String, String>,
    pub usage: AggregateUsage,
}

pub fn read_shadow_input(path: &Path) -> Result<CouncilShadowInput, CouncilRouterError> {
    let raw = fs::read_to_string(path).map_err(|source| CouncilRouterError::Read {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_str(&raw).map_err(|source| CouncilRouterError::Json {
        path: path.to_path_buf(),
        source,
    })
}

pub fn persist_decision_record(
    workspace: &Path,
    record: &CouncilDecisionRecord,
) -> Result<PathBuf, CouncilRouterError> {
    let dir = workspace.join("state/council-shadow");
    fs::create_dir_all(&dir).map_err(|source| CouncilRouterError::Write {
        path: dir.clone(),
        source,
    })?;
    let path = dir.join(format!("{}.json", filename_safe(&record.request_id)));
    let raw = serde_json::to_string_pretty(record).map_err(|source| CouncilRouterError::Json {
        path: path.clone(),
        source,
    })?;
    fs::write(&path, raw).map_err(|source| CouncilRouterError::Write {
        path: path.clone(),
        source,
    })?;
    Ok(path)
}

pub fn evaluate_shadow(
    input: CouncilShadowInput,
    policy: &CouncilPolicy,
) -> Result<CouncilDecisionRecord, CouncilRouterError> {
    if input.schema_version != INPUT_SCHEMA {
        return Err(CouncilRouterError::UnsupportedInputSchema(
            input.schema_version,
        ));
    }
    policy.validate()?;
    let route_policy = policy
        .routes
        .get(&input.route)
        .ok_or(CouncilRouterError::MissingRoute(input.route))?;

    let mut candidates = Vec::new();
    let mut assessments = Vec::new();
    let mut hashes = BTreeMap::new();
    let mut seen_ids = BTreeSet::new();
    for wire in input.candidates.clone() {
        let fallback_id = wire.candidate_id.trim().to_string();
        match Candidate::try_from(wire) {
            Ok(candidate) if seen_ids.insert(candidate.id.clone()) => {
                let reasons = direct_return_blockers(&candidate);
                hashes.insert(
                    candidate.id.clone(),
                    sha256_hex(candidate.answer.as_bytes()),
                );
                assessments.push(CandidateAssessment {
                    candidate_id: candidate.id.clone(),
                    structurally_eligible: true,
                    direct_return_eligible: reasons.is_empty(),
                    reasons,
                });
                candidates.push(candidate);
            }
            Ok(candidate) => assessments.push(CandidateAssessment {
                candidate_id: candidate.id,
                structurally_eligible: false,
                direct_return_eligible: false,
                reasons: vec!["duplicate candidate id".to_string()],
            }),
            Err(reason) => assessments.push(CandidateAssessment {
                candidate_id: fallback_id,
                structurally_eligible: false,
                direct_return_eligible: false,
                reasons: vec![reason],
            }),
        }
    }

    let eligible_ids = candidates
        .iter()
        .map(|candidate| candidate.id.clone())
        .collect::<BTreeSet<_>>();
    let mut valid_ballots = Vec::new();
    let ballot_assessments = input
        .ballots
        .iter()
        .map(|ballot| match validate_ballot(ballot, &eligible_ids) {
            Ok(()) => {
                valid_ballots.push(ballot.clone());
                BallotAssessment {
                    ballot_id: ballot.ballot_id.clone(),
                    valid: true,
                    reason: "validated".to_string(),
                }
            }
            Err(reason) => BallotAssessment {
                ballot_id: ballot.ballot_id.clone(),
                valid: false,
                reason,
            },
        })
        .collect::<Vec<_>>();

    let independent_lineages = independent_lineage_count(&candidates);
    let evidence_required = input.evidence_required || route_policy.require_evidence_support;
    let deterministic_winner = input
        .deterministic_winner
        .as_deref()
        .filter(|winner| eligible_ids.contains(*winner))
        .map(str::to_string);
    let borda = (valid_ballots.len() >= 3).then(|| aggregate_borda(&valid_ballots, &eligible_ids));
    let audit_winner = borda
        .as_ref()
        .and_then(|result| (!result.tie).then(|| result.winner.clone()).flatten());
    let provisional_winner = audit_winner.or_else(|| deterministic_winner.clone());
    let winner = provisional_winner
        .as_deref()
        .and_then(|id| candidates.iter().find(|candidate| candidate.id == id));
    let claim_coverage_bps = winner
        .map(|candidate| claim_coverage(&candidate.analyzed_claim_ids, &input.accepted_claim_ids))
        .unwrap_or(0);
    let ballot_conflicts = valid_ballots
        .iter()
        .map(|ballot| ballot.material_conflicts.len() as u16)
        .sum::<u16>();
    let winner_conflicts = winner
        .map(|candidate| candidate.material_conflicts.len() as u16)
        .unwrap_or(0);
    let material_conflicts = ballot_conflicts.saturating_add(winner_conflicts);
    let unsupported_claims = winner
        .map(|candidate| candidate.unsupported_material_claims)
        .unwrap_or(0);
    let mut reasons = Vec::new();

    let action = if candidates.len() < 2 {
        reasons.push("fewer than two structurally healthy candidates".to_string());
        CouncilAction::Abstain
    } else if evidence_required && !input.evidence_available {
        reasons.push("required evidence is unavailable".to_string());
        CouncilAction::Abstain
    } else if candidates.len() < usize::from(route_policy.initial_workers) {
        reasons.push(format!(
            "route requires {} initial healthy workers but only {} are available",
            route_policy.initial_workers,
            candidates.len()
        ));
        CouncilAction::ExpandWorkerPanel
    } else if input.route == RouteClass::EvidenceCritical
        && valid_ballots.len() < usize::from(route_policy.maximum_auditors)
    {
        reasons.push("evidence-critical route requires the full audit quorum".to_string());
        CouncilAction::ExpandAuditQuorum
    } else if valid_ballots.is_empty() {
        reasons.push("no valid blind critic ballot is available".to_string());
        CouncilAction::RunBlindCritic
    } else if valid_ballots.len() < 3 {
        let critic = &valid_ballots[0];
        let critic_agrees = deterministic_winner.as_deref() == Some(critic.winner.as_str());
        let critic_clean = ballot_is_clean(critic);
        let winner_direct =
            winner.is_some_and(|candidate| direct_return_blockers(candidate).is_empty());
        let lineage_ok = independent_lineages >= route_policy.minimum_independent_lineages;
        let coverage_ok = claim_coverage_bps >= route_policy.minimum_claim_coverage_bps;
        if valid_ballots.len() == 1
            && critic_agrees
            && critic_clean
            && winner_direct
            && lineage_ok
            && coverage_ok
        {
            reasons.push(
                "blind critic and deterministic winner agree with no material objection"
                    .to_string(),
            );
            CouncilAction::RepresentativeReturn
        } else {
            reasons.push(
                "critic disagreement or a direct-return gate requires two more auditors"
                    .to_string(),
            );
            CouncilAction::ExpandAuditQuorum
        }
    } else if borda.as_ref().is_some_and(|result| result.tie) {
        reasons.push("full audit ended in an explicit tie".to_string());
        CouncilAction::Abstain
    } else if independent_lineages < route_policy.minimum_independent_lineages {
        if candidates.len() < usize::from(route_policy.maximum_workers) {
            reasons.push("full audit lacks the required independent lineage diversity".to_string());
            CouncilAction::ExpandWorkerPanel
        } else {
            reasons
                .push("lineage diversity remains below policy after maximum workers".to_string());
            CouncilAction::Abstain
        }
    } else if material_conflicts > 0 || unsupported_claims > 0 {
        reasons
            .push("material conflicts or unsupported claims remain after full audit".to_string());
        CouncilAction::Abstain
    } else if winner.is_none() {
        reasons.push("no eligible winner could be selected".to_string());
        CouncilAction::Abstain
    } else if input.user_requested_synthesis
        || input.citation_reconciliation_required
        || claim_coverage_bps < route_policy.minimum_claim_coverage_bps
    {
        reasons.push("cross-answer synthesis or citation reconciliation is required".to_string());
        CouncilAction::RunChairman
    } else if winner.is_some_and(|candidate| {
        !candidate.format_compliant
            || !candidate.language_compliant
            || valid_ballots
                .iter()
                .any(|ballot| !ballot.must_fix.is_empty())
    }) {
        reasons.push("bounded deterministic repairs are required".to_string());
        CouncilAction::RunConstrainedEditor
    } else {
        reasons.push("full audit winner is complete and needs no synthesis".to_string());
        CouncilAction::RepresentativeReturn
    };

    let reviewer_consensus_bps = borda
        .as_ref()
        .map(|result| result.top_pick_share_bps)
        .or_else(|| {
            (valid_ballots.len() == 1
                && deterministic_winner.as_deref() == Some(valid_ballots[0].winner.as_str()))
            .then_some(10_000)
        });
    let classification = match action {
        CouncilAction::RepresentativeReturn if valid_ballots.len() >= 3 => {
            TrustClassification::HighReviewedConsensus
        }
        CouncilAction::RepresentativeReturn => TrustClassification::ModerateReviewedConsensus,
        CouncilAction::Abstain if evidence_required && !input.evidence_available => {
            TrustClassification::EvidenceInsufficient
        }
        _ => TrustClassification::LowOrContestedConsensus,
    };
    let selected_candidate = matches!(
        action,
        CouncilAction::RepresentativeReturn
            | CouncilAction::RunConstrainedEditor
            | CouncilAction::RunChairman
    )
    .then(|| winner.map(|candidate| candidate.id.clone()))
    .flatten();

    Ok(CouncilDecisionRecord {
        schema_version: "quant-m/council-decision-record/v1".to_string(),
        request_id: input.request_id,
        policy_version: policy.policy_id.clone(),
        route: input.route,
        action,
        selected_candidate,
        reasons,
        candidates: assessments,
        ballots: ballot_assessments,
        borda,
        trust: TrustEvidence {
            classification,
            semantic_agreement_bps: input.semantic_agreement_bps.map(|value| value.min(10_000)),
            evidence_support_bps: if input.evidence_available {
                Some(10_000)
            } else if evidence_required {
                Some(0)
            } else {
                None
            },
            reviewer_consensus_bps,
            accepted_claim_coverage_bps: claim_coverage_bps,
            independent_lineage_groups: independent_lineages,
            material_conflicts,
            unsupported_material_claims: unsupported_claims,
            freshness_checked: input.freshness_checked,
            final_answer_revalidated: action == CouncilAction::RepresentativeReturn,
            chairman_used: false,
            embedding_available: input.embedding_available,
            policy_version: policy.policy_id.clone(),
        },
        stage_plan: stage_plan(action),
        candidate_hashes: hashes,
        usage: input.usage,
    })
}

fn direct_return_blockers(candidate: &Candidate) -> Vec<String> {
    let mut reasons = Vec::new();
    if !candidate.format_compliant {
        reasons.push("format requirement failed".to_string());
    }
    if !candidate.language_compliant {
        reasons.push("language requirement failed".to_string());
    }
    if candidate.unsupported_material_claims > 0 {
        reasons.push("unsupported material claims exist".to_string());
    }
    if !candidate.material_conflicts.is_empty() {
        reasons.push("material conflicts exist".to_string());
    }
    reasons
}

fn validate_ballot(ballot: &AuditBallot, eligible: &BTreeSet<String>) -> Result<(), String> {
    if !ballot.valid {
        return Err("ballot marked invalid by adapter".to_string());
    }
    if ballot.ballot_id.trim().is_empty() || ballot.auditor_seat_id.trim().is_empty() {
        return Err("ballot or auditor seat id is empty".to_string());
    }
    if ballot.ranking.len() != eligible.len() {
        return Err("ranking does not contain every eligible candidate exactly once".to_string());
    }
    let ranking = ballot.ranking.iter().cloned().collect::<BTreeSet<_>>();
    if ranking.len() != ballot.ranking.len() || &ranking != eligible {
        return Err("ranking has duplicates, omissions, or unknown candidates".to_string());
    }
    if ballot.ranking.first() != Some(&ballot.winner) {
        return Err("declared winner is not ranked first".to_string());
    }
    Ok(())
}

fn aggregate_borda(ballots: &[AuditBallot], eligible: &BTreeSet<String>) -> BordaResult {
    let mut rank_sums = eligible
        .iter()
        .map(|id| (id.clone(), 0_u32))
        .collect::<BTreeMap<_, _>>();
    let mut top_picks = BTreeMap::<String, u32>::new();
    for ballot in ballots {
        for (rank, candidate) in ballot.ranking.iter().enumerate() {
            *rank_sums.entry(candidate.clone()).or_default() += rank as u32;
        }
        *top_picks.entry(ballot.winner.clone()).or_default() += 1;
    }
    let best_sum = rank_sums.values().copied().min();
    let winners = best_sum
        .map(|sum| {
            rank_sums
                .iter()
                .filter_map(|(candidate, value)| (*value == sum).then_some(candidate.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let winner = (winners.len() == 1).then(|| winners[0].clone());
    let top_pick_count = winner
        .as_ref()
        .and_then(|id| top_picks.get(id))
        .copied()
        .unwrap_or(0);
    BordaResult {
        winner,
        tie: winners.len() != 1,
        rank_sums,
        top_pick_share_bps: if ballots.is_empty() {
            0
        } else {
            ((u64::from(top_pick_count) * 10_000) / ballots.len() as u64) as u16
        },
    }
}

fn ballot_is_clean(ballot: &AuditBallot) -> bool {
    let objection = ballot.main_objection.trim().to_ascii_lowercase();
    let objection_is_material = !objection.is_empty()
        && objection != "none"
        && objection != "no material objection"
        && objection != "non-material";
    !objection_is_material && ballot.must_fix.is_empty() && ballot.material_conflicts.is_empty()
}

fn independent_lineage_count(candidates: &[Candidate]) -> u8 {
    candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.lineage_confidence,
                MetadataConfidence::Verified | MetadataConfidence::Curated
            )
        })
        .filter_map(|candidate| candidate.lineage_group.clone())
        .collect::<BTreeSet<_>>()
        .len()
        .min(usize::from(u8::MAX)) as u8
}

fn claim_coverage(winner: &BTreeSet<String>, accepted: &BTreeSet<String>) -> u16 {
    if accepted.is_empty() {
        return 10_000;
    }
    let covered = winner.intersection(accepted).count() as u64;
    ((covered * 10_000) / accepted.len() as u64) as u16
}

fn stage_plan(action: CouncilAction) -> StagePlan {
    let (stage2, stage3, skipped) = match action {
        CouncilAction::ExpandWorkerPanel => ("pending_worker_expansion", "pending", false),
        CouncilAction::RunBlindCritic => ("blind_critic", "pending", false),
        CouncilAction::ExpandAuditQuorum => ("full_audit_borda", "pending", false),
        CouncilAction::RepresentativeReturn => ("review_complete", "representative_return", true),
        CouncilAction::RunConstrainedEditor => ("review_complete", "constrained_editor", false),
        CouncilAction::RunChairman => ("review_complete", "chairman", false),
        CouncilAction::Abstain => ("review_blocked", "abstain", true),
    };
    StagePlan {
        stage1_mode: "independent_candidates".to_string(),
        stage2_mode: stage2.to_string(),
        stage3_mode: stage3.to_string(),
        stage3_skipped: skipped,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn filename_safe(value: &str) -> String {
    let safe = value
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if safe.is_empty() {
        "council-shadow".to_string()
    } else {
        safe
    }
}

pub fn render_decision(record: &CouncilDecisionRecord, persisted: Option<&Path>) -> String {
    format!(
        "Quant-M Council shadow decision\nrequest_id: {}\nroute: {:?}\naction: {:?}\nselected_candidate: {}\ntrust: {:?}\nindependent_lineages: {}\nvalid_ballots: {}\ninvalid_ballots: {}\nstage3_mode: {}\nstage3_skipped: {}\nrecord: {}\nreasons:\n{}\n\nSafety:\n  shadow policy only\n  provider calls: none\n  embeddings do not establish truth\n  execution and approval authority: none\n",
        record.request_id,
        record.route,
        record.action,
        record.selected_candidate.as_deref().unwrap_or("none"),
        record.trust.classification,
        record.trust.independent_lineage_groups,
        record.ballots.iter().filter(|ballot| ballot.valid).count(),
        record.ballots.iter().filter(|ballot| !ballot.valid).count(),
        record.stage_plan.stage3_mode,
        record.stage_plan.stage3_skipped,
        persisted
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "not written".to_string()),
        record
            .reasons
            .iter()
            .map(|reason| format!("  - {reason}"))
            .collect::<Vec<_>>()
            .join("\n"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn candidate(id: &str, lineage: &str) -> WireCandidate {
        WireCandidate {
            candidate_id: id.to_string(),
            answer: format!("Answer from {id}"),
            complete: true,
            format_compliant: true,
            language_compliant: true,
            unsupported_material_claims: 0,
            material_conflicts: Vec::new(),
            analyzed_claim_ids: BTreeSet::from(["claim-1".to_string()]),
            lineage_group: Some(lineage.to_string()),
            lineage_confidence: MetadataConfidence::Curated,
        }
    }

    fn ballot(id: &str, ranking: &[&str]) -> AuditBallot {
        AuditBallot {
            ballot_id: id.to_string(),
            auditor_seat_id: format!("seat-{id}"),
            ranking: ranking.iter().map(|value| value.to_string()).collect(),
            winner: ranking[0].to_string(),
            main_objection: "none".to_string(),
            must_fix: Vec::new(),
            material_conflicts: Vec::new(),
            valid: true,
        }
    }

    fn input(ballots: Vec<AuditBallot>) -> CouncilShadowInput {
        CouncilShadowInput {
            schema_version: INPUT_SCHEMA.to_string(),
            request_id: "request-1".to_string(),
            route: RouteClass::Standard,
            candidates: vec![
                candidate("A", "family-a"),
                candidate("B", "family-b"),
                candidate("C", "family-c"),
            ],
            deterministic_winner: Some("A".to_string()),
            semantic_agreement_bps: Some(9_500),
            embedding_available: true,
            evidence_required: false,
            evidence_available: false,
            freshness_checked: false,
            accepted_claim_ids: BTreeSet::from(["claim-1".to_string()]),
            ballots,
            citation_reconciliation_required: false,
            user_requested_synthesis: false,
            usage: AggregateUsage::default(),
        }
    }

    #[test]
    fn one_clean_critic_can_return_reviewed_representative() {
        let record = evaluate_shadow(
            input(vec![ballot("one", &["A", "B", "C"])]),
            &default_policy(),
        )
        .expect("decision");

        assert_eq!(record.action, CouncilAction::RepresentativeReturn);
        assert_eq!(record.selected_candidate.as_deref(), Some("A"));
        assert!(record.stage_plan.stage3_skipped);
        assert_eq!(
            record.trust.classification,
            TrustClassification::ModerateReviewedConsensus
        );
    }

    #[test]
    fn critic_disagreement_expands_to_full_quorum() {
        let record = evaluate_shadow(
            input(vec![ballot("one", &["B", "A", "C"])]),
            &default_policy(),
        )
        .expect("decision");

        assert_eq!(record.action, CouncilAction::ExpandAuditQuorum);
    }

    #[test]
    fn standard_route_expands_when_initial_worker_floor_is_missing() {
        let mut request = input(vec![ballot("one", &["A", "B"])]);
        request.candidates.pop();
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::ExpandWorkerPanel);
        assert!(record.reasons[0].contains("requires 3 initial healthy workers"));
    }

    #[test]
    fn malformed_ballot_contributes_no_rank_positions() {
        let mut malformed = ballot("bad", &["A", "A", "C"]);
        malformed.winner = "A".to_string();
        let record = evaluate_shadow(input(vec![malformed]), &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::RunBlindCritic);
        assert!(!record.ballots[0].valid);
        assert!(record.borda.is_none());
    }

    #[test]
    fn full_borda_winner_skips_chairman_when_complete() {
        let ballots = vec![
            ballot("one", &["A", "B", "C"]),
            ballot("two", &["A", "C", "B"]),
            ballot("three", &["B", "A", "C"]),
        ];
        let record = evaluate_shadow(input(ballots), &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::RepresentativeReturn);
        assert_eq!(record.selected_candidate.as_deref(), Some("A"));
        assert_eq!(
            record.trust.classification,
            TrustClassification::HighReviewedConsensus
        );
    }

    #[test]
    fn evidence_critical_route_abstains_without_evidence() {
        let mut request = input(vec![]);
        request.route = RouteClass::EvidenceCritical;
        request.evidence_required = true;
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::Abstain);
        assert_eq!(
            record.trust.classification,
            TrustClassification::EvidenceInsufficient
        );
    }

    #[test]
    fn malformed_worker_does_not_abort_healthy_quorum() {
        let mut request = input(vec![ballot("one", &["A", "B", "C"])]);
        request.candidates.push(WireCandidate {
            candidate_id: "D".to_string(),
            answer: String::new(),
            complete: false,
            format_compliant: true,
            language_compliant: true,
            unsupported_material_claims: 0,
            material_conflicts: Vec::new(),
            analyzed_claim_ids: BTreeSet::new(),
            lineage_group: None,
            lineage_confidence: MetadataConfidence::Unknown,
        });
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::RepresentativeReturn);
        assert!(
            record
                .candidates
                .iter()
                .any(|item| { item.candidate_id == "D" && !item.structurally_eligible })
        );
    }

    #[test]
    fn material_conflict_blocks_direct_return() {
        let mut request = input(vec![ballot("one", &["A", "B", "C"])]);
        request.candidates[0].material_conflicts = vec!["4.2M versus 42M".to_string()];
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::ExpandAuditQuorum);
    }

    #[test]
    fn high_semantic_agreement_alone_never_unlocks_direct_return() {
        let mut request = input(vec![]);
        request.semantic_agreement_bps = Some(10_000);
        request.embedding_available = true;
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::RunBlindCritic);
        assert_eq!(record.trust.semantic_agreement_bps, Some(10_000));
    }

    #[test]
    fn unknown_lineage_caps_critic_only_route() {
        let mut request = input(vec![ballot("one", &["A", "B", "C"])]);
        for candidate in &mut request.candidates {
            candidate.lineage_group = None;
            candidate.lineage_confidence = MetadataConfidence::Unknown;
        }
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::ExpandAuditQuorum);
        assert_eq!(record.trust.independent_lineage_groups, 0);
    }

    #[test]
    fn correlated_full_audit_expands_worker_panel_instead_of_claiming_high_trust() {
        let mut request = input(vec![
            ballot("one", &["A", "B", "C"]),
            ballot("two", &["A", "C", "B"]),
            ballot("three", &["B", "A", "C"]),
        ]);
        for candidate in &mut request.candidates {
            candidate.lineage_group = Some("same-family".to_string());
        }
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::ExpandWorkerPanel);
        assert_ne!(
            record.trust.classification,
            TrustClassification::HighReviewedConsensus
        );
    }

    #[test]
    fn borda_tie_is_explicit() {
        let request = input(vec![
            ballot("one", &["A", "B", "C"]),
            ballot("two", &["B", "A", "C"]),
        ]);
        let eligible = BTreeSet::from(["A".to_string(), "B".to_string(), "C".to_string()]);
        let result = aggregate_borda(&request.ballots, &eligible);

        assert!(result.tie);
        assert!(result.winner.is_none());
    }

    #[test]
    fn full_audit_requests_chairman_for_missing_accepted_claims() {
        let mut request = input(vec![
            ballot("one", &["A", "B", "C"]),
            ballot("two", &["A", "C", "B"]),
            ballot("three", &["B", "A", "C"]),
        ]);
        request.accepted_claim_ids.insert("claim-2".to_string());
        let record = evaluate_shadow(request, &default_policy()).expect("decision");

        assert_eq!(record.action, CouncilAction::RunChairman);
        assert!(!record.stage_plan.stage3_skipped);
    }

    #[test]
    fn decision_record_persists_without_candidate_answers() {
        let record = evaluate_shadow(
            input(vec![ballot("one", &["A", "B", "C"])]),
            &default_policy(),
        )
        .expect("decision");
        let tmp = TempDir::new().expect("tempdir");
        let path = persist_decision_record(tmp.path(), &record).expect("persist");
        let raw = fs::read_to_string(path).expect("read");

        assert!(raw.contains("candidate_hashes"));
        assert!(!raw.contains("Answer from A"));
    }
}
