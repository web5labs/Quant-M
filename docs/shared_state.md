# Shared State

Quant-M shared state is the small, reusable runtime layer that lets different domains keep current facts without turning every fact into a session event, wiki page, or bespoke desk table.

## What shared state is

Shared state is for current, reusable runtime knowledge:

- latest scores
- current statuses
- lightweight domain facts
- expiring coordination values
- session-linked state that later runtime steps may need to read

It is not the full audit trail and it is not the long-form operating doctrine.

## Serde normalization rule

Shared state should receive normalized records, not raw blobs.

That means:

- API or tool payloads may arrive as raw JSON at intake
- runtime logic should normalize only the needed fields into typed Rust structs
- shared state should receive the normalized result, not the original endpoint shape

If the runtime cannot explain the typed meaning of a state value, that value probably does not belong in shared state yet.

## What belongs in SQLite

SQLite is the durable, auditable history lane.

Use SQLite for:

- append-only history of shared-state changes
- durable records you may want to inspect later
- operator or agent forensics
- evidence that a hot-state value existed at a point in time

SQLite should answer:

- what changed
- when it changed
- which session produced it
- what value was written or expired
- what current shared-state view inspection can reconstruct without touching hot runtime files

## What belongs in redb

redb is the hot runtime state lane.

Use redb for:

- latest current value for a key
- fast local reads during runtime decisions
- expirable coordination values
- deterministic snapshots of current state

redb should answer:

- what is true right now
- what is the latest value for this key

## What belongs only in session logs

Session logs are for execution evidence, replay, and audit.

Keep these in session logs only:

- ordered runtime events
- retries and failures
- operator approvals and denials
- replayable execution context
- detailed step-by-step causality

If a fact needs exact event order, it belongs in session logs.

## What belongs only in wiki or docs

Wiki and docs are for human-readable doctrine and durable understanding.

Keep these there:

- operating rules
- product decisions
- architecture notes
- desk doctrine
- onboarding rails

If a fact is explanatory rather than runtime-active, it belongs in docs or wiki.

## Anti-overengineering rules

- Shared state is not a workflow engine.
- Shared state is not a policy maze.
- Shared state does not replace session logs.
- Shared state does not replace the wiki.
- Shared state stays domain-neutral.
- Shared state should not become a parking lot for raw endpoint payloads.
- Current state lives in redb; durable history lives in SQLite.
- Add new state keys only when more than one step or component benefits from them.
- Prefer typed values and small records over nested speculative schemas.
- If a state fact is desk-specific and not reusable, keep it in the desk layer instead of lifting it into core.
