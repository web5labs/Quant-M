# Quant-M Edge Bundle Source Snapshot

Date: 2026-06-12

Workspace sources used for this bundle:

- `android-node-kit/bootstrap/bootstrap-termux.sh`
- `android-node-kit/bootstrap/node.env.example`
- `android-node-kit/inventory/nodes.example.csv`
- Local-only Termux APKs under ignored `android-node-kit/apks/termux/`
- `android-node-kit/scripts/adb-stage-device.sh`
- `android-node-kit/scripts/build-termux-offline-mirror.py`
- `android-node-kit/bundles/quant-m-edge-bundle/offline-install-termux.sh`
- Local-only offline mirrors under ignored `android-node-kit/bundles/**/offline/`
- `docs/deploy-android.md`
- `configs/nodes/android-default.toml`
- `Cargo.toml`
- `android-node-kit/bundles/quant-m-edge-bundle/manifest.toml`
- `android-node-kit/bundles/quant-m-edge-bundle/README.md`

Observed inventory shape:

- `android-node-kit/inventory/nodes.example.csv` documents the columns for private local inventory.

Observed local-preparation state and gaps:

- Termux APKs are local-only and not committed.
- Termux:API companion APK should be verified locally against its official SHA-256 checksum.
- Offline Termux `.deb` mirrors are local-only and not committed.
- Quant-M Android prebuilt binaries are not present yet.
- The host NDK/linker configuration for Android Rust builds is not captured yet.
