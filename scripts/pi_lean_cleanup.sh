#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

AGGRESSIVE=0
if [[ "${1:-}" == "--aggressive" ]]; then
  AGGRESSIVE=1
fi

echo "pi_lean_cleanup: root=$ROOT_DIR mode=$([[ "$AGGRESSIVE" -eq 1 ]] && echo aggressive || echo safe)"
echo "pi_lean_cleanup: before"
du -sh ./* 2>/dev/null | sort -h

# Large build artifacts (always safe to remove)
rm -rf target

# Local runtime churn
rm -rf workspace/logs/* workspace/queue/*
find . -name ".DS_Store" -delete

if [[ "$AGGRESSIVE" -eq 1 ]]; then
  # Optional: not needed on Pi runtime.
  rm -f Quant-M.zip
fi

# Keep expected directories present.
mkdir -p workspace/logs workspace/queue workspace/state

echo "pi_lean_cleanup: after"
du -sh ./* 2>/dev/null | sort -h
