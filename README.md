# Quant-M

<p align="center">
  <img src="assets/quant-m-product.png" alt="Quant-M silver armored water bear agent mascot" width="340">
</p>

**Quant-M is a flight recorder and control plane for AI-assisted work. It preserves evidence, decisions, costs, and context so agents can continue safely instead of starting over.**

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

[Quick Start](#quick-start) | [Workflow](#workflow) | [Continuity Story](#continuity-story) | [Features](#features) | [Harness Comparison](#agent-harness-comparison) | [Release Notes](docs/release/v0.1.0-beta.md)

## Why This Exists

Quant-M is Git-like history for AI work.

It is a flight recorder for agent sessions.

It helps agents continue without pretending they remember.

It favors governed execution over unchecked autonomy.

The goal is not flashy autonomy. The goal is durable intelligence: local evidence, replayable sessions, compact handoffs, degraded-context warnings, and human authority over worker output.

## Workflow

Long AI sessions tend to fail the same way: the work grows, the context window fills, and the next agent has to guess what mattered. Quant-M turns that into a local proof loop.

| Step | What Happens | What Quant-M Leaves Behind |
| --- | --- | --- |
| 1 | An agent works through a long task. | Local session evidence |
| 2 | Context gets crowded or stale. | Degradation signals |
| 3 | Quant-M replays what happened. | Reviewable proof |
| 4 | The useful state gets compacted. | Compact packet |
| 5 | The Context Guardian prepares continuation. | Handoff file |
| 6 | A new agent resumes. | Accepted facts, not guesswork |

Quant-M does not try to make agents more magical. It makes their work easier to inspect, replay, resume, and stop.

## Continuity Story

Imagine an eight-hour research or coding session. The agent has inspected files, rejected bad paths, found useful evidence, and spent tokens getting there. Then the context window is nearly exhausted.

Without Quant-M, the next session starts by rereading chat history and hoping the summary is right.

With Quant-M, the run leaves behind session evidence, a replayable record, a compact packet, a continuity handoff, and cost records. A new agent can resume from the accepted facts instead of rebuilding the whole trail from memory.

## Why The Water Bear?

The water bear is not just a mascot. It is the product philosophy.

Tardigrades are famous for surviving brutal conditions: pressure, radiation, dehydration, cold, heat, and even space-like environments. Quant-M is built for the harsh parts of agent work: context loss, drift, stale memory, incomplete evidence, failed runs, and handoffs between models.

Most AI tools optimize for speed, autonomy, and more agents. Quant-M optimizes for survival, continuity, and resilience. The little armored creature is a reminder: make the work durable before making it autonomous.

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

| Feature | What You Get | Why It Matters |
| --- | --- | --- |
| Evidence trail | See what happened and where the proof lives. | You do not have to trust a chat summary. |
| Replay | Re-check a run without repeating side effects. | Reviews become safer and less guessy. |
| Compact packets | Turn long sessions into small continuation files. | Another agent can continue from durable state, not a fading chat scroll. |
| Context Guardian | Get handoffs when context is stale, risky, or too long. | Context rot becomes visible before it quietly damages work. |
| Worker proposals | Let workers suggest actions without silently taking over. | Agent help stays under operator control. |
| Policy gates | Keep risky actions behind explicit guardrails. | Chat text does not become authority. |
| Cost ledger | Inspect dry-run and provider-path costs locally. | Cost becomes reviewable instead of mysterious. |
| Memory/context degradation | Flag stale or unsupported context. | Quant-M does not pretend old memory is fresh truth. |
| Local-first setup | Run the proof path on your machine. | No hosted broker or API key is required to understand the system. |
| API payload normalization | Switch between OpenAI, Gemini, OpenRouter, local models, workers, and CLIs with a steadier state shape. | Provider and tool payloads can vary without corrupting your workflow records. |
| Multi-model readiness | Keep model and worker contributions attributable. | Multiple models can help without erasing accountability. |
| Edge worker direction | Run closer to laptops, SSH boxes, Raspberry Pi class devices, and Android/Termux-style nodes. | Agent work can move closer to the machine doing the job. |

## Agent Harness Comparison

This is not a leaderboard. Codex, Claude Code, Antigravity CLI, and similar coding tools are better understood as tools that can be used with Quant-M, not competitors to Quant-M.

The closer comparison is with agent harnesses and local agent runtimes such as OpenClaw, Hermes Agent, and ZeroClaw. Quant-M's lane is local governance: evidence, replay, handoff, payload consistency, token-aware continuity, and operator control.

| Harness | Best At | Typical Channel | Tool-Use Style | Token / Context Posture | Quant-M Difference |
| --- | --- | --- | --- | --- | --- |
| OpenClaw | Autonomous local assistant workflows and broad tool integration. | Messaging and gateway-style control. | Agent acts through skills/tools. | Strong long-session ambition; token saving depends on configured memory and prompts. | Quant-M is narrower: evidence, replay, compact packets, and safer continuation before autonomy. |
| Hermes Agent | CLI agent harness and agent runtime experimentation. | Terminal/CLI. | Agent drives tools through a harness. | Context handling depends on the selected harness/model loop. | Quant-M records what happened as structured local evidence and replayable state. |
| ZeroClaw | Lightweight claw-style experimentation. | Terminal/CLI. | Minimal harness/tool loop. | Usually optimized for small surface area rather than durable handoff proof. | Quant-M adds policy gates, cost records, compact packets, and context-guardian handoffs. |
| Quant-M | Evidence, replay, context handoff, local governance, typed payloads, and operator-controlled agent work. | CLI now; edge workers and channels later. | Workers propose; Quant-M records, gates, normalizes, and replays. | Compact packets and handoffs reduce context reloads; exact saved tokens are benchmark-pending by model/channel. | Built as a control plane, not just a task runner. |

## Comparison Metrics

Peer rows are intentionally marked as pending unless measured by the same local harness on the same machine. Quant-M numbers below are measured from the v0.1.0-beta clean local export.

| Metric | OpenClaw | Hermes Agent | ZeroClaw | Quant-M v0.1.0-beta |
| --- | --- | --- | --- | --- |
| Repo size | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 3.7M clean export |
| Release/binary size | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 4.2M release binary |
| Fresh build speed | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 70.83s release build |
| Startup/help speed | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | 0.44s |
| Proof-loop speed | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | consensus dry run 0.04s; replay/compact/guardian/cost below local timer precision |
| Tool-use posture | Broad autonomous tool/skill use | Harness-driven tool use | Lightweight tool loop | Tool use is gated, recorded, and replay-aware |
| Channel posture | Messaging/gateway-oriented | CLI-oriented | CLI-oriented | CLI now; channels stay policy-gated |
| Token-saving posture | Pending same-harness measurement | Pending same-harness measurement | Pending same-harness measurement | Compact packets and handoffs reduce context reload; exact saved tokens pending model/channel benchmarks |
| API consistency | Depends on harness and adapters | Depends on harness and adapters | Depends on harness and adapters | Typed payload normalization before state, replay, cost, and handoff writes |
| Safety model | Autonomy needs careful sandboxing | Harness-dependent | Harness-dependent | Local-first, no hidden model calls, workers propose, operator decides |

## API Payload Normalization

Quant-M treats model and tool output as untrusted until it is normalized. The user benefit is simple: you should be able to move between OpenAI, Gemini, OpenRouter, local models, workers, and CLI tools without rewriting your workflow records every time a payload shape changes.

The process is simple:

1. Receive a model, worker, CLI, or tool payload.
2. Parse it into a typed Quant-M record.
3. Validate required fields, policy tags, timestamps, session IDs, and domain metadata.
4. Write accepted facts to shared state, cost records, session evidence, or compact packets.
5. Keep malformed or risky payloads out of canonical state.

That normalization layer is what makes Quant-M useful beside coding tools and agent harnesses. A model can be creative, a worker can be messy, and an API can change shape; Quant-M still tries to preserve a consistent local record of what was actually accepted.

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
