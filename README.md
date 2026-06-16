# Quant-M

Quant-M is a lightweight, local-first runtime that keeps the core OpenClaw-style capabilities you highlighted, without the heavy platform surface.

v0.1 is the freeze point where Quant-M proves one local workflow can execute end to end, write normalized shared state, leave replayable session evidence, and run without brokers, models, external adapters, or live trading.

It is built as a minimal Rust worker runtime with thirteen mandatory pillars:

1. Memory system (`SOUL.md`, `USER.md`, `AGENTS.md`, `HEARTBEAT.md`, `MEMORY.md`, daily logs, SQLite hybrid search)
2. Heartbeat loop (scheduled proactive checks from `HEARTBEAT.md`)
3. Channel adapters (terminal + optional webhook)
4. Local skills registry (local `skills/` only, no remote plugin marketplace)
5. Domain-pack contract (domain-neutral metadata and capability registration)
6. Skill-registry contract (domain-neutral skill metadata and routing)
7. Workflow-registry contract (domain-neutral plan metadata above skills and shared state)
8. FSM-registry contract (domain-neutral state transitions above workflows and shared state)
9. Scheduler-registry contract (domain-neutral timing metadata above workflows and fsms)
10. Desk-pack framework (domain-neutral desk packaging above domains, skills, workflows, fsms, and schedulers)
11. Execution runtime v0 (local workflow execution over registered domains, skills, workflows, fsms, sessions, and shared state)
12. Policy-registry contract (domain-neutral policy metadata and evaluation)
13. Shared-state contract (hot current state plus durable history)

## Why this version is "lite"

- No web UI/dashboard/frontend
- No remote skill marketplace
- No heavy model or RAG services
- No extra provider/channel stacks beyond terminal + webhook
- Single binary workflow focused on worker-node operations
- Domain packs stay metadata-first and do not auto-enable live external behavior

## Quick Start

```bash
cd /path/to/quant-m
cargo run -- init --non-interactive
cargo run -- setup --non-interactive --runtime-profile laptop
cargo run -- status
```

Default config file is created at:

- `./quant-m.toml`

All workspace paths in `quant-m.toml` are portable by default:

- they are stored relative to the project root
- they resolve at runtime relative to the config file location
- they can be overridden per-path with `QUANT_M_*` environment variables

Useful path overrides:

- `QUANT_M_WORKSPACE_DIR`
- `QUANT_M_MEMORY_SQLITE_PATH`
- `QUANT_M_MEMORY_CORE_MARKDOWN`
- `QUANT_M_MEMORY_DAILY_DIR`
- `QUANT_M_STATE_SQLITE_PATH`
- `QUANT_M_HEARTBEAT_TASKS_FILE`
- `QUANT_M_WORKER_INBOX_PATH`
- `QUANT_M_WORKER_OUTBOX_PATH`
- `QUANT_M_WORKER_INFLIGHT_PATH`
- `QUANT_M_WORKER_STATE_PATH`
- `QUANT_M_WORKER_DEAD_LETTER_PATH`
- `QUANT_M_LOG_FILE`
- `QUANT_M_SKILLS_DIR`
- `QUANT_M_FOREX_REDB_PATH`
- `QUANT_M_SESSION_DIR`

Example overrides:

```bash
QUANT_M_WORKSPACE_DIR=workspace-pi cargo run -- status
QUANT_M_LOG_FILE=/tmp/quant-m.log cargo run -- status
```

Default workspace layout:

- `workspace/SOUL.md`
- `workspace/USER.md`
- `workspace/AGENTS.md`
- `workspace/HEARTBEAT.md`
- `workspace/MEMORY.md`
- `workspace/daily/`
- `workspace/memory/brain.db`
- `workspace/state/shared-state.db` (optional SQL shared state + handoffs)
- `workspace/state/shared-state.redb` (hot shared-state snapshot store)
- `workspace/state/sessions/` (append-only session logs)
- `workspace/skills/`
- `workspace/queue/inbox.ndjson`
- `workspace/queue/outbox.ndjson`
- `workspace/queue/dead-letter.ndjson`

## v0.1 Freeze

Quant-M Core is now frozen at v0.1 as a lightweight Rust agentic runtime.

v0.1 means Quant-M can:

