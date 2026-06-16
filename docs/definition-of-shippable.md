# Definition of Shippable

## Functional shippable means

The project is shippable for this phase when:

- The repo docs describe Quant-M's real runtime surface, constraints, and validation flow.
- Quant-M v0.1 can execute `workflow:mock-research-brief` end to end through the local runtime.
- That run executes at least one registered skill, writes normalized shared state, and records durable session evidence.
- Session replay is deterministic and side-effect free after the workflow run.
- No external adapter, broker, or live-trading dependency is required for the proof path.
- Core runtime paths for memory, worker jobs, heartbeat tasks, adapters, and local skills are functional or honestly documented as blocked.
- Optional integrations stay clearly opt-in and default-safe.
- Desk state and handoff behavior remain durable and reviewable before any live execution decisions.
- Validation commands pass, or blockers are documented honestly.
- The slice leaves behind at least one durable verifier such as a Rust test, CLI smoke check, or explicit manual checklist.
- Any future UI/UX work remains explicitly deferred.

## Not shippable if

- The docs still describe a different product instead of Quant-M.
- Shell, HTTP, LLM, Telegram, or live execution behavior is enabled implicitly or without clear guardrails.
- The runtime crashes, dead-letters incorrectly, or loses state in the touched slice without a repair attempt or documented blocker.
- Validation fails without a documented reason.
- Scope widens into a dashboard, remote marketplace, or live-trading product without explicit approval.
- New framework layers are added before constrained-hardware validation proves the v0.1 runtime loop.

## Human review before build

Confirm:

- target operator and runtime use case
- local-first boundary and safety defaults
- paper/sandbox versus live execution posture
- config portability expectations
- which optional integrations are truly in scope
- preservation rules for worker, memory, heartbeat, and desk state
- shippable definition
- UI/UX deferred scope
- whether unresolved desk or deployment details belong in assumptions or must be decided before implementation
