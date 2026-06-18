# Quant-M Feature Map (Diagram -> Implementation)

## Feature Truth Map

This file is a truth map, not a marketing page. The machine-readable inventory lives in Rust and is available through:

```bash
quant-m capabilities
quant-m capabilities --json
quant-m capabilities show <capability_id>
quant-m capabilities audit-docs
```

Capability status labels are exact enum-like strings:

- `shipped`: implemented, documented, tested, and usable locally without hidden provider/network assumptions
- `guarded`: implemented but blocked unless explicit config, policy, or operator approval enables it
- `dry_run`: implemented as a non-side-effecting or simulated path only
- `mock`: intentionally fake/demo behavior for proof, tests, examples, or registry shape
- `experimental`: present but not stable enough to promise as a public contract
- `design_only`: described as a pattern or future direction, not a usable runtime feature
- `external_required`: depends on local external tools or provider setup
- `unavailable`: not available in this repo/machine/config posture
- `deprecated`: retained but no longer recommended

Runtime state authority follows the same rule: Markdown explains intent and examples; Rust decides allowed states, transitions, denial behavior, and replay-compatible evidence. The current authority summary is inspectable with `quant-m fsm authority` or `quant-m fsm authority --json`.

Current major capability truth:

| Feature group | status | command surface | created artifacts | proof command | safety notes | current limitations |
| --- | --- | --- | --- | --- | --- | --- |
| Onboarding/setup | `shipped` | `quant-m onboard`, `setup`, `init`, `settings` | `quant-m.toml`, `workspace/` | `quant-m settings` | project-local config; provider setup is not permission | does not install providers or local models |
| Local shell/demo/status | `shipped` plus `experimental` TUI | `agent`, `shell`, `demo`, `doctor`, `status`, `tui` | sessions, compact/context/cost artifacts for demo | `quant-m demo` | local proof path avoids provider calls | TUI remains experimental |
| Memory | `shipped` | `memory add/search/list` | `workspace/MEMORY.md`, `workspace/memory/brain.db` | `cargo test memory` | local hashed signal; no embedding provider | markdown memory remains human-authored |
| Sessions/replay | `shipped` | `session *`, `replay` | `workspace/state/sessions/` | `quant-m session list` after demo | replay is side-effect free and computes typed final lifecycle state | legacy final-status strings remain for compatibility |
| Compact/context/boil/loop | `shipped`, `experimental`, `dry_run` | `compact`, `context-status`, `context guard`, `boil`, `loop --dry-run` | compact packets, guardian handoffs, loop reports, typed context FSM transition evidence | `quant-m context guard --json` | does not call providers or execute commands; typed guardian actions separate continue, compact, refresh, review, handoff, and block outcomes | boil is experimental; loop is dry-run only |
| Cost ledger | `shipped` | `cost summary` | cost ledger records | `quant-m cost summary` | local accounting only | no live billing reconciliation |
| Consensus/strategist/question | `dry_run` / `experimental` | `consensus --dry-run`, `strategist --dry-run`, `question ask` | sessions, shared state, proposals, reports | `cargo test consensus strategist question` | dry-run paths are not live execution | question utility is still experimental |
| Worker runtime | `guarded` | `worker submit/once/run` | queue, outbox, dead-letter, worker state, typed session transition evidence | `cargo test worker` | shell/HTTP lanes gated by config; invalid worker FSM transitions fail | mutating worker execution remains single-workspace |
| Worker proposals/cluster boundary | `shipped` | `worker proposal submit/list` | proposal JSON and index | `cargo test worker_proposals cluster_boundary` | workers propose; core decides; invalid proposal review jumps fail | proposal acceptance workflow is intentionally narrow |
| Adapters/channels | `shipped`, `guarded`, `unavailable` | `adapter send`, `channel list`, `telegram run` | logs/session events | `quant-m channel list --json` | channels are not execution authority | Telegram/webhook require config/secrets |
| LLM/providers/tools | `external_required` / `guarded` / `unavailable` | `provider list/validate`, `tool list/scan/validate`, `llm ask` | config and diagnostics only | `quant-m provider list --json` | detection does not equal permission | no provider/network calls during capability detection |
| Local filesystem skills | `guarded` | `skills list/show/run` | `workspace/skills/`, typed lifecycle session events | `cargo test skills` | shell-backed skills require typed policy approval and `skills.allow_shell_commands=true`; blocked shell skills are safety outcomes | declaration/detection is not execution permission |
| Registry-backed governance | `shipped` | `domain`, `skill`, `policy`, `workflow`, `fsm`, `scheduler`, `desk` | registry JSON output and FSM authority summary | `quant-m fsm authority --json` | metadata-first; policy evaluated before unsafe paths | built-ins are mock/minimal; workflow cursor is not a typed runtime FSM |
| Shared/domain state | `guarded` | `state *` | SQLite/redb state, handoffs, paper records | `quant-m state init && quant-m state summary` | paper/state modeling only; no live trading | domain-specific payloads require typed normalization |
| Cockpit planning | `experimental` | `cockpit plan` | JSON plan only | `cargo test terminal_cockpit` | previews only; does not launch terminal panes | no live cockpit adapter |
| Truth/project files | `shipped` | `init-truth` | local truth files | `quant-m init-truth --json` | creates doctrine files only | should stay small and generated where possible |
| Mock research/trading packs | `mock` | `domain show`, `run workflow`, `policy evaluate-skill` | sessions/shared state for mock workflows | `quant-m policy evaluate-skill mock-trading.prepare-paper-review` | mock trading is paper-only; live trading denied | no broker/exchange integration |
| Repeatable project skills | `design_only` | docs only unless installed as local skills | markdown guidance | none | pattern guidance, not runtime authority | convert to local skills only when needed |