- execute one local workflow end to end
- execute at least one registered skill
- write normalized shared state
- record durable session evidence
- replay without side effects
- run without external adapters, model calls, broker logic, or live trading
- store future model/channel preferences without contacting external services during setup

The proof path is:

```bash
./target/release/quant-m run workflow workflow:mock-research-brief
```

Reference release note:

- [docs/release-notes-v0.1.md](/Users/julio/Desktop/The-Staff/quantm/docs/release-notes-v0.1.md)
- [docs/cmux-readiness.md](/Users/julio/Desktop/The-Staff/quantm/docs/cmux-readiness.md)
- [docs/agent-shell.md](/Users/julio/Desktop/The-Staff/quantm/docs/agent-shell.md)
- [docs/tui-shell.md](/Users/julio/Desktop/The-Staff/quantm/docs/tui-shell.md)

## Core Commands

```bash
# Onboarding and config
quant-m init --non-interactive
quant-m setup --non-interactive --runtime-profile edge
quant-m config show
quant-m config show --json
quant-m config set-model openrouter qwen/qwen3-coder
quant-m config set-channel telegram disabled
quant-m config validate
quant-m provider list
quant-m provider validate openrouter
quant-m provider validate openrouter --live
quant-m tool list
quant-m tool validate codex
quant-m doctor
quant-m doctor --providers
quant-m doctor --providers --live
quant-m consensus --dry-run "Should we adopt this API design?"
quant-m strategist --dry-run
quant-m strategist --dry-run --json
quant-m question ask --mode agent-cluster "How should this be reviewed?"
quant-m question ask --mode agent-cluster "Review this API design decision" --write-proposals --json
quant-m question ask --mode staff-os-handoff "What should Codex implement next?" --json
quant-m question ask --mode harness "Which model route should handle this?"
quant-m replay <session_id>
quant-m replay <session_id> --json
quant-m state review --domain consensus
quant-m state review --domain consensus --json
quant-m cost summary
quant-m cost summary --json
quant-m agent
quant-m tui

# Worker lifecycle
quant-m worker submit '{"kind":"echo","text":"ping"}'
quant-m worker once '{"kind":"shell","command":"uptime"}'
quant-m worker run
quant-m worker proposal submit --surface cmux_lane --kind evidence --summary "Architecture lane recommends provider contracts after worker boundary hardening."
quant-m worker proposal submit --surface cmux_lane --kind evidence --summary "Architecture lane recommends provider contracts after worker boundary hardening." --json
quant-m worker proposal list
quant-m worker proposal list --surface cmux_lane
quant-m worker proposal list --status pending_review --json

# Daemon mode (worker + heartbeat loops)
quant-m daemon start

# Memory
quant-m memory add strategy "Favor high-liquidity sessions" --category core
quant-m memory search "liquidity sessions" --limit 5
quant-m memory list --limit 20

# Heartbeat
quant-m heartbeat tick
quant-m heartbeat run

# Skills (local only)
quant-m skills list
quant-m skills show <skill-name>
quant-m skills run <skill-name> "input text"

# Skill registry (metadata only)
quant-m skill list
quant-m skill show <skill-id>
quant-m skill list --domain <domain-id>
quant-m skill list --side-effect <level>

# Workflow registry (metadata only)
quant-m workflow list
quant-m workflow show <workflow-id>
quant-m workflow list --domain <domain-id>

# FSM registry (metadata only)
quant-m fsm list
quant-m fsm show <fsm-id>
quant-m fsm list --domain <domain-id>

# Scheduler registry (metadata only)
quant-m scheduler list
quant-m scheduler show <scheduler-id>
quant-m scheduler list --domain <domain-id>
quant-m scheduler list --trigger <trigger-kind>

# Desk packs (metadata only)
quant-m desk list
quant-m desk show <desk-id>
quant-m desk list --category <category>
quant-m desk list --domain <domain-id>

# Local execution runtime
quant-m run workflow <workflow-id>

# Operator shell
quant-m agent

# Operator TUI shell
quant-m tui

# Policy registry (metadata only)
quant-m policy list
quant-m policy show <policy-id>
quant-m policy list --domain <domain-id>
quant-m policy list --side-effect <level>
quant-m policy evaluate-skill <skill-id>

# Status and manual adapter sends
quant-m status
quant-m adapter send "manual alert" --kind notice

# Session history
quant-m session list
quant-m session show <session-id>
quant-m session replay <session-id>
quant-m session resume-plan <session-id>
quant-m session approve <session-id> --reason "operator rationale"
quant-m session deny <session-id> --reason "operator rationale"
quant-m session needs-info <session-id> --reason "missing evidence"

# Domain packs
quant-m domain list
quant-m domain show <domain-id>

# LLM and Telegram
quant-m llm ask "summarize this node status"
quant-m telegram run
quant-m channel list
quant-m channel list --json

# SQL shared state + handoffs
quant-m state init
quant-m state summary
quant-m state list
quant-m state show <key>
quant-m state snapshot
quant-m state expire-stale
quant-m state signal-upsert '{"signal_id":"sig-1","desk":"forex","source_venue":"dukascopy","execution_adapter":"paper_fx","account_scope":"sandbox","symbol":"EURUSD","freshness_ms":1200,"confidence":0.73,"payload_json":{"regime":"london_open"}}'
quant-m state handoff-add '{"desk":"forex","source_venue":"dukascopy","symbol":"EURUSD","signal_id":"sig-1","producer_role":"desk_worker","producer_model":"openai/gpt-4o-mini","thesis":"short-term mean reversion setup","evidence_json":{"rsi":28},"risk_flags_json":{"macro_event":false},"confidence":0.73,"recommended_action":"buy","execution_adapter":"paper_fx","account_scope":"sandbox","paper_trade_only":true}'
quant-m state handoff-list --desk forex --limit 10
```

