# Quant-M Feature Map (Diagram -> Implementation)

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

## 3.5) LLM API

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
