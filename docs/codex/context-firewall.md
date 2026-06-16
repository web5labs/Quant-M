# Context Firewall

## Goal

Minimize token leakage by making agents read only what is necessary, only when necessary, and only at the right FSM state.

The firewall sits between Quant-M project memory and any agent packet. It does not replace the wiki, compact packets, session evidence, or project spec. It decides which small, state-relevant subset is allowed into a packet.

## Token Leakage Definition

Token leakage includes:

- rereading the same docs across tasks
- loading full Markdown files when one section or summary would do
- passing stale, conflicting, or outdated context into a packet
- repeating planning work that already exists in an approved contract
- scanning the whole repo for a small task
- sending narrative context when a compact contract is enough
- sending implementation context before the FSM permits implementation
- asking implementation agents for long explanations when evidence summaries are enough

## Principle

No agent gets full context by default. Every agent gets a task packet. Every packet must justify every context item it includes.

Use this split:

- Memory is large.
- Contract is compact.
- Packet is tiny.
- Evidence is append-only.

## Context Tiers

### Tier 0: State Only

Allowed content:

- current FSM state
- allowed next action
- blocked actions
- next safe action

Use for:

- status checks
- blocked-state reports
- continuation checks

### Tier 1: Contract Only

Allowed content:

- JSON contract fields
- task goal
- constraints
- shippable criteria
- validation commands

Use for:

- ordinary implementation packets
- narrow documentation updates
- validation-only tasks

### Tier 2: Summary Only

Allowed content:

- wiki manifest summaries
- compact truth packet summaries
- context-status report
- loop dry-run candidates

Use for:

- planning a small slice
- choosing a relevant source file
- deciding whether context is stale

### Tier 3: Targeted Source Sections

Allowed content:

- exact Markdown sections
- exact source files or line-bounded excerpts
- relevant schema snippets

Use for:

- code edits
- schema edits
- high-confidence review tasks

### Tier 4: Full Source Context

Allowed content:

- whole files
- broader repo scans
- multi-file source reconstruction

Use for:

- audits
- migrations
- hard failures
- unclear ownership boundaries

Tier 4 must be rare and read-only unless a smaller implementation slice has already been approved.

## Packet Sizes

### Small Packet

Budget intent: state plus one task.

Allowed tiers: 0-1.

Use when:

- the task is already known
- validation command is known
- no source inspection is required before acting

### Medium Packet

Budget intent: state, contract, and relevant summaries.

Allowed tiers: 0-2.

Use when:

- the agent needs why the task matters
- the task needs a wiki summary or compact truth packet
- the slice can still be completed without full source context

### Large Packet

Budget intent: state, contract, summaries, and selected source excerpts.

Allowed tiers: 0-3.

Use when:

- the agent must edit or review specific files
- source sections are required for correctness
- the packet still avoids whole-project context

### Audit Packet

Budget intent: broader evidence review.

Allowed tiers: 0-4.

Use when:

- the task is an audit, repair, migration, or reconstruction
- the agent is not expected to implement broad unrelated changes
- the packet records why broad context is justified

If a task cannot fit into a small or medium packet, split the task unless the FSM state is explicitly audit, migration, or repair.

## Context Firewall Questions

Before adding context to a packet, ask:

- Does this agent need this file?
- Does this agent need the full file or a summary?
- Does this agent need source reasoning or only the contract?
- Is this context current?
- Is this context approved?
- Is this context relevant to the current FSM state?
- Will including this context improve output enough to justify the token cost?

If the answer is no, exclude it.

## Context Receipts

Every packet should save a receipt with:

- packet id
- current FSM state
- packet size
- allowed context tiers
- files included
- summaries included
- source sections included
- why each item was included
- what was excluded
- estimated token size
- expected output
- validation commands
- stop condition

Receipts should live under:

```text
workspace/state/context-packets/<packet_id>/receipt.json
workspace/state/context-packets/<packet_id>/packet.md
```

## No-Repeat Rule

Packets should reference stable compiled artifacts instead of restating them:

- project contract id
- shippable definition id
- validation manifest id
- compact packet id
- loop report id
- source evidence ids

Only include full text when the current FSM state and packet tier justify it.

## Context Decay Gate

Before a packet is issued, Quant-M should check:

- whether the relevant compact packet is present and current
- whether the contract still matches source files
- whether validation commands still exist
- whether summaries are stale relative to source files
- whether conflicts require operator review

If context is stale, regenerate or block the packet instead of sending stale assumptions.

## Reasoning Leakage Rule

Implementation packets should request short completion output:

- what changed
- files touched
- validation run
- risks remaining
- next recommended state

Long analysis belongs in planning, review, audit, or repair states, not ordinary implementation packets.

## MVP Slice

The first shippable Context Firewall slice should:

- compile one Markdown project spec into a compact JSON contract
- label fields as explicit, inferred, missing, or conflicting
- preserve evidence references for important fields
- validate the contract with Rust
- report current state and missing evidence
- generate the smallest valid agent packet
- include a context budget and receipt
- refuse to advance state without required evidence

## Integration Rules

- Reuse compact truth packets instead of creating a second summary layer.
- Reuse context-status and context-decay checks before packet generation.
- Reuse worker proposal records for non-authoritative agent packet outputs when possible.
- Reuse FSM descriptors for state and transition checks.
- Do not let packet generation mutate canonical project truth.
- Do not generate implementation packets before operator review when critical fields are missing, inferred, or conflicting.
