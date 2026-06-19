# Workflow Cursor FSM Audit

Slice: `WORKFLOW_CURSOR_FSM_01`

Purpose: validate existing workflow runtime step order with a typed Rust FSM. This does not add workflow capabilities, providers, shell execution, network behavior, trading behavior, or new side effects.

## Coverage

| Surface | Classification | Notes |
| --- | --- | --- |
| `quant-m run workflow <workflow_id>` | runtime_wired | Uses `WorkflowCursorFsm` to prepare, start steps, complete steps, block failures, and complete the workflow. |
| `workflow list` | descriptor_only | Inspects registered descriptors; no runtime cursor is needed. |
| `workflow show` | descriptor_only | Inspects registered descriptors; no runtime cursor is needed. |
| Mock research workflow | mock_only/runtime_wired | Existing local execution path is cursor validated and emits session FSM evidence. |
| Mock trading workflow | mock_only/needs_future_work | Descriptor exists, but local execution for its skills is not implemented. Cursor blocks unsupported execution through the shared runtime failure path. |
| Replay | typed evidence only | Replay reads session evidence and does not repeat workflow side effects. |

## Runtime Truth

Markdown describes workflow intent. Rust validates cursor order with `WorkflowCursorState`, `WorkflowCursorEvent`, and `WorkflowCursorFsm` in `src/fsm_core.rs`.

Invalid step ordering, including starting a second step while one is already running or completing a workflow before a step succeeds, is rejected by the typed FSM.

## Authority Status

`workflow_cursor` is `partially_wired`: the existing `run workflow` execution path is wired, while descriptor inspection remains read-only metadata inspection.
