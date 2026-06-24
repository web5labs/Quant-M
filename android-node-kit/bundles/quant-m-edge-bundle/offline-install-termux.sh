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
  echo "Push android-node-kit/bundles/quant-m-edge-bundle to /sdcard/Download/ first." >&2
  exit 1
fi

mkdir -p "$PREFIX/etc/apt/sources.list.d"

if [ ! -f "$PREFIX/etc/apt/sources.list.d/termux-main.list.disabled-by-quant-m" ]; then
  for list in "$PREFIX/etc/apt/sources.list" "$PREFIX/etc/apt/sources.list.d"/*.list; do
    [ -f "$list" ] || continue
    case "$list" in
      *quant-m-offline.list) continue ;;
    esac
    mv "$list" "$list.disabled-by-quant-m" || true
  done
fi

cat > "$SOURCE_LIST" <<EOF
deb [trusted=yes] file://$OFFLINE_REPO stable main
EOF

log "Updating apt from offline repo..."
apt update

if [ -f "$PACKAGES_FILE" ]; then
  mapfile -t PACKAGE_NAMES < "$PACKAGES_FILE"
else
  PACKAGE_NAMES=(openssh git curl termux-tools termux-api rust rsync)
fi

log "Installing offline packages: ${PACKAGE_NAMES[*]}"
apt install -y "${PACKAGE_NAMES[@]}"

log "Creating node directories..."
mkdir -p "$HOME/node/bin" "$HOME/node/config" "$HOME/node/logs" "$HOME/node/tmp"
mkdir -p "$HOME/node/bundle" "$HOME/quant-m-node/bin"
mkdir -p "$HOME/.ssh"
chmod 700 "$HOME/.ssh"

if command -v cargo >/dev/null 2>&1; then
  log "Cargo available: $(cargo --version)"
fi

for api_cmd in termux-battery-status termux-camera-photo termux-microphone-record termux-open-url; do
  if command -v "$api_cmd" >/dev/null 2>&1; then
    log "Available: $api_cmd"
  else
    log "Missing: $api_cmd"
  fi
done

log "Offline Termux dependency install complete."
