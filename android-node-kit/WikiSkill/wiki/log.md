---
type: log
updated: 2026-06-12
tags: [wiki-log]
---

# Wiki Log

## [2026-06-12] ingest | Quant-M Edge Bundle Source

Created the Android Node Kit wiki for the USB/ADB Quant-M edge-device bundle. Added source snapshot, install concept, inventory entity, and initial bundle synthesis. Recorded current gaps: missing Termux:API APK, missing prebuilt Quant-M Android binaries, and missing host NDK/linker build notes.

Added bundle README quickstart beside the manifest and updated source/index pages to reference it.

Downloaded Termux:API `v0.53.0` APK from the official GitHub release, saved its checksum file, verified SHA-256 `ecf916ff80ae751e65c092f51c055cce4de417ebeea8e449cd0f294afdbde39a`, added `termux-tools`, and added `android-node-kit/scripts/adb-stage-device.sh` for repeatable USB/ADB staging.

Built offline Termux apt mirrors under `android-node-kit/bundles/quant-m-edge-bundle/offline/`: modern `termux-main` for `aarch64` and `arm` with 78 packages per architecture, plus legacy `termux-main-21` for Android SDK 21-23 with 55 packages per architecture. Added `offline-install-termux.sh` and made ADB staging push the full bundle.

Added slim `base-runtime` profile under `android-node-kit/bundles/profiles/base-runtime`, excluding Git, Rust/Cargo, LLVM/Clang, and rsync. The ADB staging script now defaults to `PROFILE=base-runtime`; `PROFILE=dev-builder` points to the heavier Rust/Cargo bundle.

## [2026-06-13] ingest | Android USB Provisioning Instruction

Ingested user-provided Android USB provisioning guidance as `android-node-kit/WikiSkill/raw/android-usb-provisioning-instruction.md`. Added source summary, USB provisioning concept, and dedicated doc `android-node-kit/docs/edge/ANDROID_USB_PROVISIONING.md`. Updated the ADB install and bundle synthesis pages to emphasize USB-first provisioning, prebuilt Quant-M Rust runtime binaries, no npm/Node.js, and Cargo only for explicit builder-profile devices.

## [2026-06-13] ingest | Android USB Runtime Node 01 Proposal

Ingested the proposed `ANDROID_USB_RUNTIME_NODE_01` milestone as `android-node-kit/WikiSkill/raw/android-usb-runtime-node-01-proposal.md`. Added source summary, synthesis page, and `android-node-kit/docs/edge/ANDROID_USB_RUNTIME_NODE_01.md` with the definition of shippable, profile split, planned deploy commands, acceptance test, and current gaps.

## [2026-06-14] implementation | Android Deploy Lane

Added host-side deploy scripts under `deploy/android/`: `adb-provision.sh`, `push-runtime.sh`, `start-worker.sh`, `health-check.sh`, and shared `common.sh`. The scripts support USB/ADB port-forwarded SSH and direct SSH for wireless/LAN devices. Updated edge docs and runtime-node synthesis to point at the implemented deploy lane.
