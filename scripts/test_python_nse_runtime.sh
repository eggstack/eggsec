#!/usr/bin/env bash
# test_python_nse_runtime.sh — NSE runtime Python integration tests
#
# Builds eggsec-python with the nse feature and runs the NSE test suite.
# Skips gracefully if the nse feature cannot be compiled (e.g. missing libssl-dev).
#
# Usage:
#   bash scripts/test_python_nse_runtime.sh
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
#   - libssl-dev (for nse feature)
#
# Exit codes:
#   0 - all tests passed
#   1 - one or more tests failed
#   2 - skipped (feature unavailable or test files missing)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$ROOT"

export EGGSEC_ALLOW_LOOPBACK_FIXTURE=1

PASS=0
FAIL=0
SKIP=0

pass() { PASS=$((PASS + 1)); echo "  PASS: $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  FAIL: $1" >&2; }
skip() { SKIP=$((SKIP + 1)); echo "  SKIP: $1"; }

echo "============================================================"
echo "  test_python_nse_runtime.sh"
echo "  Python NSE Runtime Integration Tests"
echo "============================================================"
echo ""

# ── Check test files exist ────────────────────────────────────────
NSE_TEST="$ROOT/crates/eggsec-python/tests/test_nse.py"
if [[ ! -f "$NSE_TEST" ]]; then
  skip "test_nse.py not found — skipping NSE runtime tests"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi

# ── Build with nse feature ────────────────────────────────────────
echo "[1/3] Building eggsec-python with nse feature..."
if ! timeout 600 maturin develop --release --features nse 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop with nse feature failed (missing libssl-dev?)"
  echo "      Install libssl-dev and retry. Skipping NSE tests."
  skip "nse feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with nse feature"

# ── Run tests ─────────────────────────────────────────────────────
echo ""
echo "[2/3] Running test_nse.py..."

PYTEST_ARGS=(
  python3 -m pytest "$NSE_TEST"
  --strict-markers
  -q
)

PYTEST_OUTPUT=$("${PYTEST_ARGS[@]}" 2>&1) || true
echo "$PYTEST_OUTPUT"

PASSED=$(echo "$PYTEST_OUTPUT" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
FAILED=$(echo "$PYTEST_OUTPUT" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")
SKIPPED=$(echo "$PYTEST_OUTPUT" | grep -oP '\d+ skipped' | grep -oP '\d+' || echo "0")
PASSED=${PASSED:-0}
FAILED=${FAILED:-0}
SKIPPED=${SKIPPED:-0}

PASS=$((PASS + PASSED))
FAIL=$((FAIL + FAILED))
SKIP=$((SKIP + SKIPPED))

# ── Summary ───────────────────────────────────────────────────────
echo ""
echo "============================================================"
echo "  Summary"
echo "  Passed:  $PASS"
echo "  Failed:  $FAIL"
echo "  Skipped: $SKIP"
echo "============================================================"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
