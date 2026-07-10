# Agent Cluster Guide

Agent Cluster runs Quant-M roles across the same trusted local network, including Wi-Fi. Roles are based on device capability, not hardware category: a weak tablet can be an observe-only child, while a capable phone, tablet, Raspberry Pi, mini PC, laptop, desktop, or server can be a core.

## Good Uses For Spare Devices

- Old Android phone or tablet as a Termux child node.
- Factory-reset tablet as an observe-only watcher.
- Capable Android device as an edge proof-of-concept core.
- Raspberry Pi as a low-power core or child.
- Spare laptop or mini PC as a stronger core.

Carrier service and Ethernet are optional. Stable local Wi-Fi is enough.

## Core Requirements

- Ability to run the `quant-m` core binary.
- Writable local workspace and state storage.
- Enough disk, RAM, and power stability to remain online.
- Stable Wi-Fi or LAN and permission to bind local ports.
- Shell access.

## Safety Rules

- Factory reset repurposed devices when practical.
- Keep the cluster on trusted local Wi-Fi.
- Do not expose pairing ports to the public internet.
- Keep API keys, broker credentials, and private tokens off children.
- Keep children observe-only with no execution, approval, provider-call, or canonical-write authority.
- Do not use outdated devices for live financial execution.

## Pair A Child

On the core, inspect the selected local address and open the cockpit:

```bash
quant-m pair doctor
quant-m pair cockpit
quant-m device add --qr
```

`pair doctor` reports real interface names, local IPv4 candidates, ignored addresses, port availability, firewall guidance, and a child-side `curl` test. The server may bind `0.0.0.0`, but advertised URLs use a child-reachable private IPv4 address.

When automatic selection is wrong, choose a private address or an exact interface reported by the doctor:

```bash
quant-m pair doctor --host 192.168.1.42
quant-m pair cockpit --interface en0
quant-m device add --qr --host 192.168.1.42
```

Quant-M rejects public hostnames and mismatched explicit bind/host combinations.

On the child:

```bash
quant-m child identity
quant-m child join --url http://<core-local-ip>:8787/join/<invite_id>
```

Camera scanning is optional. Paste the printed URL when a scanner is unavailable.

On the core, review and decide manually:

```bash
quant-m child list
quant-m child approve <request_id>
quant-m child deny <request_id>
quant-m child revoke <node_id>
quant-m pair status --json
```

An approved child remains observe-only.

## Heartbeats

An approved child can report visibility:

```bash
quant-m child heartbeat --core http://<core-local-ip>:8787 --once
quant-m child list --json
```

A heartbeat does not grant execution, approval, provider-call, canonical-write, broker, exchange, or sportsbook authority. Revoked children are not counted as healthy.

## Old Android And Termux

Minimal old-device prerequisites:

```bash
pkg update
pkg install curl openssh termux-api
```

Termux:API is optional unless Android telemetry is needed. Install Termux and its add-ons from the same source when possible.

Git, Rust, Cargo, and cloning GitHub directly are development fallbacks on old children. Freshly reset Termux environments can have stale mirrors or mismatched TLS libraries before Quant-M runs. The preferred product direction is a core-hosted prebuilt child binary.

## Core-Hosted Child Bootstrap

Serve prepared child bundles from the core:

```bash
quant-m bootstrap serve \
  --bind 0.0.0.0:8788 \
  --bundle-dir ./release-bundles \
  --core-url http://<core-local-ip>:8787
```

The endpoint lists only bundles whose metadata, file size, and SHA-256 match. A child-side flow looks like:

```bash
curl -fL -o quant-m-child http://<core-local-ip>:8788/download/quant-m-child
printf '%s  %s\n' '<sha256>' quant-m-child | sha256sum -c -
chmod +x quant-m-child
./quant-m-child pair --core http://<core-local-ip>:8787 --name android-tablet-01
```

Bootstrap never auto-approves the child.

## Pack Sync Scaffold

Serve prepared, approved archives from the core:

```bash
quant-m pack serve --bind 0.0.0.0:8789 --pack-dir ./release-packs
```

The registry hides revoked packs, rejects checksum or size mismatches, filters by role, blocks path traversal and unlisted downloads, and refuses packs that request script execution. Child installation and full LAN job exchange remain later milestones.

## Validation Methods

Real-device proof may use manual Termux commands, SSH, ADB, a QR/bootstrap URL, a copied binary, or direct browser/Termux download. ADB is a useful provisioning and debugging path, not a requirement. The actual proof is a separate device completing the flow over the same trusted local network.

## Current Milestones

| Milestone | State |
| --- | --- |
| Role-first onboarding | Locally validated |
| Pairing cockpit and manual lifecycle | Locally validated |
| Child join request | Locally validated |
| Heartbeat, stale state, and revoke blocking | Locally validated |
| Wi-Fi-first advertised URLs | Locally validated |
| Core-hosted child bootstrap | Scaffolded |
| Child pack sync | Scaffolded |
| Real-device pack activation and evidence | Pending |

## More Android Material

- [Android deployment guide](../deploy/android/README.md)
- [Android node bundle](../android-node-kit/bundles/quant-m-edge-bundle/README.md)
- [Base runtime profile](../android-node-kit/bundles/profiles/base-runtime/README.md)
