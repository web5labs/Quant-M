---
type: external-doc-summary
date: 2026-05-31
source_count: 3
tags:
  - paperclip
  - adapters
  - orchestration
  - heartbeats
---

# Paperclip Agent Adapters

## Sources

- [Paperclip adapter overview](http://paperclip.inc/docs/adapters/overview)
- [Paperclip adapter guide](https://paperclipai-paperclip.mintlify.app/guides/adapters)
- [Paperclip docs / GitHub pointer](https://docs.paperclip.ing/)

## What matters for Quant-M

- Paperclip cleanly separates the control plane from the runtime through adapters.
- The adapter contract covers process adapters, HTTP adapters, session behavior, heartbeat scheduling, context modes, and secret injection.
- Local process execution and remote webhook execution are treated as different lanes with different tradeoffs.
- Heartbeat scheduling, session resume behavior, and thin-vs-fat context are all first-class config concepts.

## Borrow

- A formal adapter contract between orchestration and execution.
- Explicit session behavior modes such as `always-new` vs `resume-or-new`.
- Secret references instead of inline sensitive values.
- Test-before-enable workflow for heartbeat-driven automation.

## Avoid

- Pulling a full company/org/budget control plane into Quant-M’s first shippable runtime.
- Adding “fat context” payload sprawl before a stable thin contract exists.
- Assuming a control plane is required for local usefulness.

## Rails implication

- Quant-M can stay CLI-first while still defining a future-safe adapter surface.
- If external orchestration is added later, it should target a narrow invoke/heartbeat/session contract.
- Secret handling and heartbeat testing deserve explicit docs even in a lean runtime.
