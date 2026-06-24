#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

log() { printf '[%s] %s\n' "$(date '+%F %T')" "$*"; }

BUNDLE_DIR="${BUNDLE_DIR:-/sdcard/Download/quant-m-edge-bundle}"
SOURCE_LIST="$PREFIX/etc/apt/sources.list.d/quant-m-offline.list"
PACKAGES_FILE="$BUNDLE_DIR/termux-packages.txt"
ANDROID_SDK="$(getprop ro.build.version.sdk 2>/dev/null || true)"

if [ -z "${OFFLINE_REPO:-}" ]; then
  if [ -n "$ANDROID_SDK" ] && [ "$ANDROID_SDK" -le 23 ] && [ -d "$BUNDLE_DIR/offline/termux-main-21" ]; then
    OFFLINE_REPO="$BUNDLE_DIR/offline/termux-main-21"
  else
    OFFLINE_REPO="$BUNDLE_DIR/offline/termux-main"
  fi
fi

if [ ! -d "$OFFLINE_REPO/dists/stable/main" ]; then
  echo "Offline repo not found: $OFFLINE_REPO" >&2
  exit 1
fi

mkdir -p "$PREFIX/etc/apt/sources.list.d"

for list in "$PREFIX/etc/apt/sources.list" "$PREFIX/etc/apt/sources.list.d"/*.list; do
  [ -f "$list" ] || continue
  case "$list" in
    *quant-m-offline.list|*.disabled-by-quant-m) continue ;;
  esac
  mv "$list" "$list.disabled-by-quant-m" || true
done

cat > "$SOURCE_LIST" <<EOF
deb [trusted=yes] file://$OFFLINE_REPO stable main
EOF

log "Updating apt from offline repo: $OFFLINE_REPO"
apt update

mapfile -t PACKAGE_NAMES < "$PACKAGES_FILE"
log "Installing runtime packages: ${PACKAGE_NAMES[*]}"
apt install -y "${PACKAGE_NAMES[@]}"

log "Creating Quant-M runtime directories..."
mkdir -p "$HOME/node/bin" "$HOME/node/config" "$HOME/node/logs" "$HOME/node/tmp"
mkdir -p "$HOME/node/bundle" "$HOME/quant-m-node/bin" "$HOME/quant-m-node/workspace"
mkdir -p "$HOME/.ssh"
chmod 700 "$HOME/.ssh"

for cmd in ssh sshd curl termux-battery-status termux-camera-photo termux-microphone-record termux-open-url; do
  if command -v "$cmd" >/dev/null 2>&1; then
    log "Available: $cmd"
  else
    log "Missing: $cmd"
  fi
done

log "Base runtime install complete."
