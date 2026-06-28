# Quant-M Feature Map (Diagram -> Implementation)

## Feature Truth Map

This file is a truth map, not a marketing page. The machine-readable inventory lives in Rust and is available through:

```bash
quant-m capabilities
quant-m capabilities --json
quant-m capabilities show <capability_id>
quant-m capabilities audit-docs
```

Capability status labels are exact enum-like strings:

- `shipped`: implemented, documented, tested, and usable locally without hidden provider/network assumptions
- `guarded`: implemented but blocked unless explicit config, policy, or operator approval enables it
- `dry_run`: implemented as a non-side-effecting or simulated path only
- `mock`: intentionally fake/demo behavior for proof, tests, examples, or registry shape
- `experimental`: present but not stable enough to promise as a public contract
- `design_only`: described as a pattern or future direction, not a usable runtime feature
- `external_required`: depends on local external tools or provider setup
- `unavailable`: not available in this repo/machine/config posture
- `deprecated`: retained but no longer recommended

Runtime state authority follows the same rule: Markdown explains intent and examples; Rust decides allowed states, transitions, denial behavior, and replay-compatible evidence. The current authority summary is inspectable with `quant-m fsm authority` or `quant-m fsm authority --json`.

Current major capability truth:

| Feature group | status | command surface | created artifacts | proof command | safety notes | current limitations |
| --- | --- | --- | --- | --- | --- | --- |
| Onboarding/setup | `shipped` | `quant-m onboard`, `setup`, `init`, `settings` | `quant-m.toml`, `workspace/` | `quant-m settings` | project-local config; provider setup is not permission | does not install providers or local models |
| Local shell/demo/status | `shipped` plus `experimental` TUI | `agent`, `shell`, `demo`, `doctor`, `status`, `tui` | sessions, compact/context/cost artifacts for demo | `quant-m demo` | local proof path avoids provider calls | TUI remains experimental |
| Memory | `shipped` | `memory add/search/list` | `workspace/MEMORY.md`, `workspace/memory/brain.db` | `cargo test memory` | local hashed signal; no embedding provider | markdown memory remains human-authored |
| Sessions/replay | `shipped` | `session *`, `replay` | `workspace/state/sessions/` | `quant-m session list` after demo | replay is side-effect free and computes typed final lifecycle state | legacy final-status strings remain for compatibility |
| Compact/context/boil/loop | `shipped`, `experimental`, `dry_run` | `compact`, `context-status`, `context guard`, `boil`, `loop --dry-run` | compact packets, guardian handoffs, loop reports, typed context FSM transition evidence | `quant-m context guard --json` | does not call providers or execute commands; typed guardian actions separate continue, compact, refresh, review, handoff, and block outcomes | boil is experimental; loop is dry-run only |
| Cost ledger | `shipped` | `cost summary` | cost ledger records | `quant-m cost summary` | local accounting only | no live billing reconciliation |
| Consensus/strategist/question | `dry_run` / `experimental` | `consensus --dry-run`, `strategist --dry-run`, `question ask` | sessions, shared state, proposals, reports | `cargo test consensus strategist question` | dry-run paths are not live execution | question utility is still experimental |
| Worker runtime | `guarded` | `worker submit/once/run` | queue, outbox, dead-letter, worker state, typed session transition evidence | `cargo test worker` | shell/HTTP lanes gated by config; invalid worker FSM transitions fail | mutating worker execution remains single-workspace |
| Worker proposals/cluster boundary | `shipped` | `worker proposal submit/list` | proposal JSON and index | `cargo test worker_proposals cluster_boundary` | workers propose; core decides; invalid proposal review jumps fail | proposal acceptance workflow is intentionally narrow |
| Core/child edge cluster scaffold | `experimental` | `cluster init`, `cluster device options`, `cluster desk rails`, `cluster node register/list`, `cluster role assign`, `cluster heartbeat`, `cluster job submit`, `cluster child run`, `cluster report` | `workspace/state/cluster/` nodes, roles, leases, heartbeats, jobs, evidence, events | `cargo test cluster` | local-file/manual-sync only today; child nodes are evidence workers and never authority; desk rails are paper/evidence grammar only | LAN SSH and Termux SSH are declared as planned transports only; no live trading, live betting, or distributed consensus |
| Core/child QR pairing | `experimental` | `pair invite/invites/requests/approve/reject/revoke/events/fingerprint/serve`, `child pair/identity/unpair/pair-scan` | `workspace/state/pairing/`, `workspace/child/` | `cargo test pairing`; `cargo test --features pairing-scan-image child_pair_scan_decodes_valid_pairing_qr` | QR/link pairing enrolls pending observe-only children; invite storage hashes tokens; approval creates no role lease and no execution authority | minimal local HTTP pairing server is active; image-file QR decode is feature-gated; live camera control is not active |
| Core device add wizard | `experimental` | `device add <name>` | reuses `workspace/state/pairing/` and `workspace/state/cluster/leases.json` | `cargo test device --features core-full` | core-side onboarding wrapper only; no default lease, no proposals, no compute validation, no execution | does not start or supervise a pairing server; watch checks current pending requests only |
| Pi/Termux LAN validation | `design_only` | runbook only | validation notes outside release bundle | `docs/pi-termux-lan-validation.md` | proves hardware reality without adding authority | not executed in this Codex session; requires real devices |
| Child device telemetry | `experimental` | `quant-m-child doctor`, `quant-m-child heartbeat`, `cluster nodes`, `cluster report` | heartbeat/status JSON only | `cargo test device_telemetry --features core-full`; `cargo test cluster --features core-full` | telemetry is status evidence only and cannot grant lease/jobs/trust/authority | storage and battery may be unknown; Termux battery command is feature-gated |
| Real-device LAN smoke | `blocked` | validation run artifact | `docs/validation-runs/pi-termux-lan-2026-06-27.md` | blocked record only | no runtime authority added | blocked until Pi/DietPi core and Termux child are reachable |
| Local alpha release candidate | `experimental` | docs and validation artifacts only | `docs/local-alpha-release-notes.md`, `docs/known-limitations.md`, `docs/security-boundaries.md`, `docs/release/local-alpha-feature-matrix.md`, `docs/release/local-alpha-checklist.md` | `docs/validation-runs/local-alpha-release-candidate-2026-06-27.md` | release proof only; no new runtime authority | local alpha/lab only; public beta and production remain blocked |
| Adapters/channels | `shipped`, `guarded`, `unavailable` | `adapter send`, `channel list`, `telegram run` | logs/session events | `quant-m channel list --json` | channels are not execution authority | Telegram/webhook require config/secrets |
| LLM/providers/tools | `external_required` / `guarded` / `unavailable` | `provider list/validate`, `tool list/scan/validate`, `llm ask` | config and diagnostics only | `quant-m provider list --json` | detection does not equal permission | no provider/network calls during capability detection |
| Local filesystem skills | `guarded` | `skills list/show/run` | `workspace/skills/`, typed lifecycle session events | `cargo test skills` | shell-backed skills require typed policy approval and `skills.allow_shell_commands=true`; blocked shell skills are safety outcomes | declaration/detection is not execution permission |
| Registry-backed governance | `shipped` | `domain`, `skill`, `policy`, `workflow`, `fsm`, `scheduler`, `desk` | registry JSON output and FSM authority summary | `quant-m fsm authority --json` | metadata-first; policy evaluated before unsafe paths | built-ins are mock/minimal; workflow cursor is not a typed runtime FSM |
| Shared/domain state | `guarded` | `state *` | SQLite/redb state, handoffs, paper records | `quant-m state init && quant-m state summary` | paper/state modeling only; no live trading | domain-specific payloads require typed normalization |
| Cockpit planning | `experimental` | `cockpit plan` | JSON plan only | `cargo test terminal_cockpit` | previews only; does not launch terminal panes | no live cockpit adapter |
| Truth/project files | `shipped` | `init-truth` | local truth files | `quant-m init-truth --json` | creates doctrine files only | should stay small and generated where possible |
| Mock research/trading packs | `mock` | `domain show`, `run workflow`, `policy evaluate-skill` | sessions/shared state for mock workflows | `quant-m policy evaluate-skill mock-trading.prepare-paper-review` | mock trading is paper-only; live trading denied | no broker/exchange integration |
| Repeatable project skills | `design_only` | docs only unless installed as local skills | markdown guidance | none | pattern guidance, not runtime authority | convert to local skills only when needed |

