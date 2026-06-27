# REAL_DEVICE_LAN_SMOKE_14A

Date: 2026-06-27

Status: blocked

Reason: real Raspberry Pi/DietPi core and Termux child devices were not reachable from this Codex workspace during this run. No real LAN pairing, real Termux battery telemetry, or real Pi/DietPi execution was performed.

This record is a validation artifact, not a pass claim.

## Objective

Validate the latest Quant-M core/child local-alpha flow on real devices over a trusted LAN:

```text
pair server
  -> device add --watch
  -> child pair or QR/manual fallback
  -> explicit approval
  -> heartbeat telemetry
  -> optional observe-only lease
  -> echo evidence
  -> scalar freshness evidence
  -> scalar peg-deviation evidence
  -> cluster report
  -> zero proposals
  -> zero execution
```

## Device Matrix

Required:

| Lane | Required Device | Observed |
| --- | --- | --- |
| Core | Raspberry Pi 3 / DietPi | blocked: not reachable |
| Core fallback | laptop core | local workspace only; not a LAN hardware smoke |
| Child | Android phone/tablet with Termux | blocked: not reachable |
| Optional child | Raspberry Pi edge worker | not tested |

Record when hardware is available:

```text
core_device:
core_os:
core_arch:
core_lan_ip:
child_device:
child_os:
child_arch:
child_lan_ip:
quant_m_commit:
quant_m_child_size:
pairing_method:
```

## Commands To Run On Real Devices

Core:

```bash
quant-m pair serve --bind 0.0.0.0:8787
```

Core:

```bash
quant-m device add tablet-01 \
  --desk crypto \
  --role stablecoin_peg_watcher \
  --qr \
  --watch
```

Child:

```bash
quant-m-child doctor
quant-m-child pair --core http://<core-lan-ip>:8787 --invite <token> --name tablet-01
quant-m-child heartbeat
```

Core:

```bash
quant-m cluster nodes
quant-m cluster report
```

Optional observe lease:

```bash
quant-m cluster lease grant \
  --node node:tablet-01 \
  --desk crypto \
  --role stablecoin_peg_watcher \
  --ttl 30m \
  --authority observe
```

Echo evidence:

```bash
quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind echo \
  --payload '{"message":"lan-smoke"}'
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

Scalar freshness evidence:

```bash
quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind compute_freshness_scan \
  --payload '{}' \
  --fixture evidence_freshness \
  --backend scalar
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

Scalar peg-deviation evidence:

```bash
quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind compute_peg_deviation \
  --payload '{}' \
  --fixture stablecoin_peg_deviation \
  --backend scalar
quant-m cluster child run --node node:tablet-01
quant-m cluster report
```

## Telemetry To Capture

From `quant-m-child doctor`:

```text
device name:
hostname:
model:
os:
arch:
storage total:
storage available:
battery percent:
battery charging/status:
battery health:
authority:
execution_enabled:
approval_enabled:
canonical_write_enabled:
```

From `quant-m cluster nodes`:

```text
node:
paired:
approved:
online:
stale:
device:
os:
arch:
battery:
storage_available:
lease:
authority:
execution:
approval:
canonical_write:
jobs_enabled:
```

From `quant-m cluster report`:

```text
online_nodes:
stale_nodes:
active_leases:
recent_evidence:
recent_compute_evidence:
pending_proposals:
device_health:
```

## Required Safe Final State

The real-device run only passes if all of these are true:

- `paired=true`
- `approved=true`
- `online=true`
- `authority=observe`
- `telemetry_present=true` or `unknown_with_reason=true`
- `evidence_receipts_present=true`
- `proposal_count=0`
- `execution=false`
- `provider_calls=false`
- `canonical_write=false`
- `trading=false`
- `betting=false`

## Blockers

- Real Raspberry Pi 3 / DietPi core was not connected to this Codex workspace.
- Real Android Termux child was not connected to this Codex workspace.
- No live LAN pairing server could be verified from a second device.
- No real Termux `termux-battery-status` output was captured.

## Outcome

Result: blocked, not failed.

Architecture remains ready for local hardware validation, but this artifact does not mark `REAL_DEVICE_LAN_SMOKE_14A` as passed.

Next action: run `docs/pi-termux-lan-validation.md` on the actual Pi/DietPi and Termux devices, then replace this blocked record with observed command outputs and final safe-state evidence.
