# Pi-Inspired Feature Hardening Plan

## Purpose

Quant-M should borrow Pi's compression and lightweight ergonomics without inheriting loose extension behavior or a large plugin/runtime surface.

The upgrade should make Quant-M easier to run across terminal cockpits, model lanes, repo spaces, and constrained devices while preserving its core identity:

- Rust runtime harness first
- local-first by default
- typed state over raw blobs
- policy before execution
- session evidence before claims
- terminal cockpits as surfaces, not orchestrators

## North star

Quant-M should feel as easy as Pi at the interface and stricter underneath.

The operator should see simple commands. The runtime should quietly preserve evidence, policy decisions, state updates, replay data, and safe next actions.

## Feature Invariants

Every feature in this upgrade must obey these rules:

- No provider login grants shell, HTTP, model CLI, trading, terminal launch, or file mutation permission by itself.
- Compression outputs must cite session evidence instead of inventing a narrative.
- Shared state keeps reusable current facts only; session logs keep ordered causality; docs/wiki keep doctrine.
- Skills are declarative metadata before they are executable behavior.
- Slash commands are aliases over existing typed commands, not a second runtime.
- Cockpit adapters may preview or host lanes, but Quant-M remains the source of truth.
- Mutating commands remain serialized per Quant-M workspace until explicit locking is implemented.
- Export/share features produce audit bundles, not social sharing surfaces.

## Phase 1: Compressed Truth Packets

Status: `COMPRESSED_TRUTH_PACKET_01: COMPLETE`

### Goal

Add a first-class compaction primitive that creates the smallest safe handoff another model, agent, terminal, or edge device can use to continue work without rereading the full session.

### User-facing commands

```bash
quant-m compact <session_id>
```

### Output artifact shape

```text
workspace/state/compacted/<session_id>/
  compact.md
  compact.json
  evidence-index.json
  next-action.md
  risks.md
```

Deferred optional artifacts:

```text
evidence.json
policy-decisions.json
commands.log
files-changed.patch
```

### Compact Truth Packet fields

- Goal
- Current state
- Important decisions
- What changed
- What failed
- Evidence references
- Commands observed or requested
- Files that matter
- Policy constraints
- Open risks
- Next safe action
- Definition of shippable

### Implementation notes

- Start with deterministic extractive compaction from session logs, shared-state snapshots, and config.
- Do not require an LLM for Phase 1.
- Add optional LLM-assisted wording later only if it preserves evidence pointers and stores the raw deterministic packet.
- Store generated artifacts under workspace state, not project source docs by default.

### Hardening checks

- Compaction must be read-only.
- Compaction must fail clearly for unknown sessions.
- Compaction must include session id, run id, domain id, and source paths.
- Compaction must not call shell, HTTP, model providers, Telegram, terminal launchers, or worker queues.

## Phase 2: Context Status

Status: `CONTEXT_STATUS_01: COMPLETE`

### Goal

Give operators and future agents a visible context readiness signal before loops, agents, channels, or terminal lanes continue from stale or incomplete evidence.

### Status levels

```text
green  = compacted context is ready for the next safe action
yellow = compact exists, but some evidence is missing
red    = compact packet is missing, stale, or lacks required safety evidence
```

### Initial checks

- latest session id
- latest compact packet path
- compact packet presence
- compact packet staleness
- policy block evidence
- validation evidence
- changed-file evidence
- shippable definition evidence
- required local truth files

### User-facing commands

```bash
quant-m context-status
quant-m context-status --json
```

### Hardening checks

- Read-only.
- Do not create compact packets automatically.
- Do not mutate session history.
- Do not claim validation exists unless evidence is found.
- If no compact packet exists, recommend `quant-m compact <session_id>`.
- If validation evidence and shippable definition are both missing, mark execution readiness red.
- Preserve the same honesty posture used by the compaction module.

## Phase 2B: Context Budget Meter

Status: planned

