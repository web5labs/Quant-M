# Project Spec

## 1. Product summary

Quant-M is a lightweight, local-first Rust runtime that provides a memory system, heartbeat loop, channel adapters, local skills registry, and desk-oriented shared state without requiring a full web platform. This onboarding slice documents the current product boundary so later implementation can improve the runtime safely on a real repo.

## 2. Target user

- Primary: a technical operator or builder running a lean worker node locally or on small infrastructure.
- Secondary: a desk owner who needs structured state, handoffs, and optional LLM/channel integrations without a heavy platform.

## 3. Core problem

Small automation runtimes often sprawl into unsafe or unclear territory: shell execution gets enabled casually, network behavior is underspecified, runtime memory is fragmented, and domain-specific workflows become ad hoc scripts. Quant-M needs explicit project rails so future work strengthens the runtime rather than diluting it.

## 4. MVP outcome

The functional MVP for this repo should preserve and harden the current runtime surface:

- memory files plus searchable SQLite memory
- queue-based worker execution
- heartbeat-driven periodic task execution
- append-only session history with deterministic replay
- replay-to-resume analysis that stays read-only and operator-gated
- operator approval resolution that records human decisions without auto-executing follow-up work
- domain-pack contracts that let domains plug in as use cases instead of becoming core identity
- storage-mode boundaries that keep read-only inspection off exclusive desk storage
- skill-registry contracts that let domains advertise inspectable skills without embedding execution policy into core
- workflow-registry contracts that let domains describe inspectable plans above skills and shared state before execution exists
- fsm-registry contracts that let domains describe deterministic transitions above workflows and shared state before execution exists
- scheduler-registry contracts that let domains describe inspectable cron, polling, mtime, event, and manual timing plans before execution exists
- desk-pack frameworks that let domains package future desks as metadata without hardcoding desk behavior into core
- execution-runtime slices that prove one local workflow can execute end-to-end through the existing framework contracts
- policy-registry contracts that decide allow, approval, or deny from skill metadata before execution exists
- shared-state contracts that split hot runtime facts, durable history, session evidence, and human doctrine cleanly
- terminal and optional webhook event adapters
- terminal-cockpit planning that maps Android to Termux windows, macOS to CMUX, and Linux/Windows to TMUX without making any cockpit the orchestrator
- local-only skills registry
- optional LLM and Telegram integration behind explicit config
- shared SQL state and forex handoff persistence
- project docs and validation rails that keep the next implementation slices bounded

For the current freeze point, Quant-M v0.1 means one local workflow can execute end to end, write normalized shared state, record durable session evidence, replay cleanly without side effects, and run without brokers, models, external adapters, or live trading.

## 5. User stories

- As an operator, I want a local worker runtime that stores memory and state durably without requiring a large platform.
- As an operator, I want scheduled heartbeat tasks and queue jobs to run with explicit guardrails around shell and network access.
- As an operator, I want sessions to persist so I can inspect failures, replay what happened, and resume work without hidden state.
- As an operator, I want Quant-M to tell me whether a session is complete, blocked, replay-only, or interrupted before I approve any follow-up run.
- As an operator, I want my approval or denial to become durable session evidence without turning approval into execution.
- As a builder, I want Quant-M to host multiple domains through a stable contract so trading desks stay optional use cases, not hardcoded runtime assumptions.
- As a builder, I want skills to be registered as metadata first so routing, inspection, and policy review can happen before any execution layer exists.
- As a builder, I want workflows to be registered as metadata first so future execution can route through explicit steps, shared-state reads/writes, and skill references instead of ad hoc scripts.
- As a builder, I want fsms to be registered as metadata first so deterministic state transitions stay inspectable, typed, and separate from execution.
- As a builder, I want schedulers to be registered as metadata first so domain timing can stay explicit and inspectable before any background execution loop is introduced.
- As a builder, I want desk packs to package domains, skills, workflows, fsms, and schedulers into reusable use-case boundaries without hardcoding real desks into core.
- As a builder, I want one local workflow to execute end-to-end through the existing framework so Quant-M is proven as software, not just metadata architecture.
- As a builder, I want policies to evaluate skill metadata before execution exists so future runtimes can route safely by default.
- As an operator, I want shared runtime state to stay typed, auditable, and separate from both session logs and wiki doctrine.
- As a builder, I want project docs that tell agents which files are authoritative before they touch runtime code.
- As a desk owner, I want structured handoff records and shared state for forex workflows before considering any live execution path.

