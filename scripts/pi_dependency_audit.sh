#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

have() {
  command -v "$1" >/dev/null 2>&1
}

print_cmd() {
  local label="$1"
  local cmd="$2"
  if have "$cmd"; then
    printf "%-18s %s\n" "$label:" "$(command -v "$cmd")"
    "$cmd" --version 2>/dev/null | head -n 1 || true
  else
    printf "%-18s missing\n" "$label:"
  fi
}

pkg_status() {
  local pkg="$1"
  if have dpkg && dpkg -s "$pkg" >/dev/null 2>&1; then
    printf "installed"
  else
    printf "not-installed"
  fi
}

echo "Quant-M Pi dependency audit"
echo "repo: $ROOT_DIR"
echo

echo "Host"
uname -a
if [[ -r /proc/device-tree/model ]]; then
  printf "model: "
  tr -d '\0' < /proc/device-tree/model
  echo
fi
echo

echo "Command paths"
print_cmd "git" git
print_cmd "curl" curl
print_cmd "ssh" ssh
print_cmd "sshd" sshd
print_cmd "rustup" rustup
print_cmd "cargo" cargo
print_cmd "rustc" rustc
print_cmd "gcc" gcc
echo

echo "Debian package status"
for pkg in openssh-server curl git cargo rustc libstd-rust-1.85 libstd-rust-dev gcc libc6-dev pkg-config; do
  printf "%-22s %s\n" "$pkg" "$(pkg_status "$pkg")"
done
echo

echo "Quant-M binaries"
if [[ -x target/debug/quant-m ]]; then
  printf "core debug:          "
  wc -c target/debug/quant-m
else
  echo "core debug:          missing"
fi
if [[ -x target/release/quant-m ]]; then
  printf "core release:        "
  wc -c target/release/quant-m
else
  echo "core release:        missing"
fi
if [[ -x target/release-child/quant-m-child ]]; then
  printf "child release-child: "
  wc -c target/release-child/quant-m-child
else
  echo "child release-child: missing"
fi
echo

echo "Repo storage"
du -sh . 2>/dev/null || true
du -sh target 2>/dev/null || true
du -sh workspace 2>/dev/null || true
echo

echo "Interpretation"
cat <<'EOF'
- git: needed to clone/pull source updates; not needed to run an existing binary.
- curl: needed to install/update rustup; not needed to run an existing binary.
- ssh/sshd: needed for remote operator access; Quant-M runtime does not require SSH.
- cargo/rustc/rustup: needed to build/update from source; not needed to run an existing binary.
- gcc/libc headers/pkg-config: build-time native dependency support; not child runtime authority.
- child-min runtime should not use provider adapters, model router, SQLite/redb, reqwest, tokio, QR image libraries, execution adapters, trading, or betting.

Safe next step:
  bash scripts/pi_lean_cleanup.sh --dry-run

Optional after binaries are tested and rustup is active:
  apt purge cargo rustc libstd-rust-1.85 libstd-rust-dev
  apt autoremove --purge
EOF
