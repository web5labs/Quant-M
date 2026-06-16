# Shippable

Quant-M is shippable for v0.1.0-beta when:

- the README describes the real product
- README-only setup and first-use walkthrough work locally
- context guardian avoids repeated compaction
- session evidence and replay work without side effects
- compact packets are written and inspectable
- cost tracking works for the local proof path
- no live trading, broker, hidden provider call, or external adapter is required
- validation passes or blockers are documented honestly
- the export tree contains no build output, generated workspace state, copied repos, caches, logs, or secrets

## Current Release Posture

Public beta is allowed when a new user can clone Quant-M, follow the README only, run the first-use walkthrough, and see evidence, replay, context guardian, compact packets, and cost tracking working locally.

Public release requires release binaries, install scripts, broader fresh-machine validation, stronger community docs, security review, launch/autostart documentation, and a clean release tag.
