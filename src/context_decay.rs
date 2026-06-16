use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const CANONICAL_AUTHORITY_FLOOR: f64 = 0.75;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextItem {
    pub item_id: String,
    pub source_path: PathBuf,
    pub memory_class: MemoryClass,
    pub freshness_score: f64,
    pub validation_evidence_present: bool,
    pub usage_count: u32,
    pub shippable_relevance_score: f64,
    pub contradiction_count: u32,
    pub compact_packet_stale: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum MemoryClass {
    Ephemeral,
    Tactical,
    Strategic,
    Canonical,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ContextDecayScore {
    pub item_id: String,
    pub source_path: PathBuf,
    pub memory_class: MemoryClass,
    pub authority_score: f64,
    pub freshness_score: f64,
    pub validation_score: f64,
    pub usage_score: f64,
    pub shippable_relevance_score: f64,
    pub contradiction_penalty: f64,
    pub decay_action: DecayAction,
    pub reason: DecayReason,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecayAction {
    Keep,
    Compress,
    Demote,
    Archive,
    Deprecate,
    OperatorReview,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DecayReason {
    CanonicalTruthFile,
    ValidatedRecentContext,
    MissingValidationEvidence,
    StaleCompactPacket,
    LowAuthority,
    ContradictedContext,
    DryRunContextItem,
}

pub fn score_context_item(item: &ContextItem) -> ContextDecayScore {
    let freshness_score = clamp_score(item.freshness_score);
    let validation_score = if item.validation_evidence_present {
        1.0
    } else {
        0.55
    };
    let usage_score = usage_score(item.usage_count);
    let shippable_relevance_score = clamp_score(item.shippable_relevance_score);
    let contradiction_penalty = contradiction_penalty(item.contradiction_count);

    let mut authority_score = freshness_score
        * validation_score
        * usage_score
        * shippable_relevance_score
        * (1.0 - contradiction_penalty);

    if item.compact_packet_stale {
        authority_score *= 0.65;
    }
    if item.memory_class == MemoryClass::Canonical {
        authority_score = authority_score.max(CANONICAL_AUTHORITY_FLOOR);
    }
    authority_score = round_score(authority_score);

    let reason = classify_reason(
        item,
        validation_score,
        contradiction_penalty,
        freshness_score,
        authority_score,
    );
    let decay_action = choose_decay_action(item, authority_score, reason);

    ContextDecayScore {
        item_id: item.item_id.clone(),
        source_path: item.source_path.clone(),
        memory_class: item.memory_class,
        authority_score,
        freshness_score: round_score(freshness_score),
        validation_score: round_score(validation_score),
        usage_score: round_score(usage_score),
        shippable_relevance_score: round_score(shippable_relevance_score),
        contradiction_penalty: round_score(contradiction_penalty),
        decay_action,
        reason,
    }
}

pub fn is_canonical_truth_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name,
                "POLICY.md" | "SHIPPABLE.md" | "QUANTM.md" | "AGENTS.md"
            )
        })
}

fn choose_decay_action(
    item: &ContextItem,
    authority_score: f64,
    reason: DecayReason,
) -> DecayAction {
    if item.memory_class == MemoryClass::Canonical {
        return if reason == DecayReason::ContradictedContext {
            DecayAction::OperatorReview
        } else {
            DecayAction::Keep
        };
    }
    if reason == DecayReason::ContradictedContext {
        return match item.memory_class {
            MemoryClass::Ephemeral | MemoryClass::Tactical => DecayAction::Deprecate,
            MemoryClass::Strategic => DecayAction::OperatorReview,
            MemoryClass::Canonical => DecayAction::OperatorReview,
        };
    }
    if item.compact_packet_stale {
        return match item.memory_class {
            MemoryClass::Ephemeral => DecayAction::Archive,
            MemoryClass::Tactical | MemoryClass::Strategic => DecayAction::Compress,
            MemoryClass::Canonical => DecayAction::Keep,
        };
    }
    match item.memory_class {
        MemoryClass::Ephemeral if authority_score < 0.25 => DecayAction::Archive,
        MemoryClass::Ephemeral if authority_score < 0.45 => DecayAction::Demote,
        MemoryClass::Tactical if authority_score < 0.35 => DecayAction::Archive,
        MemoryClass::Tactical if authority_score < 0.6 => DecayAction::Compress,
        MemoryClass::Strategic if authority_score < 0.45 => DecayAction::OperatorReview,
        MemoryClass::Strategic if authority_score < 0.65 => DecayAction::Compress,
        _ => DecayAction::Keep,
    }
}

