---
page_type: concept
created: 2026-07-10
updated: 2026-07-10
source_count: 1
tags: [adaptive-routing, consensus, trust, borda]
---

# Adaptive Council Routing

Adaptive Council routing spends review budget only when cheaper gates are insufficient.

## Decision Order

1. Validate candidate structure and required evidence.
2. Detect material number, date, unit, polarity, source, and format conflicts.
3. Require an independent blind critic for ordinary reviewed consensus.
4. Expand disagreement to a three-ballot Borda quorum.
5. Return an unchanged representative when review is clean and claim coverage is sufficient.
6. Use constrained editing for bounded repairs.
7. Use Chairman synthesis only for genuine cross-answer synthesis.
8. Abstain when evidence or conflict cannot be resolved.

Semantic agreement is advisory and must not compensate for failed evidence or conflict gates. Model reputation affects lineup construction only. [Raw source](../../raw/2026-07-10-empire-llm-model-routing-optimization.md)

## Trust Components

Useful fields include semantic agreement, evidence support, reviewer consensus, accepted-claim coverage, independent lineage count, material conflicts, unsupported claims, freshness, Chairman use, and final-answer revalidation. A single user-visible probability is specifically out of scope.

## Related Pages

- [[sources/empire-llm-model-routing-optimization]]
- [[syntheses/quant-m-council-shadow-router]]