## 6. Core user flow

1. Initialize the runtime workspace and default config.
2. Add memory or workspace instructions that shape the node's identity and task behavior.
3. Submit a narrow worker job or run a heartbeat tick.
4. Observe structured adapter output, persisted session events, and shared state updates.
5. Use optional skills, LLM, Telegram, or forex state flows only when configuration allows them.
6. Validate the runtime and document blockers before expanding capabilities.

## 7. Human intent lock

- What this product is:
- A lean Rust runtime for safe local automation, memory, heartbeat tasks, and structured handoffs.
- What this product is not:
- Not a general-purpose agent platform, remote plugin marketplace, or polished dashboard product.
- What must not be rewritten:
- The local-first safety posture, especially default-disabled shell and HTTP execution.
- The clear separation between core runtime features and optional integrations.
- What should be preserved:
- Workspace markdown memory, durable state, and deterministic queue/heartbeat behavior.
- Explicit config gating for risky capabilities.
- Desk-specific docs and handoff shapes when the slice touches forex workflows.
- What "done" means for the current slice:
- The current slice changes the smallest viable runtime boundary, updates docs, and leaves behind validation or a durable verifier.
- What can be deferred:
- Any browser UI, orchestration dashboard, or live-trading automation that is not required for the current verified runtime slice.
- Current freeze rule:
- Quant-M Core is frozen at v0.1 until constrained-hardware validation is complete.

## 8. Functional requirements

- The runtime must support workspace bootstrap and durable local memory.
- The runtime config must support root-relative paths and environment-variable overrides for portable deployments.
- The runtime must expose a terminal-native onboarding/config flow for init, setup, typed config inspection, config validation, and local doctor checks without hidden network or model calls.
- The runtime must run cleanly inside a plain terminal multiplexer or future Staff OS worker lane using non-interactive setup and stable command outputs.
- The runtime must expose a read-only terminal-cockpit plan that chooses Termux windows on Android, CMUX on macOS, TMUX on Linux/Windows, and plain terminal fallback elsewhere.
- Terminal cockpit planning must preserve Quant-M as the source of truth for shared state, session evidence, policy, and workflow/FSM state; terminal surfaces may only host or preview lanes.
- The runtime may expose a thin optional operator-facing agent shell as long as the stable automation contract remains the existing CLI commands.
- The runtime may expose a thin optional operator-facing TUI shell as long as the stable automation contract remains the existing CLI commands.
- The runtime must execute queue jobs and one-shot worker jobs with bounded retries and timeouts.
- The runtime must support heartbeat task parsing and execution from `workspace/HEARTBEAT.md`.
- The runtime must emit structured adapter output to terminal and optional webhook targets.
- The runtime must keep skills local to `workspace/skills/` with no remote marketplace dependency.
- The runtime must persist shared state and handoff records for desk workflows.
- The runtime must persist append-only session events with typed identifiers for session, run, agent, step, and domain.
- The runtime must support deterministic session replay that does not execute side effects.
- The runtime must analyze persisted session events into a resume plan without executing side effects or auto-resuming work.
- The runtime must persist operator approval, denial, and needs-more-info decisions as session events and reflect them in resume-plan analysis only.
- The runtime must expose a narrow domain-pack contract for registering domain metadata, capabilities, skills, workflows, fsms, schedulers, desk packs, and policies without granting live execution by default.
- The runtime must separate read-only inspection from runtime preflight so domain and session inspection commands do not open desk runtime stores like `forex.redb`.
- The runtime must expose a skill-registry contract for registering skill descriptors, filtering them by domain and side-effect level, and inspecting them without execution.
- The runtime must expose a workflow-registry contract for registering workflow descriptors, filtering them by domain, and inspecting step metadata without execution.
- The runtime must expose an fsm-registry contract for registering fsm descriptors, filtering them by domain, and inspecting deterministic transition metadata without execution.
- The runtime must expose a scheduler-registry contract for registering scheduler descriptors, filtering them by domain and trigger kind, validating cadence fields, and inspecting timing metadata without execution.
- The runtime must expose a desk-pack framework for registering desk descriptors, filtering them by category and domain, validating references across skill/workflow/fsm/scheduler registries, and inspecting packaged desk metadata without execution.
- The runtime must expose a minimal local execution lane for running at least one registered workflow end-to-end through registered skills, shared state, fsm/session evidence, and replay-safe logging.
- The runtime must support a v0.1 proof path through `workflow:mock-research-brief` without requiring external adapters, model calls, broker logic, or live trading.
- The runtime must expose a policy-registry contract for registering policy descriptors, filtering them by domain and side-effect level, and evaluating skill descriptors without execution.
- The runtime must expose a shared-state contract with typed keys and values, hot redb-backed current state, and SQLite-backed durable history.
- Optional LLM, Telegram, shell, and HTTP features must remain explicitly gated behind configuration.
- The onboarding flow must store operator preferences for local model, remote model, OpenRouter model, external channel, runtime profile, and session path as typed Serde-backed config data for future consumers.
- The runtime must keep future orchestration integration behind a narrow adapter or handoff boundary rather than entangling the core worker with a heavy control plane.

