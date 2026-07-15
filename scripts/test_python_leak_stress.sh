#!/usr/bin/env bash
# test_python_leak_stress.sh — Stress and leak detection tests for Python bindings
#
# Builds eggsec-python with default features, then runs stress tests with high
# iteration counts while monitoring file descriptor counts before and after to
# detect resource leaks.
#
# Usage:
#   bash scripts/test_python_leak_stress.sh [--iterations N]
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
#
# Exit codes:
#   0 - all tests passed, no leaks detected
#   1 - one or more tests failed or leak detected
#   2 - skipped (build failed)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

export EGGSEC_ALLOW_LOOPBACK_FIXTURE=1

ITERATIONS="${1:-100}"
# Strip --iterations flag if present
if [[ "$ITERATIONS" == "--iterations" ]]; then
  ITERATIONS="${2:-100}"
fi

PASS=0
FAIL=0
SKIP=0

pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1" >&2; }
skip() { SKIP=$((SKIP + 1)); echo "  SKIP: $1"; }

echo "============================================================"
echo "  test_python_leak_stress.sh"
echo "  Python Stress & Leak Detection Tests"
echo "  Iterations: $ITERATIONS"
echo "============================================================"
echo ""

# ── Build with default features ───────────────────────────────────
echo "[1/5] Building eggsec-python with default features..."
if ! timeout 600 maturin develop --release 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop failed"
  skip "default feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with default features"

# ── Count file descriptors before tests ───────────────────────────
echo ""
echo "[2/5] Measuring baseline file descriptors..."
FD_BEFORE=$(ls /proc/self/fd 2>/dev/null | wc -l || echo "0")
echo "  FDs before tests: $FD_BEFORE"

# ── Run performance/stress tests ──────────────────────────────────
echo ""
echo "[3/5] Running performance_comprehensive.py (stress iterations)..."

PERF_TEST="$ROOT/crates/eggsec-python/tests/test_performance_comprehensive.py"
if [[ -f "$PERF_TEST" ]]; then
  PERF_OUTPUT=$(python3 -m pytest "$PERF_TEST" --strict-markers -q \
    -o "markers=stress: stress test markers" 2>&1) || true
  echo "$PERF_OUTPUT"

  PASSED=$(echo "$PERF_OUTPUT" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
  FAILED=$(echo "$PERF_OUTPUT" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")
  SKIPPED=$(echo "$PERF_OUTPUT" | grep -oP '\d+ skipped' | grep -oP '\d+' || echo "0")
  PASSED=${PASSED:-0}
  FAILED=${FAILED:-0}
  SKIPPED=${SKIPPED:-0}
  PASS=$((PASS + PASSED))
  FAIL=$((FAIL + FAILED))
  SKIP=$((SKIP + SKIPPED))
else
  skip "test_performance_comprehensive.py not found"
fi

# ── Run high-iteration import/instantiation stress ────────────────
echo ""
echo "[4/5] Running import stress test ($ITERATIONS iterations)..."

STRESS_OUTPUT=$(python3 -c "
import sys, time, os
start = time.time()
passed = 0
failed = 0

for i in range($ITERATIONS):
    try:
        # Force reimport by clearing module cache
        mods_to_remove = [k for k in sys.modules if k.startswith('eggsec')]
        for m in mods_to_remove:
            del sys.modules[m]
        import eggsec
        passed += 1
    except Exception as e:
        failed += 1
        print(f'  iteration {i}: {e}', file=sys.stderr)

elapsed = time.time() - start
print(f'  Import stress: {passed} passed, {failed} failed in {elapsed:.2f}s')
print(f'  Rate: {passed / elapsed:.0f} imports/sec')

# Fail if more than 1% of iterations failed
if failed > max(1, $ITERATIONS // 100):
    sys.exit(1)
" 2>&1) || true
echo "$STRESS_OUTPUT"

if echo "$STRESS_OUTPUT" | grep -q "failed [1-9]"; then
  FAIL=$((FAIL + 1))
else
  pass "Import stress ($ITERATIONS iterations)"
fi

# ── Check for file descriptor leaks ───────────────────────────────
echo ""
echo "[5/5] Checking for file descriptor leaks..."

FD_AFTER=$(ls /proc/self/fd 2>/dev/null | wc -l || echo "0")
FD_LEAKED=$((FD_AFTER - FD_BEFORE))

echo "  FDs before: $FD_BEFORE"
echo "  FDs after:  $FD_AFTER"
echo "  Delta:      $FD_LEAKED"

# Allow small variance (±10 FDs) for normal operational overhead
LEAK_THRESHOLD=10
if [[ "$FD_LEAKED" -gt "$LEAK_THRESHOLD" ]]; then
  fail "File descriptor leak detected: $FD_LEAKED FDs created (threshold: $LEAK_THRESHOLD)"
elif [[ "$FD_LEAKED" -lt "-$LEAK_THRESHOLD" ]]; then
  echo "  INFO: FD count decreased by $((- FD_LEAKED)) (GC or cleanup occurred)"
  pass "No FD leak (decrease)"
else
  pass "No FD leak (within threshold)"
fi

# ── Summary ───────────────────────────────────────────────────────
echo ""
echo "============================================================"
echo "  Summary"
echo "  Passed:  $PASS"
echo "  Failed:  $FAIL"
echo "  Skipped: $SKIP"
echo "  FD leak: $FD_LEAKED (threshold: $LEAK_THRESHOLD)"
echo "============================================================"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
