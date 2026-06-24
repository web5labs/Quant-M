---
type: source-summary
updated: 2026-06-12
source_count: 1
tags: [android, termux, quant-m, bundle]
---

# Quant-M Edge Bundle Source

Source: `android-node-kit/WikiSkill/raw/quant-m-edge-bundle-source.md`

## Summary

The workspace already has a small Android node kit with Termux APKs, a Termux bootstrap script, and a five-device inventory. Quant-M has Android deployment notes and a conservative Android node config.

The new bundle manifest lives at `android-node-kit/bundles/quant-m-edge-bundle/manifest.toml`. The bundle quickstart lives at `android-node-kit/bundles/quant-m-edge-bundle/README.md`. Together they define the intended ADB install flow, required Termux packages, expected Quant-M binary locations, verification checks, and current missing artifacts.

## Confirmed Assets

- Termux APKs are present under `android-node-kit/apks/termux/`.
- Termux:API APK is present at `android-node-kit/apks/termux/termux-api.apk`.
- Bootstrap script exists at `android-node-kit/bootstrap/bootstrap-termux.sh`.
- Offline installer exists at `android-node-kit/bundles/quant-m-edge-bundle/offline-install-termux.sh`.
- Offline package mirrors exist for `termux-main` and `termux-main-21`, both with `aarch64` and `arm`.
- Quant-M source exists at `quant-m/Quant-M`.
- Android default config exists at `quant-m/Quant-M/configs/nodes/android-default.toml`.

## Gaps

- Prebuilt Android Quant-M binaries are not staged under the bundle.
- Android NDK/linker build configuration is not documented in the bundle yet.

## Links

- [[entities/device-inventory]]
- [[concepts/adb-usb-install]]
- [[syntheses/quant-m-edge-bundle-plan]]