This maps the "What makes OpenClaw powerful" diagram to the Quant-M minimal implementation.

## 1) Memory System

Diagram intent:
- markdown identity/memory files
- hybrid memory retrieval
- local-first storage

Quant-M implementation:
- bootstrap files:
  - `workspace/SOUL.md`
  - `workspace/USER.md`
  - `workspace/AGENTS.md`
  - `workspace/MEMORY.md`
  - `workspace/daily/*.md`
- SQLite memory index:
  - `workspace/memory/brain.db`
- Hybrid scoring in `src/memory.rs`:
  - hashed local vector signal (no external embedding API)
  - keyword overlap score
  - small recency decay bonus

## 2) Heartbeat

Diagram intent:
- runs periodically without user prompting
- proactive checks and notifications

Quant-M implementation:
- heartbeat task source:
  - `workspace/HEARTBEAT.md` (`- task` bullet format)
- commands:
  - `quant-m heartbeat tick`
  - `quant-m heartbeat run`
- runtime:
  - interval from config (`heartbeat.interval_seconds`, default 1800)
  - task execution via worker task specs:
    - `shell:<command>`
    - `http:<url>`
    - `echo:<text>`
    - `json:<job-json>`

## 3) Channel Adapters

Diagram intent:
- gateway-like adapter layer to deliver events

Quant-M implementation:
- adapter hub in `src/adapters.rs`
- supported outputs:
  - terminal JSON events (default)
  - optional webhook POST target
- used by:
  - worker result notifications
  - heartbeat notifications
  - manual `adapter send` command
 - optional Telegram polling channel in `src/telegram.rs` (disabled by default)

## 3.25) Cluster Authority Boundary

Diagram intent:
- keep Staff-OS, cmux, tmux, Termux, cron, mtime, polling, and local worker surfaces subordinate to the governed core
- let worker lanes submit evidence and proposals without granting runtime authority

Quant-M implementation:
- typed classifier in `src/cluster_boundary.rs`
- supported mock/typed surfaces:
  - `staff_os_workspace`
  - `cmux_lane`
  - `tmux_worker`
  - `termux_worker`
  - `cron_worker`
  - `mtime_worker`
  - `polling_worker`
  - `local_worker`
- worker intent records can represent evidence, reviews, completion reports, state proposals, cost proposals, approval evidence, denial evidence, escalation evidence, rejected commands, and policy blocks
- guardrails:
  - workers propose; the core decides
  - worker records do not execute workflows
  - worker records do not mutate canonical shared state
  - worker records do not append accepted cost-ledger truth
  - worker records do not call providers
  - worker records do not trigger trading behavior
  - worker records do not bypass policy, replay validation, or operator approval

## 3.3) Worker Proposal Records

Diagram intent:
- let cluster workers submit structured evidence and proposals while staying non-authoritative
- keep proposal review local, durable, append-only, and separate from accepted shared state and accepted cost truth

Quant-M implementation:
- proposal store in `src/worker_proposals.rs`
- commands:
  - `quant-m worker proposal submit --surface <surface> --kind <kind> --summary "<summary>"`
  - `quant-m worker proposal submit --surface <surface> --kind <kind> --summary "<summary>" --json`
  - `quant-m worker proposal list`
  - `quant-m worker proposal list --surface cmux_lane`
  - `quant-m worker proposal list --status pending_review --json`
- artifacts:
  - `workspace/state/worker-proposals/<proposal_id>.json`
  - `workspace/state/worker-proposals/index.jsonl`
- supported proposal kinds:
  - `evidence`
  - `review`
  - `state_suggestion`
  - `cost_suggestion`
  - `completion_report`
  - `escalation`
  - `next_action_suggestion`
- default status is `pending_review`
- every record has `non_authoritative: true`
- state suggestions are not canonical, cost suggestions are not ledger truth, and completion reports are not proof until replay validates them

## 3.4) Multi-Domain Strategist Dry Run

Diagram intent:
- prove a first cluster-shaped use case across several bounded research lanes
- coordinate worker proposals, session evidence, local artifacts, and cost accounting without live providers or execution authority

Quant-M implementation:
- command:
  - `quant-m strategist --dry-run`
  - `quant-m strategist --dry-run --json`
- module:
  - `src/strategist.rs`
- deterministic mock lanes:
  - `macro_lane`
  - `forex_carry_lane`
  - `crypto_peg_risk_lane`
  - `equity_options_risk_lane`
  - `sports_event_timing_lane`
  - `operator_audit_lane`
- artifacts:
  - `workspace/state/sessions/<session_id>/strategist-report.md`
  - `workspace/state/sessions/<session_id>/strategist-report.json`
  - `workspace/state/sessions/<session_id>/strategist-evidence-index.json`
  - `workspace/state/strategist/<workflow_id>.json`
- worker proposals:
  - one non-authoritative pending-review proposal per lane
  - proposals continue to write to `workspace/state/worker-proposals/`
- cost:
  - appends one core-created dry-run ledger record
  - provider is `mock`
  - model is `deterministic-strategist-lanes`
  - actual cost is `0.00 USD`
- guardrails:
  - no live market data
  - no provider calls
  - no network requirement
  - no trading signals or order generation
  - no worker proposal auto-acceptance
  - policy result is research-only and blocks execution

## 3.45) Core/Child Edge Cluster Scaffold

Diagram intent:
- keep Raspberry Pi, DietPi, Termux, phone, tablet, tmux, and cmux lanes as child workers
- let child devices heartbeat, receive a bounded role lease, run safe local evidence jobs, and return receipts
- preserve the rule that the core owns policy, canonical shared state, replay, scoring, proposal review, and approvals

Quant-M implementation:
- local-file cluster store in `src/cluster.rs`
- commands:
  - `quant-m cluster init`
  - `quant-m cluster device options`
  - `quant-m cluster desk rails`
  - `quant-m cluster node register --name tablet-01 --surface termux_worker --capabilities echo,sleep`
  - `quant-m cluster node list`
  - `quant-m cluster role assign --node node:tablet-01 --role forex_calendar_watcher --ttl 30m`
  - `quant-m cluster heartbeat --node node:tablet-01`
  - `quant-m cluster job submit --node node:tablet-01 --desk forex --kind echo --payload '{"text":"macro calendar clear"}'`
  - `quant-m cluster child run --node node:tablet-01`
  - `quant-m cluster report`
- artifacts:
  - `workspace/state/cluster/nodes.jsonl`
  - `workspace/state/cluster/heartbeats.jsonl`
  - `workspace/state/cluster/roles.json`
  - `workspace/state/cluster/leases.json`
  - `workspace/state/cluster/events.jsonl`
  - `workspace/state/cluster/jobs/<node>.jsonl`
  - `workspace/state/cluster/evidence/<receipt>.json`
- built-in evidence-only roles:
  - `forex_calendar_watcher`
  - `stablecoin_peg_watcher`
  - `bitcoin_dca_monitor`
  - `sports_scout`
  - `stock_index_session_watcher`
  - `prediction_market_watcher`
  - `generic_evidence_collector`
  - `browser_research_worker`
- edge device options:
  - `raspberry_pi3_dietpi_core`: the single execution-leader profile for the first demo; owns leases, queues, policy gates, and receipt intake
  - `android_termux_phone`: child evidence worker over manual sync now, planned Termux SSH later
  - `android_termux_tablet`: child evidence worker for watchlists, screenshots, and manual review
  - `raspberry_pi_edge_worker`: child LAN worker profile for polling and paper evidence capture
  - `linux_laptop_edge_worker`: heavier child worker for local models or browser-assisted evidence
