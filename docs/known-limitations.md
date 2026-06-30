# Known Limitations

These limits are part of the `v0.local-alpha` truth surface.

## Release Posture

Quant-M Edge Cluster Local Alpha is for local lab use only. It is not a public beta, production deployment, autonomous trading system, betting bot, hosted agent network, or remote orchestration product.

## Hardware Proof

Real-device LAN validation passed for the current local-alpha path with a laptop fallback core and Android Termux child. The validation covers pairing, approval, heartbeat, observe-only lease, non-authoritative evidence, stale/reconnect, and revoke gating.

Fresh-device alpha still needs packaging/autostart polish and broader repeated-device proof before public beta or production claims.

## Cluster Transport

Cluster jobs are local-file/manual-sync oriented today. Pairing can use a local HTTP server, but the worker runtime is not a production remote transport layer.

## Authority

Child devices cannot approve proposals, write canonical shared state, call providers, execute trades, place bets, validate compute backends, assign leases, extend leases, or grant themselves authority.

Pairing means known device. Heartbeat means visible device. Lease means temporary bounded permission. None of those means execution authority.

## Device Telemetry

Telemetry is best-effort status evidence. Battery and storage may be unknown on some platforms. Low battery and low storage warnings are advisory only and do not change trust, job priority, lease authority, evidence weight, or execution permissions.

## Release Engineering

The local alpha still needs fresh-machine packaging proof before any wider release. Install scripts, autostart docs, systemd/Termux Boot docs, signed release binaries, and public support instructions are not yet complete.

## Security Review

The current security posture is conservative, but it is not a completed production security review. Keep pairing servers on trusted LANs only, do not expose pairing endpoints to the public internet, and do not bundle generated state, secrets, live QR tokens, or child identities into release artifacts.
