# Ingested Wiki Source: handoff.md

## Metadata

- Source path: `wiki/raw/desks/forex/handoff.md`
- Ingested at: `2026-05-31T16:18:00.931136+00:00`
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
# Forex Desk Handoff

## Current Desk Purpose
Paper-trade only carry rollover trend desk.

## Current Focus
- validate live stream parsing
- validate carry bias wiring
- validate trend agreement logic
- validate routed shared signal shape

## Expected Output
The desk should emit:
- latest shared signal in redb
- latest desk handoff in redb

## Current Reminder
Do not trade against favorable carry direction.
Do not promote stale or macro-blocked signals.
Do not execute directly.
```
