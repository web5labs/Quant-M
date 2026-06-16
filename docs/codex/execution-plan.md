# Codex Execution Plan

## Current objective

Complete `GITHUB_RELEASE_GATE_01` by preparing the release/push boundary for Quant-M now that GitHub SSH auth works and shippable validation has passed, without pushing until repo details and final human approval are recorded.

## Current FSM state

`GITHUB_RELEASE_GATE_01_VALIDATED_AWAITING_REPO_DETAILS`

## Smallest reviewable slice

- Record GitHub SSH auth readiness for `web5labs`.
- Add a release gate before any git init, commit, push, or remote deployment.
- Add a top-level `.gitignore` that keeps runtime state and local artifacts out of source control.
- Record blockers for disk, Rust validation, git identity, and target repo name.
- Keep GitHub push and remote deployment blocked until target repo name, git identity email, and final human approval are recorded.

## Files likely to inspect before editing

- `.gitignore`
- `docs/codex/github-release-gate.md`
- `docs/codex/blockers.md`
- `docs/open-questions.md`
- `docs/wiki/MANIFEST.md`
- `docs/validation-plan.md`

## Files likely to change

- `.gitignore`
- `docs/codex/github-release-gate.md`
- `docs/codex/blockers.md`
- `docs/open-questions.md`
- `docs/wiki/MANIFEST.md`
- `docs/README.md`
- `docs/codex/execution-plan.md`

## Reuse scan findings

- Existing GitHub SSH key now authenticates as `web5labs`.
- Existing validation plan already has Cargo and onboarding checks; release gate should reference it instead of inventing a second checklist.
- Existing workspace contains local runtime state and databases; `.gitignore` must prevent accidental publication.
- Existing blockers/open questions are the right place to preserve release readiness gaps.
- Rust validation passed after freeing the inactive Rust 1.92 toolchain.

## Context budget

- If the slice appears to need more than 8 files, stop and split the boundary.
- Prefer a fresh thread over bloated context compaction.

## Validation commands

```bash
python3 scripts/ingest_wiki.py --target .
python3 scripts/generate_goal_prompt.py --target .
python3 scripts/lint_project_onboarding.py --target .
cargo fmt --all
cargo test question
cargo test question_command
cargo test worker_proposals
cargo test worker_proposal_commands_parse
cargo test context_firewall
cargo test context_packet_command
cargo test cluster_boundary
cargo test channels
cargo clippy --all-targets -- -D warnings
cargo test
cargo build
```

Repo-specific runtime validation is listed in `docs/validation-plan.md`.

## Structure pass

After implementation, review:

- repeated runtime mechanics,
- service extraction opportunities,
- adapter extraction opportunities,
- thin-route/controller boundaries.

## Stop conditions

- Stop when the release gate is documented, validation passes, and missing repo details are recorded.
- Document follow-up work separately instead of implementing it immediately.
- Keep the next milestones separate: disk cleanup, Rust validation, git identity/repo selection, first commit/push, then remote deployment planning.
- Enter the repair loop only for the failing scope.
