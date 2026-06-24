---
type: source-summary
updated: 2026-06-13
source_count: 1
tags: [android, usb, runtime-node, quant-m, acceptance]
---

# Android USB Runtime Node 01 Proposal

Source: `android-node-kit/WikiSkill/raw/android-usb-runtime-node-01-proposal.md`

## Summary

The proposal names the next shippable slice as **ANDROID_USB_RUNTIME_NODE_01**. It formalizes the architecture for Android edge devices as USB-provisioned, offline-capable, Rust-runtime-first, no npm/Node.js, no Cargo by default, Termux-based, and worker-node oriented.

The default profile is a slim runtime profile for old devices. Builder tools remain isolated in a separate builder profile for stronger devices only.

## Acceptance Criteria

- Device detected over USB.
- ADB authorized.
- Termux installed.
- Termux:API installed.
- Runtime files pushed.
- Quant-M binary executable.
- Worker starts.
- Health check returns OK.
- No npm, Node.js, Git, Cargo, or internet required on default device.

## Links

- [[syntheses/android-usb-runtime-node-01]]
- [[syntheses/quant-m-edge-bundle-plan]]
- [[concepts/usb-provisioning-mode]]