- planned transport modes:
  - `ssh_lan`
  - `termux_ssh`
  - `future_http_pull`
- lead-coach knowledge-pack desk rails:
  - `forex_carry_rollover_positive_swap`
    - knowledge packs: `lead_coach/market_structure`, `lead_coach/asymmetric_risk_language`, `forex/carry_rollover`, `forex/broker_swap_terms`
    - technical language: `positive_swap_direction`, `discount_entry_zone`, `safe_margin_distance`, `rollover_carry_evidence`
    - non-negotiables: positive swap direction only, discount-zone candidates only, `0.01 paper lot` starting unit, maximum two open paper trades per pair, maximum three currency pairs
    - `USDJPY long-only` is sample language only when current broker swap evidence validates long as the positive swap direction
  - `sports_major_event_probability_scout`
    - knowledge packs: `lead_coach/event_attention`, `sports/market_baselines`, `sports/injury_and_line_movement`
    - technical language: `implied_probability`, `book_price_vs_model_price`, `closing_line_value_hypothesis`, `liquidity_and_attention_score`
    - event language: NBA playoffs, FIFA World Cup, Super Bowl, injury reports, participant motivation, public trend pressure
    - non-negotiables: no wager placement, no popularity-only candidates, price-edge evidence required
  - `crypto_dca_and_peg_monitor`
  - `stocks_options_index_session`
  - `prediction_market_event_research`
- FSMs:
  - `cluster_node`: `unregistered -> registered -> active -> stale -> suspended|retired`
  - `cluster_lease`: `requested -> granted -> renewed -> expired|revoked`
  - `cluster_job`: `created -> assigned -> accepted_by_child -> running -> evidence_returned -> recorded_by_core -> replay_verified|rejected|expired`
  - `cluster_desk_rail`: `knowledge_pack_loaded -> candidate_observed -> language_matched -> criteria_validated -> risk_box_checked -> asymmetry_scored -> paper_plan_ready|rejected`
- guardrails:
  - child roles are observe/analyze/propose only
  - child jobs are non-authoritative evidence jobs
  - desk rails validate paper candidates; they do not assert profitability or guarantee outcomes
  - child results do not mutate canonical shared state
  - child results do not approve proposals
  - child jobs do not execute trades or bets
  - `http_get` remains disabled unless config explicitly enables it
  - SSH/LAN/Termux remote execution is not active in this slice; those modes are topology metadata only

## 3.46) CORE_CHILD_QR_PAIRING_01

Purpose:
- add local QR/link pairing for tablets, phones, and edge child workers
- create short-lived invites that allow a child to request enrollment
- keep pairing separate from trust, role leasing, timing, compute validation, proposal approval, and execution

Commands:
- `quant-m pair serve --bind 0.0.0.0:8787`
- `quant-m pair fingerprint`
- `quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --link`
- `quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --qr`
- `quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --png workspace/state/pairing/tablet-01.png`
- `quant-m pair invites`
- `quant-m pair requests`
- `quant-m pair approve --request <request-id>`
- `quant-m pair reject --request <request-id>`
- `quant-m pair revoke --invite <invite-id>`
- `quant-m pair events`
- `quant-m child pair --core http://127.0.0.1:8787 --invite <invite-token>`
- `quant-m child identity`
- `quant-m child unpair`

Storage:
- `workspace/state/pairing/invites.jsonl`
- `workspace/state/pairing/requests.jsonl`
- `workspace/state/pairing/accepted-nodes.jsonl`
- `workspace/state/pairing/revoked-invites.jsonl`
- `workspace/state/pairing/events.jsonl`
- `workspace/state/pairing/core-fingerprint.json`
- `workspace/child/identity.toml`
- `workspace/child/pairing.toml`
- `workspace/child/core.toml`

Guardrails:
- invite tokens are random, short-lived, one-time by default, and stored only as hashes
- default and dev-auto-accept authority is observe
- execution, proposal approval, and canonical writes are disabled for accepted paired nodes
- child capabilities are claims only; compute claims remain unvalidated until `compute validate`
- approval creates a cluster node but does not assign a role lease
- `pair serve` runs a minimal local HTTP server for `GET /pair/i/<token>`, `POST /pair/request`, and `GET /pair/status/<request-id>`
- pairing server requests are pending by default and cannot assign leases, enable execution, approve proposals, validate compute, schedule work, or mutate canonical shared state
- PNG QR output requires `pairing-qr`
- QR image decode requires `pairing-scan-image`; live camera control is not part of this slice

## 3.47) CORE_CHILD_PAIRING_SERVER_RUNTIME_02

Purpose:
- make `pair serve` operational as a minimal local HTTP pairing server
- support tablet camera/browser onboarding through local invite pages
- keep network pairing as enrollment only, not trust or execution

Routes:
- `GET /pair/i/<invite-token>`
  - validates invite token
  - rejects expired, used, or revoked invites
  - returns a simple HTML page with core fingerprint, desk, role, expiry, authority boundary, and copyable Termux command
- `POST /pair/request`
  - validates invite token
  - rejects expired, used, or revoked invites
  - rejects authority escalation and execution/approval/canonical-write claims
  - writes a pending request
- `GET /pair/status/<request-id>`
  - returns pending, approved, rejected, expired, or revoked status
  - reports execution, approval, and canonical writes as disabled

Guardrails:
- LAN/local server only; `0.0.0.0` prints a trusted-LAN warning
- no automatic approval
- no automatic role lease
- no execution enablement
- no proposal approval
- no canonical state mutation
- no provider calls
- no compute validation
- no scheduling authority

## 3.48) CHILD_QR_IMAGE_SCAN_02

Purpose:
- decode captured QR image files for tablet/Termux pairing
- route decoded QR payloads through the same child pairing request logic as `quant-m child pair`
- avoid live camera control and authority drift

Command:
- `quant-m child pair-scan --image /tmp/quantm_pair.jpg`

Supported payloads:
- local HTTP pairing URL: `http://<local-core>:8787/pair/i/<invite-token>`
- future Quant-M URI: `quantm://pair?v=1&core=<local-core-url>&invite=<invite-token>`

Validation:
- rejects non-Quant-M QR payloads
- rejects malformed invite tokens
- rejects public/non-local URLs by default
- rejects payloads containing secret, provider, broker, exchange, execution, approval, or canonical-write fields
- rejects images with no QR payload
- rejects images with multiple conflicting pairing payloads

Guardrails:
- feature gated by `pairing-scan-image`
- image-file decode only
- no live camera access in Rust
- no automatic approval
- no automatic role lease
- no execution enablement
- no compute validation

## 3.49) CORE_CHILD_TABLET_PAIRING_E2E_03

Purpose:
- document the first end-to-end tablet pairing flow using the existing QR/link/server/scanner stack
- add doctor commands for core-side and child-side pairing troubleshooting
- keep pairing as enrollment only, never authority

Commands:
- `quant-m pair doctor`
- `quant-m pair doctor --bind 0.0.0.0:8787`
- `quant-m child doctor`
- `quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --core http://<core-lan-ip>:8787 --qr`
- `quant-m child pair-scan --image /sdcard/Download/quantm-pair.png`
- `quant-m child pair --core http://<core-lan-ip>:8787 --invite <invite-token> --name tablet-01`
- `quant-m pair approve --request <request-id>`
- `quant-m cluster heartbeat --node node:tablet-01`

Documentation:
- `docs/tablet-pairing-e2e.md`

Doctor checks:
- pairing feature availability
- core fingerprint presence
- pairing state directory presence
- active invite count
- pending request count
- accepted node count
- pairing server bind warning
- LAN URL hint
- child identity presence
- paired core URL and fingerprint
- latest pairing status
- paired node id
- last heartbeat status when a heartbeat ledger exists

