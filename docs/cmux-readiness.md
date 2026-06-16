# Quant-M cmux Readiness

## Purpose

Quant-M v0.1 is ready to run inside a plain terminal pane, `tmux` session, Termux window, or future `cmux`/Staff OS worker lane as a local-first Rust agentic runtime.

For the operating-system-agnostic cockpit contract, see `docs/terminal-cockpit-adapters.md`.

This document covers the narrow proof path only:

- local setup
- config validation
- doctor checks
- one workflow run
- session and shared-state inspection

It does not introduce any new runtime layer, daemon requirement, or external integration.

## Why Quant-M fits a terminal worker lane

Quant-M already behaves well for terminal-native orchestration because:

- setup supports `--non-interactive`
- setup and doctor do not make hidden network calls
- workflow execution is local and side-effect light for `mock-research`
- outputs are stable plain text or JSON, depending on the command
- success and failure use normal process exit codes
- no background daemon is required for the v0.1 proof path

## Recommended terminal pane flow

From the Quant-M repo root:

```bash
cargo build --release
./target/release/quant-m setup --non-interactive --runtime-profile staff-os-worker
./target/release/quant-m config validate
./target/release/quant-m doctor
./target/release/quant-m run workflow workflow:mock-research-brief
./target/release/quant-m session list
./target/release/quant-m state list
```

This sequence is the recommended terminal-pane smoke path for:

- Termux windows on Android
- `cmux`
- `tmux`
- Staff OS worker launchers
- SSH/VPS shells
- constrained-device operator shells

Run the commands sequentially for the v0.1 proof path. Read-only inspection commands can run in parallel, but mutating commands such as `doctor` and `run workflow` should not be launched at the same time against the same workspace.

## Non-interactive mode

Use `--non-interactive` for setup lanes that will be called by another runtime:

```bash
./target/release/quant-m setup --non-interactive --runtime-profile staff-os-worker
```

Current behavior:

- no prompts are shown
- no model calls are made
- no OpenRouter calls are made
- no Telegram or Discord calls are made
- no email is sent
- only typed config is written

## Output expectations

### Plain text commands

These are operator-readable and stable enough for log capture:

- `quant-m init --non-interactive`
- `quant-m setup --non-interactive`
- `quant-m config validate`
- `quant-m doctor`

Example `doctor` output shape:

```text
config_exists: true
workspace_exists: true
state_path_exists: true
session_path_exists: true
workflow_run_ok: true
shared_state_list_ok: true
session_list_ok: true
checked_binary: /path/to/quant-m
generated_session_id: session-<timestamp>-<seq>
```

### JSON-first commands

These are better for Staff OS or future `cmux` parsing:

- `quant-m run workflow <workflow_id>`
- `quant-m session list`
- `quant-m session replay <session_id>`
- `quant-m state list`
- `quant-m config show --json`

Example workflow result:

```json
{
  "session_id": "session-<timestamp>-<seq>",
  "workflow_id": "workflow:mock-research-brief",
  "domain_id": "domain:mock-research",
  "status": "ok",
  "steps_completed": 1,
  "shared_state_writes": [
    "shared.research.summary"
  ],
  "related_schedulers": [
    "scheduler:mock-research-brief"
  ],
  "final_summary": "workflow=workflow:mock-research-brief status=ok steps_completed=1 shared_state_writes=shared.research.summary"
}
```

## Staff OS later integration notes

For future Staff OS worker usage, Quant-M should be treated as:

- a local worker binary
- a deterministic workflow runner
- a session evidence generator
- a shared-state writer
- an OS-agnostic terminal cockpit plan emitter

Staff OS should call narrow commands such as:

```bash
./target/release/quant-m setup --non-interactive --runtime-profile staff-os-worker
./target/release/quant-m run workflow workflow:mock-research-brief
./target/release/quant-m session replay <session_id>
./target/release/quant-m state list
```

Staff OS should not assume:

- daemon mode is required
- network access is enabled
- live trading exists
- external adapters are required
- `cmux` is always available; Android lanes should prefer Termux windows and Linux/Windows lanes should prefer `tmux`

## Known limits

- `mock-research` is still the only end-to-end execution proof domain in v0.1.
- `doctor` validates the local workflow lane, not a full orchestration fabric.
- `doctor` and `run workflow` both touch shared state, so they should be serialized per workspace instead of run concurrently in separate panes.
- Some commands emit plain text summaries while others emit JSON; callers should choose the lane intentionally.
- The current help banner is human-facing and colored, which is fine for terminals but not intended as a machine protocol.
- Quant-M v0.1 is a framework freeze point, not a full desk-runtime release.

## Recommended constrained-device check

After local cmux-style validation passes, run the same release-binary sequence on one of:

- Raspberry Pi
- Termux Android
- VPS
- old laptop

Success means:

- release build succeeds
- `mock-research` workflow completes
- session replay stays side-effect free
- shared state is visible
- no external service is required