`quant-m agent` is the single-entry text shell for operators. It adds a clean startup banner plus compact commands like `run demo`, `state summary`, `session recent`, and `session show <session-id>` on top of the same runtime the CLI already uses. `quant-m tui` is the optional Ratatui cockpit. Both reuse the same config, session, shared-state, and workflow runtime underneath, but the CLI remains the primary interface for Staff OS, `cmux`, scripts, and other automation. See [docs/agent-shell.md](/Users/julio/Desktop/The-Staff/quantm/docs/agent-shell.md) for the first-run flow and shell command set.

## SSH Smoke Test

Use this from your coordinator machine:

```bash
ssh <user>@<host> '
cd /path/to/Quant-M &&
cargo run -- state init &&
cargo run -- state summary &&
cargo run -- state signal-upsert '\''{"signal_id":"sig-ssh-1","desk":"forex","source_venue":"dukascopy","execution_adapter":"paper_fx","account_scope":"sandbox","symbol":"EURUSD","freshness_ms":900,"confidence":0.70,"payload_json":{"source":"ssh_test"}}'\'' &&
cargo run -- state handoff-add '\''{"desk":"forex","source_venue":"dukascopy","symbol":"EURUSD","signal_id":"sig-ssh-1","producer_role":"desk_worker","producer_model":"openai/gpt-4o-mini","thesis":"ssh handoff test","execution_adapter":"paper_fx","account_scope":"sandbox","paper_trade_only":true}'\'' &&
cargo run -- state handoff-list --desk forex --limit 5
'
```

## Job Payloads

`worker submit` and `worker once` accept JSON with these kinds:

- `{"kind":"echo","text":"hello"}`
- `{"kind":"shell","command":"uptime"}`
- `{"kind":"http_get","url":"https://example.com"}`
- `{"kind":"sleep","millis":500}`

Production-safe defaults:

- `worker.allow_shell_commands = false`
- `worker.allow_http_get = false`
- `worker.http_get_mode = "dry_run"` (`dry_run` | `sandbox` | `live`)
- `worker.http_get_sandbox_hosts = []` (required in `sandbox` mode)
- `worker.max_inbox_depth = 2000`
- `skills.allow_shell_commands = false`
- webhook adapters require `https` URL
- `llm.enabled = false`
- `telegram.enabled = false`
- `chat_channels.enabled = false`

Config validation now reports the exact invalid path field and the matching override variable when a resolved path is empty or malformed.

CLI onboarding/config behavior:

