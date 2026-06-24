#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TERMUX_APK="${TERMUX_APK:-$ROOT_DIR/apks/termux/termux-app.apk}"
TERMUX_API_APK="${TERMUX_API_APK:-$ROOT_DIR/apks/termux/termux-api.apk}"
BOOTSTRAP="$ROOT_DIR/bootstrap/bootstrap-termux.sh"
PROFILE="${PROFILE:-base-runtime}"
if [[ -z "${BUNDLE_DIR:-}" ]]; then
  case "$PROFILE" in
    base-runtime)
      BUNDLE_DIR="$ROOT_DIR/bundles/profiles/base-runtime"
      ;;
    dev-builder)
      BUNDLE_DIR="$ROOT_DIR/bundles/quant-m-edge-bundle"
      ;;
    *)
      BUNDLE_DIR="$ROOT_DIR/bundles/profiles/$PROFILE"
      ;;
  esac
fi
SERIAL_ARG=()

usage() {
  cat <<'USAGE'
Usage:
  android-node-kit/scripts/adb-stage-device.sh [adb-serial]

Stages the Android edge-node base over USB:
  - installs Termux APK
  - installs Termux:API APK
  - pushes selected offline profile to /sdcard/Download/quant-m-edge-bundle/

After this script finishes, open Termux on the device and run:
  bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh

Environment overrides:
  PROFILE=base-runtime|dev-builder
  BUNDLE_DIR=/path/to/custom/profile
  TERMUX_APK=/path/to/termux.apk
  TERMUX_API_APK=/path/to/termux-api.apk
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

if [[ $# -gt 0 ]]; then
  SERIAL_ARG=(-s "$1")
fi

for required in adb "$TERMUX_APK" "$TERMUX_API_APK" "$BOOTSTRAP" "$BUNDLE_DIR"; do
  if [[ "$required" == "adb" ]]; then
    command -v adb >/dev/null 2>&1 || {
      echo "adb not found on host PATH" >&2
      exit 1
    }
  elif [[ ! -e "$required" ]]; then
    echo "missing required path: $required" >&2
    exit 1
  fi
done

echo "Checking connected device..."
adb "${SERIAL_ARG[@]}" get-state

echo "Installing Termux..."
adb "${SERIAL_ARG[@]}" install -r "$TERMUX_APK"

echo "Installing Termux:API..."
adb "${SERIAL_ARG[@]}" install -r "$TERMUX_API_APK"

echo "Using profile: $PROFILE"
echo "Pushing offline profile bundle..."
adb "${SERIAL_ARG[@]}" push "$BUNDLE_DIR" /sdcard/Download/quant-m-edge-bundle

echo "Done. Open Termux on the device, then run:"
echo "  bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh"
