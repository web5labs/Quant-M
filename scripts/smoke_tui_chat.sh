#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

TMP_DIR="$(mktemp -d "${TMPDIR:-/tmp}/quantm-tui-chat.XXXXXX")"
trap 'rm -rf "$TMP_DIR"' EXIT

CONFIG="$TMP_DIR/quant-m.toml"

if [[ -x "$ROOT/target/debug/quant-m" ]]; then
  QUANT_M=("$ROOT/target/debug/quant-m")
elif [[ -x "$ROOT/target/release/quant-m" ]]; then
  QUANT_M=("$ROOT/target/release/quant-m")
else
  QUANT_M=(cargo run --quiet --)
fi

cat <<'EOF'
Manual Quant-M TUI chat smoke

This opens a throwaway inspect-mode TUI. Try:
  /help
  /stateful should stay ask-inspect text
  /state
  /cost
  /ask does this call a provider?
  /consensus should stay blocked in inspect mode
  /quit

Expected:
  - no provider call
  - no worker write
  - no consensus dry-run artifacts in inspect mode
  - compact terminals show a single-column chat view
  - wide terminals show the evidence rail
EOF

"${QUANT_M[@]}" --config "$CONFIG" init --non-interactive >/dev/null
"${QUANT_M[@]}" --config "$CONFIG" tui chat --inspect
