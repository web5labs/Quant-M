# PI_TERMUX_LAN_VALIDATION_13

This runbook validates Quant-M local-alpha core/child operation on real devices over a trusted LAN.

This checkpoint adds no runtime authority. It is a hardware reality check for the existing safe path:

```text
pairing server
  -> device add --watch
  -> child pair or QR/manual fallback
  -> explicit approval
  -> heartbeat
  -> optional observe-only lease
  -> echo/scalar evidence
  -> cluster report
  -> zero proposals
  -> zero execution
```

## Status Target

Expected after this run:

- `paired=true`
- `approved=true`
- `online=true` after heartbeat
- `authority=observe`
- `lease=active` only if explicitly granted
- `execution=false`
- `approval=false`
- `canonical_write=false`
- `provider_calls=false`
- `proposal_count=0`
- `trade_count=0`
- `bet_count=0`

## Device Matrix

Minimum hardware:

| Lane | Device | Role |
| --- | --- | --- |
| Core | Raspberry Pi 3 on DietPi | preferred real core |
| Core fallback | laptop | acceptable if Pi is unavailable |
| Child | Android phone or tablet with Termux | edge child |
| Optional child | Raspberry Pi edge worker | second child |

Record the tested matrix:

```text
date:
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
```

## Network Boundary

Use only a trusted private LAN.

Allowed:

- core LAN IP such as `192.168.x.x`, `10.x.x.x`, or `172.16.x.x`
- `0.0.0.0:8787` bind on a trusted LAN for pairing only
- short-lived invite tokens

Not allowed:

- public internet exposure
- port forwarding
- provider calls
- broker or exchange credentials
- live trading or betting
- shell or HTTP job authority

## Core Setup

On the Pi/DietPi core or laptop fallback:

```bash
git clone <repo-url> quantm
cd quantm
git fetch origin
git checkout release/v0-local-alpha
git pull origin release/v0-local-alpha
./quantm onboard
./quantm core pair doctor --bind 0.0.0.0:8787
```

Find the core LAN IP:

```bash
hostname -I
```

Set a shell helper:

```bash
CORE_URL=http://<core-lan-ip>:8787
```

Start the pairing server in a dedicated terminal:

```bash
./quantm core pair serve --bind 0.0.0.0:8787
```

Expected:

- warning says the server is visible on local interfaces
- no execution authority is granted
- no provider call is made

To inspect install/build dependencies and cleanup candidates on the Pi:

```bash
bash scripts/pi_dependency_audit.sh
bash scripts/pi_lean_cleanup.sh --dry-run
```

The cleanup script defaults to dry-run. Use `--apply` only after the needed core or child binary has been built and tested.

## Child Setup

On Termux:

```bash
pkg update
pkg install git rust clang pkg-config openssl
git clone <repo-url> quantm
cd quantm
git fetch origin
git checkout release/v0-local-alpha
git pull origin release/v0-local-alpha
./quantm child-build
wc -c target/release-child/quant-m-child
./quantm child --workspace workspace doctor
```

Expected:

- `authority: none` before pairing
- `execution_enabled: false`
- `approval_enabled: false`
- `canonical_write_enabled: false`
- `model_router_compiled: false`
- `provider_adapters_compiled: false`

Do not run bare `./quantm` on an edge child. Bare `./quantm` on edge devices prints the role guide; core setup uses `./quantm onboard`, and child setup uses `./quantm child ...`.
- `shared_state_accept_compiled: false`
- `pairing_server_compiled: false`

Expected size baseline:

```text
quant-m-child release-child: 653120 bytes on the latest local validation host
```

Real device size may differ by target architecture. Record it rather than treating it as authority.

## Device Add Wizard

On the core, run the interactive wizard:

```bash
./target/debug/quant-m device add tablet-01 \
  --desk crypto \
  --role stablecoin_peg_watcher \
  --core "$CORE_URL" \
  --qr \
  --watch \
  --watch-timeout 120
```

Expected display:

- device name
- desk
- role
- observe authority
- local link
- child command
- QR if the build includes QR support
- execution disabled
- approval disabled
- canonical write disabled

If the server is not running, the wizard should tell you to run:

```bash
./target/debug/quant-m pair serve --bind 0.0.0.0:8787
```

