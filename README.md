# Quant-M

<p align="center">
  <img src="assets/quant-m-product.png" alt="Quant-M silver armored water bear agent mascot" width="340">
</p>

**A local-first Rust control plane for AI work that needs evidence, replay, handoff, and operator control.**

AI agents can generate work fast, but they often lose the trail. Decisions disappear into chat history. Evidence gets buried. Context gets too long. Another model continues without knowing what was proven, what was rejected, or what still needs review.

Quant-M gives AI work a memory, a flight recorder, and a safety boundary.

It helps answer:

- What happened?
- What evidence supported it?
- What changed?
- What was blocked?
- What did it cost?
- Can another agent safely continue?

> v0.1.0-beta: CLI-first, local-first, and intentionally conservative. Sharp edges are expected.

[Quick Start](#quick-start) | [Try The Proof Loop](#try-the-proof-loop) | [Features](#features) | [How It Compares](#how-quant-m-compares) | [Release Notes](docs/release/v0.1.0-beta.md)

## Why This Exists

Quant-M is Git-like history for AI work.

It is a flight recorder for agent sessions.

It helps agents continue without pretending they remember.

It favors governed execution over unchecked autonomy.

The goal is not flashy autonomy. The goal is durable intelligence: local evidence, replayable sessions, compact handoffs, degraded-context warnings, and human authority over worker output.

## Why The Water Bear?

The water bear is not just a mascot. It is the design metaphor.

Tardigrades are famous for surviving brutal conditions: pressure, radiation, dehydration, cold, heat, and even space-like environments. Quant-M is built for the harsh parts of agent work: context loss, drift, stale memory, incomplete evidence, failed runs, and handoffs between models.

The little armored creature is a reminder: make the work resilient before making it autonomous.

## Quick Start

Clone it and run it like a normal Rust CLI project:

```bash
git clone https://github.com/web5labs/Quant-M.git
cd Quant-M
cargo run --release -- init
cargo run --release -- init-truth
cargo run --release -- setup --non-interactive --runtime-profile laptop --context-guardian true
cargo run --release -- doctor
```

The first run is intentionally local. No broker. No live model call. No browser harness. No hosted service. No API key.

## Try The Proof Loop

In a few commands, you can see the core idea: create evidence, replay it, compact it, prepare a handoff, and inspect cost.

```bash
cargo run --release -- consensus --dry-run "What should a new Quant-M user inspect first?"
```

Copy the printed `session_id`, then run:

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

## Features

| Feature | Plain-English Meaning | Why It Matters |
| --- | --- | --- |
| Evidence trail | Quant-M writes local records of meaningful work. | You can inspect what happened instead of trusting vibes. |
| Replay | Re-check a run without repeating side effects. | Reviews become safer and less guessy. |
| Compact packets | Long sessions become small handoff files. | Another agent can continue from a durable packet, not a fading chat scroll. |
| Context Guardian | Watches local session evidence and prepares handoffs when risk is high. | Context rot becomes visible before it quietly damages work. |
| Worker proposals | Workers can submit evidence and suggestions. | Workers help without silently becoming the boss. |
| Policy gates | Risky actions stay behind explicit guardrails. | Chat text does not become authority. |
| Cost ledger | Dry runs and provider paths can leave cost records. | Cost becomes reviewable instead of mysterious. |
| Memory/context degradation | Stale or unsupported context is flagged. | Quant-M does not pretend old memory is fresh truth. |
| Local-first setup | The proof path runs on your machine. | No hosted broker or API key is required to understand the system. |
| Multi-model readiness | The runtime is built to record model and worker evidence. | Multiple models can contribute without erasing accountability. |
| Edge worker direction | Designed for laptops, SSH boxes, Raspberry Pi class devices, and Android/Termux-style nodes. | Agent work can move closer to the machine doing the job. |

## How Quant-M Compares

This is not a leaderboard. Quant-M is not claiming to beat coding agents, orchestration libraries, or terminal-task benchmarks at their own jobs. Its lane is local governance: evidence, replay, handoff, and operator control.

| Tool | Best At | Weak Spot | Quant-M Difference |
| --- | --- | --- | --- |
| Codex CLI | Coding tasks and repo edits. | Work can still disappear into long session history. | Quant-M preserves local evidence and handoff packets around the work. |
| Claude Code | Interactive coding and codebase reasoning. | Session continuity and proof records are still operator-managed. | Quant-M gives continuation a local record to inspect. |
| Aider/OpenCode | Fast code edits through familiar developer loops. | Less focused on governance and replay as first-class artifacts. | Quant-M treats replay and evidence as core runtime behavior. |
| CrewAI | Multi-agent orchestration patterns. | Orchestration can outpace proof if governance is bolted on later. | Quant-M starts from the audit trail. |
| LangGraph | Stateful agent graphs and application flow. | Requires app-level design around persistence and review. | Quant-M is a local control plane for evidence and operator decisions. |
| AutoGPT-style agents | Autonomous task attempts. | Unchecked autonomy can drift or hide weak evidence. | Quant-M favors governed execution over silent autonomy. |
| Hermes/OpenClaw-style harnesses | Agent harness ideas, tools, and runtime patterns. | Public examples vary in focus and operational proof. | Quant-M narrows the lane to local-first evidence, replay, compact packets, and edge readiness. |
| Quant-M | Evidence, replay, context handoff, local governance, and operator-controlled agent work. | Early beta, CLI-first, smaller ecosystem. | Built as a control plane, not just a task runner. |

## v0.1.0-beta Proof

These are local beta proof metrics, not industry leaderboard scores.

| Check | Result |
| --- | --- |
| Repo export size | 3.7M |
| Release binary | 4.2M |
| Fresh release build | 70.83s |
| Startup/help | 0.44s |
| Consensus dry run | 0.04s |
| Replay, compact, guardian, cost summary | Below timer precision in local benchmark |
| Tests | 236 library tests + 303 binary tests |
| Validation | fmt, clippy, tests, onboarding lint, README link check, secret scan, clean repo audit |

Verified from a clean local export. Empty-machine verification pending.

## Safety Defaults

Quant-M is conservative on purpose:

- It does not secretly call models.
- It does not auto-accept worker output.
- It does not treat chat text as authority.
- It does not trade.
- It does not need a hosted broker for the proof path.
- It keeps optional integrations behind configuration and policy checks.

## Useful Commands

```bash
# Setup and health
quant-m init
quant-m init-truth
quant-m setup --non-interactive --runtime-profile laptop
quant-m doctor

# Proof path
quant-m consensus --dry-run "What should we inspect first?"
quant-m replay <session_id>
quant-m compact <session_id>
quant-m context guard --json
quant-m cost summary

# Operator surfaces
quant-m agent
quant-m shell
quant-m tui
```

## Current Status

Status as of June 16, 2026: **v0.1.0-beta candidate**.

Use this as a public beta, not as production enterprise software. The current release is for local CLI users who want evidence, replay, context handoffs, cost visibility, and operator-controlled agent work.

Still developing:

- release binaries and install scripts
- fresh empty-machine Mac and Linux validation
- formal launchd/systemd autostart docs
- broader provider normalization
- worker federation
- distributed state

## Contributing

Contributions should preserve Quant-M's local-first boundary: no hidden provider calls, no implicit live execution, and no worker proposal auto-acceptance.

## License

MIT
