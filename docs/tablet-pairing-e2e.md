# Quant-M Tablet Pairing E2E

`CORE_CHILD_TABLET_PAIRING_E2E_03` documents the first real tablet pairing flow for local Quant-M child nodes.

Pairing is enrollment only. It does not create a role lease, schedule jobs, validate compute, approve proposals, mutate canonical shared state, call providers, trade, or bet.

## Core Setup

On the core node:

```bash
quant-m pair doctor
quant-m pair serve --bind 0.0.0.0:8787
```

Use `0.0.0.0` only on a trusted LAN. Use `127.0.0.1` for local testing.

Create a short-lived invite:

```bash
quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --core http://<core-lan-ip>:8787 --qr
```

For image-file QR testing, build with `pairing-qr` and save a PNG:

```bash
quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --core http://<core-lan-ip>:8787 --png workspace/state/pairing/tablet-01.png
```

## Tablet Setup

On Termux:

```bash
pkg update
pkg install git rust clang pkg-config openssl
```

Clone or sync the Quant-M workspace onto the tablet, then run:

```bash
quant-m child doctor
```

The tablet should start with no child identity and no paired core unless it has paired before.

## QR Camera Flow

The Rust QR scanner accepts image files, not live camera streams. Capture a QR image with the tablet camera or a Termux camera helper, then scan the saved file:

```bash
quant-m child pair-scan --image /sdcard/Download/quantm-pair.png
```

The scanner accepts local Quant-M pairing URLs and rejects non-local/public URLs, malformed tokens, secret-bearing payloads, execution claims, approval claims, and canonical-write claims.

## Manual Fallback

If camera capture is unavailable, copy the command printed by the core invite:

```bash
quant-m child pair --core http://<core-lan-ip>:8787 --invite <invite-token> --name tablet-01 --surface termux_worker --capabilities echo,sleep
```

The child request is pending by default. The request may ask for observe authority only.

## Approval

On the core:

```bash
quant-m pair requests
quant-m pair approve --request <request-id>
quant-m pair doctor
```

Approval registers a non-authoritative cluster node and records the accepted pairing. It does not grant a role lease or execution authority.

## Heartbeat

After approval, the child can report liveness:

```bash
quant-m cluster heartbeat --node node:tablet-01
quant-m child doctor
```

Heartbeat freshness is operational telemetry only. It does not grant jobs, authority, compute trust, or scheduling priority.

## Observe-Only Lease

After a paired child is approved and visible, the core may grant a temporary observe-only lease:

```bash
quant-m cluster lease grant --node node:tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 30m --authority observe
quant-m cluster lease list
quant-m cluster nodes
quant-m child doctor
```

A lease is not execution authority. It is not scheduling authority. It does not run jobs.

For the heartbeat and lease checkpoint, a leased tablet should still show:

- `execution_enabled: false`
- `approval_enabled: false`
- `canonical_write_enabled: false`
- `jobs_enabled: false`

Revoke a test lease when finished:

```bash
quant-m cluster lease revoke --node node:tablet-01 --reason "tablet testing complete"
quant-m cluster lease check --node node:tablet-01
```

## Echo Roundtrip

After approval, fresh heartbeat, and an active observe-only lease, the core may submit one safe local connectivity job:

```bash
quant-m cluster job submit --node node:tablet-01 --desk research --kind echo --payload '{"text":"tablet online"}'
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

The echo result is evidence only. It must not create proposals, approve work, write canonical state, validate compute backends, call providers, execute shell commands, trade, bet, or change scheduling priority.

## Scalar Compute Evidence

With an approved paired child, fresh heartbeat, active observe-only lease, timing approval, and a role that includes `compute_scalar`, the core may submit bounded scalar evidence jobs:

```bash
quant-m cluster job submit --node node:tablet-01 --desk research --kind compute_freshness_scan --payload '{}' --fixture evidence_freshness --backend scalar
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

