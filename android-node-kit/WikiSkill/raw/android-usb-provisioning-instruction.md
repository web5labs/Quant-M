# Android USB Provisioning Instruction

Date ingested: 2026-06-13

Source: user-provided pasted instruction.

## Full Text

Yes — that is the right preparation model for Quant-M edge devices. Think of it as **USB provisioning mode** first, then **Quant-M worker mode** after the device is prepared.

Google’s official Android docs say USB debugging lets an Android device communicate with a development machine through ADB, and Developer Options are where that setting lives. Termux:API is also specifically an add-on to Termux that exposes Android device functions to terminal scripts, so installing both Termux and Termux:API offline makes sense for your edge-node plan.

## Device settings checklist for USB-first Quant-M prep

On each Android phone or tablet:

### 1. Factory reset, then do minimum setup

After reset:

- Connect only long enough to complete basic Android setup if the device requires it.
- Skip extra accounts if possible.
- Disable cloud restore if this is a clean worker node.
- Set a simple local PIN you control.
- Name the device something like `quantm-edge-01`, `quantm-edge-02`, or `quantm-tablet-01`.

This matters later when you have multiple USB devices plugged in.

### 2. Enable Developer Options

Usually:

Settings -> About phone/tablet -> Build number -> tap 7 times.

On some devices it is:

Settings -> About device -> Software information -> Build number.

Then go back to Settings -> System -> Developer options, or search Settings for "Developer options."

### 3. Turn ON the important USB settings

Inside Developer options, enable:

- **USB debugging**: allows the laptop to talk to the device over USB through ADB.
- **Install via USB**, if available.
- **USB debugging security settings**, only if needed. Do not enable extra security bypass features unless needed.
- **Default USB configuration -> File Transfer / MTP**.
- **Stay awake while charging**.
- **Disable automatic system updates**, if available.
- **Disable permission monitoring / app scanning prompts only if they block local trusted APK installation**.

### 4. Turn OFF things that fight wired provisioning

Turn off:

- Bluetooth, unless needed.
- Nearby Share / Quick Share.
- Wi-Fi scanning.
- Bluetooth scanning.
- Battery saver.
- Adaptive battery for Termux.
- Background restriction for Termux and Termux:API.
- Aggressive auto screen lock.
- Auto app optimization for Termux.
- USB tethering unless the phone should provide network over USB.

The key idea: **USB is the control plane. Not Bluetooth. Not local Wi-Fi.**

### 5. Authorize the laptop

Plug the device into the laptop with a data-capable USB cable.

On the laptop:

```bash
adb devices
```

On the Android device, accept "Allow USB debugging?" and check "Always allow from this computer" only for a trusted laptop.

Then run again:

```bash
adb devices
```

You want to see `device`, not `unauthorized`.

### 6. Offline install package order

Install in this order:

1. Termux APK
2. Termux:API APK
3. Any Termux boot/helper APKs you choose to use
4. Local package archive or bootstrap files
5. Quant-M worker binary or Quant-M repo bundle

Example pattern:

```bash
adb install Termux.apk
adb install TermuxAPI.apk
adb push quantm-edge-bundle/ /sdcard/Download/quantm-edge-bundle/
```

Inside Termux, import or install from that local bundle.

## Best Quant-M edge-device profile

Use this role:

**Android edge node equals wired worker, not primary coordinator.**

It should run:

- Termux
- Termux:API
- OpenSSH
- curl
- cargo / Rust toolchain if the device is strong enough
- a prebuilt `quant-m-node` binary if the device is weaker
- health-check scripts
- USB/ADB provisioning scripts
- optional tmux session

The laptop should run:

- cmux/tmux cockpit
- main Quant-M repo
- build pipeline
- device registry
- USB provisioning script
- shared-state coordinator
- logs and audit ledger

## Security Rule

After provisioning, either keep USB debugging enabled only for trusted wired lab devices, or turn it off before the device leaves your control. USB debugging is powerful and should not be left enabled on a personal daily phone.

## Repo Destination

This should become a dedicated Quant-M doc:

`docs/edge/ANDROID_USB_PROVISIONING.md`

With sections for factory reset checklist, Developer Options settings, offline APK install, ADB authorization, Termux package bootstrap, SSH setup, USB-only operation, Quant-M worker registration, device health validation, and teardown/reset procedure.

The clean definition is:

**A Quant-M Android edge device is factory-reset, USB-authorized, Termux-preloaded, SSH-capable, and registered as a wired worker node before it ever joins the cluster.**
