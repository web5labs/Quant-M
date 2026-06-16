# Quant-M v0.1 Release Notes

Quant-M v0.1 marks the point where the project stops being framework-only architecture and becomes a usable local Rust agentic runtime.

## What v0.1 means

Quant-M v0.1 is reached when one local workflow can:

- execute end to end
- execute at least one registered skill
- write normalized shared state
- record durable session evidence
- replay cleanly without side effects
- run without external adapters, model calls, broker logic, or live trading

The proof workflow is:

- `workflow:mock-research-brief`

The proof domain is:

- `domain:mock-research`

## Included in v0.1

- local workflow execution through `quant-m run workflow <workflow_id>`
- typed skill, workflow, fsm, scheduler, and domain metadata
- normalized shared-state writes backed by redb plus SQLite history
- append-only session evidence with deterministic replay
- local-first CLI inspection that avoids unnecessary `forex.redb` locks

## Non-goals

- no live trading
- no broker integration
- no external adapter requirement
- no governance maze
- no desk-specific runtime dependency

## Edge validation checklist

Validate Quant-M v0.1 on:

- Raspberry Pi
- Termux Android
- VPS
- old laptop

For each target, confirm:

- the release binary builds successfully
- the mock-research workflow runs locally
- shared state is written
- session replay remains side-effect free
- no broker, model, or external adapter is required

## Validation commands

```bash
cargo build --release
./target/release/quant-m run workflow workflow:mock-research-brief
./target/release/quant-m session list
./target/release/quant-m session replay <session_id>
./target/release/quant-m state list
```

## Expected output examples

Workflow run:

```json
{
  "session_id": "session-<timestamp>-<seq>",
  "workflow_id": "workflow:mock-research-brief",
  "domain_id": "domain:mock-research",
  "status": "ok",
  "steps_completed": 1,
  "shared_state_writes": ["shared.research.summary"]
}
```

Replay:

```json
{
  "summary": {
    "final_status": "ok"
  },
  "state": {
    "current_fsm_state": "state:summary_drafted",
    "side_effects_replayed": false,
    "last_skill": "mock-research.capture-brief"
  }
}
```

State list:

```json
[
  {
    "key": "shared.research.summary",
    "domain_id": "domain:mock-research",
    "source": "workflow:workflow:mock-research-brief",
    "session_id": "session-<timestamp>-<seq>"
  }
]
```

## Next consumers after v0.1

- Staff OS worker
- research agent
- forex desk pack later
- crypto desk pack later

## PONboarding status

PONboarding is now treated as maintenance mode for the Quant-M roadmap:

- use it for onboarding, wiki generation, spec generation, and goal prompt generation
- prefer bug fixes and quality improvements over new feature expansion
