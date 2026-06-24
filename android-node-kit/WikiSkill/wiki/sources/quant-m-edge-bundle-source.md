---
type: source-summary
updated: 2026-06-12
source_count: 1
tags: [android, termux, quant-m, bundle]
---

# Quant-M Edge Bundle Source

Source: `android-node-kit/WikiSkill/raw/quant-m-edge-bundle-source.md`

## Summary

The public repo has a small Android node kit with manifest files, install scripts, a Termux bootstrap script, and an example inventory template. Private device inventory, Termux APKs, and offline package mirrors live only in a prepared local checkout.

The default runtime profile lives at `android-node-kit/bundles/profiles/base-runtime`. The optional heavy dev-builder notes live under `android-node-kit/bundles/profiles/dev-builder`. Together they define the intended ADB install flow, required Termux packages, expected Quant-M binary locations, verification checks, and current missing artifacts.

## Confirmed Assets

- Termux APKs are required local artifacts under ignored `android-node-kit/apks/termux/`.
- Termux:API APK is required locally at ignored `android-node-kit/apks/termux/termux-api.apk`.
- Bootstrap script exists at `android-node-kit/bootstrap/bootstrap-termux.sh`.
- Offline installer exists at `android-node-kit/bundles/quant-m-edge-bundle/offline-install-termux.sh`.
- Offline package mirrors exist for `termux-main` and `termux-main-21`, both with `aarch64` and `arm`.
- Quant-M source exists at the repository root.
- Android default config exists at `configs/nodes/android-default.toml`.

## Gaps

- Prebuilt Android Quant-M binaries are not committed and must be supplied locally.
- Android NDK/linker build configuration is not documented in the bundle yet.

## Links

- [[entities/device-inventory]]
- [[concepts/adb-usb-install]]
- [[syntheses/quant-m-edge-bundle-plan]]