## Child Pairing

Manual child command on Termux:

```bash
./target/release-child/quant-m-child --workspace workspace pair \
  --core "$CORE_URL" \
  --invite <invite-token> \
  --name tablet-01
```

Current child-min note:

- `quant-m-child pair` stores local pairing metadata for the child.
- The full core pairing request path is also available through the core `quant-m child pair` command and pairing server flow.
- If using the browser/server page, submit the request from the invite page.
- If using image QR scanning, build with the explicit child scan feature instead of `child-min`.

QR image flow when scan support is intentionally enabled:

```bash
termux-camera-photo /tmp/quantm_pair.jpg
quant-m-child --workspace workspace pair-scan --image /tmp/quantm_pair.jpg
```

Expected:

- pairing request is pending on the core
- authority requested is observe
- no lease is created
- no execution authority appears

## Interactive Approval

When the core wizard sees the request, it should print:

```text
Pairing request received
Request: <request-id>
Device name: tablet-01
Surface: termux_worker
Claimed capabilities: ...
Compute claims: unvalidated or none
Requested authority: observe
Execution requested: false
Approval requested: false
Canonical write requested: false
Approve observe-only child? [y/N]
```

Type:

```text
y
```

Expected:

- request approved
- node id is created, such as `node:tablet-01`
- paired is true
- approved is true
- online is false until heartbeat
- lease is none
- execution remains disabled
- proposal creation remains disabled

Default safety test:

- rerun on a disposable invite
- press Enter at the approval prompt
- expected result: request rejected, no node created

## Heartbeat

On the core, inspect nodes:

```bash
./target/debug/quant-m cluster nodes
```

On Termux, record a child heartbeat in the child workspace:

```bash
./target/release-child/quant-m-child --workspace workspace heartbeat
```

Current local-file note:

- `quant-m-child heartbeat` writes to the child outbox.
- The core-side `quant-m cluster heartbeat --node node:tablet-01` records heartbeat directly in the core workspace.
- Until remote sync/transport is shipped, use the core command to validate core ledger behavior:

```bash
./target/debug/quant-m cluster heartbeat --node node:tablet-01 --surface termux_worker
./target/debug/quant-m cluster nodes
```

Expected:

- node becomes online when heartbeat is fresh
- heartbeat creates no lease
- heartbeat assigns no role
- heartbeat enables no execution

## Observe-Only Lease

Grant a lease only when explicitly testing leased work:

```bash
./target/debug/quant-m cluster lease grant \
  --node node:tablet-01 \
  --desk crypto \
  --role stablecoin_peg_watcher \
  --ttl 30m \
  --authority observe
```

Inspect:

```bash
./target/debug/quant-m cluster lease list
./target/debug/quant-m cluster lease check --node node:tablet-01
./target/debug/quant-m cluster nodes
```

Expected:

- lease authority is observe
- desk is crypto
- role is stablecoin_peg_watcher
- jobs are still separately gated
- execution remains disabled

## Echo Evidence

Submit a safe echo job:

```bash
./target/debug/quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind echo \
  --payload '{"text":"lan-smoke"}'
```

Run the existing file-backed child worker path:

```bash
./target/debug/quant-m cluster child run --node node:tablet-01
./target/debug/quant-m cluster report
```

Expected:

- one replay-safe echo receipt
- no proposal
- no execution
- no provider call
- no canonical write

Child-min local echo smoke, separate from cluster queues:

```bash
cat > /tmp/quantm-child-echo.json <<'JSON'
{"job_id":"local-echo-1","kind":"echo","payload":"lan-smoke"}
JSON
./target/release-child/quant-m-child --workspace workspace run-once --job /tmp/quantm-child-echo.json
```

Expected:

- local child receipt is written under `workspace/child/outbox/job-receipts.jsonl`
- non-echo jobs are rejected by child-min

## Scalar Freshness Evidence

Submit:

```bash
./target/debug/quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind compute_freshness_scan \
  --payload '{}' \
  --fixture evidence_freshness \
  --backend scalar
```

Run:

```bash
./target/debug/quant-m cluster child run --node node:tablet-01
./target/debug/quant-m cluster report
```

Expected:

