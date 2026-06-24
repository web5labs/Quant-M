---
type: index
updated: 2026-06-13
tags: [android, adb, termux, quant-m, edge-devices]
---

# Android Node Kit Wiki

This wiki tracks the USB/ADB install bundle for Quant-M edge devices.

## Start Here

- [[syntheses/quant-m-edge-bundle-plan]]
- [[syntheses/android-usb-runtime-node-01]]
- [[sources/quant-m-edge-bundle-source]]
- [[sources/android-usb-provisioning-instruction]]
- [[sources/android-usb-runtime-node-01-proposal]]
- [[entities/device-inventory]]
- [[concepts/adb-usb-install]]
- [[concepts/usb-provisioning-mode]]

## Current State

- Public inventory template: `android-node-kit/inventory/nodes.example.csv`.
- Local private inventory: `android-node-kit/inventory/nodes.csv` (ignored by git).
- Optional dev-builder profile notes: `android-node-kit/bundles/profiles/dev-builder`.
- Termux APKs and offline mirrors are local prepared-checkout artifacts and are intentionally not committed.
- Default slim runtime profile: `android-node-kit/bundles/profiles/base-runtime`.
- USB provisioning doc: `android-node-kit/docs/edge/ANDROID_USB_PROVISIONING.md`.
- Runtime milestone doc: `android-node-kit/docs/edge/ANDROID_USB_RUNTIME_NODE_01.md`.
- Quant-M Android binaries: not staged yet.
