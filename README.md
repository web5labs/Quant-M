# Quant-M

<p align="center">
  <img src="assets/quant-m-product.png" alt="Quant-M silver armored micro-agent mascot" width="360">
</p>

**A local-first Rust agent runtime for evidence, replay, and operator control.**

Quant-M is a small CLI runtime you can clone, build, and run on your own machine. It helps AI-assisted work leave a trail: what happened, what was accepted, what was blocked, what it cost, and how to replay it without doing the work again.

> v0.1.0-beta: CLI-first, local-first, and intentionally conservative. Sharp edges are expected.

[Quick Start](#quick-start) | [First Proof Loop](#first-proof-loop) | [Features](#features) | [Benchmarks](BENCHMARKS.md) | [Release Notes](docs/release/v0.1.0-beta.md)

## Why Quant-M?

AI work gets messy fast. A model suggests something, a worker writes notes, a session gets long, and suddenly nobody remembers what was actually proven.

Quant-M keeps that work grounded:

- **Evidence:** every meaningful run writes local session artifacts.
- **Replay:** inspect what happened without repeating side effects.
- **Compact packets:** turn long sessions into handoff files a human or agent can read.
- **Context guardian:** prepare continuity handoffs only when the current context needs it.
- **Operator control:** workers can propose; they do not silently take over.
- **Local-first governance:** no broker, hosted service, or API key is needed for the proof path.

It is not a hosted agent platform, a trading bot, a Codex or Claude Code replacement, production enterprise software, or an automatic unchecked agent executor.

## Quick Start

Clone it and run it like a normal Rust CLI project:

```bash
git clone git@github.com:web5labs/Quant-M.git
cd Quant-M
cargo run --release -- init
cargo run --release -- init-truth
cargo run --release -- setup --non-interactive --runtime-profile laptop --context-guardian true
cargo run --release -- doctor
```

That is the fastest path for new users. If you prefer a release-style binary:

```bash
cargo build --release
./target/release/quant-m doctor
```

The first run is intentionally local. No broker. No live model call. No browser harness. No hosted service. No API key.

## First Proof Loop

Run a local dry run, then replay and compact it:

```bash
cargo run --release -- context-status
cargo run --release -- consensus --dry-run "What should a new Quant-M user inspect first?"
```

Copy the `session_id` printed by the consensus command, then inspect the evidence path:

```bash
SESSION_ID=session-...
cargo run --release -- replay "$SESSION_ID"
cargo run --release -- compact "$SESSION_ID"
cargo run --release -- context guard --json
cargo run --release -- cost summary
```

You should see:

- session evidence in `workspace/state/sessions/`
- replay validation with no side effects
- a compact packet in `workspace/state/compacted/`
- a continuity handoff in `workspace/state/context-guardian/`
- a local cost summary for the mock run

## First Session

Inside the operator shell:

```text
quant-m> settings
quant-m> doctor
quant-m> demo
quant-m> sessions
quant-m> /settings
quant-m> bye
```

Scripts and worker lanes can use normal commands instead:

```bash
quant-m settings
quant-m demo
quant-m session list
quant-m session replay <session_id>
quant-m state list
```

## Features

| Feature | What it does |
| --- | --- |
| Local runtime | Runs from a Rust CLI with local files, SQLite, redb, and markdown truth files. |
| Evidence history | Records session events, decisions, outputs, errors, and policy blocks. |
| Replay | Lets you inspect what happened without rerunning side effects. |
| Shared state | Stores accepted facts and workflow output separately from raw worker notes. |
| Policy gates | Keeps shell, network, worker, channel, and trading-like actions behind guardrails. |
| Worker proposals | Lets worker lanes submit evidence and proposals without auto-accepting them. |
| Context status | Reports whether the next agent has enough context to continue safely. |
| Context compaction | Creates compact handoff files for long or risky sessions. |
| Context guardian | Watches local session evidence and writes a handoff only when the compact packet is missing, stale, from a new session, or explicitly forced. |
| Memory degradation | Flags stale, unsupported, or risky context instead of silently trusting it. |
| Tool detection | Detects optional local CLIs such as Codex without making hidden provider calls. |
| Edge-friendly setup | Supports laptops, VPS shells, Raspberry Pi-style nodes, and Android/Termux-style workers. |

## Context Guardian

The context guardian is built for long Codex or worker sessions. It does not read hidden model context. It watches Quant-M's local session evidence and prepares a fresh-thread handoff when drift risk is high.

Manual check:

```bash
quant-m context guard
quant-m context guard --json
```

Force a refresh:

```bash
quant-m context guard --force
```

Run continuously:

```bash
quant-m daemon
```

or:

```bash
quant-m daemon start
```

The guardian writes:

```text
workspace/state/context-guardian/continuity-handoff.md
workspace/state/context-guardian/metadata.json
```

It avoids repeatedly rewriting compact artifacts for the same current session. A new compact is created only when the latest session changes, the compact packet is missing, stale, uses an older schema, or `--force` is used.

## Platform Context

Quant-M is the local runtime layer beneath Staff-OS-style work:

- PONboarding defines what should be built
- Staff-OS coordinates who should build it
- Quant-M controls how execution is recorded, checked, and replayed

Supported surfaces include:

- laptop terminal
- SSH or VPS shell
- Raspberry Pi-style nodes
- Android/Termux-style workers
- `tmux` workers
- future `cmux` lanes
- Staff-OS worker launchers
- cron and polling workers

## Useful Commands

```bash
# Setup and health
quant-m init
quant-m init-truth
quant-m setup --non-interactive --runtime-profile laptop
quant-m settings
quant-m doctor

# Proof path
quant-m demo
quant-m session list
quant-m session show <session_id>
quant-m session replay <session_id>

# Context and handoff
quant-m context-status
quant-m compact <session_id>
quant-m context guard
quant-m context guard --json
quant-m context guard --force

# Daemon
quant-m daemon
quant-m daemon start

# Operator surfaces
quant-m agent
quant-m shell
quant-m tui

# Worker proposals
quant-m worker proposal submit --surface cmux_lane --kind evidence --summary "Review recommends provider contracts after worker boundary hardening."
quant-m worker proposal list --status pending_review --json

# Governed questions
quant-m question ask --mode agent-cluster "How should this be reviewed?"
quant-m question ask --mode staff-os-handoff "What should Codex implement next?" --json
quant-m question ask --mode harness "Which model route should handle this?"
```

## Safety Defaults

Quant-M is conservative by default:

- local-first setup
- no hidden provider calls during onboarding
- no browser harness dependency during onboarding
- no external network behavior unless enabled
- no broker integration
- no live trading authority
- no worker proposal auto-acceptance
- no channel message can directly execute actions
- multi-model routing stays disabled until enabled

Optional integrations must be enabled deliberately and remain subject to policy checks.

## Current Status

Status as of June 16, 2026: **v0.1.0-beta candidate**.

Use this as a public beta, not as production enterprise software. The current release is for local CLI users who want evidence, replay, context handoffs, cost visibility, and operator-controlled agent work.

Verified from a clean local export. Empty-machine verification pending.

Strong today:

- local runtime
- onboarding
- evidence tracking
- replay
- shared state
- worker proposal boundaries
- context compression
- context guardian handoffs
- memory/context degradation checks
- cost ledger summaries
- conservative safety defaults

Still developing:

- public documentation polish
- formal launchd/systemd autostart docs
- broader provider normalization
- release binaries and install scripts
- worker federation
- distributed state

## Project Structure

```text
.
|-- assets/
|   `-- quant-m-product.png
|-- src/
|   |-- execution_runtime.rs
|   |-- shared_state.rs
|   |-- sessions.rs
|   |-- worker_proposals.rs
|   |-- compaction.rs
|   |-- context_status.rs
|   |-- context_guardian.rs
|   |-- agent_shell.rs
|   `-- tui_shell.rs
|-- docs/
|   |-- release-notes-v0.1.md
|   |-- definition-of-shippable.md
|   |-- agent-shell.md
|   `-- cmux-readiness.md
|-- configs/
|-- scripts/
|-- fuzz/
|-- Cargo.toml
`-- README.md
```

## Contributing

Contributions should preserve Quant-M's local-first boundary: no hidden provider calls, no implicit live execution, and no worker proposal auto-acceptance.

## License

MIT
