# Codex Chat Adversarial Review

Status: implementation checkpoint, not a security certification.

## Product Contract

A selected route must produce a working chat or stop before the TUI with a precise recovery command. Selection alone is never readiness. Codex may edit the selected Quant-M project by default, but it must not receive unrestricted filesystem authority.

## Findings

### P0: Enabled tools were treated as ready chat routes

The startup router previously accepted an enabled config entry even when the executable was missing or the tool kind had no safe chat adapter. This opened an inspect-like TUI that looked ready but could not answer.

Hardening: route selection now requires an enabled tool, a supported adapter, an executable command on `PATH`, and an active local Codex login status. Missing or logged-out routes stop before chat and show setup and validation commands.

### P0: Developer-tool numbering was ambiguous

The visible list placed Codex at `2` and the generic OpenAI CLI at `3`. A user selecting `3` for Codex could save an unsupported route.

Hardening: the contract is now stable and explicit: `1` none, `2` scan, `3` Codex CLI. Unsupported generic tools can remain registered but cannot count as chat-ready.

### P0: Writable scope followed process working directory

Codex used the launcher process working directory and accepted arbitrary `--add-dir` values. Starting Quant-M from the wrong directory or adding an external path could widen write authority beyond the intended project.

Hardening: the TUI derives the project root from the active config path, canonicalizes it, passes it as Codex `--cd`, and rejects canonical writable paths outside that root. The Codex adapter defaults to `workspace-write`; it never uses `danger-full-access`.

### P1: Configured model routes were disconnected from startup chat

Onboarding could configure a governed model route while startup checked only CLI tools. A valid model selection could therefore fall into provider setup instead of chat.

Hardening: startup and TUI route selection now fall back to the existing gated `llm::ask` path when provider use, external networking, model name, and credentials are all configured.

### P1: Chat presentation hid operational state

The old header compressed route, sandbox, model, layout, session, and evidence data into one dense line. Message provenance was present but difficult to scan.

Hardening: the chat now separates readiness, route, project scope, conversation, and evidence status. Codex starts project-writable; model and non-Codex routes remain read-only unless a hardened adapter exists.

## Residual Risks

- Codex login is checked locally before chat, but credentials can still expire between preflight and the first request. That failure must remain explicit and recoverable.
- CLI and provider requests block input while a response is in flight. Cancellation, streaming, and an elapsed-time indicator remain follow-up work.
- A subprocess can contain defects even when its Rust launcher is memory-safe. OS sandbox behavior and canonical path checks both remain part of the boundary.
- `PATH` lookup can change between readiness check and process launch. A future release should resolve and pin the executable identity for each session.
- Quant-M has not published a reproducible Hermes comparison. Performance superiority must not be claimed until the benchmark below is run on matched hardware and model routes.

## Performance Gate

Compare Quant-M and Hermes on the same device, terminal, model, prompt set, network, and warm/cold conditions. Record median and p95 for:

1. Process start to interactive TUI.
2. Local slash-command response time for `/help`, `/state`, and `/cost`.
3. Time to first useful model or CLI response.
4. Peak resident memory after startup and after ten turns.
5. Recovery time after a missing executable, expired login, provider timeout, and interrupted request.

Quant-M may be described as faster only for metrics where the checked-in benchmark evidence demonstrates it. Provider latency must be reported separately from local runtime overhead.

## Hardened Codex Implementation Prompt

```text
You are implementing the next Quant-M chat hardening slice in Rust.

Objective:
Make the selected CLI or model route open a truthful, responsive chat TUI without widening authority beyond the selected project.

Security invariants:
1. Treat selection, detection, validation, authentication, and readiness as different states.
2. Never open active chat unless the route has a supported adapter and passes its local readiness checks.
3. Codex uses workspace-write only. Never use danger-full-access or an unrestricted shell permission mode.
4. Derive the project root from trusted config context, canonicalize it, and set it as the subprocess working directory.
5. Reject relative, absolute, symlink, and add-dir paths whose canonical target escapes the project root.
6. Model routes must pass the existing side-effect gate and require explicit network and provider configuration.
7. Local slash commands must route without a provider call.
8. Never turn model or child output into approval, execution, or canonical-state authority.
9. Preserve append-only evidence and avoid writing secrets to logs, sessions, snapshots, or errors.
10. Fail closed with one concrete recovery command. Do not show a ready state after failure.

Performance requirements:
1. Keep local commands provider-free and measure median/p95 latency.
2. Add visible in-flight state, elapsed time, cancellation, and bounded request timeouts.
3. Stream output where the adapter supports it without weakening evidence capture.
4. Benchmark cold start, warm start, first useful response, ten-turn memory, and failure recovery.
5. Compare with Hermes only on matched hardware, model, prompts, and network. Do not claim a win without checked-in results.

Adversarial tests:
- selected but missing executable
- enabled tool with unsupported adapter
- stale or expired CLI login
- provider key missing and network disabled
- project launched from a different working directory
- ../ traversal, absolute outside path, and symlink escape
- executable replaced after readiness check
- huge output, malformed UTF-8, ANSI injection, timeout, Ctrl-C, and subprocess crash
- first response empty while transcript contains labels or progress output
- local slash command accidentally forwarded to a provider

Acceptance:
- Option 3 deterministically selects Codex.
- A ready Codex route opens the TUI in project-scoped workspace-write mode.
- A ready model route opens governed read-only chat.
- An unready route never opens a fake active chat.
- All escape tests fail closed.
- cargo fmt, clippy -D warnings, focused tests, and the full core-full suite pass.
- The report separates measured results, inferred risks, and untested claims.
```
