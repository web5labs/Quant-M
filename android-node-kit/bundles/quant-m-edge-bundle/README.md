# Quant-M Edge Bundle

Heavy USB/ADB staging bundle for Android edge devices that need local Rust/Cargo build tools.

Most devices should use the slim runtime profile instead:

```bash
PROFILE=base-runtime android-node-kit/scripts/adb-stage-device.sh
```

Use this heavy bundle only when a device must compile Rust locally:

```bash
PROFILE=dev-builder android-node-kit/scripts/adb-stage-device.sh
```

## Current Contents

- Manifest: `android-node-kit/bundles/quant-m-edge-bundle/manifest.toml`
- Termux APKs: `android-node-kit/apks/termux/`
- Offline Termux `.deb` mirror: `android-node-kit/bundles/quant-m-edge-bundle/offline`
- Offline installer: `android-node-kit/bundles/quant-m-edge-bundle/offline-install-termux.sh`
- Bootstrap: `android-node-kit/bootstrap/bootstrap-termux.sh`
- Inventory: `android-node-kit/inventory/nodes.csv`
- Wiki: `android-node-kit/WikiSkill/wiki/index.md`

## Missing Before Full Install

- `android-node-kit/bundles/quant-m-edge-bundle/bin/aarch64-linux-android/quant-m`
- `android-node-kit/bundles/quant-m-edge-bundle/bin/armv7-linux-androideabi/quant-m`

## Offline Package Mirror

The bundle includes local Termux apt repos for:

- `termux-main`: modern Termux packages, `aarch64` and `arm`, 78 packages per architecture.
- `termux-main-21`: Android SDK 21-23 legacy packages, `aarch64` and `arm`, 55 packages per architecture.

The mirrored root packages are:

- `openssh`
- `git`
- `curl`
- `termux-tools`
- `termux-api`
- `rust`
- `rsync`

Approximate offline mirror size: `920M`.

## ADB Install Outline

From the host, after enabling USB debugging:

```bash
android-node-kit/scripts/adb-stage-device.sh
```

Or manually:

```bash
adb devices
adb install -r android-node-kit/apks/termux/termux-app.apk
adb install -r android-node-kit/apks/termux/termux-api.apk
adb push android-node-kit/bundles/quant-m-edge-bundle /sdcard/Download/quant-m-edge-bundle
```

Then open Termux on the device and run:

```bash
bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh
```

The bootstrap installs:

- `openssh`
- `git`
- `curl`
- `termux-tools`
- `termux-api`
- `rust` for Cargo
- `rsync` when available

Peripheral helpers checked by the bootstrap:

- `termux-camera-photo`
- `termux-microphone-record`
- `termux-open-url`

After the matching Quant-M binary is built and staged:

```bash
adb push android-node-kit/bundles/quant-m-edge-bundle/bin/aarch64-linux-android/quant-m /sdcard/Download/quant-m
```

Inside Termux:

```bash
cp /sdcard/Download/quant-m ~/quant-m-node/bin/quant-m
chmod +x ~/quant-m-node/bin/quant-m
cd ~/quant-m-node
~/quant-m-node/bin/quant-m init
~/quant-m-node/bin/quant-m status
```