This maps the "What makes OpenClaw powerful" diagram to the Quant-M minimal implementation.

## 1) Memory System

Diagram intent:
- markdown identity/memory files
- hybrid memory retrieval
- local-first storage

Quant-M implementation:
- bootstrap files:
  - `workspace/SOUL.md`
  - `workspace/USER.md`
  - `workspace/AGENTS.md`
  - `workspace/MEMORY.md`
  - `workspace/daily/*.md`
- SQLite memory index:
  - `workspace/memory/brain.db`
- Hybrid scoring in `src/memory.rs`:
  - hashed local vector signal (no external embedding API)
  - keyword overlap score
  - small recency decay bonus

## 2) Heartbeat

Diagram intent:
- runs periodically without user prompting
- proactive checks and notifications

Quant-M implementation:
- heartbeat task source:
  - `workspace/HEARTBEAT.md` (`- task` bullet format)
- commands:
  - `quant-m heartbeat tick`
  - `quant-m heartbeat run`
- runtime:
  - interval from config (`heartbeat.interval_seconds`, default 1800)
  - task execution via worker task specs:
    - `shell:<command>`
    - `http:<url>`
    - `echo:<text>`
    - `json:<job-json>`

## 3) Channel Adapters

Diagram intent:
- gateway-like adapter layer to deliver events

Quant-M implementation:
- adapter hub in `src/adapters.rs`
- supported outputs:
  - terminal JSON events (default)
  - optional webhook POST target
- used by:
  - worker result notifications
  - heartbeat notifications
  - manual `adapter send` command
 - optional Telegram polling channel in `src/telegram.rs` (disabled by default)

## 3.25) Cluster Authority Boundary

Diagram intent:
- keep Staff-OS, cmux, tmux, Termux, cron, mtime, polling, and local worker surfaces subordinate to the governed core
- let worker lanes submit evidence and proposals without granting runtime authority

Quant-M implementation:
- typed classifier in `src/cluster_boundary.rs`
- supported mock/typed surfaces:
  - `staff_os_workspace`
  - `cmux_lane`
  - `tmux_worker`
  - `termux_worker`
  - `cron_worker`
  - `mtime_worker`
  - `polling_worker`
  - `local_worker`
- worker intent records can represent evidence, reviews, completion reports, state proposals, cost proposals, approval evidence, denial evidence, escalation evidence, rejected commands, and policy blocks
- guardrails:
  - workers propose; the core decides
  - worker records do not execute workflows
  - worker records do not mutate canonical shared state
  - worker records do not append accepted cost-ledger truth
  - worker records do not call providers
  - worker records do not trigger trading behavior
  - worker records do not bypass policy, replay validation, or operator approval

## 3.3) Worker Proposal Records

Diagram intent:
- let cluster workers submit structured evidence and proposals while staying non-authoritative
- keep proposal review local, durable, append-only, and separate from accepted shared state and accepted cost truth

Quant-M implementation:
- proposal store in `src/worker_proposals.rs`
- commands:
  - `quant-m worker proposal submit --surface <surface> --kind <kind> --summary "<summary>"`
  - `quant-m worker proposal submit --surface <surface> --kind <kind> --summary "<summary>" --json`
  - `quant-m worker proposal list`
  - `quant-m worker proposal list --surface cmux_lane`
  - `quant-m worker proposal list --status pending_review --json`
- artifacts:
  - `workspace/state/worker-proposals/<proposal_id>.json`
  - `workspace/state/worker-proposals/index.jsonl`
- supported proposal kinds:
  - `evidence`
  - `review`
  - `state_suggestion`
  - `cost_suggestion`
  - `completion_report`
  - `escalation`
  - `next_action_suggestion`
- default status is `pending_review`
- every record has `non_authoritative: true`
- state suggestions are not canonical, cost suggestions are not ledger truth, and completion reports are not proof until replay validates them

