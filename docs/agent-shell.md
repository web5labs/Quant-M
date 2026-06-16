# Quant-M Agent Shell

## Purpose

Quant-M Agent Shell v0 is the operator-facing text shell that sits on top of the existing CLI/runtime without changing the stable automation contract underneath.

Use it when you want:

- one terminal-native entrypoint
- a clean startup summary
- quick workflow, session, state, config, and doctor actions
- a lighter experience than raw JSON-heavy CLI inspection

Do not use it as the primary automation interface for:

- Staff OS
- `cmux`
- scripts
- CI

Those should continue to call the stable non-interactive CLI commands directly.

## Launch

From the Quant-M repo root:

```bash
cargo build --release
./target/release/quant-m agent
```

## Startup Banner

On launch, the shell prints:

- Quant-M version
- current mode
- workspace path
- runtime profile
- external network posture
- preferred local model
- preferred OpenRouter model
- available domains count
- available workflows count
- session count
- shared-state count
- hint to type `help`

This keeps the first screen readable in a normal terminal pane while still giving enough context for an operator or future Staff OS supervisor.

## Supported Commands

### General

- `help`
- `doctor`
- `config show`
- `exit`
- `quit`

### Workflow

- `run demo`
- `run mock-research`
- `run workflow <workflow_id>`

### State

- `state summary`
- `state list`
- `state list --json`
- `state show <key>`

### Sessions

- `session recent`
- `session list`
- `session list --json`
- `session show <session_id>`
- `session replay <session_id>`

## First Run Example

Example operator session:

```text
$ ./target/release/quant-m agent
Quant-M Agent Shell v0.1
mode: operator_shell
workspace: workspace
runtime_profile: laptop
network: disabled
preferred_local_model: unset
preferred_openrouter_model: qwen/qwen3-coder
domains: 2
workflows: 2
sessions: 14
shared_state_records: 1
hint: type help

quant-m> help
quant-m> doctor
quant-m> run demo
quant-m> state summary
quant-m> session recent
quant-m> quit
```

Typical run output is concise and action-oriented:

```text
Workflow run complete
alias: run demo
workflow_id: workflow:mock-research-brief
status: ok
steps_completed: 1
shared_state_writes: shared.research.summary
session_id: session-<timestamp>-<seq>
next: state summary | session replay <session_id>
```

## Why The Shell Feels Better Than Raw CLI Output

The shell keeps the same runtime behavior but improves presentation:

- startup is a readable banner instead of a cold command entry
- help is grouped by category with examples
- `session recent` shows compact summaries instead of large JSON dumps
- `state summary` gives a quick workspace pulse
- explicit `--json` is still available when the operator wants raw inspection

This keeps Quant-M pleasant in Apple Terminal, SSH, or a simple pane without adding a full TUI requirement.

## CLI vs Shell

The shell is a convenience layer for human operators.

The underlying engine remains the existing CLI/runtime:

- `run demo` and `run mock-research` route to the same execution runtime as `quant-m run workflow workflow:mock-research-brief`
- `state summary`, `state list`, and `state show` use the same typed shared-state inspection path
- `session recent`, `session show`, and `session replay` use the same append-only session history
- `config show` uses the same typed Serde-backed config rendering
- `doctor` uses the same local-only health checks as the standard CLI

## Why CLI Remains Primary

Staff OS and `cmux` should keep using direct commands such as:

```bash
./target/release/quant-m setup --non-interactive --runtime-profile staff-os-worker
./target/release/quant-m doctor
./target/release/quant-m run workflow workflow:mock-research-brief
./target/release/quant-m session list
./target/release/quant-m state list
```

That keeps automation:

- deterministic
- parseable
- non-interactive
- easy to supervise

The shell is for people. The CLI is for systems.

## Relationship To TUI

`quant-m agent` is the text operator shell.

`quant-m tui` is the optional Ratatui cockpit.

The shell should stay thin and useful even without the TUI. The TUI can later make it more visual, but it should not be required for the core operator experience.

## Known Limits

- The shell is operator-facing only.
- It does not add a model loop, broker logic, external adapters, or daemon behavior.
- It is not a conversational LLM shell.
- `doctor` and `run workflow` both mutate local state, so they should not be run concurrently against the same workspace from multiple panes.
- Staff OS and `cmux` should keep using non-interactive CLI commands, not the shell.