- `quant-m init` creates or upgrades `quant-m.toml` plus the workspace with safe defaults
- `quant-m setup` stores operator preferences for runtime profile, provider/tool preference, model preference, channel preference, and portable paths
- `quant-m setup` is interactive only when run without `--non-interactive` from a terminal
- `quant-m config show` reads typed config and prints TOML by default
- `quant-m config set-model` updates typed stored model preference without making model calls
- `quant-m config set-channel` updates typed stored channel preference without contacting Telegram, Discord, or email
- `quant-m provider list` shows configured provider metadata and whether the expected key environment variable appears present
- `quant-m provider validate <provider>` checks provider config locally without a network call
- `quant-m provider validate <provider> --live` performs an explicit live provider check
- `quant-m tool list` shows optional local tools such as Codex, Hermes, Pi Agent, OpenClaw, Ollama, and LM Studio
- `quant-m tool validate <tool>` runs a narrow safe validation command such as `--version`
- `quant-m doctor` checks local config, workspace, session path, shared-state access, and the mock-research proof lane without hidden network calls
- `quant-m doctor --providers` adds local-only provider diagnostics
- `quant-m doctor --providers --live` runs live provider diagnostics only because `--live` was explicitly requested
- `--non-interactive` is accepted for `init` and `setup` so Staff OS or cmux wrappers can drive Quant-M without prompts

For OpenRouter use, set:

- `providers.openrouter.enabled = true`
- `providers.openrouter.api_base = "https://openrouter.ai/api/v1"`
- `preferences.preferred_openrouter_model = "<your-model>"`
- env var `OPENROUTER_API_KEY`

Keep `llm.enabled = false` until you are intentionally enabling runtime model calls. Provider onboarding config does not grant model execution permission by itself, and pasted API keys are not stored in `quant-m.toml`.

For Telegram channel use, set:

- `telegram.enabled = true`
- `telegram.bot_token = "<bot-token>"` or env `TELEGRAM_BOT_TOKEN`
- optional `telegram.allowed_chat_id = <your-chat-id>`

For chat-channel planning, set:

- `chat_channels.enabled = true`
- `chat_channels.allowed_channels = ["telegram", "discord", "slack", "signal", "whatsapp", "ichat", "email"]`
- `chat_channels.default_channel = "telegram"`

Only Telegram has a live polling adapter in this slice. Discord, Slack, Signal, WhatsApp, iChat/iMessage, and email are typed channel surfaces for routing preferences and Staff OS/cmux planning; they do not send network messages until a dedicated adapter is implemented and explicitly enabled.

Channels are not execution authorities. Channel text may be classified as notification, evidence, approval evidence, denial evidence, escalation evidence, or rejected command intent. It cannot directly execute consensus, replay sessions, mutate shared state, append cost records, call providers, perform trading behavior, run shell/tool commands, or bypass policy/operator approval gates. Telegram message text is routed through this local channel-intent boundary before any reply is sent.

## Consensus Dry Run

`quant-m consensus --dry-run "<decision question>"` is Quant-M's first signature workflow. It runs deterministic mock reviewer lanes locally, writes a reviewable evidence packet, writes a consensus state record, and recommends the next safe inspection command.

This command is intentionally:

- mock-first
- provider-free
- network-free
- trading-free
- evidence-only

Artifacts are written under the configured session and workspace state paths:

- `workspace/state/sessions/<session_id>/consensus-report.md`
- `workspace/state/sessions/<session_id>/consensus-report.json`
- `workspace/state/sessions/<session_id>/evidence-index.json`
- `workspace/state/consensus/<workflow_id>.json`
- `workspace/state/cost/cost-ledger.jsonl`

Consensus is evidence, not authority. The dry-run may recommend a follow-up, but policy plus weighted state plus explicit operator approval remains the authority.

Replay a consensus decision with:

```bash
quant-m replay <session_id>
quant-m replay <session_id> --json
```

Replay validates the consensus report, evidence index, consensus state artifact, and matching `domain:consensus` shared-state record. It does not execute the recommendation, call providers, use the network, mutate artifacts, or perform trading behavior.

Review consensus shared state with:

```bash
quant-m state review --domain consensus
quant-m state review --domain consensus --json
```

State review is inspection-only. It reports stored consensus decisions, memory class, confidence, freshness, source count, contradiction count, policy result, replay/artifact/shared-state status, and the next safe command. It does not auto-decay, delete, promote, or rewrite records.

Review local workflow cost with:

```bash
quant-m cost summary
quant-m cost summary --json
quant-m cost summary --workflow <workflow_id>
quant-m cost summary --session <session_id>
```

The cost ledger is append-only local state. For consensus dry-runs, Quant-M records a mock provider/model, estimated cost, actual cost, dry-run status, session ID, workflow ID, and command. Actual dry-run cost is always `0.00 USD`. Cost summary is inspection-only: it does not call providers, use the network, mutate ledger records, execute recommendations, or perform trading behavior.

