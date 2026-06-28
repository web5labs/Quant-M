# Quant-M Edge Cluster Local Alpha

Status: `v0.local-alpha` shippable for local lab use.

Release posture: experimental local-lab release for the repo owner or tightly controlled testers.

Not a public beta. Not production. Not an autonomous trading or betting cluster.

## Release Description

Quant-M `v0.local-alpha` is an experimental local-first core/child edge cluster runtime. It supports QR/link pairing, explicit child approval, heartbeat visibility, device telemetry, observe-only leases, timing-gated evidence jobs, scalar compute evidence, desk observation evidence, playbook/model-handoff stubs, and shared-state update validation.

It does not support live trading, betting, provider calls from children, automatic proposal approval, autonomous execution, or production remote orchestration.

## Release Verdict

| Target | Verdict |
| --- | --- |
| `v0.local-alpha` | shippable |
| public beta | not yet |
| production | no |
| autonomous trading | no |
| sports betting bot | no |
| provider cluster | no |

## What This Release Proves

Quant-M can run a local-first core/child edge cluster path where child devices are known, visible, leased, timed, and evidence-only. The core keeps validation, shared state, FSM boundaries, policy, and governance.

Included local-alpha surfaces:

- core CLI
- `quant-m-child` edge binary
- core-side `device add` wizard
- QR/link pairing and manual fallback
- manual operator approval
- heartbeat visibility with paired-node auth-token verification
- device telemetry
- observe-only lease management
- echo evidence
- scalar freshness evidence
- scalar peg-deviation evidence
- desk observation evidence
- playbook/model-handoff local stub
- shared-state update validation

## Explicitly Not Included

- live trading
- live betting
- provider calls from children
- broker, exchange, or sportsbook execution
- automatic proposal approval
- child canonical writes
- production remote orchestration
- public beta support expectations

## Release Label

Use:

```text
Quant-M Edge Cluster Local Alpha
Experimental core/child lab build
No live trading
No live betting
No provider calls enabled
No autonomous proposals
No execution authority
```

Avoid:

- production ready
- public beta
- autonomous trading cluster
- agent mesh
- distributed execution platform
- sports betting bot
- crypto arbitrage bot
- deployment ready

## Binary Size Record

Latest child-min size recorded during checkpoint work:

| binary | profile | features | size |
| --- | --- | --- | --- |
| `quant-m-child` | `release-child` | `--no-default-features --features child-min` | `720,112` bytes |

Telemetry increased the child binary from `653,120` bytes to `669,728` bytes. LAN pairing and heartbeat sync increased the current local-alpha child binary to `720,112` bytes.

## Safety Record

The local-alpha release record preserves these values:

| Field | Value |
| --- | --- |
| `proposal_count` | `0` |
| `execution` | `false` |
| `provider_calls_from_children` | `false` |
| `canonical_child_write` | `false` |
| `trading` | `false` |
| `betting` | `false` |

## Validation Status

Local validation is recorded in [local-alpha-release-candidate-2026-06-27.md](validation-runs/local-alpha-release-candidate-2026-06-27.md).

Real Pi/DietPi plus Termux LAN validation is still blocked until the devices are reachable. The blocked artifact is recorded in [pi-termux-lan-2026-06-27.md](validation-runs/pi-termux-lan-2026-06-27.md) and must not be counted as pass evidence.

Next milestone after tagging: `REAL_DEVICE_LAN_SMOKE_15A`.
