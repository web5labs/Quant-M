#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

DURATION_SECONDS="${1:-30}"
JOBS="${2:-20}"
POLL_SECONDS="${3:-}"
BIN="./target/release/quant-m"

cargo build --release >/dev/null

BASE_DIR="$(mktemp -d /tmp/quantm-bench.XXXXXX)"
CFG="$BASE_DIR/quant-m.toml"
LOG="$BASE_DIR/worker.log"

cleanup() {
  if [[ -n "${PID:-}" ]]; then
    kill -INT "$PID" >/dev/null 2>&1 || true
    wait "$PID" >/dev/null 2>&1 || true
  fi
  rm -rf "$BASE_DIR"
}
trap cleanup EXIT

"$BIN" --config "$CFG" init >/dev/null
if [[ -n "$POLL_SECONDS" ]]; then
  TMP_CFG="$BASE_DIR/quant-m.override.toml"
  awk -v poll="$POLL_SECONDS" '
    BEGIN { replaced = 0 }
    /^poll_interval_seconds = / && replaced == 0 {
      print "poll_interval_seconds = " poll
      replaced = 1
      next
    }
    { print }
  ' "$CFG" >"$TMP_CFG"
  mv "$TMP_CFG" "$CFG"
fi
"$BIN" --config "$CFG" worker run >"$LOG" 2>&1 &
PID=$!
sleep 1

CPU_SUM=0
RSS_SUM=0
for _ in $(seq 1 "$DURATION_SECONDS"); do
  CPU="$(ps -o %cpu= -p "$PID" | tr -d ' ')"
  RSS="$(ps -o rss= -p "$PID" | tr -d ' ')"
  CPU_SUM="$(awk -v a="$CPU_SUM" -v b="$CPU" 'BEGIN{printf "%.6f", a+b}')"
  RSS_SUM=$((RSS_SUM + RSS))
  sleep 1
done

IDLE_CPU_AVG="$(awk -v s="$CPU_SUM" -v n="$DURATION_SECONDS" 'BEGIN{printf "%.4f", s/n}')"
IDLE_RSS_KB_AVG=$((RSS_SUM / DURATION_SECONDS))

OUTBOX="$BASE_DIR/workspace/queue/outbox.ndjson"
mkdir -p "$(dirname "$OUTBOX")"
: > "$OUTBOX"

SINGLE_START_MS="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"

"$BIN" --config "$CFG" worker submit "{\"kind\":\"echo\",\"text\":\"bench-single\"}" >/dev/null

for _ in $(seq 1 6000); do
  COUNT="$(wc -l < "$OUTBOX" | tr -d ' ')"
  if [[ "$COUNT" -ge 1 ]]; then
    break
  fi
  sleep 0.01
done

SINGLE_END_MS="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"

SINGLE_LAT_MS=$((SINGLE_END_MS - SINGLE_START_MS))
: > "$OUTBOX"

START_MS="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"

for i in $(seq 1 "$JOBS"); do
  "$BIN" --config "$CFG" worker submit "{\"kind\":\"echo\",\"text\":\"bench-$i\"}" >/dev/null
done

for _ in $(seq 1 6000); do
  COUNT="$(wc -l < "$OUTBOX" | tr -d ' ')"
  if [[ "$COUNT" -ge "$JOBS" ]]; then
    break
  fi
  sleep 0.01
done

END_MS="$(python3 - <<'PY'
import time
print(int(time.time() * 1000))
PY
)"

OUTBOX_COUNT="$(wc -l < "$OUTBOX" | tr -d ' ')"
TOTAL_MS=$((END_MS - START_MS))
AVG_PER_JOB_MS=$((TOTAL_MS / JOBS))
TASKS_PER_SEC="$(awk -v j="$JOBS" -v ms="$TOTAL_MS" 'BEGIN{if(ms==0){print "0.00"}else{printf "%.2f", (j*1000.0)/ms}}')"

echo "idle_duration_s=$DURATION_SECONDS"
if [[ -n "$POLL_SECONDS" ]]; then
  echo "worker_poll_interval_s=$POLL_SECONDS"
fi
echo "idle_avg_cpu_pct=$IDLE_CPU_AVG"
echo "idle_avg_rss_kb=$IDLE_RSS_KB_AVG"
echo "single_job_latency_ms=$SINGLE_LAT_MS"
echo "batch_jobs=$JOBS"
echo "batch_outbox_count=$OUTBOX_COUNT"
echo "batch_enqueue_to_completion_ms=$TOTAL_MS"
echo "batch_avg_ms_per_job=$AVG_PER_JOB_MS"
echo "batch_jobs_per_sec=$TASKS_PER_SEC"