Jobs are processed with:

- bounded concurrency (`1` by default)
- timeout caps
- retry cap (`max_retries`)
- inflight recovery file for crash-safe restart behavior
- dead-letter capture for invalid payload lines
- dead-letter capture for permanent `http_get` failures
- atomic inbox draining to reduce producer/consumer race loss

Session history behavior:

- worker jobs, heartbeat task executions, and skill runs create append-only session event logs
- local workflow execution also creates append-only session event logs
- operator approvals, denials, and needs-more-info decisions are appended as session events
- session replay reconstructs state from persisted events only
- session resume-plan analyzes persisted events only and proposes a gated next step when possible
- replay does not execute shell commands, network calls, trading actions, or other side effects
- resume-plan does not call shell, HTTP, broker, model, or trading APIs
- operator approval records do not execute shell, HTTP, broker, model, or trading APIs
- an approval only changes the analysis state; it never auto-resumes a blocked session
- interrupted sessions stay gated; Quant-M explains why they are blocked instead of auto-resuming
- sessions are stored under `workspace/state/sessions/`

Domain-pack behavior:

- domain packs register metadata, capabilities, skills, workflows, fsms, schedulers, desk packs, and policies without changing core runtime identity
- builtin mock domains prove the contract surface without enabling live trading or external orchestration
- domain inspection is read-only through `quant-m domain list` and `quant-m domain show`

Skill-registry behavior:

- skill descriptors register metadata only: ids, schemas, side-effect level, required capabilities, and policy tags
- `quant-m skill ...` inspects registry metadata and does not execute live skills
- mock trading skills remain paper-trade-only metadata and do not expose `TradingAction`

Workflow-registry behavior:

- workflow descriptors register metadata only: ordered steps, referenced skills, shared-state reads/writes, required inputs, expected outputs, and per-step side-effect level
- `quant-m workflow ...` inspects workflow plans and does not execute them
- mock trading workflows stay paper-only and do not expose live trading steps

Execution-runtime behavior:

- `quant-m run workflow <workflow-id>` executes one local registered workflow end-to-end without models, brokers, external adapters, or live trading
- the v0 proof lane uses `workflow:mock-research-brief` to execute one registered skill, update shared state, and record replayable session evidence
- runtime execution reuses the existing domain, skill, workflow, fsm, scheduler, shared-state, and session contracts instead of adding a second execution architecture

FSM-registry behavior:

- fsm descriptors register metadata only: deterministic states, events, transitions, shared-state reads/writes, optional workflow references, and per-transition side-effect level
- `quant-m fsm ...` inspects state-machine plans and does not execute them
- mock trading fsms stay paper-only and do not expose live trading transitions

Scheduler-registry behavior:

- scheduler descriptors register metadata only: cron, polling, mtime, event, or manual cadence plus optional workflow/fsm references and shared-state reads/writes
- `quant-m scheduler ...` inspects timing plans and does not execute scheduled work
- cadence validation stays explicit: cron, polling, mtime, event, and manual descriptors must only set the fields that match their trigger kind
- mock trading schedulers stay paper-only and do not expose live trading timing lanes

Desk-pack behavior:

- desk pack descriptors register packaging metadata only: category, referenced skills/workflows/fsms/schedulers, shared-state reads/writes, and storage profile notes
- `quant-m desk ...` inspects packaged use-case boundaries and does not execute desks
- mock trading desk packs stay paper-only and do not enable live trading or external adapters

Policy-registry behavior:

- policy descriptors register metadata only: side-effect coverage, operator-approval requirements, default decisions, and policy tags
- `quant-m policy evaluate-skill ...` evaluates skill metadata only and does not execute the skill
- mock research allows `ReadOnly`, while mock trading requires approval for `LocalWrite` and denies `TradingAction`

Shared-state behavior:

- current shared runtime state lives in `workspace/state/shared-state.redb`
- durable shared-state history is appended to the existing SQLite state DB
- `quant-m state list/show/snapshot` reconstruct shared state from SQLite history so inspection stays available even when hot runtime state is busy
- session logs still hold ordered execution evidence and are not replaced by shared state
- operating doctrine for shared state lives in [docs/shared_state.md](/Users/julio/Desktop/The-Staff/quantm/docs/shared_state.md)

