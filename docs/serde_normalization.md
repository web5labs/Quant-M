# Serde Normalization Doctrine

Quant-M is Rust-native and Serde-first.

This doctrine defines how external shapes enter the runtime.

## Core rule

Raw payloads are intake only.

Runtime truth must be typed Rust structs.

## Non-negotiable rules

1. Use Serde-backed Rust structs for API, tool, CLI, MCP, and markdown-derived runtime data whenever the runtime acts on that data.
2. Parse and retain only the fields Quant-M actually needs for the current slice.
3. Ignore unused endpoint bulk by default.
4. Expand endpoint coverage only when a real task requires more fields.
5. Do not let OpenRouter, MCP, API, webhook, or markdown source shape leak into core runtime logic.
6. Shared state should receive normalized records, not raw endpoint blobs.
7. Repetitive tasks should become FSMs.
8. Ambiguity-rich tasks may use LLMs, but the resulting runtime actions must still land in typed records and session evidence.

## Practical interpretation

- Good:
  - `worker` JSON -> `WorkerJob`
  - Forex quote JSON -> `StoneXQuotePayload` -> typed `IngestResult`
  - workflow execution -> typed `WorkflowDescriptor`/`SkillDescriptor` -> typed shared-state record
- Bad:
  - passing `serde_json::Value` deep into runtime decisions
  - storing raw endpoint blobs as if they were runtime truth
  - binding core logic directly to third-party response shape

## Current repo audit

Normalized and aligned:

- `src/worker.rs`
  - parses JSON into `InboundWorkerJob` and `WorkerJob`
- `src/forex.rs`
  - parses provider responses into typed Serde structs and maps only needed fields
- `src/execution_runtime.rs`
  - runs typed workflow/skill/state/session descriptors without raw API payloads
- `src/sessions.rs`
  - runtime evidence is typed and replay-safe

Intentional intake bridges still carrying generic JSON:

- `src/state_sql.rs`
  - `SharedSignalInput.payload_json`
  - `DeskHandoffInput.evidence_json`
  - `DeskHandoffInput.risk_flags_json`
  - `PaperOrderInput.details_json`

These are acceptable as intake/storage bridges today, but they should not become core runtime truth without typed normalization in the desk slice that needs them.

## Enforcement mindset

- Normalize first.
- Execute second.
- Persist typed truth.
- Replay from typed evidence.

If a new slice cannot explain where normalization happens, it is not ready to merge.
