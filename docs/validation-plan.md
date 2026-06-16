# Validation Plan

## Detected package manager

`cargo`

## Available scripts

- Cargo-native validation:
  - `cargo fmt --all`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - `cargo build`
- Onboarding validation:
  - `python3 scripts/ingest_wiki.py --target .`
  - `python3 scripts/generate_goal_prompt.py --target .`
  - `python3 scripts/lint_project_onboarding.py --target .`
- Optional smoke checks:
  - `cargo run -- init --non-interactive`
  - `cargo run -- setup --non-interactive --runtime-profile edge`
  - `cargo run -- config show`
  - `cargo run -- config set-model openrouter qwen/qwen3-coder`
  - `cargo run -- config set-channel telegram disabled`
  - `cargo run -- config validate`
  - `cargo run -- provider list`
  - `cargo run -- provider validate openrouter`
  - `cargo run -- tool list`
  - `cargo run -- tool validate codex`
  - `cargo run -- doctor`
  - `cargo run -- doctor --providers`
  - `cargo run -- consensus --dry-run "Should we adopt this API design?"`
  - `cargo run -- strategist --dry-run`
  - `cargo run -- strategist --dry-run --json`
  - `cargo run -- question ask --mode agent-cluster "How should this be reviewed?"`
  - `cargo run -- question ask --mode agent-cluster "Review this API design decision" --write-proposals --json`
  - `cargo run -- question ask --mode staff-os-handoff "What should Codex implement next?" --json`
  - `cargo run -- question ask --mode harness "Which model route should handle this?" --json`
  - `cargo run -- context packet --state QUESTION_TO_WORKER_PROPOSAL_01_VALIDATED --size small --json`
  - `cargo run -- replay <session-id>`
  - `cargo run -- replay <session-id> --json`
  - `cargo run -- state review --domain consensus`
  - `cargo run -- state review --domain consensus --json`
  - `cargo run -- cost summary`
  - `cargo run -- cost summary --json`
  - `cargo run -- status`
  - `cargo run -- worker proposal submit --surface cmux_lane --kind evidence --summary "Architecture lane recommends provider contracts after worker boundary hardening."`
  - `cargo run -- worker proposal submit --surface cmux_lane --kind evidence --summary "Architecture lane recommends provider contracts after worker boundary hardening." --json`
  - `cargo run -- worker proposal list`
  - `cargo run -- worker proposal list --surface cmux_lane`
  - `cargo run -- worker proposal list --status pending_review --json`
  - `QUANT_M_WORKSPACE_DIR=workspace-copy cargo run -- status`
  - `cargo run -- session list`
  - `cargo run -- session resume-plan <session-id>`
  - `cargo run -- session approve <session-id> --reason "<text>"`
  - `cargo run -- domain list`
  - `cargo run -- domain show <domain-id>`
  - `cargo run -- skill list`
  - `cargo run -- skill show <skill-id>`
  - `cargo run -- workflow list`
  - `cargo run -- workflow show <workflow-id>`
  - `cargo run -- fsm list`
  - `cargo run -- fsm show <fsm-id>`
  - `cargo run -- scheduler list`
  - `cargo run -- scheduler show <scheduler-id>`
  - `cargo run -- scheduler list --domain <domain-id>`
  - `cargo run -- scheduler list --trigger <trigger-kind>`
  - `cargo run -- desk list`
  - `cargo run -- desk show <desk-id>`
  - `cargo run -- desk list --category <category>`
  - `cargo run -- desk list --domain <domain-id>`
  - `cargo run -- run workflow <workflow-id>`
  - `cargo run -- policy list`
  - `cargo run -- policy show <policy-id>`
  - `cargo run -- policy evaluate-skill <skill-id>`
  - `cargo run -- state list`
  - `cargo run -- state show <key>`
  - `cargo run -- state snapshot`
  - `cargo run -- state expire-stale`
  - parallel `cargo run -- skill list --domain <domain-id>` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- policy evaluate-skill <skill-id>` plus `cargo run -- skill list --domain <domain-id>` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- workflow list --domain <domain-id>` plus `cargo run -- skill list --domain <domain-id>` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- fsm list --domain <domain-id>` plus `cargo run -- workflow list --domain <domain-id>` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- scheduler list --domain <domain-id>` plus `cargo run -- fsm list --domain <domain-id>` plus `cargo run -- workflow list --domain <domain-id>` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- desk list --domain <domain-id>` plus `cargo run -- scheduler list --domain <domain-id>` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- state snapshot` plus `cargo run -- domain list` smoke checks
  - parallel `cargo run -- domain list` plus `cargo run -- session replay <session-id>` smoke checks
  - targeted `cargo run -- state ...` commands for state-related slices

