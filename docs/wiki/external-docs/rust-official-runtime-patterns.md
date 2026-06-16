---
type: external-doc-summary
date: 2026-05-31
source_count: 2
tags:
  - rust
  - runtime
  - testing
  - errors
---

# Rust Official Runtime Patterns

## Sources

- Context7: `/rust-lang/rust`
- Rust docs source surfaced via Context7:
  - `library/core/src/error.rs`
  - `src/doc/rustc-dev-guide/src/test-implementation.md`
  - `src/doc/rustc-dev-guide/src/tests/best-practices.md`

## What matters for Quant-M

- Rust’s standard error model expects specific, inspectable error types that implement `Display` and `std::error::Error`; even when using `anyhow`, path and field context should still be attached close to the failing operation.
- Tests can live inside the same module as private functions, which fits Quant-M’s current style of colocated runtime tests for queueing, config, memory, and desk logic.
- Descriptive test names are preferred over issue-number-first names; the test name should say what runtime property is protected.
- Module-local visibility is a feature, not a limitation; internal helpers can stay private while still being tested directly.

## Rails implication

- Keep runtime modules narrow and separately testable.
- Keep filesystem and config errors explicit, path-aware, and operator-readable.
- Prefer colocated tests for core runtime mechanics unless a behavior truly needs a higher-level integration harness.
- Treat test names as durable verifier documentation.

## Confidence / limits

- This is language-level guidance, not a full app-architecture prescription.
- Rust docs do not prescribe an “agent runtime” shape; Quant-M still needs project-level discipline for sessions, approvals, and execution boundaries.
