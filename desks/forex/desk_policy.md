# Forex Desk Policy

## Mode
Paper trading only.

No live-money execution is allowed from this desk.

## Authority
The Forex desk is read-only with respect to market analysis.
Only the Quant Risk Manager / Strategist may approve a paper order through the execution adapter.

## Hard Rules

### Rule 0: Pair Universe (Baseline as of March 30, 2026)
Only these symbols are enabled in v1:
- AUDCHF (long-bias only)
- AUDJPY (long-bias only)
- EURAUD (short-bias only)
- EURCAD (carry-blocked; no carry entry)
- EURGBP (short-bias only)
- EURNOK (short-bias only)
- EURUSD (short-bias only)
- GBPJPY (long-bias only)
- USDCHF (long-bias only)
- USDJPY (long-bias only)

Any symbol outside this set must be rejected at ingest.

### Rule 1: Carry First
No directional candidate may be issued against the favorable swap direction.

If carry bias is:
- LongFavorable -> no short candidate allowed
- ShortFavorable -> no long candidate allowed
- Neutral or Unknown -> default to no_trade unless explicitly overridden by future policy

### Rule 2: Trend Confirmation
No entry candidate unless H1 agrees with carry direction.

### Rule 3: H4 Check
H4 should support or at least not strongly oppose the carry direction.

### Rule 4: Macro Blackout
No fresh entry candidate within the defined high-impact event blackout window.

Suggested default:
- 30 minutes before event
- 15 minutes after event

This can be tuned later.

Medium-impact policy:
- reduce confidence within 30 minutes before event
- optionally block by future tuning

Macro applies when event currency matches either side of the pair.

### Rule 5: Stale Data Block
If freshness exceeds desk threshold, block fresh entry signals.

Suggested starting threshold:
- stale if quote freshness > 5000 ms

### Rule 6: Spread Guard
If spread exceeds normal desk threshold, downgrade or block signal.

Suggested starting rule:
- flag if spread meaningfully exceeds recent baseline
- block only if clearly abnormal

### Rule 7: Routing Required
Every signal must include:
- desk = forex
- source_venue = forex_com
- execution_adapter = stonex_forex_exec
- account_scope = paper_primary
- paper_trade_only = true

### Rule 8: Minimal Action Set
Allowed actions:
- no_trade
- hold
- long_bias_hold
- short_bias_hold
- long_bias_entry_candidate
- short_bias_entry_candidate

No other action strings allowed in v1.

### Rule 9: Entry Style Guard
Entry style hints are constrained by carry direction:
- long-bias pairs: buy_limit or buy_stop only
- short-bias pairs: sell_limit or sell_stop only

No market-order execution is allowed from this desk in v1.

### Rule 10: Daily Swap Health Refresh
Swap direction can change and must be refreshed from provider data daily.

Operational baseline:
- run `swap_health` after rollover processing
- use a post-5:00pm ET buffer (suggested: 5:10pm ET)
- if refreshed direction is no longer positive for the current side, downgrade to no_trade
- if both sides are non-positive, block carry entry for that pair

## Confidence Policy
Confidence must remain strict.

Raise confidence for:
- carry + H1 agreement
- H4 support
- acceptable spread
- fresh feed
- no macro event nearby

Reduce confidence for:
- spread widening
- stale feed
- trend conflict
- missing carry data
- event proximity

## Escalation Policy
If any blocker exists, recommended action must become:
- no_trade
or
- hold

## Notes
This file is watched via mtime/file-watch.
Changes should invalidate cached policy state and reload desk rules.
