# Quant-M

Quant-M is a lightweight, local-first runtime that keeps the core OpenClaw-style capabilities you highlighted, without the heavy platform surface.

It is built as a minimal Rust worker runtime with four mandatory pillars:

1. Memory system (`SOUL.md`, `USER.md`, `AGENTS.md`, `HEARTBEAT.md`, `MEMORY.md`, daily logs, SQLite hybrid search)
2. Heartbeat loop (scheduled proactive checks from `HEARTBEAT.md`)
3. Channel adapters (terminal + optional webhook)
4. Local skills registry (local `skills/` only, no remote plugin marketplace)

## Why this version is "lite"

- No web UI/dashboard/frontend
- No remote skill marketplace
- No heavy model or RAG services
- No extra provider/channel stacks beyond terminal + webhook
- Single binary workflow focused on worker-node operations

## Quick Start

```bash
cd /path/to/quant-m
cargo run -- init
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
- `workspace/state/sessions/` (append-only session logs)
- `workspace/skills/`
- `workspace/queue/inbox.ndjson`
- `workspace/queue/outbox.ndjson`
- `workspace/queue/dead-letter.ndjson`

## Core Commands

```bash
# Worker lifecycle
quant-m worker submit '{"kind":"echo","text":"ping"}'
quant-m worker once '{"kind":"shell","command":"uptime"}'
quant-m worker run

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

# Status and manual adapter sends
quant-m status
quant-m adapter send "manual alert" --kind notice

# Session history
quant-m session list
quant-m session show <session-id>
quant-m session replay <session-id>

# LLM and Telegram
quant-m llm ask "summarize this node status"
quant-m telegram run

# SQL shared state + handoffs
quant-m state init
quant-m state summary
quant-m state signal-upsert '{"signal_id":"sig-1","desk":"forex","source_venue":"dukascopy","execution_adapter":"paper_fx","account_scope":"sandbox","symbol":"EURUSD","freshness_ms":1200,"confidence":0.73,"payload_json":{"regime":"london_open"}}'
quant-m state handoff-add '{"desk":"forex","source_venue":"dukascopy","symbol":"EURUSD","signal_id":"sig-1","producer_role":"desk_worker","producer_model":"openai/gpt-4o-mini","thesis":"short-term mean reversion setup","evidence_json":{"rsi":28},"risk_flags_json":{"macro_event":false},"confidence":0.73,"recommended_action":"buy","execution_adapter":"paper_fx","account_scope":"sandbox","paper_trade_only":true}'
quant-m state handoff-list --desk forex --limit 10
```

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

Config validation now reports the exact invalid path field and the matching override variable when a resolved path is empty or malformed.

For OpenRouter use, set:

- `llm.enabled = true`
- `llm.api_base = "https://openrouter.ai/api/v1"`
- `llm.model = "<your-model>"`
- env var `OPENROUTER_API_KEY`

For Telegram channel use, set:

- `telegram.enabled = true`
- `telegram.bot_token = "<bot-token>"` or env `TELEGRAM_BOT_TOKEN`
- optional `telegram.allowed_chat_id = <your-chat-id>`

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
- session replay reconstructs state from persisted events only
- replay does not execute shell commands, network calls, trading actions, or other side effects
- sessions are stored under `workspace/state/sessions/`

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