```bash
quant-m cluster job submit --node node:tablet-01 --desk crypto --kind compute_peg_deviation --payload '{}' --fixture stablecoin_peg_deviation --backend scalar
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

Scalar compute output is evidence only. It may record freshness counts, peg deviations, numeric confidence, backend metadata, input hash, output hash, and timing decision metadata. It must not create proposals, approve work, write canonical state, trust accelerated backends, call providers, execute shell commands, trade, bet, or change scheduling priority.

## Desk Observation Evidence

After scalar compute is working, the core may request desk-labeled observation evidence. This wraps the scalar output in a `DeskObservationEvidence` envelope while still creating no proposal:

```bash
quant-m cluster job submit --node node:tablet-01 --desk crypto --kind desk_observe_peg_deviation --payload '{"knowledge_pack_id":"stablecoin/peg-monitor"}' --fixture stablecoin_peg_deviation --backend scalar
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

```bash
quant-m cluster job submit --node node:tablet-01 --desk research --kind desk_observe_evidence_freshness --payload '{}' --fixture evidence_freshness --backend scalar
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

Desk observation evidence records desk id, role id, lease id, node id, optional knowledge pack id, evidence kind, compute metadata, input/output hashes, timing metadata, and `proposal_created: false`.

## Playbook-Bound Handoff

The core can bind a versioned playbook hash to a child lease:

```bash
quant-m playbook validate --desk crypto --role stablecoin_peg_watcher
quant-m playbook bundle --desk crypto --role stablecoin_peg_watcher
quant-m cluster lease grant --node node:tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 30m --authority observe --playbook stablecoin_peg_watcher
```

The core can then prepare a provider-neutral model handoff packet and run the local stub:

```bash
quant-m model handoff create --desk crypto --role stablecoin_peg_watcher --playbook stablecoin_peg_watcher --snapshot latest --task detect-contradictions --evidence receipt-id
quant-m model call --provider local-stub --handoff <handoff-id>
quant-m shared-state updates
quant-m shared-state update validate --update <update-id>
quant-m shared-state update accept --update <update-id> --reason "operator reviewed local-stub candidate"
quant-m shared-state facts
quant-m shared-state snapshot create --desk crypto
quant-m shared-state snapshots
```

The local stub creates a pending shared-state update proposal only. Validation can mark the candidate clean, needs-review, or rejected. Acceptance requires an explicit operator reason and writes only typed shared-state facts with no action authority. It does not call providers, create trading/betting proposals, execute, approve, trigger FSM action, or write canonical facts by default.

The following job kinds and actions remain blocked:

- `http_get`
- SIMD or accelerated compute backends
- net-edge or arbitrage workloads
- provider calls
- shell commands
- desk analysis
- proposal approval
- canonical writes

## Troubleshooting

- `core_fingerprint_exists: false`: run `quant-m pair fingerprint` or create an invite on the core.
- `active_invites: 0`: create a fresh invite; invites are short-lived and one-time.
- `pending_requests: 0`: the tablet has not submitted a request or the invite/token was rejected.
- `server_bind_warning` mentions trusted LAN: confirm the network is private before exposing the pairing server.
- `last_heartbeat_status: no heartbeat recorded`: approval succeeded, but the child has not sent a heartbeat yet.
- `last_pairing_status: Pending`: approve or reject the request on the core.
- stale node: send a heartbeat again; stale nodes should not receive future work.
- missing lease: grant an observe-only lease from the core after approval and heartbeat.
- revoked lease: grant a fresh lease from the core only if the tablet is still approved and trusted for observation.
- echo job rejected: confirm the child is approved, recently heartbeated, observe-leased, not stale, and not inside a timing cooldown.
- compute job rejected: confirm the role includes `compute_scalar`, the backend is `scalar`, the fixture is one of the approved evidence fixtures, and timing is not in cooldown.

## Expected Final State

After a successful tablet pairing:

- core has one accepted paired node
- child stores the core URL and core fingerprint
- child pairing status is approved
- heartbeat may show online when recently received
- `execution_enabled` is false
- `approval_enabled` is false
- `canonical_write_enabled` is false
- no role lease is created by pairing
- heartbeat may make the node online
- the core may grant an observe-only lease
- a leased online child may run a local `echo` roundtrip as evidence only
- a leased online child with `compute_scalar` may run scalar freshness and peg-deviation evidence jobs only
- no compute backend becomes trusted from pairing
- no execution leader is selected from pairing
- all non-echo and non-scalar-evidence jobs remain disabled until a future bounded-job checkpoint
