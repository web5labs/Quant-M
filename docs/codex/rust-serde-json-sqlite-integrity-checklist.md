# Rust, Serde, JSON, SQLite Integrity Checklist

Date: 2026-06-26

Use this checklist for focused cleanup passes that must preserve Quant-M's local-first runtime boundary.

## Source Of Truth

Read these before touching code:

1. `AGENTS.md`
2. `docs/wiki/MANIFEST.md`
3. `README.md`
4. `docs/feature-map.md`
5. `docs/project-spec.md`
6. `docs/definition-of-shippable.md`
7. `docs/serde_normalization.md`
8. `docs/shared_state.md`
9. `docs/quant-m-skills.md`

## Checklist

- [ ] Confirm the requested slice does not widen Quant-M beyond the v0.1 local-first boundary.
- [ ] Prefer `scripts/validate_integrity.sh` for the full repeatable loop.
- [ ] Keep at least 2 GiB free for a normal validation pass, or let the script choose lean test mode.
- [ ] Run `cargo fmt --all -- --check`.
- [ ] Run `cargo clippy --all-targets -- -D warnings`.
- [ ] Run `cargo test`.
- [ ] If disk is tight, rerun `scripts/validate_integrity.sh --clean-on-low-disk` or clean generated Cargo artifacts only, then rerun tests with `CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo test`.
- [ ] Scan Serde and JSON paths with `rg -n "serde_json::Value|from_str::<|from_value|to_value|json!" src docs README.md Cargo.toml`.
- [ ] Confirm raw JSON stays at intake/storage bridges unless a typed runtime slice requires normalization.
- [ ] Confirm shared state receives typed `SharedStateRecord` values and does not become a raw endpoint blob store.
- [ ] Scan SQLite paths with `rg -n "rusqlite|Connection|execute\\(|query_map|prepare\\(" src`.
- [ ] Confirm SQLite remains the durable history lane and redb remains the hot current-state lane.
- [ ] Purge code only when compiler, Clippy, tests, or source-truth docs prove it is unused, broken, duplicated, or unsafe.
- [ ] Do not delete guarded or experimental capability surfaces solely because they are inactive.
- [ ] Preserve default-safe gates for shell, HTTP, LLM, Telegram, external providers, and trading-like behavior.
- [ ] Record blockers instead of guessing when docs conflict or a cleanup would change product authority.

## 2026-06-26 Pass Notes

- `cargo fmt --all -- --check` passed.
- `cargo clippy --all-targets -- -D warnings` passed.
- Initial `cargo test` was blocked by local disk exhaustion while writing `target` artifacts.
- `cargo clean` removed generated build artifacts only.
- `CARGO_INCREMENTAL=0 RUSTFLAGS='-Cdebuginfo=0' cargo test` passed with 388 tests and 0 failures.
- No source-code purge was performed because the validation loop did not prove broken or unused runtime code.
- Added `scripts/validate_integrity.sh` so future Rust, Serde, JSON, SQLite, and onboarding checks are repo-owned.
- `scripts/validate_integrity.sh --clean-on-low-disk` passed after cleaning generated Cargo artifacts, choosing lean test mode below 2 GiB free, and rerunning 388 tests with 0 failures.
