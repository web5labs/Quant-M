# Product State Machines

This document is a human-readable summary. Runtime FSM authority lives in Rust, currently in `src/fsm_core.rs`, `src/fsm_authority.rs`, and the wired runtime modules. Use `quant-m fsm authority` for the current wired/partial/modeled status.

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

Runtime authority: `WorkerJobFsm` in Rust.

The worker job FSM protects queue execution, retry/dead-letter behavior, and worker transition evidence. Detailed transition rules live in `src/fsm_core.rs`; this document keeps the human safety contract only.

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

## Skill Execution FSM

Runtime authority: `SkillExecutionFsm` and `PolicyApprovalFsm` in Rust.

The skill FSM makes local skill execution explicit: declaration and discovery are not permission, policy approval must be reached before shell-backed execution, and blocked shell skills are safety outcomes. Detailed transition rules live in `src/fsm_core.rs`.

Exit criteria for a safe skill slice:

- Skill declaration is not execution permission.
- Shell-backed skills require `skills.allow_shell_commands=true`.
- Policy approval state must reach execution allowed before a command starts.
- Blocked shell skills are safety outcomes, not runtime failures.

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

## Context Guardian FSM

Runtime authority: `ContextGuardianFsm` in Rust.

The Context Guardian FSM turns continuation state into typed action: observe, continue, compact, refresh compact, create handoff, request operator review, or block continuation. Green/yellow/red remains a display-level status, not machine authority. Detailed transition rules live in `src/fsm_core.rs`.

Exit criteria for a safe context slice:

- Green/yellow/red remains a display label, not the only runtime state.
- Context commands expose typed state, triggering event, recommended action, transition record, block flag, and review flag.
- New sessions can compact, refresh, hand off, or block from current project evidence without assuming a persistent terminal installation.
- Boil and loop dry-run treat typed `continue` as the only execution-ready context action.

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

## Workflow Cursor FSM

Runtime authority: `WorkflowCursorFsm` in Rust.

The workflow cursor FSM validates ordering for the existing `run workflow` path. Workflow descriptors explain intended steps, but Rust controls cursor transitions during execution. Invalid ordering, such as starting a second step while one is already running or completing a workflow before a step succeeds, is rejected.

This does not add workflow capabilities or side effects. Existing side-effect gates still apply to any side-effecting path reached by workflow execution.
