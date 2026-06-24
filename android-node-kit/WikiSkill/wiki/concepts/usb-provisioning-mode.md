---
type: concept
updated: 2026-06-13
source_count: 1
tags: [usb, adb, provisioning, edge-devices]
---

# USB Provisioning Mode

Source: `android-node-kit/WikiSkill/raw/android-usb-provisioning-instruction.md`

USB provisioning mode is the preparation phase before an Android device becomes a Quant-M worker. In this mode, the laptop controls installation and registration over USB/ADB.

## Device State

A prepared device is:

- factory-reset or cleanly repurposed,
- named with a stable Quant-M node name,
- authorized for ADB from the trusted laptop,
- loaded with Termux and Termux:API,
- configured for SSH access,
- loaded from an offline local package bundle,
- ready to receive a prebuilt Quant-M Rust runtime binary.

## Control Plane

USB is the control plane during provisioning. Bluetooth, local Wi-Fi discovery, app auto-optimization, and aggressive battery management should be disabled when they interfere with wired setup.

## Runtime Boundary

Most Android edge devices should run prebuilt Quant-M binaries and should not carry developer toolchains. Cargo/Rust build tooling belongs only on explicit builder-profile devices.

## Links

- [[sources/android-usb-provisioning-instruction]]
- [[concepts/adb-usb-install]]
- [[syntheses/quant-m-edge-bundle-plan]]
