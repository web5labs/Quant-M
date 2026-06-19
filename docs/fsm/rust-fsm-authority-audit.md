# Rust FSM Authority Audit

Slice: `RUST_FSM_AUTHORITY_01`, updated by `CONTEXT_GUARDIAN_FSM_01` and `WORKFLOW_CURSOR_FSM_01`

This audit records what is wired today. Markdown explains intent and examples; Rust is the authority for repeatable runtime state transitions. The machine-readable summary is available with `quant-m fsm authority --json`.

## Current Wiring

| Area | Current Rust types | String statuses | Markdown-only docs | Command surface | Artifacts written | Transition validation | Invalid transition rejection | Session evidence | Replay reconstruction | Maturity |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Worker jobs | `WorkerJob`, `WorkerResult`, `WorkerJobState`, `WorkerJobEvent`, `WorkerJobFsm` | Result status remains `ok`/`error`; legacy transition `completed` parses as `succeeded` | Product FSM docs describe worker safety | `worker run-once`, `worker loop`, heartbeat task execution | queue inbox/outbox, inflight, dead-letter, session logs | Typed start/complete/fail/retry/dead-letter transitions | Typed FSM rejects invalid events and terminal-state events | Worker execution emits `SessionEvent::FsmTransition` from typed FSM | Replay reads transition evidence and maps to typed session state | wired |
| Sessions/replay | `SessionEvent`, `SessionReplayState`, `SessionLifecycleState` | Legacy `final_status` remains for compatibility | Compaction and product FSM docs describe lifecycle intent | `session list`, `session show`, `replay`, `resume-plan`, `compact` | session NDJSON, compact packets | Compatibility layer computes typed final state from events | New typed FSM rejects invalid transitions; old logs remain readable | All lifecycle evidence remains in session NDJSON | Replay computes `typed_final_state` and never repeats side effects | partial |
| Worker proposals | `WorkerProposalStatus`, `WorkerProposalEvent`, `transition_worker_proposal_status` | Stored status enum serializes as strings | Worker proposal doctrine appears in feature docs | `worker proposal submit/list`, question/strategist proposal paths | proposal JSON artifacts and index | Review/accept/reject/needs-info/supersede transitions are validated | Invalid jumps, including rejected-to-accepted, fail | Proposal-producing flows create session/proposal evidence indirectly | Proposal records are replay-friendly JSON but not canonical truth | wired |
| Workflow execution | `WorkflowDescriptor`, `WorkflowStepDescriptor`, `FsmDescriptor` metadata, `WorkflowCursorState`, `WorkflowCursorEvent`, `WorkflowCursorFsm` | Step ids and descriptor states are string ids | Product FSM docs summarize cursor safety | `workflow list/show`, `run workflow`, mock domain workflows | workflow/session/domain artifacts | `run workflow` validates cursor order; descriptor validation remains separate | Unknown descriptor refs and invalid cursor transitions are rejected | Workflow runs emit `workflow_cursor` transition evidence | Replay can inspect emitted cursor events and does not repeat side effects | partial |
| Policy/approval | `PolicyDescriptor`, `PolicyDecision`, `PolicyApprovalState`, `PolicyApprovalEvent`, `PolicyApprovalFsm` | Policy decisions serialize as enum/string values | Governance docs describe approval boundary | `policy list/show/evaluate-skill`, operator decision commands | session policy events, operator decisions | Typed approval FSM exists and is tested | Blocked, denied, and approval-pending states cannot execute | Policy decisions and operator decisions are session events | Replay treats approval as evidence, not execution | partial |
| Context Guardian | `ContextGuardianState`, `ContextGuardianEvent`, `ContextRecommendedAction`, `ContextGuardianFsm`, `ContextStatusReport`, `ContextGuardianReport` | User-facing green/yellow/red status remains a display field | Context docs describe intended lifecycle | `context-status`, `context guard`, `compact`, `context packet`, `boil`, `loop --dry-run` | compact packets, context reports, packet receipts, handoffs, boil reports | Typed context transitions validate compact, stale, review, handoff, and blocked states | Invalid guardian events are rejected; blocked context is terminal | Status/guardian reports include typed transition records derived from session and compact evidence | Reconstructs from latest session and compact packet without side effects | guarded/wired |
| Shared state review | `SharedStateRecord`, `StateReviewReport`, hybrid store types | Review/staleness concepts remain field/string based | Shared-state docs describe review intent | `state`, `state-review`, `state snapshot` | shared-state DB/JSON snapshots | Store validation exists; no typed fact lifecycle FSM yet | Invalid storage inputs fail; lifecycle jumps not centralized | Some state changes can be cited in sessions | Snapshots/reports are inspectable but not a typed FSM replay | partial |
| Skills execution | `SkillDescriptor`, `SideEffectLevel`, `SkillExecutionState`, `SkillExecutionEvent`, `SkillExecutionFsm`, `PolicyApprovalFsm` | `SessionEvent::SkillCall.status` remains a compatibility/display string | Skill docs describe authoring and gates | `skills list/show/run`, `policy evaluate-skill` | typed `skill_execution`/`policy_approval` session transitions, skill call/output events | Runnable shell-backed skills validate load, policy, ready, running, succeeded/failed/blocked transitions | Invalid skill lifecycle transitions fail; shell-disabled skills become `blocked`, not `failed` | `SessionEvent::FsmTransition`, `PolicyDecision`, `SkillCall`, and output/error events | Replay reads typed transition evidence and never executes commands | guarded/wired |
| Provider/tool onboarding | Config provider fields, capability registry dynamic checks | Provider/tool statuses are capability labels/config strings | Onboarding docs describe modes and provider choices | `onboarding`, `doctor`, `capabilities`, config commands | `quant-m.toml`, capability output | Detection and config checks are separated | Detection does not grant permission; provider calls stay gated | Capability checks are inspect-only | No provider validation calls during status/capability checks | partial |
| Question/consensus/strategist | Question/consensus/strategist report structs, proposal records | Dry-run/status fields remain string based | Feature docs describe question-to-proposal path | `question ask`, `consensus --dry-run`, `strategist dry-run` | session logs, consensus reports, proposals, cost ledger | Boundaries/policies/dry-run checks exist | Channel/worker command attempts are rejected | Dry-run and proposal paths emit session/proposal/cost evidence | Replay validates session artifacts without provider calls | partial |

