# Terminal Cockpit Adapters

Quant-M should run as an operating-system-agnostic Rust runtime harness.

Terminal applications are cockpit surfaces only. They host panes, tabs, or windows for model lanes, but they do not own orchestration, shared state, policy, or session evidence.

## Platform mapping

| Host | Preferred cockpit | Reason |
| --- | --- | --- |
| Android / Termux | Termux windows | Native Android terminal windows are the best fit for device constraints and mobile UX. |
| macOS | CMUX | CMUX is the preferred Apple-computer cockpit for multiple coding-agent surfaces. |
| Linux | TMUX | TMUX is the most portable terminal multiplexer for Linux shells and servers. |
| Windows | TMUX | Use TMUX through WSL, MSYS2, or a compatible terminal environment before adding a Windows-specific cockpit. |
| Unknown | Plain terminal | Fall back to printable commands and manual operator launch. |

## Runtime boundary

Quant-M owns:

- workspace config
- session ids, run ids, step ids, and replay evidence
- hot shared state in `workspace/state/shared-state.redb`
- durable shared-state history in `workspace/state/shared-state.db`
- queue, heartbeat, skill, workflow, FSM, scheduler, policy, and desk contracts
- model and lane intent as typed config or plan metadata

The cockpit owns:

- terminal windows, panes, tabs, and focus
- human-visible lane layout
- optional command launch after operator approval
- attaching to already-running model CLIs or shell sessions

The cockpit must not become the source of truth for:

- task order
- model selection
- policy decisions
- validation status
- shared state
- session replay

## CLI plan contract

Use `cockpit plan` to generate a read-only JSON plan:

```bash
cargo run -- cockpit plan --host auto
```

Plan for explicit hosts:

```bash
cargo run -- cockpit plan --host android --repo ../repo-a --model local:llama
cargo run -- cockpit plan --host macos --repo ../repo-a --repo ../repo-b --model openrouter:qwen --model openrouter:gpt
cargo run -- cockpit plan --host linux --repo ../repo-a
```

The command does not launch Termux, CMUX, TMUX, model CLIs, or shells. It emits launcher previews only.

## Multi-repo, multi-model lane shape

Each lane should have:

- a lane id
- a repo path
- an optional model label
- a Quant-M inspection command
- a cockpit-specific launcher preview

Different repo spaces can share one Quant-M workspace only when they intentionally point at the same `QUANT_M_WORKSPACE_DIR`, `QUANT_M_STATE_SQLITE_PATH`, and `QUANT_M_SESSION_DIR`.

If repo lanes should be isolated, give each lane its own workspace path and synchronize only selected shared-state records through an approved future bridge.

## Concurrency rule

Read-only commands such as `status`, `session list`, `session replay`, `state list`, and `state snapshot` can be shown in parallel cockpit panes.

Mutating commands such as workflow runs, worker loops, heartbeat ticks, state writes, and operator decisions must be serialized per Quant-M workspace unless a future locking protocol explicitly widens that boundary.

## Future adapter direction

A later adapter may convert this plan into real launch operations:

- Termux: `termux-new-session` or a Termux plugin/window API
- CMUX: `cmux new-surface`, `cmux browser open`, or socket calls
- TMUX: `tmux new-session`, `tmux new-window`, and `tmux split-window`

That future adapter should remain behind approval and should append session evidence before and after launch attempts.
