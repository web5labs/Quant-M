# Deploy QUAN-M with systemd (SSH)

This is the leanest path to run QUAN-M as a background service on Linux (Raspberry Pi, VPS, mini PC).

## 1) Build on remote host

```bash
ssh <user>@<host> '
set -e
cd /path/to/Quant-M
cargo build --release -q
./target/release/quant-m --config ./quant-m.toml status
'
```

## 2) Install the service unit

```bash
ssh <user>@<host> '
set -e
REPO_DIR=/path/to/Quant-M
RUN_USER=<user>
TMP_UNIT=/tmp/quant-m.service

sed "s|__QUANTM_USER__|$RUN_USER|g; s|__QUANTM_DIR__|$REPO_DIR|g" \
  "$REPO_DIR/configs/systemd/quant-m.service" > "$TMP_UNIT"

sudo install -m 0644 "$TMP_UNIT" /etc/systemd/system/quant-m.service
sudo systemctl daemon-reload
sudo systemctl enable --now quant-m
'
```

## 3) Verify service

```bash
ssh <user>@<host> '
sudo systemctl status --no-pager quant-m
sudo journalctl -u quant-m -n 80 --no-pager
'
```

## 4) Quick runtime smoke

```bash
ssh <user>@<host> '
cd /path/to/Quant-M
./target/release/quant-m --config ./quant-m.toml state init
./target/release/quant-m --config ./quant-m.toml state summary
'
```

## 5) Install cron hardening schedule

```bash
ssh <user>@<host> '
set -e
REPO_DIR=/path/to/Quant-M
TMP_CRON=/tmp/quant-m.cron

sed "s|__QUANTM_DIR__|$REPO_DIR|g" \
  "$REPO_DIR/configs/cron/quant-m.cron" > "$TMP_CRON"

crontab "$TMP_CRON"
crontab -l
'
```

This schedule includes:

- `swap_health` daily with a retry buffer
- hourly macro refresh for 48h horizon (weekdays)
- daily macro refresh for 7d horizon
- 15-minute local health checks

## 6) Optional secrets file

If you use OpenRouter or Telegram tokens, add them in:

- `/etc/default/quant-m`

Example:

```bash
OPENROUTER_API_KEY=your_key_here
TELEGRAM_BOT_TOKEN=your_token_here
```

Then restart:

```bash
ssh <user>@<host> 'sudo systemctl restart quant-m'
```
