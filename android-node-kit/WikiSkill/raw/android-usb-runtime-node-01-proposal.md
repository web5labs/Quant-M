# Android USB Runtime Node 01 Proposal

Date ingested: 2026-06-13

Source: user-provided proposed update.

## Architectural Stance

Quant-M Android edge devices are:

- **USB-provisioned**: prepared through ADB and wired laptop control.
- **Offline-capable**: dependencies can be transferred locally without requiring the device to go online.
- **Rust-runtime-first**: the default device receives a prebuilt Quant-M Rust binary.
- **No npm / Node.js on edge**: frontend and Node tooling stay off the phone/tablet.
- **No Cargo by default**: Rust/Cargo only belongs on explicit builder-profile devices, not old phones.
- **Termux-based runtime shell**: Termux, Termux:API, OpenSSH, curl, and termux-tools are the default lightweight base.
- **Worker-node role**: the phone/tablet acts as a Quant-M worker, not the main coordinator.

This fits Quant-M's broader principle: keep deterministic runtime in Rust, use shared state and FSM boundaries, and avoid making edge devices fragile full development environments.

## Profile Distinction

### Default Old-Device Profile

Use this for old phones/tablets:

- Termux
- Termux:API
- OpenSSH
- curl
- termux-tools
- prebuilt Quant-M binary
- config files
- worker startup script
- health-check script
- logs/session folder

### Builder-Profile Device

Use this only for stronger devices:

- Rust/Cargo
- clang/LLVM
- Git
- local build/test ability
- possibly repo sync

## Next Shippable Checkpoint

Plug in a factory-reset Android device, authorize it over USB, push the offline bundle, install Termux and Termux:API, push the prebuilt Quant-M binary, start the worker, and have the laptop receive a clean health report from the device.

Future validation command set:

```text
deploy/android/adb-provision.sh
deploy/android/push-runtime.sh
deploy/android/start-worker.sh
deploy/android/health-check.sh
```

Acceptance test:

```text
Device detected over USB.
ADB authorized.
Termux installed.
Termux:API installed.
Runtime files pushed.
Quant-M binary executable.
Worker starts.
Health check returns OK.
No npm, Node.js, Git, Cargo, or internet required on default device.
```

Slice name:

**ANDROID_USB_RUNTIME_NODE_01**

Definition of shippable:

**A clean Android phone or tablet can be factory-reset, prepared through USB debugging, provisioned offline from the laptop, and run as a Quant-M Rust worker node without needing internet access, npm, Node.js, Git, or on-device compilation.**
