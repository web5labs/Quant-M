# LLM Wiki Manifest

This is the compressed map of the project wiki.

Agents must read this file before loading larger context files.

## Context budget rule

- Read this manifest first.
- Open only the files required for the current slice.
- If the slice appears to need more than 8 files, stop and propose a smaller boundary.
- Prefer a fresh thread over dragging a degraded, bloated context forward.
- Use `docs/codex/context-firewall.md` before designing agent packets or widening context access.

## Wiki folders

| Folder | Purpose |
| --- | --- |
| `docs/wiki/raw/` | Original source materials. Preserve exactly. |
| `docs/wiki/ingested/` | Normalized summaries from raw files. |
| `docs/wiki/external-docs/` | Context7/API/library documentation summaries. |
| `docs/wiki/repo-ingest/` | Reference repo manifests and pattern summaries. |

## Core wiki files

| File | Purpose | Status |
| --- | --- | --- |
| `08-reference-repos.md` | Local and external reference repos approved for pattern study | Active |
| `raw/project/README.md` | Primary product overview for Quant-M | Active |
| `raw/project/feature-map.md` | Diagram-to-implementation mapping for the runtime | Active |
| `raw/workspace/HEARTBEAT.md` | Example heartbeat task source | Active |
| `ingested/2026-05-31-agentic-rust-runtime-synthesis.md` | Runtime design synthesis from local copies plus official docs | Active |
| `ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md` | Product hardening, onboarding, shared-state, channel, provider, evidence, and shippable-definition roadmap | Active |
| `../codex/context-firewall.md` | Token-leakage control, context tiers, packet budgets, and packet receipt rules | Planned |
| `../codex/github-release-gate.md` | GitHub push and remote-deployment release gate | Active |

Add focused wiki pages only when the project needs durable detail beyond the spec and manifest.

## Current goal context router

- Treat Quant-M as the target product and use onboarding docs to bound future implementation work.
- Read the top-level `README.md` and `docs/feature-map.md` before touching runtime modules.
- Use the workspace markdown files as evidence for memory, heartbeat, and node identity behavior.
- Use desk docs only when the slice touches forex state, routing, or handoffs.
- Treat embedded copies of `Ponboarding` and `Staff-OS` as reference material, not product requirements.
- Treat PONboarding as maintenance mode for this roadmap: use it for onboarding output quality, not as a new feature-expansion target inside Quant-M.
- Use `docs/wiki/external-docs/` for official library/framework summaries and `docs/wiki/ingested/2026-05-31-agentic-rust-runtime-synthesis.md` for the current architecture synthesis.
- Treat first-class session history as the safety backbone for future agentic workflows before adding external orchestration surfaces.
- Treat replay-to-resume planning as an analysis-only boundary: read persisted evidence first, then gate any future execution slice behind explicit approval.
- Treat operator approvals as durable evidence, not execution: recording a decision should sharpen the next-step analysis without weakening policy or replay guarantees.
- Treat domain packs as hosted use cases: the registry can describe capabilities and workflows without letting any one domain redefine the runtime core.
- Treat inspection as a first-class safety lane: reading domains and session evidence should not require exclusive desk-storage locks.
- Treat skill descriptors as metadata-first: side effects, policy tags, and schemas should be inspectable before any execution layer is introduced.
- Treat workflow descriptors as metadata-first too: steps, shared-state reads/writes, and referenced skills should be inspectable before any execution layer is introduced.
- Treat fsm descriptors as metadata-first too: states, events, transitions, shared-state reads/writes, and optional workflow references should be inspectable before any execution layer is introduced.
- Treat scheduler descriptors as metadata-first too: cron, polling, mtime, event, and manual timing plans should be inspectable before any execution loop is introduced.
- Treat desk pack descriptors as metadata-first too: packaged desks should declare their category, referenced registries, shared-state lanes, and storage profile before any execution layer is introduced.
- Treat execution-runtime slices as proof loops: the first local workflow should reuse the existing contracts and leave behind shared-state plus session evidence instead of inventing a second runtime path.
- Treat policy evaluation as metadata-first too: allow, approval, and deny decisions should be derived from descriptors before any live execution path exists.
- Treat shared state as its own lane: hot current facts, durable history, execution evidence, and doctrine should not collapse into one store.
- Use `docs/fsm/product-state-machines.md` when a slice touches worker jobs, session evidence, question-to-proposal flow, shared state, or policy-gated side effects.
- Treat the Context Firewall as the boundary for future agent packets: most packets should stay in Tier 0-2, Tier 4 is audit-only unless explicitly justified, and every packet needs a receipt.
- Treat GitHub publishing as a release gate, not a default implementation step: read `docs/codex/github-release-gate.md` before initializing git, committing, pushing, or planning remote deployment.
- Treat the adversarial review as roadmap pressure, not implementation truth: use it to prioritize product identity, onboarding, shared-state quality, channel isolation, provider normalization, cost governance, and evidence-oriented multi-model consensus.
- Keep the current product frame narrow until explicitly changed: Quant-M should present first as a governed research/runtime system, not a live-trading or broad plugin platform.

