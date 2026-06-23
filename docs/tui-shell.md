# Quant-M TUI Shell

## Purpose

Quant-M TUI Shell v0 adds a lightweight operator-facing terminal cockpit without changing the existing CLI contract.

Use it when you want:

- a quick runtime overview
- session and shared-state visibility
- a local operator shell for `mock-research`

Do not use it as the primary automation interface for:

- Staff OS
- `cmux`
- scripts
- CI

Those should continue to use the stable non-interactive CLI commands.

## Launch

From the Quant-M repo root:

```bash
cargo build --release
./target/release/quant-m tui
```

Inspect-first chat-shaped evidence cockpit:

```bash
./target/release/quant-m tui chat --inspect
```

macOS, Linux, and Termux can also use the smoke helper:

```bash
./scripts/smoke_tui_chat.sh
```

Windows PowerShell:

```powershell
cargo build
.\target\debug\quant-m.exe tui chat --inspect
```

## Keyboard shortcuts

- `q` quit
- `d` doctor view
- `w` workflow list
- `s` session list
- `t` shared-state list
- `r` run `workflow:mock-research-brief`
- `o` overview

## What the TUI shows

The first screen shows:

- Quant-M version
- workspace path
- runtime profile
- external network posture
- preferred local model
- preferred OpenRouter model
- session count
- shared-state count
- available domains
- available workflows

The TUI also keeps a last-action panel so the operator can see the most recent workflow run result or doctor refresh.

## What the TUI does

- reads the same typed config as the CLI
- uses the same execution runtime as `quant-m run workflow ...`
- reads sessions from append-only session history
- reads shared state through the existing inspect-safe path
- stays local-first and side-effect light for the `mock-research` proof domain

Chat mode adds typed evidence navigation commands:

- `/help`
- `/refresh`
- `/state [domain]`
- `/cost [session_id]`
- `/replay <session_id>`
- `/ask <question>`
- `/quit`

In the MVP, chat mode is inspect-first. `/ask` records display-only navigation text and does not call a provider. Any action above inspect-only must be represented as a typed storage mode before it can be enabled.

## What the TUI does not do

- it does not replace the CLI
- it does not add a daemon
- it does not call models
- it does not call brokers
- it does not require external adapters
- it does not enable live trading
- it does not add new registries or governance layers
- chat mode does not make chat text authoritative
- chat mode does not use a free-form command router or shell out to `quant-m`

## Staff OS and cmux guidance

Use the TUI for human operators.

Use the CLI for automation:

```bash
./target/release/quant-m setup --non-interactive --runtime-profile staff-os-worker
./target/release/quant-m doctor
./target/release/quant-m run workflow workflow:mock-research-brief
./target/release/quant-m session list
./target/release/quant-m state list
```

That keeps Quant-M script-first for future Staff OS and `cmux` lanes while still giving operators a richer terminal experience.

## Known limits

- The TUI is a thin shell over existing runtime capabilities, not a full chat agent shell.
- `mock-research` is still the only end-to-end proof workflow in v0.1.
- `doctor` and `run workflow` both mutate local state, so they should not be triggered concurrently against the same workspace from multiple panes.
