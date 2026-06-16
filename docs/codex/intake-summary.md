# Intake Summary

## Source context

- Existing Quant-M Rust runtime copied into `/Users/julio/Desktop/The-Staff/quantm`
- Local copies of `Ponboarding` and `Staff-OS` placed under `quantm/The-Sataff/` for reference during the real-project test
- Existing project docs include a product README, feature map, deployment notes, workspace memory files, and forex desk docs

## Confirmed facts

- Quant-M is intentionally local-first and CLI-driven.
- The repo already contains runtime code for memory, worker jobs, heartbeat, adapters, skills, shared state, LLM, Telegram, and forex flows.
- There is no required browser UI in the current product boundary.
- `quant-m.toml` currently contains machine-specific absolute paths.

## Assumptions

- The copied Ponboarding and Staff-OS repos are reference material, not the product under test.
- The onboarding goal is to prepare Quant-M for future implementation slices, not to implement new features yet.
- Runtime safety defaults should remain intact while the docs are stabilized.

## Open questions

- Should the next implementation slice normalize config portability first?
- Which deployment target should be treated as primary in the next real build pass?
- How much of the forex desk should remain paper-only in the near term?