## Raw files discovered

_Run `python scripts/ingest_wiki.py --target .` to update._

## Ingested summaries

_Run `python scripts/ingest_wiki.py --target .` to update._

## Context gaps

- Which deployment target is primary for the next product slice.
- Whether Telegram remains optional or graduates into a committed feature surface.
- How far the forex desk should go beyond state persistence and paper/sandbox handoffs.
- When Quant-M should graduate from conservative resume planning into an explicit operator-approved resume execution flow.
- How operator identities and approval records should evolve if the runtime ever needs multi-operator audit policy.
- How far domain packs should eventually go beyond metadata registration into isolated adapters or schedulers without bloating core.
- How far desk packs should go beyond packaging metadata before a real desk execution slice is approved.
- Which constrained hardware target should be used as the first formal Quant-M v0.1 validation device.
- Whether more runtime stores should eventually expose explicit read-only handles instead of piggybacking on command classification alone.
- How far skill routing should go before workflow execution is added, and whether schemas need stronger validation beyond descriptor names.
- How fsm descriptors should later connect to a real execution runtime without collapsing metadata and runtime behavior into one layer.
- How scheduler descriptors should eventually connect to heartbeat, worker polling, desk timing, and file-watch/event sources without collapsing metadata and runtime behavior into one layer.
- Whether policy composition eventually needs richer precedence or inheritance once workflow execution is introduced.
- Whether shared-state history eventually needs dedicated replay tooling beyond snapshot and expire operations.
- Whether "Quant-M Research Runtime" should become the official public product frame.
- What first signature workflow should demonstrate evidence-oriented multi-model consensus.
- Which provider-normalization and cost-governance contracts must exist before broader multi-model orchestration.
- What minimum operator control center would improve everyday usability without replacing the terminal/scripted lane.

## External docs needed

- _None required yet. Use external docs only when the touched slice depends on version-sensitive library behavior or provider APIs._

## Reference repos needed

- See `docs/wiki/08-reference-repos.md`.

## Ingestion Index

Updated by `scripts/ingest_wiki.py`.

### Raw files discovered

- `docs/wiki/raw/desks/forex/carry_map.md`
- `docs/wiki/raw/desks/forex/desk_policy.md`
- `docs/wiki/raw/desks/forex/handoff.md`
- `docs/wiki/raw/desks/forex/processor.md`
- `docs/wiki/raw/desks/forex/routing.md`
- `docs/wiki/raw/desks/forex/schema.md`
- `docs/wiki/raw/desks/forex/skills.md`
- `docs/wiki/raw/project/deploy-android.md`
- `docs/wiki/raw/project/deploy-systemd.md`
- `docs/wiki/raw/project/feature-map.md`
- `docs/wiki/raw/project/quant-m-adversarial-review-2026-06-14.md`
- `docs/wiki/raw/project/risk_policy.md`
- `docs/wiki/raw/workspace/AGENTS.md`
- `docs/wiki/raw/workspace/HEARTBEAT.md`
- `docs/wiki/raw/workspace/MEMORY.md`
- `docs/wiki/raw/workspace/SOUL.md`
- `docs/wiki/raw/workspace/USER.md`

