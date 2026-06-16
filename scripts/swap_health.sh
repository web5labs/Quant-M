#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

CONFIG_PATH="${QUANT_M_CONFIG:-quant-m.toml}"
BIN_PATH="${QUANT_M_BIN:-./target/release/quant-m}"
PAYLOAD_PATH="${1:-${SWAP_HEALTH_PAYLOAD:-}}"

if [[ -z "$PAYLOAD_PATH" ]]; then
  echo "usage: scripts/swap_health.sh <swap-health-payload.json>" >&2
  echo "or set SWAP_HEALTH_PAYLOAD=/path/to/payload.json" >&2
  exit 2
fi

if [[ ! -f "$PAYLOAD_PATH" ]]; then
  echo "swap_health: payload file not found: $PAYLOAD_PATH" >&2
  exit 2
fi

if [[ ! -x "$BIN_PATH" ]]; then
  cargo build --release >/dev/null
fi

PAYLOAD="$(cat "$PAYLOAD_PATH")"
"$BIN_PATH" --config "$CONFIG_PATH" state swap-health "$PAYLOAD"