### Goal

Track context pressure and context decay after context readiness is stable.

## Phase 2C: Context Firewall

Status: planned

### Goal

Prevent token leakage by generating state-specific agent packets instead of handing agents the full project, wiki, or conversation.

### Design Reference

See `docs/codex/context-firewall.md`.

### Core Rule

No agent gets full context by default. Every agent gets a task packet. Every packet must justify every context item it includes.

### Context Tiers

- Tier 0: state only
- Tier 1: contract only
- Tier 2: summary only
- Tier 3: targeted source sections
- Tier 4: full source context for audits, migrations, reconstruction, or high-risk repair

Most packets should stay in Tier 0 through Tier 2. Tier 4 should be rare.

### Output Artifact Shape

```text
workspace/state/context-packets/<packet_id>/
  packet.md
  receipt.json
```

### Receipt Fields

- packet id
- current FSM state
- packet size
- allowed context tiers
- included files, summaries, and source sections
- inclusion reason for each context item
- excluded context
- estimated token size
- expected output
- validation commands
- stop condition

### Hardening Checks

- Reuse compact truth packets, context-status, loop dry-run reports, and context-decay results before adding new context sources.
- Do not let packet generation mutate canonical project truth.
- Do not generate implementation packets when critical fields are missing, inferred, or conflicting unless an operator explicitly approves experimental mode.
- If a task cannot fit into a small or medium packet, split the task unless the FSM state is audit, migration, or repair.
- Implementation packet outputs should stay short: changed files, validation run, risks remaining, and next recommended state.

## Phase 3: Quant-M-Native Project Instruction Files

Status: `PROJECT_TRUTH_FILES_01: COMPLETE`

### Goal

Borrow Pi-style local project truth while keeping Quant-M's existing workspace doctrine.

### Files

```text
QUANTM.md
AGENTS.md
POLICY.md
SHIPPABLE.md
DESK.md
```

### Role split

- `QUANTM.md`: project purpose, runtime expectations, important paths
- `AGENTS.md`: roles, model lanes, desk boundaries
- `POLICY.md`: allowed/forbidden actions, approval rules, unsafe operations
- `SHIPPABLE.md`: completion criteria and validation gates
- `DESK.md`: domain-specific behavior such as coding, research, trading, edge, or channel desk

### Implementation notes

- Add bootstrap support that creates missing files only when requested.
- Do not overwrite existing user files.
- Add a linter that checks headings and required sections.
- Keep these files as source truth for humans; runtime config remains typed TOML/Serde.

### Commands

```bash
quant-m init-truth
quant-m init-truth --json
quant-m init-truth --force
```

## Phase 4: Loop Dry Run

Status: `LOOP_DRY_RUN_01: COMPLETE`

### Goal

Add a bounded, read-only local self-check loop that inspects the current Quant-M workspace, reports quality gaps, ranks safe improvement candidates, and identifies stale or missing context.

### Commands

```bash
quant-m loop --dry-run
quant-m loop --dry-run --json
quant-m loop --dry-run --scope repo
quant-m loop --dry-run --scope docs
quant-m loop --dry-run --scope sessions
quant-m loop --dry-run --scope truth
quant-m loop --dry-run --max-candidates 5
```

### Output artifact shape

```text
workspace/state/loops/<loop_id>/
  loop-report.md
  loop-report.json
  candidates.json
  evidence-index.json
  context-decay.json
```

### Hardening checks

- Read-only except for loop report artifacts.
- Do not mutate project files, truth files, sessions, compact packets, providers, or policy config.
- Do not create compact packets automatically.
- Do not claim validation exists unless evidence exists.
- If context-status is red, mark execution readiness as blocked.
- Never auto-deprecate `POLICY.md`, `SHIPPABLE.md`, `QUANTM.md`, or `AGENTS.md`.

## Phase 5: Context Decay

Status: `CONTEXT_DECAY_01: COMPLETE`