### Ingested summaries

- `docs/wiki/ingested/carry_map-46dc2bb1.md`
- `docs/wiki/ingested/desk_policy-05e2ca38.md`
- `docs/wiki/ingested/handoff-4dde788c.md`
- `docs/wiki/ingested/processor-a8dc7a6f.md`
- `docs/wiki/ingested/routing-ce11dcd4.md`
- `docs/wiki/ingested/schema-3ca9f775.md`
- `docs/wiki/ingested/skills-44088aaa.md`
- `docs/wiki/ingested/deploy-android-35173bda.md`
- `docs/wiki/ingested/deploy-systemd-92696538.md`
- `docs/wiki/ingested/feature-map-69eb5fef.md`
- `docs/wiki/ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md`
- `docs/wiki/ingested/risk_policy-0be955b8.md`
- `docs/wiki/ingested/agents-9f1717c3.md`
- `docs/wiki/ingested/heartbeat-02523f7d.md`
- `docs/wiki/ingested/memory-90c7549a.md`
- `docs/wiki/ingested/soul-f376f424.md`
- `docs/wiki/ingested/user-c1d39919.md`

### Reference repos requested

- approved: `local-copy:quantm/The-Sataff/Ponboarding`
- approved: `local-copy:quantm/The-Sataff/Staff-OS`
- candidate: `openclaw/openclaw`
- candidate: `nearai/ironclaw`
- candidate: `paperclipai/paperclip`
- candidate: `NousResearch/hermes-agent`
- candidate: ``OpenClaw-style local worker runtimes` if a concrete upstream repo is approved later`
- candidate: ``lightweight Rust automation runtimes` if the current module boundaries prove insufficient`

### Repo-ingest summaries

- `docs/wiki/repo-ingest/lightweight-rust-automation-runtimes-if-the-current-module-boundaries-prove-insufficient/files-to-reference.md`
- `docs/wiki/repo-ingest/lightweight-rust-automation-runtimes-if-the-current-module-boundaries-prove-insufficient/repo-map.md`
- `docs/wiki/repo-ingest/lightweight-rust-automation-runtimes-if-the-current-module-boundaries-prove-insufficient/useful-patterns.md`
- `docs/wiki/repo-ingest/local-copy-quantm-the-sataff-ponboarding/files-to-reference.md`
- `docs/wiki/repo-ingest/local-copy-quantm-the-sataff-ponboarding/repo-map.md`
- `docs/wiki/repo-ingest/local-copy-quantm-the-sataff-ponboarding/useful-patterns.md`
- `docs/wiki/repo-ingest/local-copy-quantm-the-sataff-staff-os/files-to-reference.md`
- `docs/wiki/repo-ingest/local-copy-quantm-the-sataff-staff-os/repo-map.md`
- `docs/wiki/repo-ingest/local-copy-quantm-the-sataff-staff-os/useful-patterns.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/files-to-reference.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/repo-map.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/useful-patterns.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/files-to-reference.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/repo-map.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/useful-patterns.md`
- `docs/wiki/repo-ingest/openclaw-openclaw/files-to-reference.md`
- `docs/wiki/repo-ingest/openclaw-openclaw/repo-map.md`
- `docs/wiki/repo-ingest/openclaw-openclaw/useful-patterns.md`
- `docs/wiki/repo-ingest/openclaw-style-local-worker-runtimes-if-a-concrete-upstream-repo-is-approved-later/files-to-reference.md`
- `docs/wiki/repo-ingest/openclaw-style-local-worker-runtimes-if-a-concrete-upstream-repo-is-approved-later/repo-map.md`
- `docs/wiki/repo-ingest/openclaw-style-local-worker-runtimes-if-a-concrete-upstream-repo-is-approved-later/useful-patterns.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/files-to-reference.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/repo-map.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/useful-patterns.md`
