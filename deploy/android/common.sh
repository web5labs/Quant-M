#!/usr/bin/env bash
set -euo pipefail

DEPLOY_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$DEPLOY_DIR/../.." && pwd)"
ANDROID_NODE_KIT="$REPO_ROOT/android-node-kit"

PROFILE="${PROFILE:-base-runtime}"
ADB_SERIAL="${ADB_SERIAL:-}"
SSH_USER="${SSH_USER:-}"
SSH_HOST="${SSH_HOST:-127.0.0.1}"
SSH_PORT="${SSH_PORT:-}"
ADB_SSH_LOCAL_PORT="${ADB_SSH_LOCAL_PORT:-18022}"
TERMUX_SSH_PORT="${TERMUX_SSH_PORT:-8022}"
REMOTE_ROOT="${REMOTE_ROOT:-/data/data/com.termux/files/home/quant-m-node}"
REMOTE_BIN="$REMOTE_ROOT/bin/quant-m"
REMOTE_WORKSPACE="$REMOTE_ROOT/workspace"

adb_args=()
if [[ -n "$ADB_SERIAL" ]]; then
  adb_args=(-s "$ADB_SERIAL")
fi

need() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "missing required command: $1" >&2
    exit 1
  }
}

require_ssh_user() {
  if [[ -z "$SSH_USER" ]]; then
    cat >&2 <<'EOF'
SSH_USER is required.

Find it on the device from Termux:
  whoami

Then rerun with:
  SSH_USER=u0_a123 ...
EOF
    exit 1
  fi
}

setup_adb_forward_if_needed() {
  if [[ -z "${SSH_PORT:-}" ]]; then
    need adb
    echo "Forwarding localhost:$ADB_SSH_LOCAL_PORT -> device:$TERMUX_SSH_PORT via ADB..."
    adb "${adb_args[@]}" forward "tcp:$ADB_SSH_LOCAL_PORT" "tcp:$TERMUX_SSH_PORT" >/dev/null
    SSH_HOST="127.0.0.1"
    SSH_PORT="$ADB_SSH_LOCAL_PORT"
  fi
}

ssh_base() {
  require_ssh_user
  setup_adb_forward_if_needed
  ssh -o StrictHostKeyChecking=accept-new -p "$SSH_PORT" "$SSH_USER@$SSH_HOST" "$@"
}

scp_to_device() {
  require_ssh_user
  setup_adb_forward_if_needed
  local src="$1"
  local dest="$2"
  scp -P "$SSH_PORT" "$src" "$SSH_USER@$SSH_HOST:$dest"
}