## First Enforced Rust FSMs

- Worker job transitions are now typed through `WorkerJobFsm`.
- Worker execution emits transition evidence from Rust enum states/events while preserving the old session event shape.
- Skill execution emits typed lifecycle and policy approval transition evidence before any shell command can start.
- Session replay computes a typed `SessionLifecycleState` while retaining the legacy `final_status` string for older artifacts.
- Policy/approval FSM exists as a typed safety model and test target.
- Worker proposal review transitions are validated with an explicit Rust transition function.
- Context Guardian now emits typed state, event, recommended action, transition evidence, block flag, and operator-review flag while preserving green/yellow/red display labels.
- Workflow runtime cursor transitions are validated with `WorkflowCursorFsm` for the existing `run workflow` path.

## Compatibility Decisions

- Existing session logs remain readable.
- `typed_final_state` is session machine authority; `final_status` is legacy/display compatibility.
- Legacy worker transition state `completed` parses as canonical `succeeded`.
- Existing worker result statuses remain `ok` and `error` for queue/outbox compatibility.
- Proposal artifacts still serialize statuses as snake-case strings, but status changes now have a typed transition validator.
- Existing `SessionEvent::SkillCall.status` remains a string for compatibility; typed skill lifecycle truth is emitted through `SessionEvent::FsmTransition`.
- Context Guardian typed fields are machine authority; green/yellow/red is display-level status.
- Markdown FSM files remain human-readable summaries; they are no longer described as runtime authority.

## Not Yet Wired

- Workflow descriptor inspection remains metadata-only; only the existing `run workflow` runtime path is cursor-wired.
- Shared-state fact lifecycle is not centralized in a typed FSM yet.
- Provider/tool onboarding has typed capability truth but not a dedicated onboarding FSM.