## Preferred validation order

1. Onboarding ingest and goal generation when docs change
2. Onboarding lint
3. Formatting
4. Clippy
5. Unit and integration tests
6. Build
7. Serde-normalization review for touched intake paths: confirm raw payloads stop at intake, runtime logic uses typed structs, and shared state receives normalized records rather than raw blobs
8. Targeted CLI smoke checks when a runtime lane is touched, including path-override smoke checks for config slices, setup/doctor checks for onboarding slices, session resume-plan checks for session-analysis slices, operator-decision checks for approval-resolution slices, domain inspection checks for domain-pack slices, parallel inspect checks for storage-lock slices, skill metadata checks for skill-registry slices, workflow inspection checks for workflow-registry slices, local workflow-run checks for execution-runtime slices, fsm inspection checks for fsm-registry slices, scheduler inspection checks for scheduler-registry slices, desk-pack inspection checks for desk-pack slices, policy evaluation checks for policy-registry slices, and shared-state snapshot checks for shared-state slices
9. Provider onboarding smoke checks must stay local-only unless the command explicitly includes `--live`
10. Consensus dry-run smoke checks must create session artifacts, consensus state artifacts, and shared-state evidence without provider keys or network access
11. Consensus replay smoke checks must validate artifacts and shared-state evidence without mutating artifacts or executing recommendations
12. State review smoke checks must inspect consensus shared state without mutating records, executing recommendations, or requiring providers/network access
13. Cost ledger smoke checks must confirm consensus dry-run writes an append-only ledger record and cost summary reports zero actual dry-run cost without mutating records or requiring providers/network access
14. Channel isolation smoke checks must prove channel-originated text is evidence or rejected intent only, and cannot execute consensus/replay, mutate state/cost artifacts, call providers, perform trading behavior, or bypass policy/operator approval gates
15. Cluster authority-boundary tests must prove Staff-OS, cmux, tmux, Termux, cron, mtime, polling, and local worker surfaces can submit evidence/proposals only, and cannot execute consensus, mutate canonical state, append accepted cost truth, call providers, trigger trading, bypass replay validation, or bypass policy/operator approval gates
16. Worker proposal record checks must prove proposal submit writes local non-authoritative pending-review artifacts only, proposal list is read-only, filters work, JSON output is machine-readable, and proposals cannot mutate accepted shared state, append accepted cost truth, execute consensus/replay, call providers, trigger trading, or bypass replay/policy/operator approval gates
17. Strategist dry-run checks must prove the mock multi-domain lanes create non-authoritative worker proposals, write session/state/evidence artifacts, append one zero-actual-cost core ledger record with provider `mock` and model `deterministic-strategist-lanes`, require no provider keys/network, produce no trading behavior, and preserve consensus replay, state review, cost summary, channel isolation, cluster boundary, and worker proposal tests
18. Universal question utility checks must prove `question ask` supports only Agent Cluster, Staff-OS Handoff, and Harness modes; emits the shared Question -> Evidence -> Proposal -> Policy Gate -> Cost Record -> Replayable Decision -> Next Safe Action contract; stays inspect-only; and does not create trading, chat, dashboard, onboarding, or provider-specific modes
19. Question-to-worker-proposal checks must prove agent-cluster questions generate local zero-cost worker proposal plans by default, `--write-proposals` creates pending non-authoritative worker proposal artifacts through the existing worker proposal store, and non-agent modes cannot write proposals yet
20. Context Firewall checks must prove `context packet` writes only local packet/receipt artifacts under `workspace/state/context-packets`, records included and excluded context, respects packet size tiers, and does not mutate canonical project truth, compact packets, sessions, shared state, providers, or policy config

## Durable verifier rule

Each completed slice should leave behind at least one of:

- unit test
- integration test
- regression test
- smoke test
- CLI verification command
- manual verification checklist

## Commands

```bash
python3 scripts/ingest_wiki.py --target .
python3 scripts/generate_goal_prompt.py --target .
python3 scripts/lint_project_onboarding.py --target .
cargo fmt --all
cargo test question
cargo test question_command
cargo test strategist
cargo test strategist_dry_run_commands_parse
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

If the touched slice changes runtime behavior, add a focused CLI smoke check to the completion notes.

## Serde normalization checklist

- Raw payloads are intake only.
- Runtime truth is a typed Rust struct.
- Serde is used before runtime logic depends on external shape.
- Only needed fields are retained.
- Shared state receives normalized records, not raw endpoint blobs.
- If a generic JSON bridge remains, it is justified as intake/storage only and marked for later normalization when a real slice needs it.
