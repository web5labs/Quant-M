# Android Deploy Scripts

Host-side deploy lane for Quant-M Android edge devices.

Default target: a slim `base-runtime` device that runs a prebuilt Quant-M Rust binary with no npm, Node.js, Git, Cargo, or internet requirement.

## Scripts

- `adb-provision.sh`: install Termux + Termux:API and push the selected offline profile over ADB.
- `push-runtime.sh`: push the prebuilt Quant-M binary, config, and optional workspace over SSH.
- `start-worker.sh`: start Quant-M as a background worker over SSH.
- `health-check.sh`: verify runtime dependencies and worker state over SSH.

## Provision Over USB

```bash
PROFILE=base-runtime deploy/android/adb-provision.sh
```

Then open Termux on the device:

```bash
bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh
passwd
sshd
whoami
```

Push the runtime over USB-forwarded SSH:

```bash
SSH_USER=u0_a123 QUANTM_BIN=/path/to/quant-m deploy/android/push-runtime.sh
SSH_USER=u0_a123 deploy/android/start-worker.sh
SSH_USER=u0_a123 deploy/android/health-check.sh
```

## Provision Over Wireless Debugging

Connect ADB first:

```bash
adb connect 192.168.1.23:5555
ADB_SERIAL=192.168.1.23:5555 PROFILE=base-runtime deploy/android/adb-provision.sh
```

After Termux SSH is running, either keep using ADB port forwarding:

```bash
ADB_SERIAL=192.168.1.23:5555 SSH_USER=u0_a123 QUANTM_BIN=/path/to/quant-m deploy/android/push-runtime.sh
```

Or use direct SSH if the device has a reachable IP:

```bash
SSH_HOST=192.168.1.23 SSH_PORT=8022 SSH_USER=u0_a123 QUANTM_BIN=/path/to/quant-m deploy/android/push-runtime.sh
```

## Android 5 Era Devices

```bash
TERMUX_APK=android-node-kit/apks/termux/termux-app-apt-android-5-universal.apk PROFILE=base-runtime deploy/android/adb-provision.sh
```