Guardrails:
- pairing approval still creates no role lease
- pairing approval still grants no execution, approval, canonical write, provider-call, compute-trust, scheduling, trading, or betting authority
- heartbeat freshness is telemetry only and LAN ingest requires the approved paired-node heartbeat auth token
- `pair doctor` reads fingerprint state and does not create one implicitly
- no provider calls
- no scheduling authority

## 3.50) PAIRED_CHILD_HEARTBEAT_AND_LEASE_04

Purpose:
- let approved paired children report heartbeat visibility
- let the core grant, inspect, check, and revoke observe-only leases
- keep jobs, compute validation, scheduling, proposal approval, provider calls, trading, and betting disabled as authority outcomes

Boundary:
- pairing = known device
- heartbeat = visible device
- lease = temporary bounded permission
- job = separate future checkpoint

Commands:
- `quant-m cluster heartbeat --node node:tablet-01`
- `quant-m cluster heartbeat --node node:tablet-01 --surface termux_worker`
- `quant-m cluster nodes`
- `quant-m cluster lease grant --node node:tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 30m --authority observe`
- `quant-m cluster lease list`
- `quant-m cluster lease inspect --lease <lease-id>`
- `quant-m cluster lease check --node node:tablet-01`
- `quant-m cluster lease revoke --lease <lease-id> --reason "tablet testing complete"`
- `quant-m cluster lease revoke --node node:tablet-01 --reason "tablet testing complete"`

Storage:
- `workspace/state/cluster/heartbeats.jsonl`
- `workspace/state/cluster/leases.json`
- `workspace/state/cluster/lease-events.jsonl`

Final safe state:
- paired child can become online after heartbeat
- stale status is derived from heartbeat age
- heartbeat does not create leases
- heartbeat does not assign roles
- heartbeat rejects execution/approval/canonical-write claims
- lease authority is observe-only
- lease grant requires an approved paired child
- expired/revoked leases block future work eligibility
- lease does not enable execution, approval, canonical writes, compute validation, scheduling, provider calls, jobs, trades, or bets

## 3.51) LEASED_CHILD_ECHO_ROUNDTRIP_05

Purpose:
- prove an approved paired child can receive exactly one harmless local job kind after heartbeat and observe-only lease
- turn the child response into replay-safe evidence without granting proposal, compute, provider, shell, or execution authority
- keep lease as a boundary permission, not a scheduler or trading signal

Allowed job kind:
- `echo`

Required gates:
- node exists
- node is paired and approved
- node has a fresh heartbeat
- node has an active, non-expired, non-revoked observe-only lease
- timing gate allows evaluation

Denied job kinds and actions:
- `http_get`
- `compute_freshness_scan`
- `compute_peg_deviation`
- provider calls
- shell execution
- desk analysis
- proposal approval
- canonical writes

Evidence behavior:
- child writes an echo receipt artifact
- receipt remains replay-safe and non-authoritative
- receipt is not promoted to a proposal
- cluster report may display recent evidence only

## 3.52) LEASED_CHILD_SCALAR_COMPUTE_EVIDENCE_06

Purpose:
- allow an approved paired child to run bounded scalar numeric evidence workloads after heartbeat, observe-only lease, and timing approval
- keep scalar compute inside evidence processing, before proposal candidates, desk FSM authority, provider calls, or execution
- preserve scalar Rust as the source of truth; SIMD remains disabled for cluster execution

Allowed job kinds:
- `compute_freshness_scan`
- `compute_peg_deviation`

Allowed backend:
- `scalar`

Allowed fixtures:
- `evidence_freshness`
- `evidence_freshness_scan`
- `stablecoin_peg`
- `stablecoin_peg_deviation`
- `boundary_ambiguous_peg_scan`

Required gates:
- node is paired and approved
- heartbeat is fresh
- observe-only lease is active, non-expired, and non-revoked
- role has `compute_scalar`
- timing gate allows evaluation

Evidence behavior:
- receipt remains replay-safe and non-authoritative
- artifact records backend requested, backend used, scalar validation outcome, numeric confidence, input hash, output hash, timing decision id, node id, role id, and lease id
- cluster report may summarize compute evidence
- compute output does not create proposals or alter authority

Rejected:
- SIMD or accelerated backend execution
- net-edge or arbitrage workloads
- forex/sports strategy signals
- `http_get`
- shell execution
- provider calls
- desk proposals
- proposal approval
- canonical writes
- trading or betting

## 3.53) LEASED_CHILD_DESK_OBSERVE_EVIDENCE_07

Purpose:
- bind leased child scalar compute outputs into desk-labeled observation evidence
- make tablets desk-aware without making them proposal or decision makers
- keep observation evidence below core-only aggregation, proposal creation, risk decisions, trading, and betting

Allowed job kinds:
- `desk_observe_evidence_freshness`
- `desk_observe_peg_deviation`

Evidence envelope:
- `DeskObservationEvidence`
  - node id
  - lease id
  - desk id
  - role id
  - optional knowledge pack id
  - evidence kind
  - observe-only authority
  - timing decision id
  - input hash
  - output hash
  - compute metadata
  - numeric confidence
  - replay-safe status
  - `proposal_created: false`

Allowed evidence kinds:
- `evidence_freshness_observation`
- `stablecoin_peg_deviation_observation`

Required gates:
- approved paired child
- fresh heartbeat
- active observe-only lease
- lease desk/role matches job desk/role
- role has `compute_scalar`
- timing gate allows evaluation
- fixture is local and allowlisted
- backend is scalar
- payload contains no provider, HTTP, shell, net-edge, arbitrage, proposal, execution, trade, or bet intent

Rejected:
- provider calls
- `http_get`
- shell
- net-edge or arbitrage language
- desk analysis/strategy decisions
- proposal creation
- approval
- canonical writes
- execution
- trading or betting

## 3.54) DESK_PLAYBOOK_MODEL_HANDOFF_08

Purpose:
- give every desk role a versioned playbook bundle containing desk language, knowledge-pack refs, rails, allowed evidence kinds, forbidden outputs, model task limits, and shared-state snapshot refs
- create provider-neutral model handoff packets from playbook plus shared-state snapshot
- let a local stub model return structured output and shared-state update proposals only
- keep provider calls feature-gated and disabled by default

Core rule:
- playbooks travel to models
- authority does not

Source-of-truth rule:
- the playbook is instruction context
- the shared-state snapshot is factual context
- the model output is candidate analysis
- the core validator decides whether shared state changes
- the FSM decides whether validated shared state matters
- risk decides whether action is permitted

Commands:
- `quant-m playbook list`
- `quant-m playbook inspect --desk crypto --role stablecoin_peg_watcher`
- `quant-m playbook validate --desk crypto --role stablecoin_peg_watcher`
- `quant-m playbook bundle --desk crypto --role stablecoin_peg_watcher`
- `quant-m playbook hash --playbook stablecoin_peg_watcher`
- `quant-m cluster lease grant --node node:tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 30m --authority observe --playbook stablecoin_peg_watcher`
- `quant-m model handoff create --desk crypto --role stablecoin_peg_watcher --playbook stablecoin_peg_watcher --snapshot latest --task detect-contradictions`
- `quant-m model handoff inspect --handoff <handoff-id>`
- `quant-m model handoff export --handoff <handoff-id> --out /tmp/handoff.json`
- `quant-m model call --provider local-stub --handoff <handoff-id>`
- `quant-m shared-state updates`
- `quant-m shared-state update inspect --update <update-id>`

Storage:
- `workspace/playbooks/desks/`
- `workspace/playbooks/roles/`
- `workspace/playbooks/bundles/`
- `workspace/playbooks/handoffs/`
- `workspace/state/shared/snapshots/`
- `workspace/state/shared/update-proposals/`
- `workspace/state/model-handoffs/`
- `workspace/state/model-calls/`
- `workspace/state/model-outputs/`
- `workspace/state/model-validation/`

