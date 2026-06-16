#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CONFIG_PATH="${QUANT_M_CONFIG:-quant-m.toml}"
BIN_PATH="${QUANT_M_BIN:-./target/release/quant-m}"
PAIR="${1:-${HEALTHCHECK_PAIR:-USDJPY}}"
TS="$(date -u +%Y-%m-%dT%H:%M:%SZ)"

if [[ ! -x "$BIN_PATH" ]]; then
  cargo build --release >/dev/null
fi

status_ok=1
summary_ok=1
macro_ok=1

status_out="$("$BIN_PATH" --config "$CONFIG_PATH" status 2>&1)" || status_ok=0
summary_out="$("$BIN_PATH" --config "$CONFIG_PATH" state summary 2>&1)" || summary_ok=0
macro_out="$("$BIN_PATH" --config "$CONFIG_PATH" state macro-get-pair "$PAIR" 2>&1)" || macro_ok=0

echo "healthcheck ts=$TS pair=$PAIR status_ok=$status_ok summary_ok=$summary_ok macro_ok=$macro_ok"

if [[ "$status_ok" -ne 1 || "$summary_ok" -ne 1 || "$macro_ok" -ne 1 ]]; then
  echo "status_out=$status_out"
  echo "summary_out=$summary_out"
  echo "macro_out=$macro_out"
  exit 1
fi
