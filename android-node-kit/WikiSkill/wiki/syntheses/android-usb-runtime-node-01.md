---
type: synthesis
updated: 2026-06-13
source_count: 1
tags: [android, quant-m, milestone, acceptance, runtime-node]
---

# Android USB Runtime Node 01

Source: `android-node-kit/WikiSkill/raw/android-usb-runtime-node-01-proposal.md`

## Definition

**ANDROID_USB_RUNTIME_NODE_01** is shippable when a clean Android phone or tablet can be factory-reset, prepared through USB debugging, provisioned offline from the laptop, and run as a Quant-M Rust worker node without needing internet access, npm, Node.js, Git, or on-device compilation.

## Architecture

The default Android edge node is:

- USB-provisioned.
- Offline-capable.
- Rust-runtime-first.
- No npm or Node.js.
- No Cargo by default.
- Termux-based.
- A worker node, not the coordinator.

## Default Runtime Payload

- Termux.
- Termux:API.
- OpenSSH.
- curl.
- termux-tools.
- prebuilt Quant-M binary.
- config files.
- worker startup script.
- health-check script.
- logs/session folder.

## Builder Profile Payload

Only stronger builder devices should receive:

- Rust/Cargo.
- clang/LLVM.
- Git.
- local build/test ability.
- optional repo sync.

## Host Commands

The host deploy lane now exists:

```text
deploy/android/adb-provision.sh
deploy/android/push-runtime.sh
deploy/android/start-worker.sh
deploy/android/health-check.sh
```

## Acceptance Test

- Device detected over USB.
- ADB authorized.
- Termux installed.
- Termux:API installed.
- Runtime files pushed.
- Quant-M binary executable.
- Worker starts.
- Health check returns OK.
- No npm, Node.js, Git, Cargo, or internet required on default device.

## Current Gaps

- Prebuilt Quant-M Android binaries are not staged yet.
- On-device worker command arguments may need adjustment once the final Quant-M CLI surface is locked.

## Links

- [[sources/android-usb-runtime-node-01-proposal]]
- [[syntheses/quant-m-edge-bundle-plan]]
- [[concepts/usb-provisioning-mode]]
