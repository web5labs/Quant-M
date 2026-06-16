# Ingested Wiki Source: risk_policy.md

## Metadata

- Source path: `wiki/raw/project/risk_policy.md`
- Ingested at: `2026-05-31T16:18:00.933514+00:00`
- Source extension: `.md`

## Agent summary

_TBD: Summarize the source in 5-10 bullets._

## Key facts

_TBD_

## Implementation relevance

_TBD: Explain how this source affects the project spec, architecture, data model, API plan, or UI/UX handoff._

## Risks / constraints

_TBD_

## Open questions

_TBD_

## Source excerpt

```text
# Strategist Risk Policy

## Forex Desk
The strategist may only consider Forex desk signals that:
- are routed to `stonex_forex_exec`
- belong to `paper_primary`
- are fresh
- pass desk policy
- contain complete routing metadata

Any Forex signal missing routing metadata must be rejected.

Any Forex signal marked stale must not become a fresh paper entry.

Any Forex signal that conflicts with carry direction must be rejected.
```
