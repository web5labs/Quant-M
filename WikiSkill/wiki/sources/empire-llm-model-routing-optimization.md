---
page_type: source
created: 2026-07-10
updated: 2026-07-10
source_count: 1
tags: [empire-llm, council, model-routing, rust, serde]
confidence: high_for_architecture_low_for_uncalibrated_thresholds
---

# Empire-LLM Model Routing Optimization

## Source

`WikiSkill/raw/2026-07-10-empire-llm-model-routing-optimization.md`

## Core Thesis

The safe optimization is an adaptive Council cascade: independent workers, deterministic evidence and conflict gates, an adaptive blind-review quorum, and conditional Chairman synthesis. Embeddings may indicate convergence but cannot establish truth. [Raw source](../../raw/2026-07-10-empire-llm-model-routing-optimization.md)

## Durable Requirements

- Preserve independent drafts and anonymous review.
- Apply eligibility gates before aggregate scoring.
- Keep Borda as the contested/full-audit escalation method.
- Validate complete ballots and reject malformed or partial rankings.
- Use model lineage metadata to construct lineups, not to bias anonymous answer selection.
- Skip Chairman generation only when the reviewed winner is already complete and conflict-free.
- Revalidate any answer changed by an editor or Chairman.
- Represent trust as inspectable components rather than a probability of truth.
- Bound calls, tokens, cost, retries, and deadlines through explicit policy.
- Degrade visibly when embeddings, reviewers, evidence, or providers are unavailable.

## Explicit Non-Goals

The source rejects a universal truth score, learned routing from weak labels, automatic replacement of manual selections, direct return for consequential work based on agreement alone, and an immediate orchestration rewrite. [Raw source](../../raw/2026-07-10-empire-llm-model-routing-optimization.md)

## Adaptation Notes

The source assumes Empire's FastAPI orchestration and recommends a narrow Rust consensus core. Quant-M is already Rust-native, so the equivalent boundary is a provider-free deterministic module behind a shadow CLI. Live provider orchestration, embeddings, and SSE compatibility are not implied by this ingest.

## Related Pages

- [[concepts/adaptive-council-routing]]
- [[syntheses/quant-m-council-shadow-router]]
