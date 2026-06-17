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

[Quick Start](#quick-start) | [How It Works](#how-it-works) | [Continuity Story](#continuity-story) | [Features](#features) | [Where It Fits](#where-quant-m-fits) | [Release Notes](docs/release/v0.1.0-beta.md)

## Why This Exists

Quant-M is Git-like history for AI work.

It is a flight recorder for agent sessions.

It helps agents continue without pretending they remember.

It favors governed execution over unchecked autonomy.

The goal is not flashy autonomy. The goal is durable intelligence: local evidence, replayable sessions, compact handoffs, degraded-context warnings, and human authority over worker output.

## How It Works

Long AI sessions tend to fail the same way: the work grows, the context window fills, and the next agent has to guess what mattered. Quant-M turns that into a local proof loop.

An agent works.

Quant-M records evidence.

When context becomes stale, Quant-M creates a compact packet.

The Context Guardian prepares a continuation handoff.

A new agent resumes from accepted facts instead of rereading thousands of lines.

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

Clone it and start locally:

```bash
git clone https://github.com/web5labs/Quant-M.git
cd Quant-M
./quantm
```

The launcher builds the local release binary if needed, prepares safe local state, and opens the Quant-M shell.

Inside the shell, try:

```text
demo
doctor
help
exit
```

If you paste outer-terminal commands while inside `quant-m>`, Quant-M will still run the local ones it recognizes. For example, `doctor`, `quant-m doctor`, and `./quantm doctor` all run the local doctor check.

The first run is intentionally local. No broker. No live model call. No browser harness. No hosted service. No API key.

## Onboarding Preview

Run the guided setup when you want the first-run questions instead of jumping straight into the shell:

```bash
./quantm onboard
```

For a throwaway demo config that will not touch your local setup:

```bash
./quantm --config /tmp/quant-m-demo.toml onboard
```

The flow is deliberately short: workspace, device type, network posture, model provider, developer tools, operator channel, continuity guard, and final review.

<details>
<summary><strong>HTML-style terminal preview</strong></summary>

<br>

<pre>
<strong>🧠 Welcome to Quant-M</strong>

<strong>Quant-M is a local-first Rust runtime for governed agent work.</strong>
It stores memory, sessions, shared state, and replay evidence on this device.
This device can run as a Quant-M Agent Node.

────────────────────────────────────────
<strong>Step 1: Workspace</strong>
Choose where local memory and sessions live.

<strong>Where should Quant-M store its local memory, state, sessions, and queues?</strong> [./workspace]:

────────────────────────────────────────
<strong>Step 2: Device</strong>
Pick the closest runtime profile.

<strong>Choose this device type.</strong>

   1   💻 Laptop or desktop
   2   📱 Phone / Termux / edge device
   3   🏢 Staff-OS worker node
   4   🖥️ VPS / server

<strong>Select</strong> [1]:

────────────────────────────────────────
<strong>Step 3: Network</strong>
Keep first-run safe unless you opt in.

<strong>Should Quant-M use the internet?</strong>

   1   🔒 No, local only
   2   ✋ Ask me before network use
   3   🌐 Yes, allow provider checks

<strong>Select</strong> [1]:

────────────────────────────────────────
<strong>Step 4: Models</strong>
Codex can work locally without OpenRouter.

<strong>Do you want to connect a model provider now?</strong>

   1   ⏭️ Skip for now
   2   🔑 Use OPENROUTER_API_KEY from my environment
   3   💾 Paste and save OpenRouter key locally
   4   📋 Show me the export command

<strong>Select</strong> [1]:

────────────────────────────────────────
<strong>Step 5: Developer tools</strong>
Detect Codex, Gemini, Claude, OpenCode, and Antigravity-style CLIs already on PATH.

<strong>Scan for supported developer tools?</strong>

   1   🔎 Yes, scan PATH
   2   ⏭️ Skip for now

<strong>Select</strong> [1]:

────────────────────────────────────────
<strong>Step 6: Operator channel</strong>
Choose the default way Quant-M talks to you.

<strong>How should Quant-M talk to you?</strong>

   1   ⌨️ Terminal
   2   🔗 Webhook later
   3   ✈️ Telegram later

<strong>Select</strong> [1]:

────────────────────────────────────────
<strong>Step 7: Continuity</strong>
Keep long sessions recoverable with local handoff packets.

<strong>Start context guardian automatically when the Quant-M daemon runs?</strong>

   1   🛡️ Yes, keep continuity handoffs ready
   2   ⏭️ No, I will run context guard manually

<strong>Select</strong> [1]:

╭─ Onboarding review ─────────────────────╮
│ workspace       ./workspace
│ device_type     laptop
│ network         local only / ask before live checks
│ model_provider  none
│ tools           codex
│ channel         none
│ guardian        enabled
│ config          /tmp/quant-m-demo.toml
╰─────────────────────────────────────────╯

<strong>Ready to save this onboarding profile?</strong>

   1   ✅ Save and continue
   2   🔁 Start over

<strong>Select</strong> [1]:

✓ Setup complete.

Next:
  ./quantm agent
  ./quantm doctor
  ./quantm demo
</pre>

For the fuller colored HTML version, open [`docs/onboarding-mockup.html`](docs/onboarding-mockup.html).

</details>

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

Quant-M keeps the homepage promise small and sharp:

- Evidence trail: see what happened and where the proof lives.
- Replay: re-check a run without repeating side effects.
- Compact packets: turn long sessions into small continuation files.
- Context Guardian: prepare handoffs when context is stale, risky, or too long.
- Cost ledger: inspect dry-run and provider-path costs locally.
- Policy gates: keep risky actions behind explicit operator control.
- API payload normalization: switch between OpenAI, Gemini, OpenRouter, local models, workers, and CLIs with a steadier state shape.
- Local-first setup: run the proof path without a hosted broker or API key.

## Where Quant-M Fits

Coding agents generate code.

Agent harnesses coordinate tools and workers.

Quant-M preserves evidence, replays work, normalizes payloads, tracks cost, and helps the next agent continue safely.

Codex, Claude Code, Antigravity CLI, and similar tools are better understood as tools that can run beside Quant-M, not competitors to Quant-M. Deeper harness comparisons and benchmark notes live in [BENCHMARKS.md](BENCHMARKS.md).

## API Payload Normalization

Quant-M treats model and tool output as untrusted until it is normalized. The user benefit is simple: you should be able to move between OpenAI, Gemini, OpenRouter, local models, workers, and CLI tools without rewriting your workflow records every time a payload shape changes.

That normalization layer is what makes Quant-M useful beside coding tools and agent harnesses. A model can be creative, a worker can be messy, and an API can change shape; Quant-M still preserves a consistent local record of what was actually accepted.

## v0.1.0-beta Proof

The beta proof path is intentionally small:

- 539 tests passing
- 3.7M clean repository export
- 4.2M release binary
- local-first proof path with evidence, replay, compact packets, context guardian handoff, and cost summary

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
# Start local shell
./quantm

# Run any CLI command through the launcher
./quantm doctor
./quantm demo

# Proof path
./quantm consensus --dry-run "What should we inspect first?"
./quantm replay <session_id>
./quantm compact <session_id>
./quantm context guard --json
./quantm cost summary

# Operator surfaces
./quantm agent
./quantm shell
./quantm tui
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
