#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

PROFILE="${PROFILE:-base-runtime}"
ADB_SERIAL="${ADB_SERIAL:-}"
WIRELESS_ADB="${WIRELESS_ADB:-}"
SSH_USER="${SSH_USER:-}"
QUANTM_BIN="${QUANTM_BIN:-}"
QUANTM_ARGS="${QUANTM_ARGS:-worker run}"
SKIP_PROVISION="${SKIP_PROVISION:-0}"
SKIP_RUNTIME="${SKIP_RUNTIME:-0}"

usage() {
  cat <<'USAGE'
Usage:
  deploy/android/onboard.sh

Single-lane Android onboarding for a Quant-M edge worker.
Run this from a prepared laptop checkout that has the local offline payload
under android-node-kit/apks and android-node-kit/bundles.

Most common copy-paste:
  bash deploy/android/onboard.sh

Optional one-liners:
  QUANTM_BIN=/path/to/android/quant-m bash deploy/android/onboard.sh
  WIRELESS_ADB=192.168.1.23:5555 QUANTM_BIN=/path/to/android/quant-m bash deploy/android/onboard.sh
  ADB_SERIAL=device_serial QUANTM_BIN=/path/to/android/quant-m bash deploy/android/onboard.sh

Environment:
  PROFILE=base-runtime          default slim runtime profile
  QUANTM_BIN=/path/to/binary    prebuilt Android Quant-M binary
  WIRELESS_ADB=host:port        optional adb connect target
  ADB_SERIAL=serial             optional adb device serial
  SSH_USER=u0_a123              optional Termux user override
  SKIP_PROVISION=1              skip APK/offline bundle staging
  SKIP_RUNTIME=1                skip binary push/start/health-check
USAGE
}

say() {
  printf '\n==> %s\n' "$*"
}

die() {
  echo "error: $*" >&2
  exit 1
}

need() {
  command -v "$1" >/dev/null 2>&1 || die "missing required command: $1"
}

adb_for_device() {
  if [[ -n "$ADB_SERIAL" ]]; then
    adb -s "$ADB_SERIAL" "$@"
  else
    adb "$@"
  fi
}

choose_device() {
  need adb

  if [[ -n "$WIRELESS_ADB" ]]; then
    say "Connecting wireless ADB: $WIRELESS_ADB"
    adb connect "$WIRELESS_ADB" >/dev/null
    ADB_SERIAL="$WIRELESS_ADB"
    export ADB_SERIAL
    return
  fi

  if [[ -n "$ADB_SERIAL" ]]; then
    export ADB_SERIAL
    return
  fi

  devices=()
  while IFS= read -r device; do
    devices+=("$device")
  done < <(adb devices | awk 'NR > 1 && $2 == "device" {print $1}')
  case "${#devices[@]}" in
    0)
      die "no authorized Android device found. Enable USB debugging or set WIRELESS_ADB=host:port."
      ;;
    1)
      ADB_SERIAL="${devices[0]}"
      ;;
    *)
      echo "Multiple authorized devices found:"
      printf '  %s\n' "${devices[@]}"
      printf 'Paste the device serial to use: '
      read -r ADB_SERIAL
      [[ -n "$ADB_SERIAL" ]] || die "ADB serial is required when multiple devices are attached."
      ;;
  esac
  export ADB_SERIAL
}

infer_termux_user() {
  if [[ -n "$SSH_USER" ]]; then
    export SSH_USER
    return
  fi

  local uid
  uid="$(
    adb_for_device shell dumpsys package com.termux 2>/dev/null \
      | sed -n 's/.*userId=\([0-9][0-9]*\).*/\1/p' \
      | tr -d '\r' \
      | head -n 1
  )"

  if [[ "$uid" =~ ^[0-9]+$ && "$uid" -ge 10000 ]]; then
    SSH_USER="u0_a$((uid - 10000))"
    export SSH_USER
    return
  fi

  cat <<'EOF'

I could not infer the Termux SSH user automatically.
On the device, open Termux and run:
  whoami
EOF
  printf 'Paste the Termux user here: '
  read -r SSH_USER
  [[ -n "$SSH_USER" ]] || die "SSH_USER is required to push and start the runtime."
  export SSH_USER
}

find_quantm_binary() {
  if [[ -n "$QUANTM_BIN" ]]; then
    [[ -f "$QUANTM_BIN" ]] || die "QUANTM_BIN does not exist: $QUANTM_BIN"
    export QUANTM_BIN
    return
  fi

  local candidate
  for candidate in \
    "$REPO_ROOT/dist/android/aarch64/quant-m" \
    "$REPO_ROOT/dist/android/armv7/quant-m" \
    "$REPO_ROOT/target/aarch64-linux-android/release/quant-m" \
    "$REPO_ROOT/target/armv7-linux-androideabi/release/quant-m"
  do
    if [[ -f "$candidate" ]]; then
      QUANTM_BIN="$candidate"
      export QUANTM_BIN
      return
    fi
  done

  cat <<'EOF'

No prebuilt Android Quant-M binary was found automatically.
Provisioning can continue, but runtime push/start will be skipped.

Later, rerun:
  QUANTM_BIN=/path/to/android/quant-m bash deploy/android/onboard.sh
EOF
  SKIP_RUNTIME=1
  export SKIP_RUNTIME
}

wait_for_termux_setup() {
  cat <<'EOF'

On the Android device, open Termux and paste this single line:

  bash /sdcard/Download/quant-m-edge-bundle/offline-install-termux.sh && passwd && sshd

Notes:
  - Choose a simple temporary SSH password when passwd asks.
  - Keep Termux open until sshd has started.
EOF
  printf '\nPress Enter here after the Termux command finishes... '
  read -r _
}

run_step() {
  say "$1"
  shift
  "$@"
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

cd "$REPO_ROOT"

choose_device
say "Using ADB device: $ADB_SERIAL"
adb_for_device get-state >/dev/null

if [[ "$SKIP_PROVISION" != "1" ]]; then
  run_step "Installing Termux, Termux:API, and staging the offline $PROFILE bundle" \
    "$SCRIPT_DIR/adb-provision.sh" "$ADB_SERIAL"
  adb_for_device shell monkey -p com.termux 1 >/dev/null 2>&1 || true
  wait_for_termux_setup
fi

find_quantm_binary

if [[ "$SKIP_RUNTIME" == "1" ]]; then
  say "Provisioning finished; runtime push/start skipped."
  echo "Device staged for profile: $PROFILE"
  exit 0
fi

need ssh
need scp
infer_termux_user

say "Using Termux SSH user: $SSH_USER"
export PROFILE ADB_SERIAL SSH_USER QUANTM_BIN QUANTM_ARGS

run_step "Pushing Quant-M runtime" "$SCRIPT_DIR/push-runtime.sh"
run_step "Starting Quant-M worker" "$SCRIPT_DIR/start-worker.sh"
run_step "Running health check" "$SCRIPT_DIR/health-check.sh"

say "Android Quant-M worker onboarding complete."