Guardrails:
- playbook authority is observe/analyze only
- leases may bind a playbook id/hash
- queued jobs reject playbook hash mismatch against active lease
- handoff packets are replay-safe JSON
- local stub creates pending shared-state update proposals only
- OpenRouter and direct OpenAI provider calls are feature-gated stubs only in this checkpoint
- provider API keys remain core-side and never enter playbooks, handoffs, QR, or child identity
- model output cannot mutate shared state directly
- model output cannot create proposals, strategy decisions, risk decisions, trade decisions, or bet decisions
- forbidden trade/bet/execution/canonical-write language is rejected

## 3.55) PLAYBOOK_HANDOFF_ADVERSARIAL_HARDENING_08A_AND_SHARED_STATE_UPDATE_VALIDATION_09

Purpose:
- harden playbook-bound model handoffs before any real provider calls are enabled
- keep model output as candidate analysis until core validation accepts typed shared-state facts
- prevent playbooks, evidence, or model output from becoming hidden authority

Core law:
- playbooks travel to models
- authority does not

Implementation:
- all playbook, bundle, handoff, section, update, fact, and snapshot hashes use `sha256` over `canonical_json_v1`
- handoff packets separate `system_boundary`, `playbook_contract`, `shared_state_snapshot`, `evidence_quoted_data`, `model_task`, `output_schema`, and `forbidden_outputs`
- evidence is explicitly marked as quoted data, not instruction
- allowed model tasks are limited to evidence summarization, fact extraction, contradiction detection, observation labeling, and shared-state update suggestions
- model-derived facts default to `ephemeral` decay and never become canonical by default

Commands:
- `quant-m shared-state update validate --update <update-id>`
- `quant-m shared-state update accept --update <update-id> --reason "<reason>"`
- `quant-m shared-state update reject --update <update-id> --reason "<reason>"`
- `quant-m shared-state facts`
- `quant-m shared-state fact inspect --fact <fact-id>`
- `quant-m shared-state snapshots`
- `quant-m shared-state snapshot create --desk crypto`
- `quant-m shared-state snapshot inspect --snapshot <snapshot-id>`

Storage:
- `workspace/state/shared/pending-updates.jsonl`
- `workspace/state/shared/accepted-facts.jsonl`
- `workspace/state/shared/rejected-updates.jsonl`
- `workspace/state/shared/contradictions.jsonl`
- `workspace/state/shared/snapshots.jsonl`
- `workspace/state/shared/validation-events.jsonl`

Guardrails:
- provider calls remain disabled by default
- child nodes cannot create provider-bound handoffs
- `validate` may mark a candidate clean/review/rejected but does not accept facts
- `accept` requires an operator reason and writes accepted facts only after validation
- pending updates are labeled candidate/unvalidated/not in shared state
- accepted facts still have no execution, proposal approval, FSM, trading, or betting authority

## 3.56) EDGE_RUNTIME_MINIMIZATION_10

Purpose:
- shift from feature expansion to runtime minimization for edge devices
- split the repo mentally and practically into a rich core CLI and a tiny child CLI
- reduce child command surface, storage assumptions, and authority exposure

Implementation:
- binary: `quant-m`
- child binary: `quant-m-child`
- documented feature profiles:
  - `core-full`
  - `child-min`
  - `child-pairing`
  - `child-scan-image`
  - `child-compute`
  - `dev-all`
- documented release profiles:
  - `release`
  - `release-core`
  - `release-child`
- documentation: `docs/edge-runtime-minimization.md`

Child command surface:
- `quant-m-child pair`
- `quant-m-child pair-scan`
- `quant-m-child identity`
- `quant-m-child doctor`
- `quant-m-child heartbeat`
- `quant-m-child run-once`

Child-min exclusions:
- no model router commands
- no provider adapters
- no shared-state accept/reject CLI
- no pairing server
- no QR image scan unless `child-scan-image` is enabled
- no playbook authoring
- no proposal approval
- no execution adapters
- no trading or betting commands

Storage:
- `workspace/child/identity.toml`
- `workspace/child/core.toml`
- `workspace/child/playbook-cache/`
- `workspace/child/outbox/`
- `workspace/child/logs/`
- `workspace/child/outbox/heartbeats.jsonl`
- `workspace/child/outbox/job-receipts.jsonl`

Measured state:
- `quant-m-child` child-min dev binary before pruning: `4,233,888` bytes
- `quant-m-child` release-child after pruning: `653,120` bytes
- `quant-m` core dev binary before pruning: `34,693,752` bytes
- package-level `child-min` dependency tree now contains only `anyhow`, `chrono`, `clap`, `serde`, `serde_json`, and `toml`
- `child-min` no longer compiles `reqwest`, `tokio`, `rusqlite`, `redb`, `ratatui`, `crossterm`, `postcard`, `rand`, `ring`, `image`, `qrcode`, or `rqrr`

Guardrails:
- no new desk feature
- no provider call
- no FSM proposal
- no new database
- no live network complexity
- child remains sensor/runner only

## 3.57) CORE_DEVICE_ADD_WIZARD_11

Purpose:
- make adding a tablet, phone, Raspberry Pi, or Termux worker easier from the core CLI
- wrap the existing pairing, request, approval, heartbeat visibility, and optional observe-only lease layers
- preserve the rule that operator convenience does not create child authority

Command:
- `quant-m device add <name> --desk <desk-id> --role <role-id>`
- optional display flags:
  - `--qr`
  - `--link`
  - `--png <path>`
- optional orchestration flags:
  - `--watch`
  - `--auto-approve-observe`
  - `--grant-observe-lease`
  - `--lease-ttl <duration>`
  - `--ttl <duration>`
  - `--core <url>`
  - `--no-server`
  - `--json`

Implementation:
- Rust module: `src/device.rs`
- core CLI only: `src/main.rs`
- library gate: not compiled under `child-min`
- reuses pairing functions for invite, terminal QR rendering, PNG generation, request inspection, and approval
- reuses cluster functions for approved-node lookup, heartbeat-derived node status, and observe-only lease grant

Storage:
- no new storage
- invite/request/approval state comes from `workspace/state/pairing/`
- optional observe lease state comes from `workspace/state/cluster/leases.json`
- lease events continue to use `workspace/state/cluster/lease-events.jsonl`

Final safe state:
- paired child may be known after approval
- child may be online or stale based on existing heartbeat records
- lease is `none` by default
- explicit lease is observe-only
- execution is disabled
- proposal creation is disabled
- approval is disabled
- canonical writes are disabled
- provider calls are disabled
- compute validation is `none`

Guardrails:
- no new authority model
- no new lease model
- no new node registry
- no pairing storage duplication
- no child-side wizard or child dependency expansion
- no jobs beyond existing cluster gates
- no compute validation
- no provider calls
- no proposal creation
- no shared-state acceptance
- no execution, trading, or betting
- `--auto-approve-observe` is capped to a short TTL and observe authority only
- `--grant-observe-lease` requires an already approved paired node

Edge impact:
- `quant-m-child` affected: no
- new child dependency: no
- feature-gated out of `child-min`: yes
- default feature impact: none
- child-min validation path: `cargo check --bin quant-m-child --no-default-features --features child-min`
- release-child size before this checkpoint: `653,120` bytes
- release-child size after this checkpoint: `653,120` bytes

## 3.58) DEVICE_ADD_INTERACTIVE_APPROVAL_12

Purpose:
- make `quant-m device add --watch` a real terminal approval flow
- keep device onboarding core-side while reusing existing pairing, approval, rejection, heartbeat, and lease logic
- improve operator usability without making the child more powerful

Command additions:
- `--serve`
- `--bind <addr>`
- `--watch-timeout <seconds>`
- `--watch-poll <seconds>`

