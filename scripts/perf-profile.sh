#!/usr/bin/env bash
# perf-profile.sh — manual performance profiling orchestrator (Phase A).
#
# Local-only, loopback fixtures. Informational: never a merge gate.
# Normal `make check` does not invoke this script.
#
# Usage:
#   bash scripts/perf-profile.sh --suite all --trials 5 --warmup 1
#   bash scripts/perf-profile.sh --suite loadtest --trials 5 --warmup 1
#   bash scripts/perf-profile.sh --suite fanout --trials 3 --warmup 1
#   bash scripts/perf-profile.sh --suite distributed --trials 3 --warmup 1
#   bash scripts/perf-profile.sh --suite pool --trials 5 --warmup 1
#
# Suites: all | loadtest | fanout | distributed | session | pool
# Results stream as key=value / JSON lines; nothing is committed.
# Summaries belong in architecture/performance.md (medians/ranges, not raw logs).
set -euo pipefail
ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT"

SUITE="all"
TRIALS=5
WARMUP=1

while [[ $# -gt 0 ]]; do
  case "$1" in
    --suite) SUITE="${2:-all}"; shift 2 ;;
    --trials) TRIALS="${2:-5}"; shift 2 ;;
    --warmup) WARMUP="${2:-1}"; shift 2 ;;
    -h|--help)
      echo "Usage: $0 [--suite all|loadtest|fanout|distributed|pool] [--trials N] [--warmup N]"
      exit 0
      ;;
    *) echo "Unknown arg: $1" >&2; exit 2 ;;
  esac
done

export EGGSEC_PERF_TRIALS="$TRIALS"
export EGGSEC_PERF_WARMUP="$WARMUP"
export EGGSEC_ALLOW_LOOPBACK_FIXTURE=1

echo "perf-profile: suite=$SUITE trials=$TRIALS warmup=$WARMUP"
echo "perf-profile: host=$(uname -srm) cpus=$(nproc 2>/dev/null || echo unknown) rustc=$(rustc --version 2>/dev/null || echo unknown)"
echo "perf-profile: release mode required; results are informational (no merge gate)."

run_target() {
  local pkg="$1"; local target="$2"; local filter="$3"
  echo "--- target: $pkg :: $target [$filter] ---"
  if command -v /usr/bin/time >/dev/null 2>&1; then
    /usr/bin/time -v cargo test --release -p "$pkg" --test "$target" -- --ignored --nocapture "$filter" 2>&1 | tail -n 60 || true
  else
    cargo test --release -p "$pkg" --test "$target" -- --ignored --nocapture "$filter" 2>&1 | tail -n 60 || true
  fi
}

run_lib_target() {
  local pkg="$1"; local filter="$2"
  echo "--- target: $pkg :: lib [$filter] ---"
  if command -v /usr/bin/time >/dev/null 2>&1; then
    /usr/bin/time -v cargo test --release -p "$pkg" --lib -- "$filter" --nocapture 2>&1 | tail -n 40 || true
  else
    cargo test --release -p "$pkg" --lib -- "$filter" --nocapture 2>&1 | tail -n 40 || true
  fi
}

case "$SUITE" in
  loadtest|fanout|distributed)
    run_target "eggsec" "perf_baseline" "$SUITE"
    ;;
  session)
    run_lib_target "eggsec" "distributed::remote::session_tests"
    ;;
  pool)
    run_target "eggsec-web-proxy" "perf_pool_baseline" "pool"
    ;;
  all)
    run_target "eggsec" "perf_baseline" "loadtest"
    run_target "eggsec" "perf_baseline" "fanout"
    run_target "eggsec" "perf_baseline" "distributed"
    run_lib_target "eggsec" "distributed::remote::session_tests"
    run_target "eggsec-web-proxy" "perf_pool_baseline" "pool"
    ;;
  *)
    echo "Unknown suite: $SUITE (expected all|loadtest|fanout|distributed|session|pool)" >&2
    exit 2
    ;;
esac

echo "perf-profile: done. Record medians/ranges in architecture/performance.md; do not commit raw logs."
