# Security Boundaries

This document describes the `v0.local-alpha` edge-cluster boundary.

## Core Principle

Child devices may collect and return bounded evidence. The core owns validation, shared state, policy, FSM authority, and governance.

## Pairing

Pairing enrolls a known device. It does not assign a role lease, approve proposals, validate compute, call providers, execute trades or bets, write canonical shared state, or create scheduling authority.

## Heartbeat

Heartbeat makes a child visible as online or stale. It does not create leases, extend leases, change authority, or mark evidence as trusted.

LAN heartbeat ingest requires the paired child heartbeat auth token issued after operator approval. A node id alone is not enough to mark a child online.

## Lease

A lease is temporary, explicit, bounded permission. Local alpha leases are observe-only. A child cannot create, modify, extend, or revoke its own lease.

Expired or revoked leases block work eligibility.

## Telemetry

Device telemetry is advisory status evidence. It may show hostname, OS, architecture, battery, and storage status when available. Telemetry cannot affect authority, trust, scheduling priority, proposal approval, or execution.

## Timing

Timing decides when a role may collect, refresh, evaluate, or propose evidence. Timing is not execution authority.

## Compute

Scalar compute evidence is evidence only. Compute speed, backend claims, benchmark output, or SIMD readiness cannot increase trust, role authority, scheduling priority, proposal approval, or execution permission.

Replay must prefer scalar truth even when metadata says accelerated compute was used.

## Prohibited In Local Alpha

- live trading
- live betting
- broker or exchange execution
- sportsbook execution
- provider calls from children
- shell execution from children
- child canonical writes
- child proposal approval
- automatic shared-state acceptance
- public internet pairing exposure