- scalar backend only
- freshness evidence receipt recorded
- no proposal
- no execution
- no provider call

## Scalar Peg-Deviation Evidence

Submit:

```bash
./target/debug/quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind compute_peg_deviation \
  --payload '{}' \
  --fixture stablecoin_peg_deviation \
  --backend scalar
```

Run:

```bash
./target/debug/quant-m cluster child run --node node:tablet-01
./target/debug/quant-m cluster report
```

Expected:

- peg-deviation evidence receipt recorded
- no net-edge or arbitrage semantics
- no proposal
- no execution
- no provider call

## Desk Observation Evidence

If the scalar paths pass, test the desk observation envelope:

```bash
./target/debug/quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind desk_observe_peg_deviation \
  --payload '{"knowledge_pack_id":"stablecoin/peg-monitor"}' \
  --fixture stablecoin_peg_deviation \
  --backend scalar
./target/debug/quant-m cluster child run --node node:tablet-01
./target/debug/quant-m cluster report
```

Expected:

- `DeskObservationEvidence` is recorded
- `proposal_created=false`
- no strategy, risk, trade, or bet decision is created

## Negative Checks

These should fail or remain disabled:

```bash
./target/debug/quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind http_get \
  --payload '{"url":"https://example.com"}'
```

```bash
./target/debug/quant-m cluster job submit \
  --node node:tablet-01 \
  --desk crypto \
  --kind compute_peg_deviation \
  --payload '{}' \
  --fixture net_edge_arbitrage \
  --backend scalar
```

```bash
cat > /tmp/quantm-child-reject.json <<'JSON'
{"job_id":"reject-1","kind":"model_handoff","payload":"should fail"}
JSON
./target/release-child/quant-m-child --workspace workspace run-once --job /tmp/quantm-child-reject.json
```

Expected:

- `http_get` rejected by default
- net-edge/arbitrage fixture rejected
- child-min rejects non-echo local job
- no proposals created
- no execution enabled

## Cluster Report Acceptance

Run:

```bash
./target/debug/quant-m cluster report
```

Record:

```text
nodes:
online:
leases:
jobs:
evidence_receipts:
proposal_count:
execution_enabled:
provider_calls:
notes:
```

Accept only if:

- at least one node is paired and approved
- heartbeat is fresh
- optional observe lease is active only if explicitly granted
- echo evidence is recorded
- scalar freshness evidence is recorded when tested
- scalar peg-deviation evidence is recorded when tested
- desk observation evidence is recorded when tested
- proposal count is zero
- execution is false
- provider calls are zero

## Cleanup

Revoke the test lease:

```bash
./target/debug/quant-m cluster lease revoke --node node:tablet-01 --reason "Pi/Termux LAN validation complete"
./target/debug/quant-m cluster lease check --node node:tablet-01
```

Stop the pairing server with Ctrl-C.

Archive validation notes outside any release bundle. Do not ship:

- `target/`
- generated `workspace/state/`
- logs
- copied repos
- secrets
- QR screenshots containing live invite tokens

## Troubleshooting

Pairing server unreachable:

- confirm both devices are on the same LAN
- confirm the core URL uses the LAN IP, not `127.0.0.1`
- confirm firewall allows TCP 8787 on the core
- rerun `quant-m pair doctor --bind 0.0.0.0:8787`

No pending request:

- invite may be expired or used
- child may have used the wrong core URL
- token may have been copied incorrectly
- rerun `device add --watch --watch-timeout 120`

Approval creates no online node:

- online requires heartbeat
- run `cluster heartbeat --node node:tablet-01 --surface termux_worker`

Lease rejected:

- node must be approved
- role must match required capabilities
- `stablecoin_peg_watcher` requires `compute_scalar`
- lease TTL must be valid

Compute job rejected:

- backend must be `scalar`
- fixture must be allowlisted for the job kind
- role must include `compute_scalar`
- timing must allow evaluation

## Public-Beta Blockers After This

This runbook does not make Quant-M public beta ready.

Remaining release work:

1. Release binary packaging
2. Install scripts
3. Autostart/systemd/Termux Boot docs
4. Pairing LAN security review
5. README-only first-use walkthrough
6. Experimental label review
7. Public beta tag
