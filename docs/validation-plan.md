# Validation Plan

## Detected package manager

`cargo`

## Available scripts

- Cargo-native validation:
  - `cargo fmt --all`
  - `cargo clippy --all-targets -- -D warnings`
  - `cargo test`
  - `cargo build`
- Integrity validation:
  - `scripts/validate_integrity.sh`
  - `scripts/validate_integrity.sh --clean-on-low-disk`
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
  - `cargo run -- cluster init`
  - `cargo run -- cluster device options`
  - `cargo run -- cluster device options --json`
  - `cargo run -- cluster desk rails`
  - `cargo run -- cluster desk rails --json`
  - `cargo run -- timing list`
  - `cargo run -- timing inspect --desk forex`
  - `cargo run -- timing next --desk forex`
  - `cargo run -- timing check --desk forex --role forex_calendar_watcher --dry-run`
  - `cargo run -- timing check --node tablet-01 --dry-run`
  - `cargo run -- timing cooldowns`
  - `cargo run -- compute capabilities`
  - `cargo run -- compute freshness-scan --fixture evidence_freshness --backend scalar`
  - `cargo run -- compute peg-deviation --fixture stablecoin_peg_deviation --backend scalar`
  - `cargo run -- compute peg-deviation --fixture boundary_ambiguous_peg_scan --backend scalar`
  - `cargo run -- compute bench --workload peg-deviation --samples 1000 --manual`
  - `cargo run -- compute validate --node node:tablet-01 --backend scalar --fixture evidence_freshness`
  - `cargo run -- compute validations`
  - `cargo run -- compute mismatches`
  - `cargo run -- compute quarantine`
  - `cargo run -- cluster node register --name tablet-01 --surface termux_worker --capabilities echo,sleep,compute_scalar --json`
  - `cargo run -- cluster role assign --node node:tablet-01 --role generic_evidence_collector --ttl 30m --json`
  - `cargo run -- cluster heartbeat --node node:tablet-01`
  - `cargo run -- cluster job submit --node node:tablet-01 --desk research --kind compute_freshness_scan --payload '{}' --fixture evidence_freshness --backend scalar --json`
  - `cargo run -- cluster child run --node node:tablet-01 --json`
  - `cargo run -- pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --link`
  - `cargo run -- pair doctor`
  - `cargo run -- pair doctor --bind 0.0.0.0:8787`
  - `cargo run -- child pair --core http://127.0.0.1:8787 --invite <invite-token>`
  - `cargo run -- pair requests`
  - `cargo run -- pair approve --request <request-id>`
  - `cargo run -- child identity`
  - `cargo run -- child doctor`
  - `cargo run -- cluster nodes`
  - `cargo run -- cluster heartbeat --node node:tablet-01`
  - `cargo run -- cluster lease grant --node node:tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 30m --authority observe`
  - `cargo run -- cluster lease list`
  - `cargo run -- cluster lease check --node node:tablet-01`
  - `cargo run -- cluster lease revoke --node node:tablet-01 --reason "smoke complete"`
  - `cargo run -- numeric bench stablecoin-peg --samples 1024`
  - `cargo run -- numeric bench stablecoin-peg --samples 1024 --backend scalar --json`
  - `cargo run -- cluster node register --name tablet-01 --surface termux_worker --capabilities echo,sleep`
  - `cargo run -- cluster role assign --node node:tablet-01 --role forex_calendar_watcher --ttl 30m`
  - `cargo run -- cluster heartbeat --node node:tablet-01`
  - `cargo run -- cluster job submit --node node:tablet-01 --desk forex --kind echo --payload '{"text":"macro calendar clear"}'`
  - `cargo run -- cluster child run --node node:tablet-01`
  - `cargo run -- cluster report`
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

