# REAL_DEVICE_LAN_SMOKE_15A

Date: 2026-06-30

Status: passed with current local-alpha transport limits

Verdict: `real_lan_validated`

This record validates the current Quant-M local-alpha core/child LAN path on separate real devices. It does not add runtime authority and does not claim public beta, production readiness, broker integration, shadow trading, child pack sync, or remote execution APIs.

## Commit

```text
branch: release/v0-local-alpha
commit: cfe9d3d
working_tree_after_test: clean
```

## Device Matrix

| Lane | Device | OS / Runtime | LAN |
| --- | --- | --- | --- |
| Core | laptop fallback | macOS / Quant-M core | `10.0.0.69:8787` |
| Child | Android `9032Z` / `Apollo_8_4G_TMO` | Termux / `armeabi-v7a` | `10.0.0.10` |

Child binary:

```text
target: Android Termux arm / armeabi-v7a
quant-m-child size: 524272 bytes
build source: clean archive of cfe9d3d
```

## Commands Run

Core readiness:

```bash
./target/debug/quant-m pair doctor --bind 0.0.0.0:8787
./target/debug/quant-m cluster report
```

Core pairing server:

```bash
./target/debug/quant-m pair serve --bind 0.0.0.0:8787
```

Invite:

```bash
./target/debug/quant-m pair invite \
  --name android-9032z \
  --desk crypto \
  --role stablecoin_peg_watcher \
  --ttl 10m \
  --core http://10.0.0.69:8787 \
  --json
```

Child build and doctor, run inside Termux from a clean source archive:

```bash
cargo build --bin quant-m-child --profile release-child --no-default-features --features child-min
wc -c target/release-child/quant-m-child
target/release-child/quant-m-child --workspace workspace doctor
```

Child pair:

```bash
target/release-child/quant-m-child --workspace workspace pair \
  --core http://10.0.0.69:8787 \
  --invite <invite-token> \
  --name android-9032z
```

Core approval:

```bash
./target/debug/quant-m pair requests --json
./target/debug/quant-m pair approve --request pair_req_36edf9fe0c5aa56e --accepted-by operator --json
```

Child heartbeat:

```bash
target/release-child/quant-m-child --workspace workspace heartbeat
```

Observe-only lease:

```bash
./target/debug/quant-m cluster lease grant \
  --node node:android-9032z \
  --desk crypto \
  --role stablecoin_peg_watcher \
  --ttl 30m \
  --authority observe \
  --json
```

Echo evidence:

```bash
./target/debug/quant-m cluster job submit \
  --node node:android-9032z \
  --desk crypto \
  --kind echo \
  --payload '{"message":"lan-smoke"}' \
  --json

./target/debug/quant-m cluster child run --node node:android-9032z --json
```

Scalar freshness evidence:

```bash
./target/debug/quant-m cluster job submit \
  --node node:android-9032z \
  --desk crypto \
  --kind compute_freshness_scan \
  --payload '{}' \
  --fixture evidence_freshness \
  --backend scalar \
  --json

./target/debug/quant-m cluster child run --node node:android-9032z --json
```

Stale/reconnect:

```bash
adb shell svc wifi disable
sleep 125
./target/debug/quant-m cluster nodes
adb shell svc wifi enable
target/release-child/quant-m-child --workspace workspace heartbeat
./target/debug/quant-m cluster nodes
```

Revoke:

```bash
./target/debug/quant-m cluster lease revoke \
  --node node:android-9032z \
  --reason "REAL_DEVICE_LAN_SMOKE_15A complete" \
  --json

./target/debug/quant-m cluster job submit \
  --node node:android-9032z \
  --desk crypto \
  --kind echo \
  --payload '{"message":"post-revoke-should-block"}' \
  --json
```

Expected post-revoke result:

```text
Error: no active cluster lease for node 'node:android-9032z'
```

## Observed Results

Pairing request:

```text
request_id: pair_req_36edf9fe0c5aa56e
source_addr: 10.0.0.10
requested_authority: observe
status: pending
```

Approval:

```text
node_id: node:android-9032z
authority_level: observe
execution_enabled: false
approval_enabled: false
canonical_write_enabled: false
```

Heartbeat:

```text
core heartbeat synced
paired: true
approved: true
execution: disabled
approval: disabled
canonical_write: disabled
```

Core node state after heartbeat:

```text
node:android-9032z paired=true approved=true online=true stale=false
authority=observe execution=false approval=false canonical_write=false jobs_enabled=false
```

Evidence:

```text
echo receipt: result_status=ok replay_safe=true promoted_to_proposal=false
scalar receipt: result_status=ok replay_safe=true promoted_to_proposal=false
scalar output: non_authoritative=true proposal_created=false validation_outcome=accepted_scalar_only
pending_proposals: 0
```

Stale/reconnect:

```text
after Wi-Fi disabled and >120s: online=false stale=true
after Wi-Fi enabled and fresh heartbeat: online=true stale=false
```

Revoke:

```text
lease state: revoked
post-revoke job submit: blocked, no active cluster lease
```

## Safety Boundary

Confirmed:

- `execution=false`
- `approval=false`
- `canonical_write=false`
- `jobs_enabled=false` in node status
- child model router not compiled
- child provider adapters not compiled
- child shared-state accept path not compiled
- child pairing server not compiled
- evidence receipts are non-authoritative
- `pending_proposals=0`
- no broker, exchange, trading, betting, provider, shell-job, or canonical-write authority was granted

## Limitations

The LAN pairing and heartbeat path are real child-over-LAN evidence.

Echo and scalar evidence used the current local-alpha `cluster child run` manual-sync/core-mediated path. This is the documented local-alpha limitation; it is not a pass claim for a future LAN job-fetch, child-submit, SSH installer, child pack sync, or remote execution API.

`cluster nodes` correctly reflected the revoked lease as `lease=none` after revoke. `cluster report` still counted `active_leases: 1` immediately after revoke even though `cluster lease list` showed the lease as `revoked`; this should be treated as a report-counting bug before stronger release claims.

## Cleanup

- Pairing server stopped.
- Host `target/` and temporary smoke scripts/tarball removed.
- Android temporary scripts/tarball and on-device Cargo `target/` removed.
- Termux build dependencies remained installed as device setup.
- Ignored local state remains ignored: `quant-m.local.toml`, `workspace/`.

## Final State

```text
REAL_DEVICE_LAN_SMOKE_15A: passed for local-alpha LAN pairing, heartbeat, observe-only lease, non-authoritative evidence, stale/reconnect, and revoke gate.
Public beta: still blocked.
Production/trading/shadow trading: still blocked.
Next implementation milestone: child bootstrap / transport hardening, not Forex or broker APIs.
```