Storage mode behavior:

- `StorageMode::Inspect` is used for read-only domain and session inspection commands and does not open `forex.redb`
- `StorageMode::Inspect` is also used for `quant-m skill list/show`
- `StorageMode::Inspect` is also used for `quant-m workflow list/show`
- `StorageMode::Inspect` is also used for `quant-m fsm list/show`
- `StorageMode::Inspect` is also used for `quant-m scheduler list/show`
- `StorageMode::Inspect` is also used for `quant-m desk list/show`
- `StorageMode::Inspect` is also used for `quant-m policy list/show/evaluate-skill`
- `StorageMode::Inspect` is also used for `quant-m state list/show/snapshot/expire-stale`
- `StorageMode::SessionWrite` is used for operator decision recording and the local `quant-m run workflow ...` lane, which writes session/shared-state evidence without opening `forex.redb`
- `StorageMode::RuntimePreflight` and `StorageMode::WorkerRun` still open the required runtime stores and surface real lock errors

`http_get` lane behavior:

- `dry_run`: validates request and logs, but does not perform network calls
- `sandbox`: real request, but host must match `worker.http_get_sandbox_hosts`
- `live`: real request to validated outbound HTTPS URL
- retry behavior is bounded and only retryable HTTP failures are requeued
- non-retryable HTTP failures are written to dead-letter with explicit reason code

## Architecture Notes

- `src/memory.rs`: SQLite + markdown memory with lightweight hybrid scoring (hashed-vector + keyword)
- `src/heartbeat.rs`: periodic task parser/executor from `HEARTBEAT.md`
- `src/adapters.rs`: terminal and webhook event delivery
- `src/skills.rs`: local skill discovery and run-command support
- `src/worker.rs`: queue lifecycle, execution, outbox/state persistence
- `src/forex.rs`: Forex desk canonical structs, StoneX payload mapping, and redb latest-state store
- `src/domain.rs`: domain-pack trait, registry, and builtin mock domains
- `src/skill_registry.rs`: skill descriptors, side-effect levels, and registry filters
- `src/workflow_registry.rs`: workflow descriptors, step metadata, and inspect-only registry filters
- `src/fsm_registry.rs`: fsm descriptors, transition metadata, workflow validation, and inspect-only registry filters
- `src/scheduler_registry.rs`: scheduler descriptors, cadence validation, and inspect-only registry filters
- `src/desk_registry.rs`: desk pack descriptors, storage profiles, and inspect-only packaging filters
- `src/execution_runtime.rs`: local workflow execution, mock-research skill execution, shared-state updates, and session evidence
- `src/policy_registry.rs`: policy descriptors, evaluation decisions, and registry filters
- `src/shared_state.rs`: typed shared-state records, redb hot store, and SQLite durable history

Feature mapping to your diagram is documented in:

- `docs/feature-map.md`

Android deployment notes are in:

- `docs/deploy-android.md`

systemd deployment notes are in:

- `docs/deploy-systemd.md`

## Quality Gates

Validated in this folder:

```bash
cargo fmt --all
cargo clippy --all-targets -- -D warnings
cargo test
```

## Edge Validation

Prepare these targets for v0.1 runtime validation:

- Raspberry Pi
- Termux Android
- VPS
- old laptop

For each target, run:

```bash
cargo build --release
./target/release/quant-m run workflow workflow:mock-research-brief
./target/release/quant-m session list
./target/release/quant-m session replay <session_id>
./target/release/quant-m state list
```

Expected workflow output shape:

```json
{
  "session_id": "session-<timestamp>-<seq>",
  "workflow_id": "workflow:mock-research-brief",
  "domain_id": "domain:mock-research",
  "status": "ok",
  "steps_completed": 1,
  "shared_state_writes": ["shared.research.summary"]
}
```

Expected replay checks:

- `final_status` is `ok`
- `current_fsm_state` is `state:summary_drafted`
- `side_effects_replayed` is `false`

Expected state list shape:

```json
[
  {
    "key": "shared.research.summary",
    "domain_id": "domain:mock-research",
    "source": "workflow:workflow:mock-research-brief",
    "session_id": "session-<timestamp>-<seq>"
  }
]
```

## v0.1 Non-Goals

- no live trading
- no broker integration
- no external adapter requirement
- no governance maze
- no desk-specific runtime dependency

