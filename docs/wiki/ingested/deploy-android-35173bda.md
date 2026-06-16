# Ingested Wiki Source: deploy-android.md

## Metadata

- Source path: `wiki/raw/project/deploy-android.md`
- Ingested at: `2026-05-31T16:18:00.932730+00:00`
- Source extension: `.md`

## Agent summary

_TBD: Summarize the source in 5-10 bullets._

## Key facts

_TBD_

## Implementation relevance

_TBD: Explain how this source affects the project spec, architecture, data model, API plan, or UI/UX handoff._

## Risks / constraints

_TBD_

## Open questions

_TBD_

## Source excerpt

```text
# Deploy Quant-M to Android (Termux + SSH)

## 1) Build for Android target

Example targets:
- `aarch64-linux-android`
- `armv7-linux-androideabi`

```bash
rustup target add aarch64-linux-android
cargo build --release --target aarch64-linux-android
```

If your NDK toolchain is configured, the binary will be at:

- `target/aarch64-linux-android/release/quant-m`

## 2) Copy to Termux node

```bash
scp target/aarch64-linux-android/release/quant-m \
  user@android-node:/data/data/com.termux/files/home/bin/quant-m
```

## 3) Initialize on node

```bash
ssh user@android-node
chmod +x ~/bin/quant-m
mkdir -p ~/quant-m-node
cd ~/quant-m-node
~/bin/quant-m init
~/bin/quant-m status
```

## 4) Start worker loop

```bash
~/bin/quant-m worker run
```

Or daemon mode:

```bash
~/bin/quant-m daemon start
```

## 5) Coordinator pattern

From coordinator machine:

```bash
ssh user@android-node 'cd ~/quant-m-node && ~/bin/quant-m worker submit '\''{"kind":"shell","command":"uptime"}'\'''
```

Then fetch results:

```bash
ssh user@android-node 'cd ~/quant-m-node && tail -n 20 workspace/queue/outbox.ndjson'
```

## 6) Safety defaults

Quant-M defaults are conservative for older Android devices:

- concurrency `1`
- polling interval `30s`
- command timeout `60s`
- bounded retries
- log rotation enabled

Tune these in `quant-m.toml` as needed.

If you need command execution or HTTP fetch jobs, explicitly enable:

- `worker.allow_shell_commands = true`
- `worker.allow_http_get = true`
- `skills.allow_shell_commands = true`

If you need Telegram + OpenRouter LLM:

- `llm.enabled = true`
- export `OPENROUTER_API_KEY=...`
- `telegram.enabled = true`
- set `telegram.bot_token` (or export `TELEGRAM_BOT_TOKEN=...`)
```