## 3.4) Multi-Domain Strategist Dry Run

Diagram intent:
- prove a first cluster-shaped use case across several bounded research lanes
- coordinate worker proposals, session evidence, local artifacts, and cost accounting without live providers or execution authority

Quant-M implementation:
- command:
  - `quant-m strategist --dry-run`
  - `quant-m strategist --dry-run --json`
- module:
  - `src/strategist.rs`
- deterministic mock lanes:
  - `macro_lane`
  - `forex_carry_lane`
  - `crypto_peg_risk_lane`
  - `equity_options_risk_lane`
  - `sports_event_timing_lane`
  - `operator_audit_lane`
- artifacts:
  - `workspace/state/sessions/<session_id>/strategist-report.md`
  - `workspace/state/sessions/<session_id>/strategist-report.json`
  - `workspace/state/sessions/<session_id>/strategist-evidence-index.json`
  - `workspace/state/strategist/<workflow_id>.json`
- worker proposals:
  - one non-authoritative pending-review proposal per lane
  - proposals continue to write to `workspace/state/worker-proposals/`
- cost:
  - appends one core-created dry-run ledger record
  - provider is `mock`
  - model is `deterministic-strategist-lanes`
  - actual cost is `0.00 USD`
- guardrails:
  - no live market data
  - no provider calls
  - no network requirement
  - no trading signals or order generation
  - no worker proposal auto-acceptance
  - policy result is research-only and blocks execution

## 3.5) Universal Question Utility

Diagram intent:
- give Quant-M one governed question interface for every near-term route
- keep Agent Cluster, Staff-OS Handoff, and Harness as the only utility modes
- force raw input through evidence, proposal, policy, cost, replay, and next-action fields before work expands

Quant-M implementation:
- command:
  - `quant-m question ask --mode agent-cluster "<question>"`
  - `quant-m question ask --mode agent-cluster "<question>" --write-proposals`
  - `quant-m question ask --mode staff-os-handoff "<question>" --json`
  - `quant-m question ask --mode harness "<question>"`
- module:
  - `src/question.rs`
- universal question:
  - `What should happen next, based on the available evidence, policy, cost, state, and operator goal?`
- shared contract:
  - `question`
  - `evidence`
  - `proposal`
  - `policy_gate`
  - `cost_record`
  - `replayable_decision`
  - `next_safe_action`
- supported modes:
  - `agent_cluster`
  - `staff_os_handoff`
  - `harness`
- guardrails:
  - no trading, chat, dashboard, onboarding, or provider-specific question modes
  - domain-specific work must flow through one of the three utility modes
  - workers propose and the core decides
  - provider use remains budget-gated through harness mode
- agent-cluster proposal bridge:
  - default output is an inspect-only worker proposal plan
  - `--write-proposals` materializes the plan into pending non-authoritative worker proposal records
  - written proposal records continue to use `workspace/state/worker-proposals/`
  - `--write-proposals` is currently limited to `agent_cluster`
  - generated plan cost is zero actual and local-only

## 3.6) LLM API

Quant-M implementation:
- minimal OpenAI-compatible chat completion client in `src/llm.rs`
- intended target: OpenRouter (`llm.api_base`)
- used by:
  - `quant-m llm ask`
  - Telegram `/ask ...` messages

## 4) Skills Registry

Diagram intent:
- local skill files, instantly available
- avoid remote supply-chain exposure

Quant-M implementation:
- local path only:
  - `workspace/skills/<skill>/SKILL.md`
  - optional `SKILL.toml` with `[run].command`
- commands:
  - `quant-m skills list`
  - `quant-m skills show <name>`
  - `quant-m skills run <name> <input>`
- no remote registry/install path in this lite build

## 5) Android Worker Runtime Focus

Required deployment model from your brief:
- coordinator (Pi/VPS) controls worker nodes
- Android nodes execute narrow tasks and return JSON/logs

Quant-M implementation:
- queue files:
  - inbox: `workspace/queue/inbox.ndjson`
  - outbox: `workspace/queue/outbox.ndjson`
  - dead-letter: `workspace/queue/dead-letter.ndjson`
  - inflight: `workspace/queue/inflight.json`
- lifecycle commands:
  - `quant-m worker submit <job-json>`
  - `quant-m worker once <job-json>`
  - `quant-m worker run`
  - `quant-m daemon start`
- reliability behavior:
  - atomic inbox drain (`rename` to snapshot before parse)
  - durable batch file to recover unprocessed drained jobs on crash
  - malformed lines moved to dead-letter queue
  - daemon supervises worker/heartbeat with exponential backoff on failures
  - graceful daemon shutdown via shared cancellation signal
  - shell/http execution gated behind explicit config flags
- health/state:
  - `workspace/state/worker_state.json`
  - `quant-m status`