## Post-v0.1 Consumers

- Staff OS worker
- research agent
- forex desk pack later
- crypto desk pack later

## Fuzzing (Lean)

Quant-M includes one initial `cargo-fuzz` target:

- `ingest_json` (real worker JSON/NDJSON ingestion path)

Install:

```bash
rustup toolchain install nightly --profile minimal
cargo +nightly install cargo-fuzz
```

Run:

```bash
cargo +nightly fuzz run ingest_json -- -max_total_time=60
```

Replay a crash artifact:

```bash
cargo +nightly fuzz run ingest_json fuzz/artifacts/ingest_json/<crash-file>
```

Notes:

- Input is size-capped in the fuzz target to avoid pathological allocations.
- Invalid input is expected and should return clean parser errors, not panics.

## Runtime Benchmark

Use the lightweight daemon benchmark harness:

```bash
# default profile
./scripts/bench_worker_runtime.sh 20 20

# compare polling profiles (baseline vs optimized)
./scripts/bench_worker_runtime.sh 20 20 30
./scripts/bench_worker_runtime.sh 20 20 3
```

## Forex Desk redb (Lean v1)

State command additions for Forex desk:

```bash
# Ingest one provider payload (StoneX/FOREX.com shape)
cargo run -- state forex-ingest '{"symbol":"EURUSD","bid":1.1000,"ask":1.1002,"trend_h1":"Up","trend_h4":"Up","swap_long":0.8,"swap_short":-0.6}'

# Read latest canonical signal and handoff by symbol
cargo run -- state forex-get-signal EURUSD
cargo run -- state forex-get-handoff EURUSD

# Apply daily rollover/swap health snapshot (post-5pm ET)
cargo run -- state swap-health '{"source":"stonex_api","as_of_ms":1774909800000,"pairs":[{"symbol":"AUDJPY","swap_long":0.41,"swap_short":-0.91},{"symbol":"EURUSD","swap_long":-0.94,"swap_short":0.14}]}'

# Inspect latest stored swap health for one pair
cargo run -- state swap-health-get AUDJPY

# Refresh macro context from MQL5 (medium/high + desk currencies)
cargo run -- state macro-refresh-mql5 --hours-ahead 48

# Read pair-scoped macro state
cargo run -- state macro-get-pair USDJPY
```

redb keys:

- `latest_signal:forex:<SYMBOL>`
- `handoff:forex:<SYMBOL>`
- `swap_health:forex:<SYMBOL>`
- `pair_macro:forex:<SYMBOL>`
- `macro_event:<event_id>`

Cron helper:

```bash
# payload file contains SwapHealthInput JSON
./scripts/swap_health.sh /path/to/swap_health_payload.json
```

Example cron (daily with 10-minute post-5pm ET buffer):

```cron
10 17 * * * cd /Users/julio/android-garage/quant-m/Quant-M && ./scripts/swap_health.sh /Users/julio/android-garage/quant-m/workspace/state/swap_health_payload.json >> /Users/julio/android-garage/quant-m/workspace/logs/swap_health.log 2>&1
```

Macro refresh cadence examples:

```cron
# Hourly refresh for next 48h
5 * * * 1-5 cd /Users/julio/android-garage/quant-m/Quant-M && ./target/release/quant-m state macro-refresh-mql5 --hours-ahead 48 >> /Users/julio/android-garage/quant-m/workspace/logs/macro_refresh.log 2>&1

# Daily full refresh for next 7 days
20 17 * * * cd /Users/julio/android-garage/quant-m/Quant-M && ./target/release/quant-m state macro-refresh-mql5 --hours-ahead 168 >> /Users/julio/android-garage/quant-m/workspace/logs/macro_refresh.log 2>&1
```

## Pi Lean Cleanup

Before copying or deploying to Raspberry Pi, prune local build/fuzz artifacts:

```bash
./scripts/pi_lean_cleanup.sh --aggressive
```

Safe mode (keeps fuzz corpus):

```bash
./scripts/pi_lean_cleanup.sh
```

## One-Command Pi Deploy

From your local machine:

```bash
./scripts/deploy_pi.sh <user@host> <remote_repo_dir> [run_user]
```

Example:

```bash
./scripts/deploy_pi.sh pi@192.168.1.50 /home/pi/Quant-M pi
```
