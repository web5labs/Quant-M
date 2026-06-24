#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$SCRIPT_DIR/common.sh"

usage() {
  cat <<'USAGE'
Usage:
  SSH_USER=u0_a123 deploy/android/health-check.sh

Checks the Android runtime over SSH.

Environment:
  SSH_USER=u0_a123               required
  ADB_SERIAL=device_serial       optional for ADB port-forward mode
  SSH_HOST=<device-ip> SSH_PORT=8022 for direct SSH mode
USAGE
}

if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  usage
  exit 0
fi

ssh_base "set -e
echo DEVICE=\$(getprop ro.product.model 2>/dev/null || true)
echo SDK=\$(getprop ro.build.version.sdk 2>/dev/null || true)
echo USER=\$(whoami)
command -v ssh >/dev/null && echo SSH_CLIENT=ok
command -v curl >/dev/null && echo CURL=ok
command -v termux-battery-status >/dev/null && echo TERMUX_API=ok
if command -v node >/dev/null 2>&1; then echo NODE=unexpected; else echo NODE=absent; fi
if command -v npm >/dev/null 2>&1; then echo NPM=unexpected; else echo NPM=absent; fi
if command -v cargo >/dev/null 2>&1; then echo CARGO=unexpected_default; else echo CARGO=absent; fi
test -x '$REMOTE_BIN' && echo QUANTM_BIN=ok || echo QUANTM_BIN=missing
if [ -f '$REMOTE_ROOT/quant-m-worker.pid' ]; then
  pid=\$(cat '$REMOTE_ROOT/quant-m-worker.pid')
  if kill -0 \"\$pid\" 2>/dev/null; then echo WORKER=running; else echo WORKER=not_running; fi
else
  echo WORKER=no_pid
fi"
