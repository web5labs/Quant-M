# CHILD_HEARTBEAT_REVOKE_P0C_B Validation

Verdict target: `child_heartbeat_revoke_p0c_b_validated`

This slice proves an approved child can report heartbeat visibility, the core can classify health, and revoke prevents a child from being treated as healthy. It does not claim real Android/Termux LAN proof.

## Simulated Local Flow

Create a core invite:

```bash
quant-m device add --qr
```

Join from the child workspace:

```bash
quant-m child join --url http://<core-lan-ip>:8787/join/<invite_id>
```

Approve on the core:

```bash
quant-m child list
quant-m child approve <request_id>
```

Send one heartbeat from the child:

```bash
quant-m child heartbeat --core http://<core-lan-ip>:8787 --once
```

Inspect health on the core:

```bash
quant-m child list --json
quant-m pair status --json
```

Expected result:

- approved child heartbeat is accepted
- `child list` includes the last heartbeat timestamp
- `pair status` includes healthy/stale/pending/unknown/denied counts
- heartbeat authority remains observe-only

## Revoke Check

On the core:

```bash
quant-m child revoke <node_id>
```

After revoke, a heartbeat from that node is recorded as revoked/unhealthy and is not counted as healthy.

## Safety Checklist

- Heartbeat is visibility only.
- Heartbeat does not grant provider-call authority.
- Heartbeat does not grant execution authority.
- Heartbeat does not grant approval authority.
- Heartbeat does not grant canonical shared-state write authority.
- Heartbeat does not grant broker, exchange, or sportsbook execution.
- Revoked children are not healthy or active.
- Child heartbeat payloads contain no provider keys or private tokens.

## Known Later Milestones

- P0C-C: real Android/Termux LAN proof with physical device.
- Camera QR scanning remains unsupported in this runtime.
- Separate `quant-m-child` binary path remains pending.
- Child pack sync remains a later milestone.