Interactive watch behavior:
- creates a short-lived observe-only invite
- prints the local link and child command
- optionally renders terminal QR or PNG through the existing QR path
- checks current pending requests for the invite
- also detects an existing pending request for the same device name when the operator reruns the wizard
- displays request id, device name, surface, claimed capabilities, compute-claim posture, requested authority, and disabled authority fields
- prompts `Approve observe-only child? [y/N]`
- approves only on explicit `y` or `yes`
- rejects on empty/default input
- prints final safe state

Server behavior:
- `--serve` explicitly starts the existing local pairing server in the core process
- binding to `0.0.0.0` still prints the trusted-LAN warning
- without `--serve`, the wizard tells the operator how to start `quant-m pair serve`
- no hidden network exposure is introduced

Default safe state:
- manual approval required
- no lease by default
- no jobs run
- no compute validation
- no provider calls
- no proposal creation
- no execution
- no approval authority
- no canonical writes

Guardrails:
- uses existing pairing request approval/rejection storage
- uses existing cluster observe-lease storage only when `--grant-observe-lease` is explicit
- `--auto-approve-observe` remains observe-only and TTL capped
- child requests with execution, approval, canonical-write, provider credential, broker, or exchange-key claims remain rejected at pairing intake
- no child command surface expansion
- no scheduler, job, proposal, provider, compute-validation, trading, or betting authority is added

Edge impact:
- `quant-m-child` affected: no
- new child dependency: no
- release-child size before this checkpoint: `653,120` bytes
- release-child size after this checkpoint: `653,120` bytes

## 3.59) PI_TERMUX_LAN_VALIDATION_13

Purpose:
- prove the latest core/child local-alpha flow on real devices over a trusted LAN
- validate Raspberry Pi/DietPi core behavior and Termux child behavior
- capture hardware, OS, architecture, LAN IPs, commit, binary size, and safe final state
- avoid adding new runtime authority while moving from local simulation toward hardware evidence

Runbook:
- `docs/pi-termux-lan-validation.md`

Minimum matrix:
- Raspberry Pi 3 on DietPi as preferred core
- laptop as core fallback
- Android phone or tablet with Termux as child
- optional Raspberry Pi edge child

Validated flow:
- core build
- child-min build
- pairing server on trusted LAN
- `device add --watch`
- manual or QR/link pairing
- explicit terminal approval
- heartbeat
- optional observe-only lease
- echo evidence
- scalar freshness evidence
- scalar peg-deviation evidence
- desk observation evidence if enabled
- cluster report
- cleanup

Important current limitation:
- cluster job execution is still local-file/manual-sync oriented
- `quant-m cluster child run` validates the existing file-backed cluster worker path
- `quant-m-child run-once` remains intentionally tiny and accepts local echo JSON only
- LAN SSH, Termux SSH, and remote job transport are not shipped in this checkpoint

Expected safe final state:
- paired child approved
- heartbeat fresh
- observe lease active only if explicitly granted
- proposal count zero
- execution false
- approval false
- canonical writes false
- provider calls zero
- trading false
- betting false

Guardrails:
- no new child dependency
- no new runtime authority
- no provider calls
- no proposal creation
- no execution, trading, or betting
- no public internet exposure
- no generated workspace state, build output, logs, copied repos, secrets, or live QR tokens in release bundles

## 3.60) CHILD_DEVICE_TELEMETRY_14

Purpose:
- add minimal child-device telemetry for operator visibility
- surface device status during child doctor, child heartbeat, cluster node inspection, and cluster report
- keep telemetry as status evidence only

Telemetry fields:
- device display name
- hostname when locally available
- model hint when locally available
- OS
- architecture
- storage total/available/used percent when supported
- battery percent/charging/status/health/temperature when supported
- collection errors
- collection timestamp

Implementation:
- shared Rust module: `src/device_telemetry.rs`
- child CLI integration: `src/bin/quant-m-child.rs`
- core cluster heartbeat/status/report integration: `src/cluster.rs`
- no new database
- no large `sysinfo` or battery dependency
- no child SQLite/redb dependency
- no async stack

Feature flags:
- `device-telemetry`
- `device-telemetry-termux`
- `device-telemetry-df-fallback`
- `child-min` includes `device-telemetry`
- `child-termux` enables `device-telemetry-termux`

Detection strategy:
- OS/arch use `std::env::consts`
- hostname uses `/proc/sys/kernel/hostname` or `HOSTNAME`
- model hint uses `/proc/device-tree/model` or `/sys/devices/virtual/dmi/id/product_name`
- Linux battery uses `/sys/class/power_supply`
- Termux battery JSON parser supports `termux-battery-status` output
- fixed Termux command invocation is behind `device-telemetry-termux`
- storage is unknown by default unless a future supported path or explicit fallback is enabled

Display:
- `quant-m-child doctor` prints a Device, Storage, Battery, and Authority section
- `quant-m-child heartbeat` records telemetry in the child heartbeat JSON
- cluster heartbeat records may include optional telemetry
- `quant-m cluster nodes` shows device, OS, arch, battery, storage, and advisory warnings
- `quant-m cluster report` includes a Device health section when telemetry exists

Advisory warnings:
- low battery below 20%
- low storage below 1 GiB
- warnings do not block jobs
- warnings do not revoke leases
- warnings do not change timing or authority

Security and privacy:
- telemetry payloads over 16 KiB are rejected
- telemetry containing authority or credential language is rejected
- telemetry must not include environment dumps, API keys, broker/exchange/sportsbook credentials, location, MAC address, phone number, contacts, installed apps, clipboard, photos, microphone, camera, or SMS

Guardrails:
- telemetry does not create leases
- telemetry does not extend leases
- telemetry does not enable jobs
- telemetry does not validate compute
- telemetry does not increase evidence score
- telemetry does not increase trust
- telemetry does not create proposals
- telemetry does not execute, trade, or bet

Measured child impact:
- release-child before telemetry: `653,120` bytes
- release-child after telemetry: `669,728` bytes
- release-child after LAN pairing and heartbeat sync: `720,112` bytes
- delta: `16,608` bytes

## 3.61) REAL_DEVICE_LAN_SMOKE_14A

Purpose:
- run the latest Quant-M core/child flow on actual Pi/DietPi and Termux devices
- capture real telemetry output from child doctor, heartbeat, cluster nodes, and cluster report
- prove local-alpha hardware behavior before packaging work
- add no new runtime authority

Validation artifact:
- `docs/validation-runs/pi-termux-lan-2026-06-27.md`

Current result:
- `blocked`
- no real Raspberry Pi/DietPi core was reachable from this Codex workspace
- no real Android Termux child was reachable from this Codex workspace
- no real LAN pairing server was verified from a second device
- no real Termux battery telemetry was captured

Required pass criteria when hardware is available:
- paired child approved
- child telemetry present or unknown with reason
- heartbeat fresh
- observe-only authority
- optional observe lease active only if explicitly granted
- echo evidence recorded
- scalar freshness evidence recorded
- scalar peg-deviation evidence recorded
- proposal count zero
- execution false
- provider calls false
- canonical writes false
- trading false
- betting false

Guardrails:
- blocked validation records must not be treated as pass evidence
- no feature should be promoted to public beta based on this blocked run
- validation notes must not include secrets, logs with invite tokens, build output, generated workspace state, or copied repos

## 4.5) Desk Timing Frameworks

Purpose:
- make time a first-class risk control for every Quant-M desk
- declare when each desk or child role may wake, collect, refresh, evaluate, or propose
- keep timing separate from execution authority

Implementation:
- Rust module: `src/timing.rs`
- commands:
  - `quant-m timing list`
  - `quant-m timing inspect --desk forex`
  - `quant-m timing next --desk forex`
  - `quant-m timing check --desk forex --role forex_calendar_watcher --dry-run`
  - `quant-m timing check --node tablet-01 --dry-run`
  - `quant-m timing cooldowns`
