#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

MIN_FREE_MB="${QUANT_M_VALIDATE_MIN_FREE_MB:-2048}"
CLEAN_ON_LOW_DISK=0
SKIP_TESTS=0

usage() {
  cat <<'EOF'
usage: scripts/validate_integrity.sh [--clean-on-low-disk] [--skip-tests]

Runs the Quant-M Rust, Serde, JSON, SQLite, and onboarding integrity loop.

Options:
  --clean-on-low-disk  Run cargo clean if free disk is below the threshold.
  --skip-tests         Skip cargo test; useful when doing a docs-only quick pass.

Environment:
  QUANT_M_VALIDATE_MIN_FREE_MB  Minimum free MB before tests prefer lean mode.
                                Default: 2048.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --clean-on-low-disk)
      CLEAN_ON_LOW_DISK=1
      shift
      ;;
    --skip-tests)
      SKIP_TESTS=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

free_mb() {
  df -Pm . | awk 'NR == 2 { print $4 }'
}

section() {
  printf '\n==> %s\n' "$1"
}

section "workspace"
echo "root=$ROOT_DIR"
echo "min_free_mb=$MIN_FREE_MB"

FREE_MB="$(free_mb)"
echo "free_mb=$FREE_MB"

if [[ "$FREE_MB" -lt "$MIN_FREE_MB" ]]; then
  echo "warning: low disk space before validation"
  if [[ "$CLEAN_ON_LOW_DISK" -eq 1 ]]; then
    section "cargo clean"
    cargo clean
    FREE_MB="$(free_mb)"
    echo "free_mb_after_clean=$FREE_MB"
  else
    echo "hint: rerun with --clean-on-low-disk to purge generated Cargo artifacts"
  fi
fi

section "onboarding lint"
python3 scripts/lint_project_onboarding.py --target .

section "cargo fmt"
cargo fmt --all -- --check

section "cargo clippy"
cargo clippy --all-targets -- -D warnings

section "serde/json scan"
rg -n "serde_json::Value|from_str::<|from_value|to_value|json!" src docs README.md Cargo.toml || true

section "sqlite scan"
rg -n "rusqlite|Connection|execute\\(|query_map|prepare\\(" src || true

if [[ "$SKIP_TESTS" -eq 1 ]]; then
  section "cargo test"
  echo "skipped by --skip-tests"
else
  section "cargo test"
  FREE_MB="$(free_mb)"
  echo "free_mb_before_tests=$FREE_MB"
  if [[ "$FREE_MB" -lt "$MIN_FREE_MB" ]]; then
    echo "low disk detected; running lean cargo test"
    CARGO_INCREMENTAL=0 RUSTFLAGS="${RUSTFLAGS:-} -Cdebuginfo=0" cargo test
  else
    cargo test
  fi
fi

section "done"
echo "integrity validation passed"
