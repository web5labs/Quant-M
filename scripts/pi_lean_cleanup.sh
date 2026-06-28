#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

APPLY=0
AGGRESSIVE=0

for arg in "$@"; do
  case "$arg" in
    --apply)
      APPLY=1
      ;;
    --dry-run)
      APPLY=0
      ;;
    --aggressive)
      AGGRESSIVE=1
      ;;
    *)
      echo "usage: $0 [--dry-run] [--apply] [--aggressive]" >&2
      exit 2
      ;;
  esac
done

run_or_print() {
  if [[ "$APPLY" -eq 1 ]]; then
    "$@"
  else
    printf "would run:"
    printf " %q" "$@"
    printf "\n"
  fi
}

echo "pi_lean_cleanup: root=$ROOT_DIR mode=$([[ "$APPLY" -eq 1 ]] && echo apply || echo dry-run) aggressive=$AGGRESSIVE"
echo
echo "Current repo storage:"
du -sh ./* 2>/dev/null | sort -h || true
echo

echo "Repo-local cleanup candidates"
run_or_print rm -rf target
run_or_print find . -name ".DS_Store" -delete

if [[ -d workspace/logs ]]; then
  run_or_print find workspace/logs -type f -delete
fi
if [[ -d workspace/queue ]]; then
  run_or_print find workspace/queue -type f -delete
fi

if [[ "$AGGRESSIVE" -eq 1 ]]; then
  run_or_print rm -f Quant-M.zip
fi

if [[ "$APPLY" -eq 1 ]]; then
  mkdir -p workspace/logs workspace/queue workspace/state
  echo
  echo "After cleanup:"
  du -sh ./* 2>/dev/null | sort -h || true
else
  cat <<'EOF'

Dry run only. Re-run with:
  bash scripts/pi_lean_cleanup.sh --apply

This script removes repo-local build/runtime churn only. It does not purge apt
packages such as git, curl, openssh-server, cargo, rustc, gcc, or libc headers.
EOF
fi
