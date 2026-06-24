# Quant-M Edge Bundle Source Snapshot

Date: 2026-06-12

Workspace sources used for this bundle:

- `android-node-kit/bootstrap/bootstrap-termux.sh`
- `android-node-kit/bootstrap/node.env.example`
- `android-node-kit/inventory/nodes.csv`
- `android-node-kit/apks/termux/termux-app.apk`
- `android-node-kit/apks/termux/termux-app-apt-android-5.apk`
- `android-node-kit/apks/termux/termux-app-apt-android-5-universal.apk`
- `android-node-kit/apks/termux/termux-api.apk`
- `android-node-kit/apks/termux/termux-api-checksums-sha256.txt`
- `android-node-kit/scripts/adb-stage-device.sh`
- `android-node-kit/scripts/build-termux-offline-mirror.py`
- `android-node-kit/bundles/quant-m-edge-bundle/offline-install-termux.sh`
- `android-node-kit/bundles/quant-m-edge-bundle/offline/`
- `quant-m/Quant-M/docs/deploy-android.md`
- `quant-m/Quant-M/configs/nodes/android-default.toml`
- `quant-m/Quant-M/Cargo.toml`
- `android-node-kit/bundles/quant-m-edge-bundle/manifest.toml`
- `android-node-kit/bundles/quant-m-edge-bundle/README.md`

Observed inventory:

- `android-node-kit/inventory/nodes.csv` contains 5 recorded devices.

Observed gaps:

- Termux APKs are present.
- Termux:API companion APK is present and verified against its official SHA-256 checksum.
- Offline Termux `.deb` mirrors are present for `termux-main` and `termux-main-21`, covering `aarch64` and `arm`.
- Quant-M Android prebuilt binaries are not present yet.
- The host NDK/linker configuration for Android Rust builds is not captured yet.
