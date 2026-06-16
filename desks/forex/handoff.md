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
