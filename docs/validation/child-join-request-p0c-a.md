# CHILD_JOIN_REQUEST_P0C_A Validation

Verdict target: `child_join_request_p0c_a_validated`

This slice proves the child can consume a core join URL, create a local identity, and submit a pending observe-only pair request. It does not implement heartbeat or real-device LAN proof.

## Core Setup

Start from a core workspace and create a pairing invite:

```bash
quant-m pair cockpit
quant-m device add --qr
```

Copy the printed URL:

```text
http://<core-lan-ip>:8787/join/<invite_id>
```

If serving over LAN, keep the pairing server on trusted Wi-Fi only:

```bash
quant-m pair serve --bind 0.0.0.0:8787
```

## Child Join

On the child device or child workspace:

```bash
quant-m child identity
quant-m child join --url http://<core-lan-ip>:8787/join/<invite_id>
```

Camera scanning is not required. Paste the URL manually when QR scanning is unavailable.

Expected child output includes:

- child identity ID
- child fingerprint
- request ID
- `status: pending`
- core approval command
- observe-only safety summary

## Core Approval Later

On the core:

```bash
quant-m child list
quant-m child approve <request_id>
```

P0C-A stops at pending request submission. Manual approval still happens on the core and is not automatic.

## Safety Checklist

- Child stores no provider keys.
- Child requests observe-only authority.
- Child does not request provider-call authority.
- Child does not request execution authority.
- Child does not request approval authority.
- Child does not request canonical shared-state write authority.
- Child does not request broker, exchange, or sportsbook authority.
- Child cannot auto-approve itself.

## Known Later Milestones

- P0C-B: child heartbeat and revoke health visibility.
- P0C-C: real Android/Termux LAN proof.
- Child binary bootstrap remains the Git-free packaging path for old devices.
