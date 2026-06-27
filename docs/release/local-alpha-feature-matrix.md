# Local Alpha Feature Matrix

Scope: `v0.local-alpha`.

| Feature | Status | Command surface | Authority boundary |
| --- | --- | --- | --- |
| Core CLI | local-alpha ready | `quant-m` / `cargo run --features core-full -- ...` | core owns policy and FSM authority |
| Child binary | local-alpha ready | `quant-m-child` | child-safe commands only |
| Device add wizard | local-alpha ready | `quant-m device add <name>` | wrapper around pairing and observe-only lease flow |
| QR/link pairing | local-alpha ready | `quant-m pair *`, `quant-m child pair *` | enrolls pending/approved child only |
| Pairing server | local-alpha ready on trusted LAN | `quant-m pair serve` or `device add --serve` | no lease or execution is granted by server |
| Manual approval | local-alpha ready | `device add --watch`, `pair approve/reject` | operator decision required |
| Heartbeat | local-alpha ready | `cluster heartbeat`, `quant-m-child heartbeat` | online/stale visibility only |
| Device telemetry | local-alpha ready | `quant-m-child doctor`, `cluster nodes`, `cluster report` | advisory status evidence only |
| Observe-only lease | local-alpha ready | `cluster lease grant/list/check/revoke` | temporary bounded permission only |
| Echo evidence | local-alpha ready | `cluster job submit --kind echo`, `cluster child run` | evidence receipt only |
| Scalar freshness evidence | local-alpha ready | `compute freshness-scan`, cluster compute job | evidence only |
| Scalar peg-deviation evidence | local-alpha ready | `compute peg-deviation`, cluster compute job | no net-edge or arbitrage semantics |
| Desk observation evidence | local-alpha ready | cluster desk observation path | `proposal_created=false` |
| Playbook/model handoff stub | local-alpha ready | model router/playbook tests | local stub only, no provider call by default |
| Shared-state update validation | local-alpha ready | state/model router validation paths | candidate validation only; acceptance stays core/operator gated |
| Real Pi/DietPi + Termux smoke | blocked | runbook artifact | blocked until real devices are reachable |
| Live trading | not shipped | none | disabled |
| Live betting | not shipped | none | disabled |
| Public beta packaging | not shipped | none | requires release engineering and hardware proof |

