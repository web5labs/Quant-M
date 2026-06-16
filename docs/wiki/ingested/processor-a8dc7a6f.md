# Ingested Wiki Source: processor.md

## Metadata

- Source path: `wiki/raw/desks/forex/processor.md`
- Ingested at: `2026-05-31T16:18:00.931452+00:00`
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
# Forex Desk Processor

## Purpose
This file defines how the Forex desk ingests StoneX / FOREX.com data and converts it into Quant-M canonical Rust shared state.

The Forex desk is push-first:
- Hot path: live price stream
- Warm path: trading API
- Cold path: scheduled macro / swap refresh

## Sources

### Hot
- StoneX / FOREX.com live prices streaming endpoint
- Used for latest bid/ask updates

### Warm
- StoneX / FOREX.com trading API
- Used for account snapshot, open positions, paper orders, instrument metadata, and any snapshot/recovery calls

### Cold
- swap / financing refresh
- economic calendar refresh
- optional news refresh

## Macro Source
- Source: MQL5 Economic Calendar
- Retrieval method: polling (HTTP POST to calendar content endpoint)
- Use only Medium and High events
- Use only currencies in the Forex desk universe:
  AUD, CAD, CHF, EUR, GBP, JPY, NOK, USD

## Ingestion Rule
Incoming provider payloads are not the working format.

Required flow:
1. Receive raw payload
2. Deserialize into provider-specific Rust struct
3. Map into canonical Rust struct
4. Update in-memory hot state
5. Persist latest signal and handoff to redb

## Provider Payload Requirements
At minimum, the live quote payload must provide:
- symbol
- bid
- ask
- event timestamp if available

If event timestamp is not provided, set event timestamp to local receive time and mark lower confidence in freshness handling.

## Canonical Outputs
This desk must be able to produce:
- QuoteTick
- ForexPayload
- SharedSignal
- DeskHandoff

## Mapping Rules

### Provider Tick -> QuoteTick
- provider symbol -> QuoteTick.symbol
- provider bid -> QuoteTick.bid
- provider ask -> QuoteTick.ask
- provider event timestamp -> QuoteTick.event_ts_ms
- local receive timestamp -> QuoteTick.recv_ts_ms
- freshness = processed_ts - event_ts

### QuoteTick + Desk Context -> ForexPayload
ForexPayload must include:
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

### ForexPayload -> SharedSignal
SharedSignal must always include:
- routing metadata
- freshness metadata
- signal metadata
- risk metadata
- evidence list
- desk_payload

### SharedSignal -> DeskHandoff
DeskHandoff must contain the summarized strategist-readable view:
- thesis
- evidence
- risk flags
- confidence
- recommended action
- routing fields

## Validation Rules
Reject or downgrade invalid data when:
- bid <= 0
- ask <= 0
- ask < bid
- symbol is empty
- payload is malformed
- freshness exceeds desk threshold
- spread is unrealistic

## Symbol Normalization
Normalize symbols into Quant-M standard format:
- USDJPY
- EURUSD
- GBPJPY

Store instrument routing separately if provider requires a different format.

## Recovery Rules
If live stream disconnects:
1. mark stream degraded
2. use warm snapshot path if available
3. keep stale flag true until a fresh stream message arrives
4. do not issue new entry candidate while stale

## Shared Stat

[Truncated by ingest script.]
```
