# Core Child QR Pairing

`CORE_CHILD_QR_PAIRING_01` adds local QR/link pairing for Quant-M edge devices.

Pairing is discovery plus enrollment. It is not authority.

## Threat Model

The QR/link may contain a local core URL and a short-lived invite token. It must not contain provider credentials, broker credentials, exchange keys, sportsbook credentials, private keys, long-lived node secrets, trading authority, betting authority, proposal approval, or canonical write authority.

Pairing creates a pending child request by default. Approval creates an observe-only paired node record and a cluster node. Role leases, timing policy, compute validation, job policy, and replay still apply separately.

## Operator Flow

Create a link invite:

```bash
quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --link
```

Create a terminal QR invite:

```bash
quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --qr
```

Save a PNG QR when built with `pairing-qr`:

```bash
quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 10m --png workspace/state/pairing/tablet-01.png
```

Inspect requests and approve manually:

```bash
quant-m pair requests
quant-m pair approve --request <request-id>
```

## Tablet Flow

Normal camera:

1. Scan the QR from the core terminal.
2. Open the local link.
3. Copy the `quant-m child pair ...` command.
4. Paste it into Termux.

Manual fallback:

```bash
quant-m child pair --core http://127.0.0.1:8787 --invite <invite-token>
```

Termux image capture path is available behind the `pairing-scan-image` feature:

```bash
termux-camera-photo /tmp/quantm_pair.jpg
quant-m child pair-scan --image /tmp/quantm_pair.jpg
```

## Dev Auto-Accept

```bash
quant-m pair invite --name tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 5m --dev-auto-accept
```

Dev auto-accept is capped at observe authority. Execution, approval, and canonical writes remain disabled.

## Storage

Core:

- `workspace/state/pairing/invites.jsonl`
- `workspace/state/pairing/requests.jsonl`
- `workspace/state/pairing/accepted-nodes.jsonl`
- `workspace/state/pairing/revoked-invites.jsonl`
- `workspace/state/pairing/events.jsonl`
- `workspace/state/pairing/core-fingerprint.json`

Child:

- `workspace/child/identity.toml`
- `workspace/child/pairing.toml`
- `workspace/child/core.toml`

## Current Limits

`pair serve` runs a minimal local HTTP server for LAN onboarding. It serves only pairing invite pages, pending request intake, and request status checks.

Supported routes:

- `GET /pair/i/<invite-token>`
- `POST /pair/request`
- `GET /pair/status/<request-id>`

The server does not approve requests, assign leases, validate compute, schedule work, execute commands, or mutate canonical shared state.

QR terminal rendering and PNG output require `--features pairing-qr`.

QR image decode requires `--features pairing-scan-image`. It decodes image files only; Quant-M still does not control live camera access.

## Paired Child Heartbeat And Lease

Pairing approval makes a child known, not leased.

After approval:

```bash
quant-m cluster nodes
quant-m cluster heartbeat --node node:tablet-01
quant-m cluster lease grant --node node:tablet-01 --desk crypto --role stablecoin_peg_watcher --ttl 30m --authority observe
quant-m cluster lease check --node node:tablet-01
```

Heartbeat makes the child visible as online or stale. The core may then grant an observe-only lease. The lease is temporary permission metadata for a future bounded-job checkpoint; it does not execute jobs, validate compute, approve proposals, mutate canonical state, call providers, trade, or bet.

## Hard Rule

A paired child is known to the core, not trusted by the core.

Only a leased, timed, policy-valid, role-bound child can run bounded evidence jobs. Only the core can validate, score, approve, and govern.
