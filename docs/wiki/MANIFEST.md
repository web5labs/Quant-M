# LLM Wiki Manifest

This is the compressed map of the project wiki.

Agents must read this file before loading larger context files.

## Context Budget Rule

- Read this manifest first.
- Open only the files required for the current slice.
- If the slice appears to need more than 8 files, stop and propose a smaller boundary.
- Prefer a fresh thread over dragging a degraded, bloated context forward.
- Use `docs/codex/context-firewall.md` before designing agent packets or widening context access.

## Wiki Folders

| Folder | Purpose |
| --- | --- |
| `docs/wiki/raw/` | Curated source notes that should remain small and intentional. |
| `docs/wiki/ingested/` | Curated summaries from source notes and public architecture references. |
| `docs/wiki/external-docs/` | Library, runtime, and reference documentation summaries. |
| `docs/wiki/repo-ingest/` | Reference repo pattern summaries. Keep summaries only, never copied repos. |

## Core Wiki Files

| File | Purpose | Status |
| --- | --- | --- |
| `08-reference-repos.md` | Reference repos approved or considered for pattern study | Active |
| `ingested/2026-05-31-agentic-rust-runtime-synthesis.md` | Runtime design synthesis from curated references | Active |
| `ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md` | Product hardening and shippability roadmap | Active |
| `external-docs/rust-official-runtime-patterns.md` | Rust runtime pattern notes | Active |
| `external-docs/serde-config-contracts.md` | Serde config contract notes | Active |
| `../governance/runtime-doctrine.md` | Durable runtime doctrine moved out of generated workspace state | Active |
| `../codex/context-firewall.md` | Context budget, packet, and handoff rules | Active |
| `../codex/github-release-gate.md` | GitHub push and release gate | Active |

## Current Goal Context Router

- Treat Quant-M as the target product and use onboarding docs to bound future implementation work.
- Read the top-level `README.md`, `docs/project-spec.md`, and `docs/definition-of-shippable.md` before touching runtime modules.
- Treat `workspace/` as generated runtime state. Do not store canonical doctrine there.
- Use `docs/governance/runtime-doctrine.md` for safety posture, evidence rules, and operator defaults.
- Use desk docs only when the slice touches domain state, routing, or handoffs.
- Use `docs/wiki/external-docs/` and `docs/wiki/repo-ingest/` for curated reference patterns.
- Treat copied local repos and raw session transcripts as private development evidence unless explicitly curated.
- Treat first-class session history as the safety backbone for future agentic workflows.
- Treat replay-to-resume planning as analysis-only unless an operator explicitly approves execution.
- Treat operator approvals as durable evidence, not execution authority.
- Treat domain packs as hosted use cases: registries can describe capabilities without letting any one domain redefine the runtime core.
- Treat inspection as a first-class safety lane.
- Treat descriptors as metadata-first until a runtime execution slice is approved.
- Treat shared state as its own lane: current facts, durable history, execution evidence, and doctrine should not collapse into one store.
- Use `docs/fsm/product-state-machines.md` when a slice touches worker jobs, session evidence, question-to-proposal flow, shared state, or policy-gated side effects.
- Treat GitHub publishing as a release gate, not a default implementation step.
- Keep the product frame narrow: Quant-M is a governed research/runtime system, not a live-trading product or broad plugin marketplace.

## Context Gaps

- Which deployment target is primary for the next product slice.
- Whether Telegram remains optional or graduates into a committed feature surface.
- Which first signature workflow should demonstrate evidence-oriented multi-model consensus.
- Which provider-normalization and cost-governance contracts must exist before broader multi-model orchestration.
- What minimum operator control center would improve everyday usability without replacing the terminal/scripted lane.

## External Docs Needed

None required now. Use external docs only when the touched slice depends on version-sensitive library behavior or provider APIs.

## Reference Repos Needed

See `docs/wiki/08-reference-repos.md`.

## Ingestion Index

Updated by `scripts/ingest_wiki.py`.

### Raw Files Discovered

- `docs/wiki/raw/README.md`

### Ingested Summaries

- `docs/wiki/ingested/2026-05-31-agentic-rust-runtime-synthesis.md`
- `docs/wiki/ingested/quant-m-adversarial-review-2026-06-14-769e90f8.md`

### Repo-Ingest Summaries

- `docs/wiki/repo-ingest/openclaw-openclaw/README.md`
- `docs/wiki/repo-ingest/openclaw-openclaw/repo-map.md`
- `docs/wiki/repo-ingest/openclaw-openclaw/useful-patterns.md`
- `docs/wiki/repo-ingest/openclaw-openclaw/files-to-reference.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/README.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/repo-map.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/useful-patterns.md`
- `docs/wiki/repo-ingest/nearai-ironclaw/files-to-reference.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/README.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/repo-map.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/useful-patterns.md`
- `docs/wiki/repo-ingest/paperclipai-paperclip/files-to-reference.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/README.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/repo-map.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/useful-patterns.md`
- `docs/wiki/repo-ingest/nousresearch-hermes-agent/files-to-reference.md`