0. Use `scripts/validate_integrity.sh` for Rust, Serde, JSON, SQLite, and onboarding integrity passes when the full loop is appropriate
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
21. Core/child edge cluster checks must prove local-file node registration, read-only edge device option listing, read-only lead-coach knowledge-pack desk rails, single execution-leader device metadata, child-only Termux/phone/tablet/Pi worker profiles, Forex positive-swap carry rail limits, sports event-scouting rail criteria, role leasing, heartbeat, role-scoped job submission, child evidence receipt writing, stale detection/reporting, and default-denied `http_get`; child results and desk rails must remain non-authoritative and must not mutate canonical shared state, approve proposals, execute trades/bets, call providers, guarantee profit, or bypass replay/policy/operator approval gates
22. Desk Timing Framework checks must prove timing policies load from fixtures/defaults, every desk has a default timing policy, cron/polling/mtime/session window/event window/heartbeat/cooldown triggers are represented, cron expressions validate, polling rejects too-fast intervals, mtime rejects paths outside the workspace or secrets, stale evidence is rejected, cooldown blocks proposals, event windows can force watch-only mode, Forex rollover and sports pregame windows are detected, tablet-safe roles enforce 60-second minimum polling, child nodes cannot override timing policy, and no timing trigger can execute trades, bets, shell, provider calls, remote network actions, or authority escalation
23. EDGE_COMPUTE_SCALAR_FIRST_SIMD_READY_01 checks must prove scalar Rust is the source of truth, backend capability reports split hardware/compile/implementation/self-test/scalar-equivalence claims, default selected backend is scalar, unvalidated child compute claims remain metadata only, core validation/mismatch/quarantine storage paths exist, evidence freshness scan splits fresh/stale evidence, peg deviation scan computes numeric evidence only without net edge/arbitrage/proposals, boundary-near values are marked `BoundaryAmbiguous`, tablet input and benchmark limits reject oversized or unsafe jobs, benchmark output is inspect-only and never evidence/proposal/scheduler authority, SIMD capability does not increase evidence weight, and compute cannot mutate FSM authority, policy decisions, proposal approval, child trust, role assignment, execution logic, trades, bets, shell, provider calls, or remote network actions
24. EDGE_COMPUTE_CORE_VALIDATION_ROUNDTRIP_02 checks must prove `compute validate` writes a core-side validation record, validation is required before backend usability, unsupported accelerated backends write mismatch records and quarantine entries, quarantined backends are not selected as trusted usable backends, `compute validations`, `compute mismatches`, and `compute quarantine` read local ledgers, replay metadata prefers scalar despite accelerated backend metadata, child capability claims alone do not validate a backend, and validation cannot mutate FSM authority, policy decisions, proposal approval, role assignment, scheduling priority, trades, bets, shell, provider calls, or remote network actions
25. EDGE_COMPUTE_CLUSTER_EVIDENCE_BINDING_03 checks must prove cluster compute jobs require active leases, fresh child heartbeats, role capabilities, scalar-only backend selection, timing-gate approval, and non-quarantined backends; child run can execute scalar evidence freshness and peg deviation workloads; receipts record backend requested, backend used, validation outcome, scalar verification, numeric confidence, input/output hashes, and timing decision id; cluster reports show compute evidence as evidence only; compute output cannot increase authority, create proposals directly, alter trust/scheduling priority, execute trades/bets, call providers, run shell, or bypass FSM/policy/replay gates
26. CORE_CHILD_QR_PAIRING_01 checks must prove the core can generate short-lived local invites, stores invite token hashes instead of plaintext tokens, renders copyable local links and feature-gated QR outputs, records pending child pairing requests with node public keys, rejects expired/used/revoked invites and authority/execution/canonical-write escalation claims, approval creates observe-only paired node records and cluster node enrollment without role leases or execution authority, child identity/core metadata are stored locally, event logs are replay-safe, and pairing cannot approve proposals, mutate canonical state, execute trades/bets, call providers, validate compute, or bypass timing/FSM/policy/replay gates
27. CORE_CHILD_PAIRING_SERVER_RUNTIME_02 checks must prove `pair serve` runs a minimal local HTTP pairing server with `GET /pair/i/<invite-token>`, `POST /pair/request`, and `GET /pair/status/<request-id>`; invite pages expose core fingerprint, role/desk, expiry, observe-only boundary, and copyable Termux command; server requests default to pending; expired, used, and revoked invites are rejected; authority escalation and execution/approval/canonical-write claims are rejected; approval remains operator-driven; server pairing does not assign leases, enable execution, approve proposals, validate compute, schedule work, mutate canonical shared state, call providers, or bypass timing/FSM/policy/replay gates; pairing server events are replay-safe
28. CHILD_QR_IMAGE_SCAN_02 checks must prove `child pair-scan --image <path>` decodes a captured QR image file when built with `pairing-scan-image`, accepts only local Quant-M pairing URLs or approved `quantm://pair` payloads, extracts core URL and invite token, routes through the same child pairing request path as `child pair`, rejects non-Quant-M QR payloads, malformed payloads, secret-bearing payloads, public/non-local URLs by default, images without QR payloads, and conflicting QR payloads, and cannot assign leases, enable execution, approve proposals, validate compute, call providers, schedule work, mutate canonical shared state, or bypass timing/FSM/policy/replay gates
29. CORE_CHILD_TABLET_PAIRING_E2E_03 checks must prove the documented tablet flow covers core setup, Termux/tablet setup, QR image scan, manual fallback, operator approval, heartbeat, troubleshooting, and expected final node state; `pair doctor` reports pairing feature status, fingerprint presence, state directory presence, active invites, pending requests, accepted nodes, bind warning, and LAN hint; `child doctor` reports identity, paired core, stored fingerprint, pairing status, node id, and heartbeat status; approved tablet pairing remains observe-only and cannot assign leases, enable execution, approve proposals, validate compute, schedule work, call providers, trade, bet, or mutate canonical shared state
30. PAIRED_CHILD_HEARTBEAT_AND_LEASE_04 checks must prove approved paired children can heartbeat, unpaired children cannot heartbeat, heartbeat marks nodes online/stale without creating leases or assigning roles, heartbeat rejects execution/approval/canonical-write claims, the core can grant observe-only leases only to approved paired nodes, lease authority cannot exceed pairing authority, child nodes cannot self-assign leases, expired/revoked leases block future work eligibility, lease events are replay-safe, doctor/status commands show heartbeat and lease state, and leases do not enable execution, approval, canonical writes, compute validation, scheduling, jobs, provider calls, trades, or bets
31. LEASED_CHILD_ECHO_ROUNDTRIP_05 checks must prove an approved paired child with a fresh heartbeat and active observe-only lease can receive only an `echo` job, run it locally, and write a replay-safe evidence receipt; unpaired, unapproved, offline/stale, missing-lease, expired-lease, or revoked-lease nodes must be blocked; `http_get`, compute jobs, shell, provider calls, desk analysis, proposal approval, canonical writes, execution, trading, betting, and scheduling authority must remain disabled; echo output must not create proposals, mutate shared truth, increase role authority, validate compute backends, or bypass timing/FSM/policy/replay gates
32. LEASED_CHILD_SCALAR_COMPUTE_EVIDENCE_06 checks must prove an approved paired child with fresh heartbeat, active observe-only lease, role `compute_scalar` capability, and timing approval can run only scalar `compute_freshness_scan` and `compute_peg_deviation` jobs; receipts must record backend requested, backend used, scalar validation outcome, numeric confidence, input hash, output hash, timing decision id, node id, role id, and lease id as replay-safe evidence; expired, revoked, stale/offline, missing-lease, unpaired, unapproved, unsupported backend, net-edge/arbitrage workload, `http_get`, shell, provider calls, desk proposals, proposal approval, canonical writes, execution, trading, betting, and scheduling authority must remain blocked; compute evidence cannot promote itself to a proposal or increase authority
33. LEASED_CHILD_DESK_OBSERVE_EVIDENCE_07 checks must prove leased children can wrap allowlisted scalar freshness and peg-deviation outputs in `DeskObservationEvidence` envelopes with node id, lease id, desk id, role id, optional knowledge pack id, evidence kind, observe authority, timing decision id, compute metadata, input/output hashes, numeric confidence, replay-safe status, and `proposal_created=false`; desk/role mismatches, expired or revoked leases, stale heartbeat, timing blocks, provider-call intent, `http_get`, shell intent, net-edge/arbitrage language, canonical writes, approvals, execution, trading, betting, and proposal creation must be rejected; desk observation replay must remain side-effect-free and must not create `ProposalCandidate`, `StrategyDecision`, `RiskDecision`, `TradeDecision`, or `BetDecision`
34. DESK_PLAYBOOK_MODEL_HANDOFF_08 checks must prove each desk role can load and validate a versioned playbook with knowledge-pack refs, forbidden outputs, allowed evidence kinds, allowed model tasks, and stable hash; playbook bundles are replay-safe and hash-stable; observe leases can bind playbook id/hash; queued child evidence rejects playbook hash mismatch; model handoff packets record playbook hash, knowledge-pack hashes, shared-state snapshot id/hash, evidence ids, task kind, model policy, output schema, and forbidden outputs as provider-neutral JSON; local-stub model calls create pending shared-state update proposals only; OpenRouter/direct OpenAI calls remain feature-gated and disabled by default; provider keys stay core-side; model output cannot mutate shared state directly, create proposals, create strategy/risk/trade/bet decisions, execute, approve, call providers, or bypass timing/FSM/policy/replay gates
35. PLAYBOOK_HANDOFF_ADVERSARIAL_HARDENING_08A_AND_SHARED_STATE_UPDATE_VALIDATION_09 checks must prove canonical JSON SHA-256 hashes are used for playbooks, bundles, handoff sections, handoffs, update proposals, accepted facts, and snapshots; handoff sections separate system boundary, playbook contract, snapshot, evidence-as-quoted-data, model task, output schema, and forbidden outputs; playbook validation rejects hidden trading, betting, execution, provider credential, ambiguous strategy, missing forbidden-output, and missing knowledge-pack language; shared-state updates stay candidate/unvalidated until core validation; validation rejects playbook hash mismatch, snapshot hash mismatch, missing evidence lineage, forbidden trade/bet/canonical-write/provider credential claims, stale evidence, contradictions, and policy violations; acceptance requires an operator reason, writes append-only accepted facts with `ephemeral` default decay, and creates no FSM, proposal, execution, provider, trading, or betting authority; snapshots are append-only and replay-safe
36. EDGE_RUNTIME_MINIMIZATION_10 checks must prove `quant-m-child --no-default-features --features child-min` builds and tests, exposes only child-safe commands, uses file-first child storage, rejects non-echo `run-once` jobs in child-min, reports model router/provider/shared-state-accept/pairing-server surfaces as not compiled, documents the core-vs-child feature matrix, records child/core binary size measurements, documents the child-min dependency tree, keeps core-full operator commands available, and adds no desk feature, provider call, FSM proposal, database, live network complexity, execution adapter, trading command, or betting command
37. CHILD_DEPENDENCY_PRUNING_10A checks must prove core-only dependencies are optional and feature-gated, `child-min` library compilation cuts out rich core modules, `cargo tree --no-default-features --features child-min` excludes model router, provider adapters, shared-state acceptance storage, pairing server, terminal UI, image/PNG QR, SQLite, redb, reqwest, tokio, ratatui, crossterm, postcard, rand, ring, qrcode, rqrr, and image, `quant-m-child --profile release-child --no-default-features --features child-min` builds and records size, child-min tests and clippy pass, core cluster/model-router regressions pass with `core-full`, dev-all clippy passes, and no child runtime capability, desk feature, provider call, FSM proposal, database, live network complexity, execution adapter, trading command, or betting command is added
38. CORE_DEVICE_ADD_WIZARD_11 checks must prove `quant-m device add <name>` is a core-side wrapper around existing pairing and cluster lease functions; it creates short-lived observe-only pairing invites, can render link/QR/PNG onboarding material, can inspect pending requests, defaults to manual approval and no lease, caps local lab auto-approval to observe-only short TTLs, grants an observe-only lease only when explicitly requested and only for an approved paired node, reuses existing pairing and cluster lease storage without a new registry, reports final state with execution/proposals/approval/canonical writes/provider calls/compute validation disabled, keeps `quant-m-child --no-default-features --features child-min` compiling without new child dependencies, and cannot start jobs beyond existing gates, validate compute, call providers, accept shared-state updates, create proposals, execute, trade, or bet
39. DEVICE_ADD_INTERACTIVE_APPROVAL_12 checks must prove `quant-m device add --watch` detects pending pairing requests, displays request details, prompts for manual approval, approves only on explicit yes input, rejects on empty/default input, creates observe-only paired nodes only through existing pairing approval, grants no lease by default, grants an observe-only lease only when explicitly requested for an approved paired node, can explicitly start the existing pairing server with `--serve` without hidden network exposure, preserves trusted-LAN warnings for `0.0.0.0`, reuses pairing and cluster lease storage, keeps `quant-m-child --no-default-features --features child-min` compiling and size-stable, and cannot validate compute, run jobs beyond existing gates, call providers, accept shared-state updates, create proposals, execute, approve, write canonical state, trade, or bet
40. PI_TERMUX_LAN_VALIDATION_13 checks must prove the documented real-device runbook covers Raspberry Pi/DietPi core setup, laptop fallback core setup, Termux child setup, build commands, feature flags, trusted-LAN pairing server start, `device add --watch`, QR or manual pairing fallback, explicit approval, heartbeat, optional observe-only lease, echo evidence, scalar freshness evidence, scalar peg-deviation evidence, optional desk observation evidence, cluster report, cleanup, troubleshooting, expected safe final state, child-min size recording, and current local-file/manual-sync transport limitations; the runbook must require proposal count zero, execution false, provider calls disabled, canonical writes disabled, trading/betting disabled, no public internet exposure, no release-bundled generated state/secrets/live QR tokens, and no new runtime authority
41. CHILD_DEVICE_TELEMETRY_14 checks must prove child-min can collect best-effort device display name, hostname, OS, architecture, storage unknown-or-available status, and battery unknown-or-available status without failing; Termux battery JSON fixture parsing handles percent, charging, and malformed JSON without panic; heartbeat records optional device telemetry; `quant-m-child doctor` and heartbeat surface telemetry while keeping execution/approval/canonical-write disabled; `cluster nodes` and `cluster report` display telemetry when present; low battery and low storage warnings are advisory only; telemetry payloads with authority or credential claims are rejected; telemetry does not create leases, extend leases, enable jobs, validate compute, increase evidence weight, increase authority, create proposals, call providers, execute, trade, or bet; child-min builds and clippy passes without large system-info dependencies; release-child size is recorded before and after
42. REAL_DEVICE_LAN_SMOKE_14A checks must prove an actual Raspberry Pi/DietPi or laptop fallback core and actual Termux child run the LAN flow from `docs/pi-termux-lan-validation.md`; the validation run artifact under `docs/validation-runs/` must record core and child device details, OS/arch, child binary size, storage, battery, pairing method, commands run, telemetry outputs, pass/fail results, blockers, and safe final state; a blocked artifact is acceptable only when real devices are unavailable and must not be counted as pass evidence; a passing run must confirm paired=true, approved=true, online=true, authority=observe, telemetry present or unknown with reason, evidence receipts present, proposal_count=0, execution=false, provider_calls=false, canonical_write=false, trading=false, betting=false, and no new runtime authority
43. LOCAL_ALPHA_RELEASE_CANDIDATE_15 checks must prove the repo contains a README quickstart, local-alpha release notes, known limitations, security boundaries, validation-run artifact, release checklist, binary size record, and feature matrix; local validation must include formatting, cluster, pairing, timing, device telemetry, model router, child-min check, child-min clippy, dev-all clippy, and onboarding lint; real-device validation must either pass with Pi/DietPi or laptop fallback core plus Termux child, or be explicitly documented as blocked and not counted as pass evidence; the release label must stay `v0.local-alpha`; public beta, production, autonomous trading, autonomous betting, provider calls from children, execution, proposal approval, canonical writes, and new runtime authority must remain blocked

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
scripts/validate_integrity.sh
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
cargo test cluster
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