- storage paths:
  - `workspace/state/timing/policies.json`
  - `workspace/state/timing/events.jsonl`
  - `workspace/state/timing/cooldowns.json`
  - `workspace/state/timing/stale-rejections.jsonl`

Timing grammar:
- `TimingTrigger`
  - `Cron`
  - `Polling`
  - `Mtime`
  - `SessionWindow`
  - `EventWindow`
  - `Heartbeat`
  - `Cooldown`
- `MarketSession`
  - Forex Tokyo, London, New York
  - crypto always-on
  - US equities premarket, regular, after-hours
  - sports pregame, live, postgame
- `DeskTimingPolicy`
  - desk id
  - role id
  - allowed triggers
  - stale evidence limit
  - min/max refresh cadence
  - cooldown after proposal
  - tablet-safe flag

Desk defaults:
- Forex: hybrid cron, polling, session windows, event windows, heartbeat, cooldown
- Bitcoin DCA: cron, polling, crypto always-on session, cooldown
- Stablecoin peg/Albatraoz: polling, event windows, cooldown
- Stocks/options indexes: cron, polling, US equity sessions, event windows, cooldown
- Sports: cron, event windows, session windows, polling, cooldown
- Prediction markets: polling, event windows, mtime, cooldown

Timing FSM outputs:
- `WaitTiming`
- `RejectStaleEvidence`
- `RejectCooldown`
- `WatchOnlyEventWindow`
- `AllowEvaluation`

Tablet and child rules:
- tablet-safe roles enforce 60-second minimum polling
- tablet heartbeat defaults to 60 seconds
- tablet role lease default remains 30 minutes
- child nodes cannot override assigned timing policy, cooldown, lease expiration, authority level, or role
- mtime triggers must stay inside the workspace and cannot watch secrets/keys/system paths

Guardrails:
- timing decisions are replay-safe policy decisions, not execution authority
- timing cannot execute trades, bets, shell, provider calls, or remote network actions
- dry-run CLI checks are inspect-only for this checkpoint
- event windows can force watch-only mode
- stale evidence and active cooldowns reject evaluation/proposal

## 5.5) EDGE_COMPUTE_SCALAR_FIRST_SIMD_READY_01

Purpose:
- add a scalar-first edge compute framework after timing-to-Desk-FSM gates
- prepare Quant-M for optional future SIMD without trusting child-node capability claims
- keep replay deterministic across tablets, ARM boards, and x86 machines

Core rule:
- scalar Rust is the source of truth
- SIMD is optional future acceleration
- child compute claims are untrusted evidence until core-side validation records verify them

Non-goals:
- production SIMD execution
- live trading or live betting
- SIMD role authority
- benchmark-based scheduling authority
- child self-certification
- C, C++, assembly, Python, or GPU dependencies

Forbidden compute authority targets:
- FSM authority
- policy decisions
- proposal approval
- child node trust
- LLM routing
- desk role assignment
- execution logic

Implementation:
- Rust module layout:
  - `src/compute/mod.rs`
  - `src/compute/backend.rs`
  - `src/compute/capability.rs`
  - `src/compute/validation.rs`
  - `src/compute/scalar.rs`
  - `src/compute/boundary.rs`
  - `src/compute/bench_policy.rs`
  - `src/compute/fixtures.rs`
- Rust module: `src/numeric_hotpath.rs`
- commands:
  - `quant-m compute capabilities`
  - `quant-m compute freshness-scan --fixture evidence_freshness --backend scalar`
  - `quant-m compute peg-deviation --fixture stablecoin_peg_deviation --backend scalar`
  - `quant-m compute bench --workload peg-deviation --samples 10000 --manual`
  - `quant-m numeric bench stablecoin-peg --samples 1024`
- cargo features:
  - `compute`
  - `compute-bench`
  - `compute-simd-experimental`
- storage paths:
  - `workspace/state/cluster/compute-validations.jsonl`
  - `workspace/state/compute/mismatches.jsonl`
  - `workspace/state/compute/backend-quarantine.json`
- fixtures:
  - `fixtures/compute/evidence_freshness_scan.json`
  - `fixtures/compute/stablecoin_peg_deviation_scan.json`
  - `fixtures/compute/boundary_ambiguous_peg_scan.json`

First scalar workloads:
- evidence freshness scan:
  - inputs: evidence ids, timestamps, now timestamp, stale-after seconds
  - outputs: fresh ids, stale ids, counts
- peg deviation scan:
  - inputs: prices, target peg, stale flags
  - outputs: absolute deviations, bps deviations, max deviation, stale count, numeric confidence
  - no net edge
  - no arbitrage language
  - no proposal classification

Boundary ambiguity:
- threshold-near values are marked `BoundaryAmbiguous`
- boundary-ambiguous evidence cannot create a proposal candidate by itself
- scalar verification is required before later acceleration can be trusted

Guardrails:
- no behavior changes
- no execution authority
- no trades or bets
- no provider calls
- no shell or remote execution
- no benchmark output as evidence
- no benchmark output as proposal input
- no benchmark score as evidence weight
- no SIMD capability as evidence weight
- no backend is usable unless hardware, compile, implementation, self-test, and scalar equivalence are all true

## 5.6) EDGE_COMPUTE_CORE_VALIDATION_ROUNDTRIP_02

Purpose:
- move from compute schemas to core-side validation records
- keep scalar-first compute as evidence processing only
- prove backend trust is written, inspected, and enforced through local ledgers before any cluster scheduling integration

Commands:
- `quant-m compute validate --node node:tablet-01 --backend scalar --fixture evidence_freshness`
- `quant-m compute validations`
- `quant-m compute mismatches`
- `quant-m compute quarantine`

Operational behavior:
- scalar validation runs a deterministic fixture and appends `workspace/state/cluster/compute-validations.jsonl`
- unsupported accelerated backend validation appends a mismatch and updates `workspace/state/compute/backend-quarantine.json`
- mismatches are append-only records under `workspace/state/compute/mismatches.jsonl`
- quarantined backends are not selected as trusted usable backends
- replay metadata records accelerated requests as audit data, while replay remains scalar-first

Still not allowed:
- production SIMD execution
- cluster role authority from compute speed
- scheduling priority from benchmark score
- child self-certification
- proposal generation from compute output
- live trading or betting

## 5.7) EDGE_COMPUTE_CLUSTER_EVIDENCE_BINDING_03

Purpose:
- bind scalar-first compute into the child/cluster evidence pathway
- let tablets and edge nodes run bounded scalar evidence jobs under lease, heartbeat, timing, validation, and replay constraints
- keep compute capability as eligibility metadata only, never authority

Commands:
- `quant-m cluster job submit --node node:tablet-01 --desk research --kind compute_freshness_scan --payload '{}' --fixture evidence_freshness --backend scalar`
- `quant-m cluster job submit --node node:edge-peg-01 --desk crypto --kind compute_peg_deviation --payload '{}' --fixture stablecoin_peg_deviation --backend scalar`
- `quant-m cluster child run --node node:tablet-01`
- `quant-m cluster report`

Behavior:
- compute jobs require an active role lease
- compute jobs require a fresh child heartbeat
- compute jobs pass timing gates before queueing/running
- compute jobs reject quarantined accelerated backends
- scalar freshness scan and peg deviation scan run as evidence jobs only
- evidence artifacts include backend requested, backend used, scalar fallback status, scalar verification status, numeric confidence, input hash, output hash, and timing decision id

Hard boundary:
- child compute output may return numeric evidence and metadata only
- child compute output may not return `enter`, `buy`, `sell`, `bet`, `arb`, `edge approved`, `proposal accepted`, or risk approval language
- compute evidence does not create proposals directly
- compute capability can affect workload eligibility
- compute capability cannot affect authority, trust score, proposal confidence, or scheduling priority

## 3.5) Universal Question Utility

