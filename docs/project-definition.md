# Project Definition

## One-line product thesis

Quant-M is a lightweight, local-first Rust worker runtime for memory-backed automation, heartbeat-driven checks, controlled adapters, and desk-specific handoffs without the heavy surface area of a full agent platform.

## Target user

- A technical operator who wants a lean worker runtime on a laptop, VPS, or Android-adjacent node.
- A builder who needs durable memory, scheduled checks, narrow job execution, and explicit safety gates.
- A desk owner running local or paper-trading workflows who wants structured handoffs without adopting a full web control plane.

## User pain

- Existing agent platforms often bundle more UI, orchestration, and remote platform surface than a focused worker node needs.
- Safe local automation is easy to weaken when shell, network, and channel features are enabled without tight runtime boundaries.
- Shared memory, heartbeats, and desk handoffs often end up scattered across scripts instead of living in one durable runtime.
- Forex or other desk-specific workflows need structured state and handoff records without forcing immediate live execution.

## Desired outcome

- Preserve Quant-M as a focused Rust CLI/runtime with clear operational boundaries and portable project memory.
- Make the repo safe for repeated agent work by documenting source-of-truth docs, validation commands, constraints, and implementation priorities.
- Define what functional shippable means for the current runtime so future slices improve the worker, heartbeat, memory, skills, and desk flows without uncontrolled scope growth.

## Core workflow

1. Bootstrap or validate the workspace and memory files for the node.
2. Accept narrow jobs through the worker queue or one-shot execution path.
3. Run heartbeat tasks and emit structured adapter events.
4. Persist memory, shared SQL state, and desk-specific handoffs safely.
5. Keep optional LLM, webhook, Telegram, and forex features explicitly gated behind config and documentation.
6. Validate the runtime through cargo checks and project-specific smoke commands before widening scope.

## What the MVP must prove

- Quant-M can act as a reliable local-first worker runtime with memory, heartbeat, adapters, and skills.
- The repo can support future agent-driven implementation work without losing the current safety posture.
- Desk and handoff state can be recorded durably before any live execution decisions are introduced.
- The project remains intentionally lean rather than drifting toward a heavy platform clone.

## What this is not

- Not a browser-first control plane.
- Not a remote plugin marketplace.
- Not a promise of live-trading autonomy.
- Not a replacement for human review, validation, or explicit approval on unsafe capabilities.

## Assumptions

See `docs/assumptions.md`.

## Open questions

See `docs/open-questions.md`.