## 6) Pi-Inspired Compression Hardening

Diagram intent:
- keep long agent sessions usable
- hand work between models, terminals, and devices
- preserve lightweight ergonomics without a large extension surface

Planned Quant-M implementation:
- hardening plan:
  - `docs/pi-inspired-feature-hardening-plan.md`
- first primitive:
  - `quant-m compact <session_id>`
- planned artifact shape:
  - `workspace/state/compacted/<session_id>/compact.md`
  - `workspace/state/compacted/<session_id>/compact.json`
  - `workspace/state/compacted/<session_id>/evidence-index.json`
  - `workspace/state/compacted/<session_id>/next-action.md`
  - `workspace/state/compacted/<session_id>/risks.md`
- guardrails:
  - compaction is read-only
  - compact packets cite session evidence
  - provider login never grants execution permission
  - slash commands route through existing policy/storage-mode gates

## 7) Context Status Gate

Diagram intent:
- prevent loops, agents, or channels from continuing on stale or incomplete context
- reuse compact packets instead of inventing another summary layer
- make missing validation, policy, or shippable evidence visible

Quant-M implementation:
- command:
  - `quant-m context-status`
  - `quant-m context-status --json`
- reads:
  - latest session under `workspace/state/sessions/`
  - compact packet under `workspace/state/compacted/<session_id>/compact.json`
  - local truth files such as `QUANTM.md`, `POLICY.md`, `SHIPPABLE.md`, and `AGENTS.md`
- output:
  - `context_state`: `green`, `yellow`, or `red`
  - compact packet presence/staleness
  - policy, validation, changed-file, and shippable evidence booleans
  - missing context list
  - recommended next action
- guardrails:
  - read-only
  - does not create compact packets automatically
  - does not mutate session history
  - does not infer validation or shippable success from silence

## 8) Project Truth Files

Diagram intent:
- give future loops and agents stable local truth
- make `context-status` missing-context checks actionable
- keep policy and shippable criteria human-readable without creating a giant doctrine layer

Quant-M implementation:
- command:
  - `quant-m init-truth`
  - `quant-m init-truth --json`
  - `quant-m init-truth --force`
- creates missing files under the configured workspace:
  - `QUANTM.md`
  - `POLICY.md`
  - `SHIPPABLE.md`
  - `AGENTS.md`
- safety defaults:
  - no live trading
  - no credential edits
  - no shell, HTTP, network, terminal, or cockpit escalation without explicit approval
  - preserve evidence
  - do not claim validation without proof
  - generated agents define lanes without granting permissions
- guardrails:
  - does not overwrite existing files unless `--force` is provided
  - reports files as created, present, or overwritten
  - recommends `quant-m context-status` after creation

## 9) Loop Dry Run

Diagram intent:
- bring GrepLoop/Hermes-style local improvement scanning into Quant-M without mutation
- rank safe next actions from evidence instead of launching autonomous edits
- surface stale or missing context before a future apply loop exists

Quant-M implementation:
- command:
  - `quant-m loop --dry-run`
  - `quant-m loop --dry-run --json`
  - `quant-m loop --dry-run --scope repo|docs|sessions|truth|all`
  - `quant-m loop --dry-run --max-candidates <n>`
- outputs:
  - `workspace/state/loops/<loop_id>/loop-report.md`
  - `workspace/state/loops/<loop_id>/loop-report.json`
  - `workspace/state/loops/<loop_id>/candidates.json`
  - `workspace/state/loops/<loop_id>/evidence-index.json`
  - `workspace/state/loops/<loop_id>/context-decay.json`
- guardrails:
  - read-only except loop report artifacts
  - does not mutate truth files, compact packets, sessions, source, docs, provider config, or policy config
  - red `context-status` marks execution readiness as blocked
  - context decay never auto-deprecates canonical truth files

## 10) Context Decay

Diagram intent:
- degrade stale, weakly evidenced, or contradicted context without mutating project files
- make decay scoring reusable across loops, compaction, context status, branching, and future evidence export
- preserve canonical project truth as operator-reviewed doctrine rather than disposable context

Quant-M implementation:
- shared module:
  - `src/context_decay.rs`
- core types:
  - `ContextItem`
  - `MemoryClass`
  - `ContextDecayScore`
  - `DecayAction`
  - `DecayReason`
- scoring fields:
  - authority, freshness, validation, usage, shippable relevance, and contradiction penalty
- integration:
  - `loop_dry_run` uses the shared scorer when writing `context-decay.json`
- guardrails:
  - deterministic scoring
  - no file mutation
  - missing validation lowers authority
  - stale compact packets lower authority
  - repeated references raise authority
  - `POLICY.md`, `SHIPPABLE.md`, `QUANTM.md`, and `AGENTS.md` are never auto-deprecated
