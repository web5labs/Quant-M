# Android USB Provisioning For Quant-M Edge Devices

Quant-M Android edge devices use **USB provisioning mode** first, then **Quant-M worker mode** after the device is prepared.

The clean target state:

**A Quant-M Android edge device is factory-reset, USB-authorized, Termux-preloaded, SSH-capable, and registered as a wired worker node before it joins the cluster.**

## Device Role

Android edge devices are wired workers, not the primary coordinator.

The Android node runs:

- Termux
- Termux:API
- OpenSSH
- curl
- Termux tools for Android intents and URL/file opening
- a prebuilt Quant-M Rust runtime binary for the device architecture
- health-check scripts

The Android node should not run npm or Node.js for Quant-M. Most devices should not install Rust/Cargo/LLVM either; those belong only to an explicit builder profile.

The laptop/coordinator runs:

- main Quant-M repo
- build pipeline
- USB provisioning script
- device registry
- shared-state coordinator
- logs and audit ledger

## Factory Reset Checklist

After factory reset:

- Complete only the minimum Android setup required by the device.
- Skip extra accounts where possible.
- Disable cloud restore for clean worker nodes.
- Set a simple local PIN you control.
- Name the device consistently, such as `quantm-edge-01`, `quantm-edge-02`, or `quantm-tablet-01`.

## Developer Options

Enable Developer Options:

1. Open Settings.
2. Go to About phone/tablet.
3. Tap Build number 7 times.
4. Return to Settings and open Developer options.

On some devices, Build number is under Software information.

## USB Settings

Enable:

- USB debugging.
- Install via USB, if available.
- USB debugging security settings only if needed for local trusted install behavior.
- Default USB configuration: File Transfer / MTP.
- Stay awake while charging.
- Disable automatic system updates, if available.

Disable only if they block trusted local APK installation:

- permission monitoring prompts,
- app scanning prompts.

## Reduce Provisioning Interference

Turn off when possible:

- Bluetooth, unless needed.
- Nearby Share / Quick Share.
- Wi-Fi scanning.
- Bluetooth scanning.
- Battery saver.
- Adaptive battery for Termux.
- Background restrictions for Termux and Termux:API.
- Aggressive auto screen lock.
- Auto app optimization for Termux.
- USB tethering unless the phone should provide network over USB.

USB is the provisioning control plane. Do not depend on Bluetooth or local Wi-Fi for first install.

## ADB Authorization

Plug the device into the laptop with a data-capable USB cable.

On the laptop:

```bash
adb devices
```

On the Android device, accept the USB debugging prompt. Check "Always allow from this computer" only for your trusted laptop.

Run again:

```bash
adb devices
```

The device should show `device`, not `unauthorized`.

## Offline Install Order

Install in this order:

1. Termux APK.
2. Termux:API APK.
3. Optional helper APKs, only when required.
4. Offline Termux package profile.
5. Prebuilt Quant-M worker binary and config.

Default slim runtime install:

```bash
PROFILE=base-runtime android-node-kit/scripts/adb-stage-device.sh
```

Equivalent deploy-lane command:

```bash
PROFILE=base-runtime deploy/android/adb-provision.sh
```

For Android 5 era devices:

```bash
TERMUX_APK=android-node-kit/apks/termux/termux-app-apt-android-5-universal.apk PROFILE=base-runtime android-node-kit/scripts/adb-stage-device.sh
```

Then open Termux and run:

```bash
bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh
```

## Runtime Profile

Use `base-runtime` for most old phones/tablets.

Included:

- `openssh`
- `curl`
- `termux-tools`
- `termux-api`

Excluded:

- npm
- Node.js
- Git
- Rust/Cargo
- LLVM/Clang
- rsync

Use `dev-builder` only for a stronger device that must compile Rust locally.

## Worker Registration

After installing the runtime profile:

1. Copy the correct prebuilt Quant-M binary into `~/quant-m-node/bin/quant-m`.
2. Copy the device config into `~/quant-m-node`.
3. Run Quant-M init/status commands.
4. Record the device in private local `android-node-kit/inventory/nodes.csv`; use `android-node-kit/inventory/nodes.example.csv` as the public column template.
5. Start SSH only after the device has a known identity and local PIN.

Host-side helper scripts:

```text
deploy/android/push-runtime.sh
deploy/android/start-worker.sh
deploy/android/health-check.sh
```

## Health Validation

Check:

```bash
curl --version
sshd -h 2>&1 | head
termux-battery-status
termux-camera-info
termux-open-url about:blank
```

Camera and microphone commands may require Android runtime permissions from the Termux:API app.

## Security

USB debugging is powerful. Keep it enabled only for trusted wired lab devices. Disable it before any device leaves controlled use.

## Teardown

For a device leaving the cluster:

1. Stop Quant-M.
2. Remove SSH keys or rotate coordinator credentials.
3. Disable USB debugging.
4. Factory reset if the device will be reused outside the Quant-M lab.
