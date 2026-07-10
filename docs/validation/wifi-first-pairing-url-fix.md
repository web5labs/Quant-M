# WIFI_FIRST_PAIRING_URL_FIX Validation

Verdict target: `wifi_first_pairing_url_validated`

This patch makes Agent Cluster pairing Wi-Fi-first and device-agnostic. LAN means the same trusted local network, including Wi-Fi. Ethernet is optional.

## Commands

Diagnose the selected child-reachable URL:

```bash
quant-m pair doctor
```

Open the cockpit:

```bash
quant-m pair cockpit
```

Create a nearby-device invite:

```bash
quant-m device add --qr
```

If automatic address selection chooses the wrong interface, override the advertised child URL:

```bash
quant-m pair doctor --host 192.168.1.42
quant-m pair cockpit --host 192.168.1.42
quant-m device add --qr --host 192.168.1.42
```

## Expected Behavior

- A private Wi-Fi/LAN IPv4 address is preferred for child QR/join URLs.
- `0.0.0.0` may be used for binding but is never used as the advertised child URL.
- `127.0.0.1` is local-only and is not used for phone/tablet QR URLs when a private address exists.
- Docker, VM, bridge, and tunnel-style interfaces are not preferred over Wi-Fi.
- `--interface` uses exact system interface names reported by `pair doctor`.
- Manual `--host` values must be private IPv4 addresses and must be compatible with an explicit bind.
- Join metadata preserves the advertised host for child pair-request callbacks.
- If no reachable Wi-Fi/LAN address is detected, output explains manual `--host <your-wifi-ip>` recovery.
- User-facing language says same trusted local network, Wi-Fi supported, Ethernet optional.

## Child Test

On the child device, while connected to the same Wi-Fi/local network:

```bash
curl -fsS http://<core-wifi-or-lan-ip>:8787/
```

Then join:

```bash
quant-m child join --url http://<core-wifi-or-lan-ip>:8787/join/<invite_id>
```

## Troubleshooting

If the phone/tablet cannot open the URL, check:

- both devices are on the same Wi-Fi/local network
- VPN is off
- firewall allows local incoming connections
- guest Wi-Fi client isolation is disabled
- the selected interface is not Docker, VM, bridge, or tunnel-only
- the advertised URL does not use `0.0.0.0`, `127.0.0.1`, or `localhost`

## Safety Boundary

This patch does not add provider calls, model routing, broker/exchange/sportsbook execution, child pack sync, approval authority, execution authority, or canonical child writes.