## 9. Non-functional requirements

- Local-first by default.
- Portable across copied workspaces, laptops, Raspberry Pi nodes, and Termux-style Android environments without hard-coded machine paths.
- Small enough to understand and validate without a giant platform dependency graph.
- Deterministic enough that crashes, retries, and queue recovery are auditable.
- Honest about which capabilities are real, which are optional, and which are still paper-only or sandbox-only.
- Replay-safe enough that audit and recovery flows do not re-trigger external actions.
- Conservative enough that resume-readiness stays analysis-only until a future explicit resume execution slice is approved.
- Explicit enough that operator intent is auditable and separate from actual runtime execution.
- Stable enough that future desk domains can plug into the runtime without forcing forex-first abstractions into core.
- Safe enough that operators and agents can inspect runtime evidence in parallel without tripping exclusive desk-storage locks.
- Narrow enough that future workflow and fsm execution can route through stable metadata rather than ad hoc desk-specific wiring.
- Explicit enough that future desk timing can stay domain-specific instead of collapsing cron, polling, mtime, and event behavior into one global scheduler assumption.
- Packaged enough that future desks can ship as reusable metadata bundles instead of leaking desk-specific assumptions into core runtime paths.
- Small enough that the first execution proof can run fully local on edge hardware before any external adapters or model providers are required.
- Typed enough that external payload shape is normalized before runtime use and does not become the framework's source of truth.
- Explicit enough that future execution can defer to policy evaluation results instead of inferring approval rules from domain code.
- Clear enough that current runtime facts, durable history, execution evidence, and doctrine each live in the right storage lane.
- Token-efficient enough that future agents receive state-specific packets instead of broad project memory by default.
- Stable enough to freeze the core after v0.1 proof and validate it on constrained hardware before adding new framework layers.

## 10. Tech stack

- Rust 2024 edition CLI/runtime.
- `tokio` for async runtime behavior.
- `rusqlite` for local memory and shared state.
- `reqwest` with `rustls` for optional outbound HTTP.
- Markdown workspace files for identity, memory, and heartbeat configuration.
- Python onboarding scripts for project context, wiki ingestion, goal generation, and linting.

## 11. Framework and library rules

- Favor current module boundaries before introducing new crates or abstractions.
- Preserve the existing split between memory, heartbeat, adapters, skills, worker, state, LLM, and desk-specific modules unless a slice proves the split is wrong.
- Prefer config-driven safety gates over hard-coded unsafe behavior.
- Treat raw payloads as intake only; runtime truth must be typed Rust structs.
- Use Serde-backed structs for API, CLI, tool, MCP, and markdown-derived runtime data before runtime logic depends on it.
- Parse and retain only the fields Quant-M actually needs; ignore unused endpoint bulk by default.
- Do not let OpenRouter, MCP, API, or markdown source shape leak into core runtime logic.
- Repetitive tasks should become FSMs; ambiguity-rich tasks may use LLMs.
- Shared state should receive normalized records, not raw blobs.
- Treat the copied `Ponboarding` and `Staff-OS` repos as planning/reference material, not as dependencies that Quant-M must mirror.
- Use official Rust and Serde docs as the contract source for typed config and serialization behavior.
- Use OpenClaw, IronClaw, Paperclip, and Hermes as pattern references only: borrow runtime boundaries and execution rails, not full product surface area.

