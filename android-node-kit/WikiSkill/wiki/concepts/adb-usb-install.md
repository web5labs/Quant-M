---
type: concept
updated: 2026-06-13
source_count: 2
tags: [adb, usb, termux, installation]
---

# ADB USB Install

Sources:

- `android-node-kit/bundles/quant-m-edge-bundle/manifest.toml`
- `android-node-kit/WikiSkill/raw/android-usb-provisioning-instruction.md`

The planned install path uses USB and ADB to avoid depending on Wi-Fi during initial setup. USB is the provisioning control plane; Bluetooth and local Wi-Fi are not required for first install.

## Flow

1. Enable developer options and USB debugging on the Android device.
2. Confirm the host sees the device with `adb devices`.
3. Install the compatible Termux APK from `android-node-kit/apks/termux/`.
4. Install the Termux:API companion APK once it has been added to the bundle.
5. Open Termux once on-device so its home directory exists.
6. Push the selected offline profile bundle to `/sdcard/Download/quant-m-edge-bundle`.
7. Run the profile's offline installer inside Termux.
8. Push the matching prebuilt Quant-M Rust runtime binary and Android config.
9. Run Quant-M init/status checks.

## Notes

The default install profile is `base-runtime`, which avoids npm, Node.js, Git, Rust/Cargo, and LLVM/Clang. Builder devices can opt into `dev-builder`, but ordinary edge devices should receive prebuilt Quant-M binaries.

## Links

- [[sources/quant-m-edge-bundle-source]]
- [[sources/android-usb-provisioning-instruction]]
- [[concepts/usb-provisioning-mode]]
- [[entities/device-inventory]]
- [[syntheses/quant-m-edge-bundle-plan]]
