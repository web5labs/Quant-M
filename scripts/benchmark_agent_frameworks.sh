#!/usr/bin/env bash
set -euo pipefail

# Static benchmark scorer for heterogeneous agent frameworks.
# Usage:
#   ./scripts/benchmark_agent_frameworks.sh <repo1> <repo2> ...
# If no repos are provided, set QUANTM_BENCHMARK_REPOS to a colon-separated
# list of local checkout paths.

if [ "$#" -gt 0 ]; then
  REPOS=("$@")
elif [ -n "${QUANTM_BENCHMARK_REPOS:-}" ]; then
  IFS=':' read -r -a REPOS <<< "$QUANTM_BENCHMARK_REPOS"
else
  echo "usage: $0 <repo1> <repo2> ..." >&2
  echo "or set QUANTM_BENCHMARK_REPOS=/path/to/repo1:/path/to/repo2" >&2
  exit 2
fi

if ! command -v rg >/dev/null 2>&1; then
  echo "error: rg is required" >&2
  exit 1
fi

IGNORE_DIRS=(
  ".git"
  "target"
  "node_modules"
  ".venv"
  "venv"
  "__pycache__"
  "dist"
  "build"
  "workspace"
  ".next"
  "out"
  ".cache"
)

RG_IGNORE_ARGS=()
for d in "${IGNORE_DIRS[@]}"; do
  RG_IGNORE_ARGS+=("-g" "!${d}/**")
  RG_IGNORE_ARGS+=("-g" "!**/${d}/**")
done

filtered_file_count_for_repo() {
  local repo="$1"
  rg --files "${RG_IGNORE_ARGS[@]}" "$repo" | wc -l | tr -d ' '
}

filtered_tests_count_for_repo() {
  local repo="$1"
  (rg --files "${RG_IGNORE_ARGS[@]}" "$repo" | rg '(^|/)(tests?/|test_|.*_test\.|.*\.test\.|.*\.spec\.)' -N || true) | wc -l | tr -d ' '
}

filtered_size_mb_for_repo() {
  local repo="$1"
  local bytes=0
  local f size
  while IFS= read -r -d '' f; do
    size="$(stat -f%z "$f" 2>/dev/null || echo 0)"
    bytes=$((bytes + size))
  done < <(
    find "$repo" \
      \( \
        -name .git -o \
        -name target -o \
        -name node_modules -o \
        -name .venv -o \
        -name venv -o \
        -name __pycache__ -o \
        -name dist -o \
        -name build -o \
        -name workspace -o \
        -name .next -o \
        -name out -o \
        -name .cache \
      \) -type d -prune -o -type f -print0
  )
  awk -v b="$bytes" 'BEGIN { printf "%d", (b + 1048575) / 1048576 }'
}

dep_count_for_repo() {
  local repo="$1"
  local total=0
  local c=0

  if [ -f "$repo/Cargo.toml" ] && [ -f "$repo/Cargo.lock" ]; then
    c="$( (rg '^name = "' "$repo/Cargo.lock" || true) | wc -l | tr -d ' ' )"
    total=$((total + c))
  fi

  if [ -f "$repo/go.mod" ]; then
    c="$(awk '
      BEGIN { inreq = 0; c = 0 }
      /^require \(/ { inreq = 1; next }
      inreq && /^\)/ { inreq = 0; next }
      inreq && $1 !~ /^\/\// { c++ }
      !inreq && /^require / { c++ }
      END { print c }
    ' "$repo/go.mod")"
    total=$((total + c))
  fi

  if [ -f "$repo/package.json" ]; then
    if command -v jq >/dev/null 2>&1; then
      c="$(jq '((.dependencies // {}) + (.devDependencies // {})) | length' "$repo/package.json" 2>/dev/null || echo "0")"
    else
      # Fallback approximate count if jq is not available.
      c="$(rg -n '"(dependencies|devDependencies)"' "$repo/package.json" >/dev/null 2>&1 && echo "1" || echo "0")"
    fi
    total=$((total + c))
  fi

  if [ -f "$repo/requirements.txt" ]; then
    c="$( (rg -v '^\s*#|^\s*$' "$repo/requirements.txt" || true) | wc -l | tr -d ' ' )"
    total=$((total + c))
  fi

  if [ -f "$repo/pyproject.toml" ]; then
    c="$(awk '
      BEGIN { in_project = 0; in_deps = 0; c = 0 }
      /^\[project\]/ { in_project = 1; next }
      /^\[/ && $0 !~ /^\[project\]/ { in_project = 0; in_deps = 0 }
      in_project && /^\s*dependencies\s*=\s*\[/ { in_deps = 1; next }
      in_deps {
        if ($0 ~ /\]/) { in_deps = 0; next }
        if ($0 ~ /".*"/) c++
      }
      END { print c }
    ' "$repo/pyproject.toml")"
    total=$((total + c))
  fi

  echo "$total"
}

lang_for_repo() {
  local repo="$1"
  local has_rust=0
  local has_go=0
  local has_py=0
  local has_node=0

  [ -f "$repo/Cargo.toml" ] && has_rust=1
  [ -f "$repo/go.mod" ] && has_go=1
  if [ -f "$repo/pyproject.toml" ] || [ -f "$repo/requirements.txt" ]; then
    has_py=1
  fi
  [ -f "$repo/package.json" ] && has_node=1

  if [ "$has_rust" -eq 1 ] && [ "$has_go" -eq 1 ]; then
    echo "rust+go"
  elif [ "$has_rust" -eq 1 ] && [ "$has_py" -eq 1 ]; then
    echo "rust+python"
  elif [ "$has_rust" -eq 1 ]; then
    echo "rust"
  elif [ "$has_go" -eq 1 ]; then
    echo "go"
  elif [ "$has_py" -eq 1 ] && [ "$has_node" -eq 1 ]; then
    echo "python+node"
  elif [ "$has_py" -eq 1 ]; then
    echo "python"
  elif [ "$has_node" -eq 1 ]; then
    echo "node"
  else
    echo "mixed"
  fi
}

integration_hits_for_repo() {
  local repo="$1"
  local hits=0
  local k
  for k in openai anthropic openrouter ollama gemini mistral groq cohere huggingface llama vllm webhook telegram discord slack mcp sqlite postgres mysql redis docker kubernetes browser automation cron; do
    if rg -i -q "${RG_IGNORE_ARGS[@]}" "$k" "$repo"; then
      hits=$((hits + 1))
    fi
  done
  echo "$hits"
}

scalability_hits_for_repo() {
  local repo="$1"
  local hits=0
  local k
  for k in 'tokio::spawn' 'JoinSet' 'Semaphore' 'async fn' 'worker' 'queue' 'scheduler' 'cron' 'retry' 'backoff' 'rate limit' 'multiprocess' 'threadpool' 'goroutine' 'context\.Context' 'channel' 'pubsub' 'kafka' 'rabbitmq' 'sqs'; do
    if rg -i -q "${RG_IGNORE_ARGS[@]}" "$k" "$repo"; then
      hits=$((hits + 1))
    fi
  done
  echo "$hits"
}

clamp_0_10() {
  awk -v v="$1" 'BEGIN { if (v < 0) v = 0; if (v > 10) v = 10; printf "%.2f", v }'
}

base_speed_for_lang() {
  case "$1" in
    rust) echo "8.2" ;;
    go) echo "7.7" ;;
    python+node) echo "5.3" ;;
    python) echo "5.5" ;;
    node) echo "6.2" ;;
    *) echo "5.8" ;;
  esac
}