fn classify_reason(
    item: &ContextItem,
    validation_score: f64,
    contradiction_penalty: f64,
    freshness_score: f64,
    authority_score: f64,
) -> DecayReason {
    if item.memory_class == MemoryClass::Canonical {
        return if contradiction_penalty > 0.0 {
            DecayReason::ContradictedContext
        } else {
            DecayReason::CanonicalTruthFile
        };
    }
    if contradiction_penalty > 0.0 {
        return DecayReason::ContradictedContext;
    }
    if item.compact_packet_stale {
        return DecayReason::StaleCompactPacket;
    }
    if validation_score < 1.0 {
        return DecayReason::MissingValidationEvidence;
    }
    if freshness_score > 0.8 && authority_score >= 0.7 {
        return DecayReason::ValidatedRecentContext;
    }
    if authority_score < 0.45 {
        return DecayReason::LowAuthority;
    }
    DecayReason::DryRunContextItem
}

fn usage_score(usage_count: u32) -> f64 {
    (0.45 + f64::from(usage_count.min(5)) * 0.11).min(1.0)
}

fn contradiction_penalty(contradiction_count: u32) -> f64 {
    (f64::from(contradiction_count.min(4)) * 0.2).min(0.8)
}

fn clamp_score(value: f64) -> f64 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(0.0, 1.0)
}

fn round_score(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn item(memory_class: MemoryClass) -> ContextItem {
        ContextItem {
            item_id: "item-001".to_string(),
            source_path: PathBuf::from("workspace/state/compacted/session/compact.json"),
            memory_class,
            freshness_score: 1.0,
            validation_evidence_present: true,
            usage_count: 5,
            shippable_relevance_score: 1.0,
            contradiction_count: 0,
            compact_packet_stale: false,
        }
    }

    #[test]
    fn ephemeral_stale_item_decays_faster_than_strategic_item() {
        let mut ephemeral = item(MemoryClass::Ephemeral);
        ephemeral.freshness_score = 0.2;
        ephemeral.validation_evidence_present = false;
        ephemeral.usage_count = 0;

        let mut strategic = ephemeral.clone();
        strategic.memory_class = MemoryClass::Strategic;

        let ephemeral_score = score_context_item(&ephemeral);
        let strategic_score = score_context_item(&strategic);

        assert!(ephemeral_score.authority_score <= strategic_score.authority_score);
        assert!(matches!(
            ephemeral_score.decay_action,
            DecayAction::Archive | DecayAction::Demote
        ));
        assert!(matches!(
            strategic_score.decay_action,
            DecayAction::OperatorReview | DecayAction::Compress
        ));
    }

    #[test]
    fn canonical_truth_files_are_never_auto_deprecated() {
        let mut canonical = item(MemoryClass::Canonical);
        canonical.source_path = PathBuf::from("workspace/POLICY.md");
        canonical.freshness_score = 0.1;
        canonical.validation_evidence_present = false;
        canonical.contradiction_count = 3;

        let score = score_context_item(&canonical);

        assert_ne!(score.decay_action, DecayAction::Deprecate);
        assert_ne!(score.decay_action, DecayAction::Archive);
        assert_eq!(score.decay_action, DecayAction::OperatorReview);
        assert!(score.authority_score >= CANONICAL_AUTHORITY_FLOOR);
    }

    #[test]
    fn missing_validation_lowers_authority_score() {
        let validated = item(MemoryClass::Tactical);
        let mut unvalidated = validated.clone();
        unvalidated.validation_evidence_present = false;

        assert!(
            score_context_item(&unvalidated).authority_score
                < score_context_item(&validated).authority_score
        );
    }

    #[test]
    fn contradiction_penalty_lowers_authority_score() {
        let clean = item(MemoryClass::Tactical);
        let mut contradicted = clean.clone();
        contradicted.contradiction_count = 2;

        assert!(
            score_context_item(&contradicted).authority_score
                < score_context_item(&clean).authority_score
        );
        assert_eq!(
            score_context_item(&contradicted).reason,
            DecayReason::ContradictedContext
        );
    }

    #[test]
    fn validated_recent_compact_packet_gets_keep() {
        let score = score_context_item(&item(MemoryClass::Tactical));

        assert_eq!(score.decay_action, DecayAction::Keep);
        assert_eq!(score.reason, DecayReason::ValidatedRecentContext);
    }

    #[test]
    fn stale_unvalidated_compact_packet_gets_compress_or_operator_review() {
        let mut compact = item(MemoryClass::Strategic);
        compact.validation_evidence_present = false;
        compact.compact_packet_stale = true;

        let score = score_context_item(&compact);

        assert!(matches!(
            score.decay_action,
            DecayAction::Compress | DecayAction::OperatorReview
        ));
        assert_eq!(score.reason, DecayReason::StaleCompactPacket);
    }

    #[test]
    fn deterministic_score_for_same_input() {
        let input = item(MemoryClass::Tactical);

        assert_eq!(score_context_item(&input), score_context_item(&input));
    }
}
