# Runtime Doctrine

Quant-M is a local-first Rust runtime for governed AI work. These rules are stable project doctrine, not generated workspace state.

## Purpose

- Preserve session evidence for meaningful work.
- Distill long-running work into compact, reviewable handoffs.
- Keep terminal, shell, and worker lanes as surfaces, not trusted orchestrators.
- Favor deterministic, bounded behavior that can run on constrained machines.

## Safety Posture

Default posture: strict and evidence-first.

Forbidden without explicit approval:

- live trading
- credential edits
- shell escalation
- HTTP or network escalation
- terminal or cockpit launch escalation
- provider, model, or CLI execution implied only by configuration

Required behavior:

- Preserve evidence for meaningful actions.
- Do not claim validation without proof.
- Do not claim changed files without file evidence.
- Do not treat missing policy, missing validation, or a missing shippable definition as success.

## Runtime Boundaries

- `POLICY` defines safety boundaries.
- `SHIPPABLE` defines what done means for the current release phase.
- Worker lanes can propose evidence and actions, but they do not grant themselves authority.
- Heartbeat tasks are periodic checks, not a permission bypass.
- Memory and shared state should hold accepted facts separately from raw worker notes.

## Operator Defaults

- Preferred workflow: coordinator to worker to structured result.
- Primary deployment targets: laptop terminal, SSH/VPS shell, and Android/Termux-style workers.
- Local session evidence is the source of truth for replay, handoff, and review.
