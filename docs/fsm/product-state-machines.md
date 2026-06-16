# Product State Machines

This file records the product/runtime state machines that matter for Quant-M implementation slices. It complements `docs/fsm/project-execution-fsm.md`, which controls agent project work.

## Purpose

- Keep runtime states explicit before adding new execution behavior.
- Route future implementation through existing state evidence instead of ad hoc flags.
- Preserve the local-first safety boundary: policy blocks, replay, approval, and execution are separate states.

## Current Product Checkpoint

- Project onboarding state: `READY_FOR_FUNCTIONAL_BUILD_GOAL`.
- Current execution-plan state: `QUESTION_TO_WORKER_PROPOSAL_01_VALIDATED`.
- Latest sampled runtime session: `workspace/state/sessions/session-1781527274619145-159.ndjson`.
- Latest sampled runtime outcome: `worker_job` failed safely because `worker.http_get_sandbox_hosts` denied `example.com`.
- Latest compacted checkpoint: `workspace/state/compacted/session-1781135013000105-67/`.
- Latest loop checkpoint sampled: `workspace/state/loops/loop-1781332719284737/`.

## Worker Job FSM

Known states:

- `queued`
- `executing`
- `succeeded`
- `failed`
- `dead_lettered`

Known transitions:

- `queued -> executing`: worker starts a queued job.
- `executing -> succeeded`: job completes and records durable evidence.
- `executing -> failed`: job fails or a policy gate denies execution.
- `failed -> dead_lettered`: retry budget is exhausted and the job is moved out of the active queue.

Exit criteria for a safe worker slice:

- Every execution attempt records a session event.
- Policy decisions are recorded before side effects.
- Failed jobs preserve enough evidence to replay or explain the failure.
- Replays remain read-only and must not re-trigger side effects.

## Session Evidence FSM

Known states:

- `created`
- `recording`
- `completed`
- `failed`
- `compacted`
- `replay_only`
- `resume_plan_ready`

Known transitions:

- `created -> recording`: first event is appended.
- `recording -> completed`: terminal success evidence is recorded.
- `recording -> failed`: terminal failure or policy block evidence is recorded.
- `completed -> compacted`: a compact truth packet is generated.
- `failed -> compacted`: a compact truth packet is generated for a failed session.
- `compacted -> replay_only`: checkpoint is used for analysis without side effects.
- `replay_only -> resume_plan_ready`: analysis produces a next safe action, not execution.

Exit criteria for a safe session slice:

- Session files stay append-only.
- Compacted packets cite source evidence.
- Resume plans are analysis-only until explicitly approved.
- Operator approval is stored as evidence, not treated as automatic execution.

## Question To Proposal FSM

Known states:

- `question_received`
- `evidence_collected`
- `proposal_planned`
- `policy_reviewed`
- `pending_review_written`
- `rejected`

Known transitions:

- `question_received -> evidence_collected`: local context is gathered.
- `evidence_collected -> proposal_planned`: a bounded worker proposal plan is produced.
- `proposal_planned -> policy_reviewed`: proposal write or execution implications are checked.
- `policy_reviewed -> pending_review_written`: `--write-proposals` records non-authoritative pending proposals.
- `policy_reviewed -> rejected`: the request is out of scope or unsafe.

Exit criteria for the current validated slice:

- Default question handling stays inspect-only.
- Proposal writes require explicit `--write-proposals`.
- Written proposals remain pending and non-authoritative.
- Staff-OS handoff and harness execution remain separate milestones.

## Shared State FSM

Known states:

- `uninitialized`
- `hot_state_ready`
- `history_recorded`
- `expired`
- `snapshot_ready`

Known transitions:

- `uninitialized -> hot_state_ready`: state store opens and accepts typed keys.
- `hot_state_ready -> history_recorded`: a durable SQLite history row is written.
- `hot_state_ready -> expired`: an expirable fact is no longer current.
- `history_recorded -> snapshot_ready`: current state and history are summarized for inspection.

Exit criteria for a safe shared-state slice:

- Runtime facts, durable history, execution evidence, and doctrine remain separate lanes.
- Read-only inspection must not require exclusive runtime storage locks.
- Raw payloads are normalized before they become shared-state truth.

## Policy Boundary

Any transition that can cause shell execution, HTTP access, LLM/provider calls, Telegram delivery, broker interaction, or live trading must pass through an explicit policy decision event before the side effect occurs.

Policy-blocked sessions are successful safety evidence, not failed onboarding.