## 12. API and integration plan

- The runtime exposes a CLI surface rather than an HTTP API for core operation.
- Optional integrations include webhook delivery, OpenAI-compatible chat completion endpoints, and Telegram polling.
- Any future external integration must document setup steps, secrets expectations, and failure modes before it is considered part of the functional slice.
- Forex desk state and handoff persistence should remain compatible with paper or sandbox execution adapters first.
- If a future control plane or orchestrator is added, Quant-M should expose a narrow invoke/heartbeat/session contract instead of importing orchestration assumptions into the worker core.
- Embedded copies of `Ponboarding` remain reference material, and PONboarding itself should be treated as maintenance mode for this product line.

## 13. Data model

- Core runtime artifacts:
  - `workspace/memory/brain.db`
  - `workspace/state/shared-state.db`
  - `workspace/state/forex.redb`
  - `workspace/state/sessions/*.ndjson`
  - queue files under `workspace/queue/`
  - identity and memory markdown under `workspace/`
- Key logical entities:
  - session ids, run ids, agent ids, step ids, and domain ids
  - memory entries
  - heartbeat tasks
  - worker jobs
- session events
- resume plans
- operator decision records
- domain packs and domain capabilities
- skill descriptors and side-effect levels
- workflow ids, workflow descriptors, and workflow step descriptors
- fsm ids, state ids, event ids, descriptors, and transitions
- scheduler ids, cadence descriptors, and scheduler descriptors
- desk ids, categories, storage profiles, and desk pack descriptors
- workflow run results and local execution session evidence
- policy descriptors and metadata-only policy decisions
- shared-state keys, typed values, and records
- adapter events
  - skill definitions
  - shared desk signals and handoffs
- Project planning artifacts:
  - the onboarding docs under `docs/`

## 14. Routes and pages

Not applicable for the current functional product. Quant-M is a CLI/runtime project with no required browser surface in this phase.

## 15. Components

Logical product components:

- workspace bootstrap
- memory index
- heartbeat executor
- worker queue and execution path
- session log and replay reader
- session analyzer and resume-plan boundary
- operator approval recorder
- domain registry and pack inspection surface
- storage-mode router for inspect, session-write, runtime-preflight, and worker-run lanes
- skill registry and side-effect classification surface
- workflow registry and inspectable step surface
- fsm registry and inspectable transition surface
- scheduler registry and inspectable cadence surface
- desk pack registry and inspectable use-case packaging surface
- local execution runtime for workflow proof runs
- policy registry and skill-evaluation surface
- shared-state store and snapshot surface
- adapter delivery layer
- OS-agnostic terminal cockpit planner
- local skills runner
- optional LLM and Telegram integrations
- shared state and forex handoff layer

Planned hardening track:

- Pi-inspired compression and ergonomics are tracked in `docs/pi-inspired-feature-hardening-plan.md`.
- The highest-priority primitive in that plan is the Compressed Truth Packet: an evidence-backed compaction artifact for long sessions, handoffs, terminal lanes, and constrained devices.
- The next packetization primitive is the Context Firewall: a planned token-budget gate that turns approved contracts, compact packets, context-status, and targeted source sections into small agent packets with receipts.

## 16. Server actions and API routes

No server routes are required for the current Quant-M phase. If a future control plane is introduced, it must remain explicitly out of scope until approved in the spec.

## 17. Auth requirements

No auth layer is required for the local CLI/runtime itself. Optional integrations that need secrets must rely on user-supplied environment variables or config fields and document them clearly.

## 18. Billing and monetization requirements

Out of scope for the current runtime slice.

## 19. AI workflow

Optional AI use is additive, not mandatory:

1. Local runtime behavior works without any external model provider.
2. When enabled, `src/llm.rs` should use an OpenAI-compatible endpoint with clearly documented configuration.
3. AI-assisted features must remain bounded, inspectable, and easy to disable.
4. Any automation that could materially affect execution decisions should preserve evidence, confidence, and reviewable outputs.
5. Quant-M should prefer local skills, replayable artifacts, and explicit approvals before adopting self-improving or autonomous orchestration patterns seen in larger agent systems.

## 20. Error, loading, and empty states

