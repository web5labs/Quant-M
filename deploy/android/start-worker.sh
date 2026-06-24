#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

usage() {
  cat <<'USAGE'
Usage:
  SSH_USER=u0_a123 deploy/android/start-worker.sh

Starts Quant-M as a background worker process over SSH.

Environment:
  QUANTM_ARGS="worker run"       command args, default worker run
  SSH_USER=u0_a123               required
  ADB_SERIAL=device_serial       optional for ADB port-forward mode
  SSH_HOST=<device-ip> SSH_PORT=8022 for direct SSH mode
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

QUANTM_ARGS="${QUANTM_ARGS:-worker run}"

ssh_base "test -x '$REMOTE_BIN'"
ssh_base "mkdir -p '$REMOTE_ROOT/logs' '$REMOTE_WORKSPACE'"
ssh_base "cd '$REMOTE_ROOT' && nohup '$REMOTE_BIN' $QUANTM_ARGS > '$REMOTE_ROOT/logs/quant-m-worker.log' 2>&1 & echo \$! > '$REMOTE_ROOT/quant-m-worker.pid'"

echo "Quant-M worker start requested."
echo "Log: $REMOTE_ROOT/logs/quant-m-worker.log"
