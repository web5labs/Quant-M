---
page_type: synthesis
created: 2026-07-10
updated: 2026-07-10
source_count: 1
tags: [quant-m, implementation, shadow-mode, council]
status: implemented_shadow_slice
---

# Quant-M Council Shadow Router

## Accepted Slice

Quant-M will implement a deterministic shadow router that consumes analyzed candidates and anonymous audit ballots. It will not call models or embeddings. The router will expose:

- versioned route policy;
- wire-to-domain candidate validation;
- fail-closed anonymous ballot validation;
- deterministic Borda aggregation with explicit ties;
- adaptive critic/quorum decisions;
- conditional representative, constrained-editor, and Chairman decisions;
- component trust evidence;
- bounded usage metadata and replayable decision records.

This matches the source's recommendation to validate policy in shadow mode before allowing cheaper routes to alter delivered answers. [Raw source](../../raw/2026-07-10-empire-llm-model-routing-optimization.md)

## Safety Boundary

- No provider or embedding calls.
- No automatic model substitution.
- No execution or approval authority.
- No claim that semantic convergence establishes truth.
- No calibrated cost or accuracy claim yet.

## Remaining Milestones

1. Collect real shadow records.
2. Compare decisions with accepted Council outcomes.
3. Calibrate thresholds by route and language.
4. Add route-specific validators.
5. Enable adaptive calls only after precision evidence is adequate.

## Related Pages

- [[sources/empire-llm-model-routing-optimization]]
- [[concepts/adaptive-council-routing]]
