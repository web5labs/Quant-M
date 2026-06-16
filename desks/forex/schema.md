# Forex Desk Schema

## Canonical Shared Signal Envelope

Required top-level sections:
- signal_id
- routing
- freshness
- signal
- risk
- evidence
- desk_payload

## routing
Required fields:
- desk
- source_venue
- source_channel
- execution_adapter
- account_scope
- market_type
- instrument_id
- symbol
- paper_trade_only

## freshness
Required fields:
- event_ts_ms
- recv_ts_ms
- processed_ts_ms
- freshness_ms
- stale
- quality

## signal
Required fields:
- signal_type
- confidence
- thesis
- recommended_action
- requires_review

## risk
Required fields:
- risk_flags
- blockers
- risk_score
- exposure_ok
- policy_ok

## evidence
A short list of strings.
Keep concise.

## desk_payload
Forex desk payload must include:
- bid
- ask
- spread_bps
- carry_bias
- swap_long
- swap_short
- trend_m15
- trend_h1
- trend_h4
- ma20
- ma50
- next_high_impact_minutes

## Desk Handoff
Required fields:
- signal_id
- desk
- source_venue
- execution_adapter
- account_scope
- symbol
- thesis
- evidence
- risk_flags
- confidence
- recommended_action
- created_at_ms

## PairMacroState
Required fields:
- pair
- next_medium_event_minutes
- next_high_event_minutes
- macro_blackout
- upcoming_events
- updated_at_ms

## MacroEvent
Required fields:
- event_id
- source
- currency
- impact
- title
- scheduled_at_ms
- actual
- forecast
- previous
- affected_pairs

## Value Guidance

### carry_bias
Allowed values:
- LongFavorable
- ShortFavorable
- Neutral
- Unknown

### trend_*
Allowed values:
- Up
- Down
- Sideways
- Unknown

### signal_type
Allowed values:
- no_trade
- hold
- long_bias_hold
- short_bias_hold
- long_bias_entry_candidate
- short_bias_entry_candidate

### quality
Allowed values:
- Hot
- Warm
- Cold

## Example Evidence
- Carry favors long
- H1 trend up
- H4 supportive
- Spread within normal range
- No high-impact event within 30 minutes

## Example Risk Flags
- spread_elevated
- stale_feed
- macro_event_near
- carry_unknown
- trend_conflict
