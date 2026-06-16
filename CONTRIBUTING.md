# Contributing

Quant-M is private while the runtime hardens. Contributions should keep the project local-first, auditable, and conservative.

## Ground Rules

- Keep the README product-facing and plain.
- Do not commit build output, local workspace state, logs, copied repos, caches, or secrets.
- Preserve worker proposal boundaries: workers can submit evidence, but they do not accept truth.
- Preserve policy gates around shell, network, channels, providers, and trading-like actions.
- Add focused tests when changing runtime behavior.

## Before Opening a Change

Run:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test
python3 scripts/lint_project_onboarding.py --target .
```

## Good First Areas

- documentation polish
- onboarding clarity
- local CLI ergonomics
- focused tests around replay, context, and policy boundaries
- examples that do not require external services
