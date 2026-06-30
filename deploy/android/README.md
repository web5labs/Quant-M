# Android Deploy Scripts

Host-side deploy lane for Quant-M Android edge devices.

Default target: a slim `base-runtime` device that runs a prebuilt Quant-M Rust binary with no npm, Node.js, Git, Cargo, or internet requirement.

Old Android and Termux devices should not be expected to clone GitHub or build Quant-M from source during normal Agent Cluster onboarding. Git HTTPS can fail on stale or mismatched Termux package sets with errors like `git-remote-https` aborts or `cannot locate symbol` from networking/TLS libraries. Treat Git/Cargo on-device builds as a development fallback only.

If a tablet hits that Termux package failure, repair the Termux environment with package updates, `termux-change-repo`, and Git/curl/TLS package reinstalls. Product direction is to route around that class of failure: the core should host or push a prebuilt child binary over local Wi-Fi, then the child pairs and syncs approved packs.

## Simple Onboarding

From the prepared Quant-M repo on the laptop, plug in one authorized Android device and run:

```bash
bash deploy/android/onboard.sh
```

If you already have the prebuilt Android Quant-M binary, include it in the same copy-paste:

```bash
QUANTM_BIN=/path/to/android/quant-m bash deploy/android/onboard.sh
```

For wireless debugging:

```bash
WIRELESS_ADB=192.168.1.23:5555 QUANTM_BIN=/path/to/android/quant-m bash deploy/android/onboard.sh
```

The launcher handles the normal flow:

- detect or connect to the Android device
- install Termux and Termux:API
- stage the selected offline bundle
- open Termux
- show the one device-side command to paste
- infer the Termux SSH user when possible
- push the Quant-M binary
- start the worker
- run the health check

This flow is the preferred old-device path because the child does not need GitHub, Cargo, Rust, or source checkout health on the tablet.

The APKs and offline package mirrors stay local in ignored paths, so the GitHub repo remains lightweight while the prepared laptop checkout can still deploy offline.

Device-side Termux command shown by the launcher:

```bash
bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh && passwd && sshd
```

## Scripts

- `onboard.sh`: guided one-command install, runtime push, worker start, and health-check lane.
- `adb-provision.sh`: install Termux + Termux:API and push the selected offline profile over ADB.
- `push-runtime.sh`: push the prebuilt Quant-M binary, config, and optional workspace over SSH.
- `start-worker.sh`: start Quant-M as a background worker over SSH.
- `health-check.sh`: verify runtime dependencies and worker state over SSH.

The scripts below remain available when you need to run a specific step by hand.

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
