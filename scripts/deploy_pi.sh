#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "usage: scripts/deploy_pi.sh <user@host> <remote_repo_dir> [run_user]" >&2
  echo "example: scripts/deploy_pi.sh pi@192.168.1.50 /home/pi/Quant-M pi" >&2
  exit 2
fi

TARGET="$1"
REMOTE_REPO_DIR="$2"
RUN_USER="${3:-}"
FAST_BUILD="${QUANT_M_FAST_BUILD:-1}"

if [[ -z "$RUN_USER" ]]; then
  RUN_USER="$(id -un)"
fi

echo "deploy_pi: target=$TARGET repo=$REMOTE_REPO_DIR run_user=$RUN_USER fast_build=$FAST_BUILD"

ssh "$TARGET" \
  REPO_DIR="$REMOTE_REPO_DIR" \
  RUN_USER="$RUN_USER" \
  FAST_BUILD="$FAST_BUILD" \
  'bash -se' <<'REMOTE'
set -euo pipefail

cd "$REPO_DIR"
mkdir -p workspace/logs workspace/locks workspace/state

if [[ ! -x ./target/release/quant-m ]]; then
  if [[ "${FAST_BUILD:-1}" == "1" ]]; then
    CARGO_PROFILE_RELEASE_LTO=off \
    CARGO_PROFILE_RELEASE_CODEGEN_UNITS=16 \
    cargo build --release -q
  else
    cargo build --release -q
  fi
fi

TMP_UNIT=/tmp/quant-m.service
sed "s|__QUANTM_USER__|$RUN_USER|g; s|__QUANTM_DIR__|$REPO_DIR|g" \
  "$REPO_DIR/configs/systemd/quant-m.service" > "$TMP_UNIT"
sudo install -m 0644 "$TMP_UNIT" /etc/systemd/system/quant-m.service

sudo systemctl daemon-reload
sudo systemctl enable --now quant-m

TMP_CRON=/tmp/quant-m.cron
sed "s|__QUANTM_DIR__|$REPO_DIR|g" \
  "$REPO_DIR/configs/cron/quant-m.cron" > "$TMP_CRON"
crontab "$TMP_CRON"

./scripts/healthcheck.sh USDJPY >/dev/null
./target/release/quant-m --config ./quant-m.toml state macro-refresh-mql5 --hours-ahead 48 >/dev/null

echo "---- systemd status ----"
sudo systemctl status --no-pager quant-m | sed -n '1,40p'
echo "---- crontab ----"
crontab -l
echo "deploy_ok=1"
REMOTE
