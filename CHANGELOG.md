# Changelog

All notable release changes are tracked here.

## v0.1.0-beta - 2026-06-16

Beta release candidate for Quant-M as a local-first Rust control plane for governed AI work.

### Added

- CLI-first setup, initialization, health, and operator shell commands.
- Local session evidence with deterministic replay.
- Consensus dry-run workflow that writes evidence, shared state, and a cost record without provider calls.
- Compact packets for reducing long or risky session context into handoff artifacts.
- Context guardian that creates continuity handoffs only when needed.
- Context status and context degradation checks.
- Worker proposal boundaries: workers propose, the core decides.
- Cost ledger summary for local proof workflows.
- Public repository basics: license, security policy, contributing guide, shippable gate, benchmarks, and CI.

### Hardened

- Repository export hygiene: build outputs, generated workspace state, copied repos, caches, local scaffolding, and raw private wiki debris removed from the public surface.
- Governance doctrine moved from generated workspace state into `docs/governance/runtime-doctrine.md`.
- README-first install and first-use walkthrough verified from a clean local export.

### Caveats

- This is beta software.
- CLI-first experience only.
- Release binaries and installer scripts are not yet published.
- Autostart docs are not complete.
- Provider normalization and cost governance UI are still developing.
- Not production enterprise software.
