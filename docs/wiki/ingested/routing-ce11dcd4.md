# Ingested Wiki Source: routing.md

## Metadata

- Source path: `wiki/raw/desks/forex/routing.md`
- Ingested at: `2026-05-31T16:18:00.931764+00:00`
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
# Forex Desk Routing

## Desk
forex

## Source Venue
forex_com

## Source Channels
### Hot
- live_prices_stream

### Warm
- trading_api

### Cold
- macro_refresh
- swap_refresh

## Execution Adapter
stonex_forex_exec

## Account Scope
paper_primary

## Market Type
spot_fx

## Paper Mode
true

## Routing Contract
Every Forex signal must carry:

- desk = forex
- source_venue = forex_com
- source_channel = live_prices_stream or trading_api
- execution_adapter = stonex_forex_exec
- account_scope = paper_primary
- market_type = spot_fx
- paper_trade_only = true

## Active Pair Set
Only this 10-pair universe is routed by this desk:
- AUDCHF
- AUDJPY
- EURAUD
- EURCAD
- EURGBP
- EURNOK
- EURUSD
- GBPJPY
- USDCHF
- USDJPY

Any other symbol is ignored/rejected by policy.

## Symbol Conventions
Quant-M symbol format:
- USDJPY
- EURUSD
- GBPJPY
- AUDJPY
- USDCHF

Provider-specific instrument formats may differ.
The adapter is responsible for mapping provider instrument identifiers into Quant-M symbols and vice versa.

## Execution Responsibility
The Forex desk never executes directly.

The desk only produces a routed signal that tells the strategist:
- what market the signal belongs to
- which execution adapter would be used
- which account scope applies

Only the strategist may approve a paper order request.

## redb Keys
Suggested keys:
- latest_signal:forex:USDJPY
- handoff:forex:USDJPY
- account:forex:paper_primary

## Failure Rules
If routing metadata is missing or inconsistent:
- do not promote signal
- mark policy failure
- downgrade to no_trade
```
