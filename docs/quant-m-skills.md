# Quant-M Skills

Quant-M has two useful layers:

- the codebase: Rust modules, configs, queues, state stores, and runtime commands
- the markdown base: durable project memory, operating doctrine, desk knowledge, workflow notes, and replayable evidence

The repeatable skill pattern is to turn every important code capability into a small markdown counterpart that explains intent, inputs, outputs, guardrails, evidence, and the next safe action. The markdown base should not replace the code. It should make the code legible to future agents and operators.

## Capability Maturity

Skill docs must not overclaim runtime behavior. Use the same maturity labels as `quant-m capabilities`. `SKILL.md` discovery is separate from runnable shell behavior: `skills run` is `guarded` unless `skills.allow_shell_commands=true`, and detection does not equal permission.

## Runtime Skill FSM

Markdown explains skill authoring. Rust controls skill execution lifecycle.

Runnable local skills pass through `declared -> loaded -> policy_checked -> ready|blocked -> running -> succeeded|failed`. Shell-backed skills also pass through `policy_approval` before a command can start. If shell execution is disabled, Quant-M records `blocked`, not `failed`.

Compatibility note: typed `skill_execution` and `policy_approval` transition evidence is machine authority. `SessionEvent::SkillCall.status` remains for display and older artifact compatibility.

Proof command:

```bash
cargo test skills
```

## Markdown Base Map

| Code or runtime area | Markdown counterpart | Purpose |
| --- | --- | --- |
| `src/config.rs`, `quant-m.toml`, `configs/` | `docs/project-spec.md` | Runtime profiles, path overrides, provider posture, deployment assumptions |
| `src/memory.rs`, `workspace/MEMORY.md`, `workspace/daily/` | `docs/governance/runtime-doctrine.md` | What should be remembered, why it matters, and how it decays or compacts |
| `src/heartbeat.rs`, `workspace/HEARTBEAT.md` | `docs/governance/runtime-doctrine.md` | Proactive checks, scheduled tasks, and expected evidence |
| `src/skill_registry.rs`, `workspace/skills/` | `docs/quant-m-skills.md` | Skill contracts, side-effect levels, routing tags, and validation requirements |
| `src/workflow_registry.rs` | `docs/fsm/product-state-machines.md` | Workflow intent, steps, state reads/writes, and replay expectations |
| `src/fsm_registry.rs` | `docs/fsm/product-state-machines.md` | Allowed state transitions and what evidence advances each state |
| `src/scheduler_registry.rs` | `docs/fsm/product-state-machines.md` | Timing rules, triggers, cadence, and disabled/default behavior |
| `src/policy_registry.rs`, `workspace/POLICY.md` | `docs/governance/runtime-doctrine.md` | Safety boundaries, approvals, blocks, and operator decisions |
| `src/shared_state.rs`, `src/state_sql.rs` | `docs/shared_state.md` | Canonical state keys, history rules, handoffs, and non-authoritative records |
| `src/sessions.rs`, `src/replay` behavior | `docs/governance/runtime-doctrine.md` | What counts as proof, how replay should be read, and what not to claim |
| `src/cluster_boundary.rs`, `src/worker_proposals.rs` | `docs/feature-map.md` | Worker proposal rules: workers propose, core decides |
| `src/strategist.rs`, `src/question.rs`, `src/consensus.rs` | `docs/feature-map.md` | Question utility, strategist dry runs, consensus boundaries, and next-action rules |
| `src/forex.rs` | `docs/feature-map.md` | Internal paper-only domain validation, evidence quality, carry filters, and forbidden actions |

## Repeatable Project Skills

### 1. Project Context Intake

Use when a feature request is broad, underspecified, or crosses runtime boundaries.

The skill gathers goal, scope, constraints, operator intent, affected modules, policy risks, and validation expectations before implementation. It should produce a short project-context packet in markdown.

Markdown home:
- `docs/wiki/raw/project/context-intake.md`
- `docs/open-questions.md`

### 2. Model Project Onboarding

Use when Quant-M absorbs a new subproject, desk, runtime lane, or external reference repo.

The skill creates or updates the project spec, definition of shippable, wiki source folders, repo-ingest notes, validation plan, and FSM execution notes. This is already visible in `LLM_PROJECT_ONBOARDING.md` and `docs/wiki/`.

