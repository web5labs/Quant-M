# Quant-M Android Base Runtime Profile

Slim offline install profile for devices that only need to run a prebuilt Quant-M binary and expose Android peripherals.

Approximate profile size after mirroring modern and Android 5/6 packages: `54M`.

Approximate per-device USB payload excluding Quant-M:

- Modern Termux APK + Termux:API APK + base runtime profile: `93 MiB`.
- Android 5 APK + Termux:API APK + base runtime profile: `86 MiB`.
- Android 5 universal APK + Termux:API APK + base runtime profile: `140 MiB`.

## Includes

- `openssh`
- `curl`
- `termux-tools`
- `termux-api`

## Excludes

- `git`
- `rust`
- Cargo
- LLVM/Clang build toolchain
- `rsync`

Use this profile for most old phones/tablets. Ship Quant-M as a prebuilt binary for the device architecture instead of compiling on-device.

## Install

```bash
PROFILE=base-runtime android-node-kit/scripts/adb-stage-device.sh
```

Then open Termux:

```bash
bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh
```