echo "repo|lang|size_mb|files|workflows|tests|bench_dirs|fuzz_dirs|deps|integration_hits|scalability_hits|robustness|speed|integrations|scalability|overall"
for repo in "${REPOS[@]}"; do
  if [ ! -d "$repo" ]; then
    echo "warning: missing repo: $repo" >&2
    continue
  fi

  name="$(basename "$repo")"
  lang="$(lang_for_repo "$repo")"
  size_mb="$(filtered_size_mb_for_repo "$repo")"
  files="$(filtered_file_count_for_repo "$repo")"
  if [ -d "$repo/.github/workflows" ]; then
    workflows="$(find "$repo/.github/workflows" -type f 2>/dev/null | wc -l | tr -d ' ')"
  else
    workflows="0"
  fi
  tests="$(filtered_tests_count_for_repo "$repo")"
  bench_dirs="$(find "$repo" -maxdepth 3 -type d \( -name bench -o -name benches -o -name benchmark -o -name benchmarks \) 2>/dev/null | wc -l | tr -d ' ')"
  fuzz_dirs="$(find "$repo" -maxdepth 3 -type d -name fuzz 2>/dev/null | wc -l | tr -d ' ')"
  deps="$(dep_count_for_repo "$repo")"
  integ_hits="$(integration_hits_for_repo "$repo")"
  scale_hits="$(scalability_hits_for_repo "$repo")"

  robustness_raw="$(awk -v w="$workflows" -v t="$tests" -v b="$bench_dirs" -v f="$fuzz_dirs" 'BEGIN { print (w * 0.7) + (t * 0.01) + (b * 1.2) + (f * 1.5) }')"
  robustness="$(clamp_0_10 "$robustness_raw")"

  base_speed="$(base_speed_for_lang "$lang")"
  size_penalty="$(awk -v s="$size_mb" 'BEGIN { if (s > 100) print 2.0; else if (s > 50) print 1.0; else print 0.0 }')"
  dep_penalty="0.0"
  if [ "$deps" -gt 600 ] 2>/dev/null; then
    dep_penalty="1.0"
  elif [ "$deps" -gt 200 ] 2>/dev/null; then
    dep_penalty="0.5"
  fi
  speed_raw="$(awk -v b="$base_speed" -v bd="$bench_dirs" -v sp="$size_penalty" -v dp="$dep_penalty" 'BEGIN { print b + (bd * 0.6) - sp - dp }')"
  speed="$(clamp_0_10 "$speed_raw")"

  integrations_raw="$(awk -v i="$integ_hits" 'BEGIN { print i / 2.4 }')"
  integrations="$(clamp_0_10 "$integrations_raw")"

  scalability_raw="$(awk -v s="$scale_hits" -v f="$fuzz_dirs" 'BEGIN { print (s / 1.8) + (f * 0.3) }')"
  scalability="$(clamp_0_10 "$scalability_raw")"

  overall_raw="$(awk -v r="$robustness" -v sp="$speed" -v i="$integrations" -v sc="$scalability" 'BEGIN { print (r * 0.35) + (sp * 0.25) + (i * 0.20) + (sc * 0.20) }')"
  overall="$(clamp_0_10 "$overall_raw")"

  echo "$name|$lang|$size_mb|$files|$workflows|$tests|$bench_dirs|$fuzz_dirs|$deps|$integ_hits|$scale_hits|$robustness|$speed|$integrations|$scalability|$overall"
done
