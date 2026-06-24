#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

usage() {
  cat <<'USAGE'
Usage:
  SSH_USER=u0_a123 QUANTM_BIN=/path/to/quant-m deploy/android/push-runtime.sh

Pushes the prebuilt Quant-M Rust runtime and optional config files over SSH.

Connection modes:
  USB/ADB forward: leave SSH_HOST/SSH_PORT unset, set ADB_SERIAL if needed.
  Direct SSH: set SSH_HOST=<device-ip> SSH_PORT=8022.

Environment:
  QUANTM_BIN=/path/to/quant-m               required
  QUANTM_CONFIG=/path/to/android.toml       optional
  QUANTM_WORKSPACE=/path/to/workspace       optional directory
  SSH_USER=u0_a123                          required
  ADB_SERIAL=device_serial_or_wireless_host optional
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

QUANTM_BIN="${QUANTM_BIN:-}"
QUANTM_CONFIG="${QUANTM_CONFIG:-}"
QUANTM_WORKSPACE="${QUANTM_WORKSPACE:-}"

if [[ -z "$QUANTM_BIN" || ! -f "$QUANTM_BIN" ]]; then
  echo "QUANTM_BIN is required and must point to a prebuilt Quant-M binary." >&2
  exit 1
fi

need ssh
need scp

echo "Preparing remote runtime directories..."
ssh_base "mkdir -p '$REMOTE_ROOT/bin' '$REMOTE_WORKSPACE' '$REMOTE_ROOT/logs' '$REMOTE_ROOT/config'"

echo "Pushing Quant-M binary..."
scp_to_device "$QUANTM_BIN" "$REMOTE_BIN"
ssh_base "chmod +x '$REMOTE_BIN'"

if [[ -n "$QUANTM_CONFIG" ]]; then
  [[ -f "$QUANTM_CONFIG" ]] || {
    echo "QUANTM_CONFIG does not exist: $QUANTM_CONFIG" >&2
    exit 1
  }
  echo "Pushing Quant-M config..."
  scp_to_device "$QUANTM_CONFIG" "$REMOTE_ROOT/config/quant-m.toml"
fi

if [[ -n "$QUANTM_WORKSPACE" ]]; then
  [[ -d "$QUANTM_WORKSPACE" ]] || {
    echo "QUANTM_WORKSPACE does not exist: $QUANTM_WORKSPACE" >&2
    exit 1
  }
  echo "Pushing Quant-M workspace..."
  scp -r -P "$SSH_PORT" "$QUANTM_WORKSPACE"/. "$SSH_USER@$SSH_HOST:$REMOTE_WORKSPACE/"
fi

echo "Runtime pushed to $REMOTE_BIN"
