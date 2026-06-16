# Ingested Wiki Source: skills.md

## Metadata

- Source path: `wiki/raw/desks/forex/skills.md`
- Ingested at: `2026-05-31T16:18:00.932477+00:00`
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
# Forex Desk Skills

## Desk Identity
This desk studies foreign exchange markets using a strict baseline:
- only consider paper trades in the direction of favorable carry / positive swap bias
- require trend agreement
- avoid low-quality or stale signals
- never execute directly

## Active Pair Baseline (March 30, 2026)
Positive swap long pairs:
- AUD/CHF
- AUD/JPY
- GBP/JPY
- USD/CHF
- USD/JPY

Positive swap short pairs:
- EUR/AUD
- EUR/GBP
- EUR/NOK
- EUR/USD

No positive side (carry-blocked):
- EUR/CAD

## Primary Strategy
Carry rollover trend strategy.

### Core principle
Do not fight the positive swap direction.

If the pair favors long carry:
- only consider long opportunities

If the pair favors short carry:
- only consider short opportunities

If carry is neutral or unknown:
- prefer no trade

## What the desk should pay attention to

### 1. Carry / Swap Bias
This is the first directional filter.
Classify:
- LongFavorable
- ShortFavorable
- Neutral
- Unknown

### 2. Trend Direction
Use simple trend agreement first:
- M15
- H1
- H4

Prefer:
- H1 aligned with carry direction
- H4 aligned or at least not strongly opposing
- M15 for short-term timing context only

### 3. Spread Quality
Wide spread reduces confidence.
Abnormally wide spread is a risk flag.

### 4. Macro Event Risk
The desk must avoid fresh entry candidates too close to a high-impact event.

### 5. Freshness
If live quote data is stale, confidence must drop sharply.
Stale signals should not become entry candidates.

## Entry Candidate Logic

### Long candidate
Requires:
- carry bias = LongFavorable
- H1 trend = Up
- H4 trend = Up or Sideways
- spread acceptable
- no high-impact event blackout
- data not stale

### Short candidate
Requires:
- carry bias = ShortFavorable
- H1 trend = Down
- H4 trend = Down or Sideways
- spread acceptable
- no high-impact event blackout
- data not stale

### Entry style constraints
- Long-bias candidates: buy_limit or buy_stop
- Short-bias candidates: sell_limit or sell_stop

## Recommended Action Vocabulary
Use only these actions in v1:
- no_trade
- hold
- long_bias_hold
- short_bias_hold
- long_bias_entry_candidate
- short_bias_entry_candidate

## Evidence Quality
Good evidence:
- H1 above MA50 in long carry regime
- H1 below MA50 in short carry regime
- H4 agreement
- spread normal
- no immediate macro risk

Weak evidence:
- M15 only
- stale quote
- unknown carry
- contradictory timeframes

## Risk Flags
Common risk flags:
- spread_elevated
- stale_feed
- macro_event_near
- carry_unknown
- trend_conflict
- weak_alignment

## What this desk must never do
- trade against favorable carry direction
- treat stale data as actionable
- ignore macro blackout rules
- call execution directly
- invent routing metadata
```