### Goal

Extract context degradation into a reusable Rust primitive so loop dry-runs, context status, compaction, slash commands, future branching, and evidence export can share one scoring model.

### Implementation

- module:
  - `src/context_decay.rs`
- core structs and enums:
  - `ContextItem`
  - `MemoryClass`
  - `ContextDecayScore`
  - `DecayAction`
  - `DecayReason`
- memory classes:
  - `ephemeral`
  - `tactical`
  - `strategic`
  - `canonical`
- decay actions:
  - `keep`
  - `compress`
  - `demote`
  - `archive`
  - `deprecate`
  - `operator_review`

### Scoring Fields

- `authority_score`
- `freshness_score`
- `validation_score`
- `usage_score`
- `shippable_relevance_score`
- `contradiction_penalty`
- `reason`

### Hardening checks

- Deterministic scoring.
- No file mutation.
- Missing validation lowers authority.
- Stale compact packets lower authority.
- Repeated usage raises authority.
- Contradicted context is reviewed or deprecated depending on memory class.
- Canonical truth files are never auto-deprecated or archived.
- `loop_dry_run` now uses the shared module for `context-decay.json`.

## Phase 6: Slash Command Router

### Goal

Make Quant-M easier to use in CLI shell, TUI, Telegram, iChat, and future cockpit panes without adding a second execution system.

### First slash commands

```text
/status
/doctor
/compact
/handoff
/evidence
/replay
/policy
/approve
/block
/shipcheck
```

### Implementation notes

- Slash commands should parse into existing command structs or small typed request enums.
- They should be available in `agent_shell`, TUI command input, and channel adapters.
- They should never bypass storage-mode classification or approval policy.

### Hardening checks

- Every slash command maps to an existing permission class: inspect, session-write, runtime-preflight, or worker-run.
- Unknown commands return suggestions, not fallback shell execution.
- Channel-originated slash commands include source channel, operator identity, and session evidence.

## Phase 7: Declarative Tiny Skills

### Goal

Keep Pi-like reusability while avoiding arbitrary extension execution.

### Skill folder shape

```text
workspace/skills/rust-audit/
  SKILL.md
  policy.json
  commands.json
```

### Required metadata

- Purpose
- Inputs
- Allowed tools
- Forbidden tools
- Expected output
- Validation command
- Side-effect level
- Policy tags

### Implementation notes

- Phase 5A: metadata validation only.
- Phase 5B: allow execution only through existing `skills run` gates.
- Phase 5C: connect skills to workflow and policy registries.

### Hardening checks

- Missing policy means inspect-only.
- Unknown command fields are ignored or rejected explicitly.
- Skill execution cannot widen shell/network settings.
- Local skills stay local; no marketplace.

## Phase 8: Session Branching

### Goal

Let operators compare paths safely without mutating the original session timeline.

### Commands

```bash
quant-m session fork <session_id> --from-step <step_id>
quant-m session fork <session_id> --from-step <step_id> --label safer-path
```

### Branch behavior

- Fork creates a new session id.
- Fork records parent session id and step id.
- Fork starts replay-only unless an operator explicitly approves mutation.
- Fork may carry selected shared-state context as references, not raw uncontrolled copies.

### Hardening checks

- Original session logs remain append-only.
- Fork does not replay side effects.
- Fork lineage is visible in session show/replay/export.

## Phase 7: Evidence Export Bundles

### Goal

Turn Pi-style shareability into Quant-M auditability.

### Commands

```bash
quant-m export session <session_id>
quant-m export session <session_id> --format markdown
quant-m export session <session_id> --format json
```

### Bundle shape

```text
session-summary.md
evidence.json
commands.log
policy-decisions.json
compact-handoff.md
```

### Hardening checks

- Redact configured secrets and environment variables.
- Include hash/checksum metadata for exported files.
- Export is read-only.
- Export path must stay under workspace unless explicitly approved by the operator.

