# Ingested Wiki Source: carry_map.md

## Metadata

- Source path: `wiki/raw/desks/forex/carry_map.md`
- Ingested at: `2026-05-31T16:18:00.929216+00:00`
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
# Forex Carry Map

Baseline reference date: March 30, 2026.

## Positive Swap Long
- AUDCHF
- AUDJPY
- GBPJPY
- USDCHF
- USDJPY

## Positive Swap Short
- EURAUD
- EURGBP
- EURNOK
- EURUSD

## Carry-Blocked (No Positive Side)
- EURCAD

## Notes
- This map is policy input for desk direction bias.
- Runtime must reject symbols outside the 10-pair universe.
- If provider swap data conflicts with this map, downgrade to `no_trade`.
```
