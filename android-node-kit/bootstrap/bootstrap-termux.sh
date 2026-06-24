#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail

log() { printf '[%s] %s\n' "$(date '+%F %T')" "$*"; }
apt_safe() { apt -o Dpkg::Options::="--force-confold" "$@"; }

ANDROID_SDK="$(getprop ro.build.version.sdk 2>/dev/null || true)"
if [ -z "${OFFLINE_REPO:-}" ]; then
  if [ -n "$ANDROID_SDK" ] && [ "$ANDROID_SDK" -le 23 ] && [ -d "/sdcard/Download/quant-m-edge-bundle/offline/termux-main-21" ]; then
    OFFLINE_REPO="/sdcard/Download/quant-m-edge-bundle/offline/termux-main-21"
  else
    OFFLINE_REPO="/sdcard/Download/quant-m-edge-bundle/offline/termux-main"
  fi
fi
if [ -d "$OFFLINE_REPO/dists/stable/main" ]; then
  log "Offline Termux repo found: $OFFLINE_REPO"
  mkdir -p "$PREFIX/etc/apt/sources.list.d"
  printf 'deb [trusted=yes] file://%s stable main\n' "$OFFLINE_REPO" > "$PREFIX/etc/apt/sources.list.d/quant-m-offline.list"
else
  log "Offline Termux repo not found; using configured package sources."
fi

log "Updating package index..."
apt_safe update -y

log "Installing base packages..."
apt_safe install -y openssh git curl termux-tools termux-api rust

if command -v cargo >/dev/null 2>&1; then
  log "Cargo available: $(cargo --version)"
else
  log "Cargo was not found after installing rust; install the Termux rust package manually."
fi

if command -v termux-battery-status >/dev/null 2>&1; then
  log "Termux:API CLI available."
else
  log "Termux:API CLI not found; install the Termux:API app APK and package."
fi

for api_cmd in termux-camera-photo termux-microphone-record termux-open-url; do
  if command -v "$api_cmd" >/dev/null 2>&1; then
    log "Peripheral helper available: $api_cmd"
  else
    log "Peripheral helper missing: $api_cmd"
  fi
done

log "Trying optional rsync..."
if apt_safe install -y rsync; then
  log "rsync installed."
else
  log "rsync not available on this device; use scp fallback."
fi

log "Creating node directories..."
mkdir -p "$HOME/node/bin" "$HOME/node/config" "$HOME/node/logs" "$HOME/node/tmp"
mkdir -p "$HOME/node/bundle" "$HOME/quant-m-node/bin"
mkdir -p "$HOME/.ssh"
chmod 700 "$HOME/.ssh"

cat > "$HOME/node/bin/start-sshd.sh" <<'EOF'
#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail
sshd
echo "sshd started on port 8022"
EOF
chmod +x "$HOME/node/bin/start-sshd.sh"

cat > "$HOME/node/bin/node-info.sh" <<'EOF'
#!/data/data/com.termux/files/usr/bin/bash
set -euo pipefail
USER_NAME="$(whoami)"
IP=""
if command -v ifconfig >/dev/null 2>&1; then
  IP="$(ifconfig 2>/dev/null | awk '/^wlan0:/{w=1; next} w && /inet / {print $2; exit}' || true)"
fi
if [ -z "${IP}" ] && command -v ip >/dev/null 2>&1; then
  IP="$(ip -o -4 addr show scope global 2>/dev/null | awk '!/ lo / {print $4}' | cut -d/ -f1 | head -n1 || true)"
fi
echo "user=${USER_NAME}"
echo "ip=${IP:-unknown}"
echo "ssh_port=8022"
EOF
chmod +x "$HOME/node/bin/node-info.sh"

if command -v termux-wake-lock >/dev/null 2>&1; then
  termux-wake-lock || true
fi

log "Bootstrap complete."
log "Run next:"
log "1) passwd"
log "2) ~/node/bin/start-sshd.sh"
log "3) ~/node/bin/node-info.sh"
