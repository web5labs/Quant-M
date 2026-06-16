---
type: external-doc-summary
date: 2026-05-31
source_count: 4
tags:
  - openclaw
  - workspace
  - bootstrap
  - sessions
---

# OpenClaw Agent Runtime

## Sources

- OpenClaw docs search snippets:
  - [Agent runtime](https://docs.openclaw.ai/agent)
  - [Agent bootstrapping](https://docs.openclaw.ai/start/bootstrapping)
  - [Agent workspace](https://docs.openclaw.ai/agent-workspace)
  - [Sessions](https://docs.openclaw.ai/cli/sessions)

## What matters for Quant-M

- OpenClaw treats the workspace as the agent’s home and primary working directory, with bootstrap files and session state built around that contract.
- Bootstrapping is explicit first-run behavior rather than hidden side effect; pre-seeded workspaces can skip that ritual.
- The workspace is default `cwd`, but the docs are careful to distinguish that from a true hard sandbox.
- Sessions are a first-class persisted artifact, not just transient chat state.

## Borrow

- Workspace-as-memory and workspace-as-runtime-contract.
- Explicit bootstrap/setup steps for first-run initialization.
- Separate persistent session/state concepts from code repo contents.
- Honest documentation about the difference between “default working directory” and real isolation.

## Avoid

- Letting the product story drift into “do everything” autonomy before the safety model is proven.
- Treating workspace confinement as sufficient security without additional sandboxing or approval controls.
- Pulling in broad channel and browser surfaces before the lean runtime is fully shippable.

## Rails implication

- Quant-M should keep its workspace files central to runtime identity and memory.
- If Quant-M grows session semantics, they should be explicit, persisted, and queryable.
- Any future sandbox language in Quant-M docs should be precise about what is and is not isolated.
