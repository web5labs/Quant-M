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