- If workspace files are missing, bootstrap must recreate the safe defaults or report the gap clearly.
- If queue payloads are malformed, they should be rejected or dead-lettered instead of crashing the runtime.
- If optional integrations are disabled or misconfigured, the runtime should fail safely and explain which setting is missing.
- If desk state or handoff persistence is unavailable, the runtime should preserve the blocker without pretending execution succeeded.
- If a session is replayed, the replay path must not trigger shell, network, or trading side effects.
- If a session is analyzed for resume readiness, the analyzer must stay read-only and explain blocked reasons clearly.
- If an operator decision is recorded, it must not weaken policy gates or trigger execution by itself.
- If a domain pack is registered, it must not imply live adapters, trading, or orchestration unless a later approved slice explicitly adds those execution surfaces.
- If a command is classified as read-only inspection, it must not take exclusive desk-storage locks just to show metadata or session evidence.
- If a skill is registered, its side-effect level and policy tags must be explicit before any future execution slice is allowed to run it.
- If a workflow is registered, its steps must declare shared-state reads/writes and any referenced skill ids before any future execution slice is allowed to run it.
- If an fsm is registered, its transitions must declare valid states, events, shared-state reads/writes, and any referenced workflow ids before any future execution slice is allowed to run it.
- If a scheduler is registered, its cadence fields and any referenced workflow or fsm ids must be explicit and internally valid before any future execution slice is allowed to run it.
- If a desk pack is registered, its referenced skills, workflows, fsms, schedulers, and storage profile must be explicit before any future execution slice is allowed to use it as a packaged desk boundary.
- If a workflow is executed through the local runtime, it must stay replay-safe, avoid external adapters unless a later approved slice adds them, and leave behind session/shared-state evidence.
- If external data enters the runtime, it must be normalized into typed Serde-backed structs before core runtime logic or shared state depends on it.
- If a policy evaluates a skill, the result must be metadata-only and must not trigger execution, external adapters, or desk storage access.
- If shared state is updated, it must not replace session logs for ordered evidence or docs/wiki for durable doctrine.

## 21. Tests and validation

List the commands, manual checks, and durable verifiers expected for each completed slice.

Expected for this slice:

- Run onboarding ingest, goal generation, and lint after doc updates.
- Run `cargo fmt --all`, `cargo clippy --all-targets -- -D warnings`, and `cargo test` when the runtime code changes.
- Use targeted smoke checks such as `cargo run -- status`, `cargo run -- session list`, `cargo run -- session resume-plan <id>`, `cargo run -- session approve <id> --reason "<text>"`, `cargo run -- domain list`, `cargo run -- skill list`, `cargo run -- workflow list`, `cargo run -- run workflow <workflow-id>`, `cargo run -- fsm list`, `cargo run -- scheduler list`, `cargo run -- desk list`, `cargo run -- policy evaluate-skill <skill-id>`, `cargo run -- state snapshot`, or specific worker/state commands when the touched area warrants it.
- Keep durable verifiers near the changed runtime behavior, preferably as Rust tests.

## 22. Preservation rules

- Prefer extending existing modules before creating parallel runtime paths.
- Do not weaken config guards for shell, HTTP, Telegram, or live execution without explicit approval and tests.
- Keep the CLI and runtime flows auditable through structured state, logs, or tests.
- Preserve the lean runtime identity; large platform features need a separate product decision before implementation.
- Use borrowed framework patterns only when they reduce ambiguity or improve safety in the current slice.

## 23. Non-goals

See `docs/non-goals.md`.

## 24. UI/UX deferred scope

Any future UI, dashboard, or operator cockpit is deferred to `docs/codex/handoff-to-ui-ux.md` and must not be bundled into the first functional runtime goal.

## 25. Definition of shippable

See `docs/definition-of-shippable.md`.

## 26. Risks and assumptions

See `docs/assumptions.md`.

## 27. Deferred follow-ups

Document useful next steps that are intentionally not part of the current slice.

- Decide whether Telegram should stay optional or become a first-class supported channel.
- Decide whether a future adapter contract should target orchestrators like Staff-OS/Paperclip without importing their control-plane complexity.
- Clarify how far forex execution should go beyond paper/sandbox state management.
- Add packaging, deployment, and node-bootstrap polish only after the core runtime remains stable.