## Phase 8: Provider Login As Configuration, Not Permission

### Goal

Improve onboarding for Codex, Claude, Gemini, OpenRouter, and local models without making providers policy authorities.

### Commands

```bash
quant-m login codex
quant-m login claude
quant-m login gemini
quant-m login openrouter
quant-m login local
```

### Implementation notes

- Login stores provider availability metadata and config pointers.
- Secrets should prefer environment variables or OS-native secure storage where available.
- Login must never enable shell, HTTP, worker-run, cockpit launch, or trading permissions.

### Hardening checks

- `quant-m login` updates config only.
- `quant-m policy evaluate-skill` remains the gate for capability use.
- Provider health checks are explicit and network-gated.

## Phase 9: Status Strip and Cockpit Integration

### Goal

Surface enough runtime truth for operators in terminal panes without building a large dashboard.

### Minimal status strip

```text
Provider: unset
Mode: local
Policy: strict
Context: green
State: clean
Evidence: recording
Shell: blocked
HTTP: blocked
Next Gate: none
```

### Integration points

- `quant-m status --strip`
- `quant-m cockpit plan` includes optional status-strip command
- TUI overview adds context status and evidence state
- Agent shell startup banner includes context status once implemented

## Recommended Implementation Order

1. `COMPRESSED_TRUTH_PACKET_01` — complete
2. `CONTEXT_STATUS_01` — complete
3. `PROJECT_TRUTH_FILES_01` — complete
4. `LOOP_DRY_RUN_01` — complete
5. `CONTEXT_DECAY_01` — complete
6. `LOOP_CANDIDATES_01`
7. `LOOP_APPLY_APPROVAL_01`
8. `SLASH_COMMAND_ROUTER_01`
9. Declarative Tiny Skills hardening
10. Session Branching
11. Evidence Export Bundles
12. Provider Login
13. Status Strip and cockpit polish

## Definition of Shippable

The upgrade is shippable when:

- `quant-m compact` creates deterministic, evidence-backed handoff artifacts.
- Context status appears in CLI and operator shell without provider dependencies.
- Project instruction files can be bootstrapped and linted without overwriting user work.
- Slash commands map to typed commands and preserve storage-mode policy.
- Tiny skills validate declarative metadata before execution is considered.
- Session forks preserve lineage and never replay side effects by default.
- Evidence exports are read-only and redact configured sensitive values.
- Provider login cannot grant execution permission.
- Cockpit plans continue to emit previews only unless a later launch adapter is explicitly approved.

## Explicit Non-Goals

- No full plugin marketplace.
- No arbitrary extension execution.
- No browser-first UI dependency.
- No theme system beyond minimal terminal readability.
- No provider abstraction sprawl before compaction and evidence flows are stable.
- No social sharing surface.
- No cockpit adapter that launches terminals without an approval/evidence layer.

## Validation Matrix

| Area | Required validation |
| --- | --- |
| Compaction | Unit tests for packet fields, missing sessions, read-only behavior |
| Context budget | Threshold tests and JSON/plain output tests |
| Project files | Bootstrap does not overwrite; lint catches missing sections |
| Slash commands | Parse tests and storage-mode classification tests |
| Skills | Metadata validation tests; missing policy defaults to inspect-only |
| Branching | Fork lineage tests; original session remains unchanged |
| Export | Redaction tests; bundle shape tests; path-boundary tests |
| Login | Config-only mutation tests; network-gated health checks |
| Cockpit | Host mapping tests; no terminal spawn in planning mode |

## Open Design Questions

- Should compact packets live only under `workspace/state/compacted`, or should selected handoffs also be copied into docs/wiki?
- Should context-budget red status become a hard policy gate immediately, or begin as advisory?
- Should project instruction files live at repo root, workspace root, or both?
- Should provider login use OS keychains where available, or stay environment-variable-first until later?
- Should session fork support shared-state overlays, or only lineage references in the first version?
