# FSM Consolidation Audit

Slice: `FSM_CONSOLIDATION_AND_TRUTH_CLEANUP_01`, updated by `COMMIT_BOUNDARY_CLEANUP_01`

This audit freezes the current FSM migration boundary. Markdown explains why; Rust decides runtime state; replay proves what happened.

## Dirty Worktree Ownership

Current dirty and untracked files are classified for commit planning. This file does not delete, revert, or bless unrelated changes.

| File | Classification | Notes |
| --- | --- | --- |
| `src/fsm_core.rs` | recent FSM authority work; required untracked file | Central runtime FSM authority; required by build/tests; must be staged with FSM slices. |
| `src/fsm_authority.rs` | recent FSM authority work; required untracked file | Static authority summary for `quant-m fsm authority`; required by build/tests; stage with consolidation slice. |
| `src/capabilities.rs` | recent capability truth work; required untracked file | Capability truth inventory; required by build/tests; stage with capability slice. |
| `src/boil.rs` | recent boil/context work; required untracked file | Boil reports and Context Guardian typed fields; required by build/tests; stage with boil/context slice. |
| `docs/fsm/rust-fsm-authority-audit.md` | docs/audit work; untracked required doc | Human audit of Rust authority levels. |
| `docs/fsm/fsm-consolidation-audit.md` | docs/audit work; untracked required doc | Commit-boundary and consolidation audit. |
| `README.md` | docs/audit work plus onboarding/capability docs | Updated capability/FSM authority and onboarding model language. |
| `docs/feature-map.md` | recent capability truth work and docs/audit work | Feature truth map and authority summary pointer. |
| `docs/fsm/product-state-machines.md` | docs/audit work | Human summary shortened to avoid duplicating Rust transition truth. |
| `docs/quant-m-skills.md` | recent skill/FSM docs work | Skill FSM and compatibility field clarification. |
| `docs/codex/context-firewall.md` | recent boil/context work and docs/audit work | Context firewall/guardian integration notes. |
| `docs/pi-inspired-feature-hardening-plan.md` | docs/audit work or earlier planning docs | Review with planning-doc commit; no runtime effect. |
| `LLM_PROJECT_ONBOARDING.md` | docs/audit work or earlier onboarding docs | Review with onboarding/capability docs; no runtime effect. |
| `src/main.rs` | recent FSM authority, capability, boil/context, and command wiring work | Includes `fsm authority` and other recent command surfaces; stage with corresponding runtime slices. |
| `src/lib.rs` | recent FSM authority/capability/boil module exposure | Exposes required modules; stage with runtime slices. |
| `src/worker.rs` | recent FSM authority work | Worker job lifecycle and transition evidence. |
| `src/sessions.rs` | recent FSM authority work | Session replay typed final state and compatibility behavior. |
| `src/skills.rs` | recent FSM authority work | Skill execution and policy approval lifecycle evidence. |
| `src/context_status.rs` | recent boil/context work | Typed Context Guardian report fields. |
| `src/context_guardian.rs` | recent boil/context work | Guardian tick/handoff typed state. |
| `src/context_firewall.rs` | recent boil/context work | Context packet receipts include typed guardian fields. |
| `src/loop_dry_run.rs` | recent boil/context work | Loop readiness follows typed `continue`, not display color alone. |
| `src/worker_proposals.rs` | recent FSM authority work | Worker proposal transition validation. |
| `src/agent_shell.rs`, `src/consensus.rs`, `src/cost_ledger.rs`, `src/domain.rs`, `src/question.rs`, `src/strategist.rs` | unrelated pre-existing work or earlier slice work needing review | Modified before this cleanup; review before staging with a slice. |
| `cd /Users/julio/Desktop/The-Staff/quantm/AGENTS.md` | accidental junk; needs human review | Tiny initialized agent note under accidental literal `cd ` path; not a tracked repo file. |
| `cd /Users/julio/Desktop/The-Staff/quantm/HEARTBEAT.md` | accidental junk; needs human review | Tiny initialized heartbeat note under accidental literal `cd ` path. |
| `cd /Users/julio/Desktop/The-Staff/quantm/MEMORY.md` | accidental junk; needs human review | Tiny initialized memory note under accidental literal `cd ` path. |
| `cd /Users/julio/Desktop/The-Staff/quantm/SOUL.md` | accidental junk; needs human review | Tiny initialized doctrine note under accidental literal `cd ` path. |
| `cd /Users/julio/Desktop/The-Staff/quantm/USER.md` | accidental junk; needs human review | Tiny initialized user note under accidental literal `cd ` path. |
| `cd /Users/julio/Desktop/The-Staff/quantm/queue/*.ndjson` | generated artifact; safe to remove after approval | Empty queue artifacts under accidental literal `cd ` path. |

## Accidental `cd ` Tree

The untracked directory named `cd ` contains a nested `Users/julio/Desktop/The-Staff/quantm/` path with `AGENTS.md`, `HEARTBEAT.md`, `MEMORY.md`, `SOUL.md`, `USER.md`, and empty `queue/*.ndjson` files. This matches a likely shell/path typo or accidental initialization target, not a normal Quant-M source tree.

Recommendation: safe to remove after human approval. It was not deleted in this slice.

## Recommended Commit Groups

1. `CAPABILITY_TRUTH_LAYER_01`: `src/capabilities.rs`, capability docs, README capability status sections, capability-related `src/main.rs` changes.
2. `RUST_FSM_AUTHORITY_01`: `src/fsm_core.rs`, `src/worker.rs`, `src/sessions.rs`, `src/worker_proposals.rs`, FSM authority audit baseline.
3. `SKILL_POLICY_FSM_GATE_01`: `src/skills.rs`, skill/policy lifecycle wiring, skill docs, relevant `src/main.rs`/`src/lib.rs` wiring.
4. `CONTEXT_GUARDIAN_FSM_01`: `src/context_status.rs`, `src/context_guardian.rs`, `src/context_firewall.rs`, `src/loop_dry_run.rs`, `src/boil.rs`, context docs.
5. `FSM_CONSOLIDATION_AND_TRUTH_CLEANUP_01` / `COMMIT_BOUNDARY_CLEANUP_01`: `src/fsm_authority.rs`, `quant-m fsm authority`, this audit doc, authority-level docs.
6. Human review before staging: `src/agent_shell.rs`, `src/consensus.rs`, `src/cost_ledger.rs`, `src/domain.rs`, `src/question.rs`, `src/strategist.rs`, and the accidental `cd ` tree.

## Authority Rule

Only convert another surface to a Rust FSM when at least three apply:

- It gates execution, side effects, provider calls, shell, network, channel sends, or trading-like behavior.
- It affects replay or session truth.
- It affects whether another agent can safely continue.
- It has more than three valid states and invalid transitions matter.
- It currently uses strings or Markdown in a way that can cause ambiguous runtime behavior.

Next recommended safety slice after consolidation: `SIDE_EFFECT_POLICY_GATE_01`.
