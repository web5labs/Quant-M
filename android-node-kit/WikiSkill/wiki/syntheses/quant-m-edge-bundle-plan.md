---
type: synthesis
updated: 2026-06-13
source_count: 2
tags: [quant-m, android, edge-bundle, termux]
---

# Quant-M Edge Bundle Plan

Sources:

- `android-node-kit/WikiSkill/raw/quant-m-edge-bundle-source.md`
- `android-node-kit/WikiSkill/raw/android-usb-provisioning-instruction.md`
- `android-node-kit/WikiSkill/raw/android-usb-runtime-node-01-proposal.md`

The default bundle should prepare Android devices as USB-provisioned Quant-M workers: Termux, Termux:API, SSH, curl, Android peripheral helpers, and a prebuilt Quant-M Rust runtime binary. npm/Node.js are out of scope. Rust/Cargo belongs only in an explicit builder profile.

## Bundle Manifest

The active manifest is:

- `android-node-kit/bundles/profiles/base-runtime/manifest.toml`

The quickstart is:

- `android-node-kit/bundles/profiles/base-runtime/README.md`

The host staging script is:

- `android-node-kit/scripts/adb-stage-device.sh`

The default slim runtime profile is:

- `android-node-kit/bundles/profiles/base-runtime`

The USB provisioning doc is:

- `android-node-kit/docs/edge/ANDROID_USB_PROVISIONING.md`

The next runtime milestone is:

- `android-node-kit/docs/edge/ANDROID_USB_RUNTIME_NODE_01.md`

The offline installer is:

- `android-node-kit/bundles/profiles/base-runtime/offline-install-termux.sh`

It records:

- Termux APK payloads.
- The verified Termux:API APK.
- Offline Termux apt mirrors for `termux-main` and `termux-main-21`.
- Base runtime packages: `openssh`, `curl`, `termux-tools`, `termux-api`.
- Dev builder packages: `openssh`, `git`, `curl`, `termux-tools`, `termux-api`, `rust`, `rsync`.
- USB-first device preparation checklist.
- Optional package: `rsync` in the builder profile.
- Quant-M source and expected binary paths.
- ADB install steps.
- Verification checks.
- Current device inventory count.

## Runtime Profile State

`base-runtime` installs:

- `openssh`
- `curl`
- `termux-tools`
- `termux-api`

It also prepares:

- `$HOME/node`
- `$HOME/node/bundle`
- `$HOME/quant-m-node/bin`

It intentionally excludes npm, Node.js, Git, Rust/Cargo, LLVM/Clang, and rsync.

`dev-builder` is the heavy opt-in profile for devices that must compile Rust locally.

## Next Artifacts To Add

- `android-node-kit/bundles/quant-m-edge-bundle/bin/aarch64-linux-android/quant-m`
- `android-node-kit/bundles/quant-m-edge-bundle/bin/armv7-linux-androideabi/quant-m`
- Build notes for the Android NDK and linker setup used to produce those binaries.
- Worker registration script or checklist that appends new nodes to private local `android-node-kit/inventory/nodes.csv`.
- Final Quant-M CLI args for `deploy/android/start-worker.sh` once the worker command is locked.

## Offline Mirror State

- Base `termux-main`: 58 packages for `aarch64`, 58 packages for `arm`.
- Base `termux-main-21`: 43 packages for `aarch64`, 43 packages for `arm`.
- Base runtime profile size: `54M`; modern per-device payload with APKs is about `93 MiB`.
- Dev builder `termux-main`: 78 packages for `aarch64`, 78 packages for `arm`.
- Dev builder `termux-main-21`: 55 packages for `aarch64`, 55 packages for `arm`.
- The installer chooses `termux-main-21` automatically for Android SDK 21-23 when present.

## Links

- [[sources/quant-m-edge-bundle-source]]
- [[sources/android-usb-provisioning-instruction]]
- [[sources/android-usb-runtime-node-01-proposal]]
- [[syntheses/android-usb-runtime-node-01]]
- [[entities/device-inventory]]
- [[concepts/adb-usb-install]]
- [[concepts/usb-provisioning-mode]]
