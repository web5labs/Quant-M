# Intake Summary

## Source context

- Existing Quant-M Rust runtime is the product under review.
- Earlier private reference copies were used only for pattern study and are not part of the public export.
- Existing project docs include a product README, feature map, deployment notes, governance doctrine, and runtime docs.

## Confirmed facts

- Quant-M is intentionally local-first and CLI-driven.
- The repo already contains runtime code for memory, worker jobs, heartbeat, adapters, skills, shared state, LLM, Telegram, and forex flows.
- There is no required browser UI in the current product boundary.
- `quant-m.toml` should remain portable and use relative paths or environment variables.

## Assumptions

- External and private references are pattern material, not product requirements.
- The onboarding goal is to prepare Quant-M for future implementation slices, not to implement new features yet.
- Runtime safety defaults should remain intact while the docs are stabilized.

## Open questions

- Should the next implementation slice normalize config portability first?
- Which deployment target should be treated as primary in the next real build pass?
- How much of the forex desk should remain paper-only in the near term?
