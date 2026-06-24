---
type: source-summary
updated: 2026-06-13
source_count: 1
tags: [android, adb, usb, provisioning, quant-m]
---

# Android USB Provisioning Instruction

Source: `android-node-kit/WikiSkill/raw/android-usb-provisioning-instruction.md`

## Summary

The instruction defines Quant-M Android device preparation as a two-phase process: USB provisioning mode first, then Quant-M worker mode after the device is prepared.

The device should be factory-reset, minimally configured, named with a stable Quant-M node identity, USB-authorized via ADB, loaded with Termux and Termux:API offline, and registered as a wired worker before joining the cluster.

## Key Requirements

- USB debugging is the primary control plane.
- Termux plus Termux:API are required for terminal-driven Android peripheral access.
- Install order is Termux APK, Termux:API APK, optional helper APKs, local package archive/bootstrap, then Quant-M worker binary or repo bundle.
- Android edge devices should act as wired workers, not the primary coordinator.
- Laptop/coordinator owns the build pipeline, registry, shared-state coordination, logs, and audit ledger.
- USB debugging should stay enabled only for trusted lab devices or be disabled before a device leaves controlled use.

## Maintained Outcome

The repo should maintain a dedicated provisioning doc at `android-node-kit/docs/edge/ANDROID_USB_PROVISIONING.md`.

## Links

- [[concepts/adb-usb-install]]
- [[concepts/usb-provisioning-mode]]
- [[syntheses/quant-m-edge-bundle-plan]]
