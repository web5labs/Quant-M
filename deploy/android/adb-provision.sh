#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

usage() {
  cat <<'USAGE'
Usage:
  deploy/android/adb-provision.sh [adb-serial]

Installs Termux + Termux:API and pushes the selected offline runtime profile.

Environment:
  PROFILE=base-runtime|dev-builder
  TERMUX_APK=/path/to/termux.apk
  TERMUX_API_APK=/path/to/termux-api.apk
  ADB_SERIAL=device_serial_or_wireless_host:port

Examples:
  PROFILE=base-runtime deploy/android/adb-provision.sh
  ADB_SERIAL=192.168.1.23:5555 PROFILE=base-runtime deploy/android/adb-provision.sh
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 0 ]]; then
  ADB_SERIAL="$1"
  export ADB_SERIAL
fi

"$ANDROID_NODE_KIT/scripts/adb-stage-device.sh" ${ADB_SERIAL:+"$ADB_SERIAL"}

cat <<'NEXT'

Next on the device:
  1. Open Termux once.
  2. Run:
     bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh
  3. Set a Termux SSH password:
     passwd
  4. Start SSH:
     sshd
  5. Find SSH user:
     whoami

Then from the laptop:
  SSH_USER=<termux-user> deploy/android/push-runtime.sh
NEXT
