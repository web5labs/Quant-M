# ANDROID_USB_RUNTIME_NODE_01

## Definition Of Shippable

A clean Android phone or tablet can be factory-reset, prepared through USB debugging, provisioned offline from the laptop, and run as a Quant-M Rust worker node without needing internet access, npm, Node.js, Git, or on-device compilation.

## Architectural Stance

Quant-M Android edge devices are:

- USB-provisioned through ADB and wired laptop control.
- Offline-capable through locally transferred APKs and Termux package mirrors.
- Rust-runtime-first with a prebuilt Quant-M binary per architecture.
- Free of npm and Node.js on the edge device.
- Free of Cargo by default.
- Termux-based for shell, Android API bridge, SSH, and lightweight tools.
- Worker nodes, not primary coordinators.

## Default Old-Device Profile

Use `base-runtime` for old phones/tablets.

Payload:

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

Must not require:

- npm.
- Node.js.
- Git.
- Cargo.
- internet access.
- on-device compilation.

## Builder Profile

Use `dev-builder` only for stronger devices that need local build/test ability.

Builder-only payload:

- Rust/Cargo.
- clang/LLVM.
- Git.
- local build/test tools.
- optional repo sync.

## Deploy Commands

The preferred deploy lane is the guided one-command launcher:

```text
deploy/android/onboard.sh
```

The lower-level deploy lane is still available for step-by-step troubleshooting:

```text
deploy/android/adb-provision.sh
deploy/android/push-runtime.sh
deploy/android/start-worker.sh
deploy/android/health-check.sh
```

See `deploy/android/README.md` for USB, wireless-debugging, ADB-forwarded SSH, and direct SSH examples.

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

- Prebuilt Quant-M Android binaries still need to be staged.
- On-device worker command arguments may need adjustment once the final Quant-M CLI surface is locked.