Diagram intent:
- give Quant-M one governed question interface for every near-term route
- keep Agent Cluster, Staff-OS Handoff, and Harness as the only utility modes
- force raw input through evidence, proposal, policy, cost, replay, and next-action fields before work expands

Quant-M implementation:
- command:
  - `quant-m question ask --mode agent-cluster "<question>"`
  - `quant-m question ask --mode agent-cluster "<question>" --write-proposals`
  - `quant-m question ask --mode staff-os-handoff "<question>" --json`
  - `quant-m question ask --mode harness "<question>"`
- module:
  - `src/question.rs`
- universal question:
  - `What should happen next, based on the available evidence, policy, cost, state, and operator goal?`
- shared contract:
  - `question`
  - `evidence`
  - `proposal`
  - `policy_gate`
  - `cost_record`
  - `replayable_decision`
  - `next_safe_action`
- supported modes:
  - `agent_cluster`
  - `staff_os_handoff`
  - `harness`
- guardrails:
  - no trading, chat, dashboard, onboarding, or provider-specific question modes
  - domain-specific work must flow through one of the three utility modes
  - workers propose and the core decides
  - provider use remains budget-gated through harness mode
- agent-cluster proposal bridge:
  - default output is an inspect-only worker proposal plan
  - `--write-proposals` materializes the plan into pending non-authoritative worker proposal records
  - written proposal records continue to use `workspace/state/worker-proposals/`
  - `--write-proposals` is currently limited to `agent_cluster`
  - generated plan cost is zero actual and local-only

## 3.6) LLM API

Quant-M implementation:
- minimal OpenAI-compatible chat completion client in `src/llm.rs`
- intended target: OpenRouter (`llm.api_base`)
- used by:
  - `quant-m llm ask`
  - Telegram `/ask ...` messages

## 4) Skills Registry

Diagram intent:
- local skill files, instantly available
- avoid remote supply-chain exposure

Quant-M implementation:
- local path only:
  - `workspace/skills/<skill>/SKILL.md`
  - optional `SKILL.toml` with `[run].command`
- commands:
  - `quant-m skills list`
  - `quant-m skills show <name>`
  - `quant-m skills run <name> <input>`
- no remote registry/install path in this lite build

## 5) Android Worker Runtime Focus

Required deployment model from your brief:
- coordinator (Pi/VPS) controls worker nodes
- Android nodes execute narrow tasks and return JSON/logs

Quant-M implementation:
- queue files:
  - inbox: `workspace/queue/inbox.ndjson`
  - outbox: `workspace/queue/outbox.ndjson`
  - dead-letter: `workspace/queue/dead-letter.ndjson`
  - inflight: `workspace/queue/inflight.json`
- lifecycle commands:
  - `quant-m worker submit <job-json>`
  - `quant-m worker once <job-json>`
  - `quant-m worker run`
  - `quant-m daemon start`
- reliability behavior:
  - atomic inbox drain (`rename` to snapshot before parse)
  - durable batch file to recover unprocessed drained jobs on crash
  - malformed lines moved to dead-letter queue
  - daemon supervises worker/heartbeat with exponential backoff on failures
  - graceful daemon shutdown via shared cancellation signal
  - shell/http execution gated behind explicit config flags
- health/state:
  - `workspace/state/worker_state.json`
  - `quant-m status`

## 6) Pi-Inspired Compression Hardening

Diagram intent:
- keep long agent sessions usable
- hand work between models, terminals, and devices
- preserve lightweight ergonomics without a large extension surface

Planned Quant-M implementation:
- hardening plan:
  - `docs/pi-inspired-feature-hardening-plan.md`
- first primitive:
  - `quant-m compact <session_id>`
- planned artifact shape:
  - `workspace/state/compacted/<session_id>/compact.md`
  - `workspace/state/compacted/<session_id>/compact.json`
  - `workspace/state/compacted/<session_id>/evidence-index.json`
  - `workspace/state/compacted/<session_id>/next-action.md`
  - `workspace/state/compacted/<session_id>/risks.md`
- guardrails:
  - compaction is read-only
  - compact packets cite session evidence
  - provider login never grants execution permission
  - slash commands route through existing policy/storage-mode gates

## 7) Context Status Gate

Diagram intent:
- prevent loops, agents, or channels from continuing on stale or incomplete context
- reuse compact packets instead of inventing another summary layer
- make missing validation, policy, or shippable evidence visible

Quant-M implementation:
- command:
  - `quant-m context-status`
  - `quant-m context-status --json`
- reads:
  - latest session under `workspace/state/sessions/`
  - compact packet under `workspace/state/compacted/<session_id>/compact.json`
  - local truth files such as `QUANTM.md`, `POLICY.md`, `SHIPPABLE.md`, and `AGENTS.md`
- output:
  - `context_state`: `green`, `yellow`, or `red`
  - compact packet presence/staleness
  - policy, validation, changed-file, and shippable evidence booleans
  - missing context list
  - recommended next action
- guardrails:
  - read-only
  - does not create compact packets automatically
  - does not mutate session history
  - does not infer validation or shippable success from silence

## 8) Project Truth Files

Diagram intent:
- give future loops and agents stable local truth
- make `context-status` missing-context checks actionable
- keep policy and shippable criteria human-readable without creating a giant doctrine layer

Quant-M implementation:
- command:
  - `quant-m init-truth`
  - `quant-m init-truth --json`
  - `quant-m init-truth --force`
- creates missing files under the configured workspace:
  - `QUANTM.md`
  - `POLICY.md`
  - `SHIPPABLE.md`
  - `AGENTS.md`
- safety defaults:
  - no live trading
  - no credential edits
  - no shell, HTTP, network, terminal, or cockpit escalation without explicit approval
  - preserve evidence
  - do not claim validation without proof
  - generated agents define lanes without granting permissions
- guardrails:
  - does not overwrite existing files unless `--force` is provided
  - reports files as created, present, or overwritten
  - recommends `quant-m context-status` after creation

## 9) Loop Dry Run

Diagram intent:
- bring GrepLoop/Hermes-style local improvement scanning into Quant-M without mutation
- rank safe next actions from evidence instead of launching autonomous edits
- surface stale or missing context before a future apply loop exists

Quant-M implementation:
- command:
  - `quant-m loop --dry-run`
  - `quant-m loop --dry-run --json`
  - `quant-m loop --dry-run --scope repo|docs|sessions|truth|all`
  - `quant-m loop --dry-run --max-candidates <n>`
- outputs:
  - `workspace/state/loops/<loop_id>/loop-report.md`
  - `workspace/state/loops/<loop_id>/loop-report.json`
  - `workspace/state/loops/<loop_id>/candidates.json`
  - `workspace/state/loops/<loop_id>/evidence-index.json`
  - `workspace/state/loops/<loop_id>/context-decay.json`
- guardrails:
  - read-only except loop report artifacts
  - does not mutate truth files, compact packets, sessions, source, docs, provider config, or policy config
  - red `context-status` marks execution readiness as blocked
  - context decay never auto-deprecates canonical truth files

## 10) Context Decay

Diagram intent:
- degrade stale, weakly evidenced, or contradicted context without mutating project files
- make decay scoring reusable across loops, compaction, context status, branching, and future evidence export
- preserve canonical project truth as operator-reviewed doctrine rather than disposable context

Quant-M implementation:
- shared module:
  - `src/context_decay.rs`
- core types:
  - `ContextItem`
  - `MemoryClass`
  - `ContextDecayScore`
  - `DecayAction`
  - `DecayReason`
- scoring fields:
  - authority, freshness, validation, usage, shippable relevance, and contradiction penalty
- integration:
  - `loop_dry_run` uses the shared scorer when writing `context-decay.json`
- guardrails:
  - deterministic scoring
  - no file mutation
  - missing validation lowers authority
  - stale compact packets lower authority
  - repeated references raise authority
  - `POLICY.md`, `SHIPPABLE.md`, `QUANTM.md`, and `AGENTS.md` are never auto-deprecated
