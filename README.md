# Quant-M

![Quant-M silver armored micro-agent mascot](assets/quant-m-product.png)

**Local-first Rust control plane for governed AI work.**

Quant-M helps you turn research, model output, worker notes, and operator decisions into evidence-backed workflows. It keeps local session history, shared state, policy checks, context handoffs, and replay records close to the machine where the work happens.

> v0.1.0-beta: CLI-first, local-first, and intentionally conservative. Sharp edges are expected.

[Documentation](docs/README.md) | [Quick Start](#quick-start) | [First-Use Walkthrough](#first-use-walkthrough) | [Benchmarks](BENCHMARKS.md) | [Release Notes](docs/release/v0.1.0-beta.md)

## Why Quant-M?

Long-running AI work gets messy. Context drifts, worker output becomes hard to trust, and it is easy to forget which model said what, which policy blocked an action, or what is safe to do next.

Quant-M keeps the work grounded:

- evidence is written to local session logs
- workers can propose, but they do not decide
- policy checks stay between output and action
- shared state records what has been accepted
- replay lets you inspect a run without repeating side effects
- compact packets distill long sessions into reviewable handoffs
- context guardian creates handoffs only when needed
- context and memory degradation are reported instead of hidden
- operator control stays above automation

Quant-M is not a hosted agent platform, trading bot, Codex or Claude Code replacement, production enterprise suite, or automatic unchecked agent executor. It is a local runtime for governed work.

## Quick Start

Clone, build, initialize local state, then run the first proof command.

```bash
git clone git@github.com:web5labs/Quant-M.git
cd Quant-M
cargo build --release
./target/release/quant-m init
./target/release/quant-m init-truth
./target/release/quant-m setup --non-interactive --runtime-profile laptop --context-guardian true
./target/release/quant-m doctor
```

The proof path is intentionally local. It does not require a broker, live model call, browser harness, hosted service, or API key.

## First-Use Walkthrough

Run these commands from the repo root after `cargo build --release`.

```bash
./target/release/quant-m init
./target/release/quant-m init-truth
./target/release/quant-m setup --non-interactive --runtime-profile laptop --context-guardian true
./target/release/quant-m context-status
./target/release/quant-m consensus --dry-run "What should a new Quant-M user inspect first?"
```

Copy the `session_id` printed by the consensus command, then inspect the evidence path:

```bash
SESSION_ID=session-...
./target/release/quant-m replay "$SESSION_ID"
./target/release/quant-m compact "$SESSION_ID"
./target/release/quant-m context guard --json
./target/release/quant-m cost summary
```

Expected result:

- consensus writes session evidence under `workspace/state/sessions/`
- replay validates that evidence without side effects
- compact writes a compact packet under `workspace/state/compacted/`
- context guardian writes a continuity handoff under `workspace/state/context-guardian/`
- cost summary reports the local mock cost ledger

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