Markdown home:
- `LLM_PROJECT_ONBOARDING.md`
- `docs/definition-of-shippable.md`
- `docs/wiki/MANIFEST.md`
- `docs/wiki/repo-ingest/`

### 3. Runtime Capability Mapping

Use when code gains a module, CLI command, registry entry, or state surface.

The skill records the feature in plain English: diagram intent, implementation file, commands, artifacts, guardrails, and proof path. `docs/feature-map.md` is already the strongest example of this.

Markdown home:
- `docs/feature-map.md`
- `docs/feature-map.md`

### 4. Registry Contract Authoring

Use when adding or changing domain, skill, workflow, FSM, scheduler, desk, or policy descriptors.

The skill captures descriptor fields, side-effect level, required capabilities, policy tags, state reads/writes, and replay expectations. It should keep metadata-first behavior explicit.

Markdown home:
- `docs/quant-m-skills.md`
- `docs/fsm/product-state-machines.md`
- `docs/governance/runtime-doctrine.md`

### 5. Evidence And Replay Discipline

Use when Quant-M performs meaningful work or claims completion.

The skill preserves session artifacts, shared-state history, loop reports, evidence indexes, and validation output. Its core rule is simple: no proof, no claim.

Markdown home:
- `docs/governance/runtime-doctrine.md`
- `workspace/state/sessions/`
- `workspace/state/loops/`

### 6. Worker Boundary Review

Use when Staff-OS, cmux, tmux, Termux, cron, polling, or local workers enter the loop.

The skill checks whether a worker is only submitting non-authoritative evidence or whether it is trying to mutate canonical state, spend budget, call providers, execute workflows, or bypass policy.

Markdown home:
- `docs/wiki/raw/project/worker-boundary.md`
- `docs/feature-map.md`

### 7. Policy-Gated Decisioning

Use when a question, consensus run, strategist dry run, provider route, or operator action needs a decision.

The skill forces the output through evidence, proposal, policy gate, cost record, replayability, and next safe action. It keeps decisions inspectable instead of conversationally implied.

Markdown home:
- `workspace/POLICY.md`
- `docs/wiki/raw/project/policy.md`
- `docs/wiki/raw/project/governed-decisions.md`

### 8. Domain Pack Design

Use when creating a governed domain pack.

The skill defines identity, scope, strategy constraints, schema, routing, handoff, evidence quality, risk flags, and forbidden actions. Public beta domain docs should stay curated and should not make Quant-M look like a trading bot.

Markdown home:
- `docs/feature-map.md`
- `docs/governance/runtime-doctrine.md`
- curated domain docs only when approved for public export

### 9. Validation And Hardening

Use when changing parsers, descriptors, runtime requests, queue payloads, shared-state records, or config formats.

The skill links the intended change to cargo tests, dry-run commands, replay checks, and known failure modes. Private fuzz harnesses can be maintained outside the public beta export.

Markdown home:
- `docs/validation-plan.md`
- `docs/pi-inspired-feature-hardening-plan.md`

### 10. Deployment Readiness

Use when preparing Quant-M for systemd, cron, Android, edge, laptop, or daemon operation.

The skill records environment assumptions, disabled-by-default surfaces, path overrides, logs, safety boundaries, and rollback notes.

Markdown home:
- `docs/deploy-systemd.md`
- `configs/`

## Suggested Skill File Shape

Each reusable Quant-M skill should fit this template:

```markdown
# Skill Name

## Use When
- The trigger conditions for this skill.

## Inputs
- Files, commands, state keys, or operator decisions needed.

## Procedure
1. Read the relevant markdown base files.
2. Inspect the matching code/runtime files.
3. Make the smallest safe change or produce the requested packet.
4. Capture validation evidence.
5. Record the next safe action.

## Outputs
- Markdown packet, code change, state proposal, validation log, or handoff.

## Guardrails
- Side-effect level.
- Required approvals.
- Forbidden actions.

## Validation
- Commands or artifacts that prove the result.
```

## First Quant-M Skill Pack

The first local pack should be small and high-leverage:

1. `quant-m-context-intake`
2. `quant-m-markdown-base-maintainer`
3. `quant-m-registry-contract-author`
4. `quant-m-evidence-replay-auditor`
5. `quant-m-worker-boundary-reviewer`
6. `quant-m-desk-pack-designer`
7. `quant-m-validation-hardener`

That set would cover most repeated work in this project without becoming a second codebase. The markdown base becomes the durable thinking layer; the Rust code remains the authority for execution.
