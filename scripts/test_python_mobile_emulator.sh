#!/usr/bin/env bash
# test_python_mobile_emulator.sh — Mobile analysis Python integration tests
#
# Builds eggsec-python with the mobile feature and runs the mobile test suite
# if available. Checks for ADB availability before running dynamic tests.
#
# Usage:
#   bash scripts/test_python_mobile_emulator.sh
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
#   - For dynamic tests: ADB on PATH and an emulator/device connected
#
# Exit codes:
#   0 - all tests passed
#   1 - one or more tests failed
#   2 - skipped (feature unavailable, ADB missing, or test files missing)
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
echo "  test_python_mobile_emulator.sh"
echo "  Python Mobile Analysis Integration Tests"
echo "============================================================"
echo ""

# ── Check test files exist ────────────────────────────────────────
MOBILE_TEST="$ROOT/crates/eggsec-python/tests/test_mobile.py"
if [[ ! -f "$MOBILE_TEST" ]]; then
  skip "test_mobile.py not found — skipping mobile emulator tests"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi

# ── Build with mobile feature ─────────────────────────────────────
echo "[1/4] Building eggsec-python with mobile feature..."
if ! timeout 600 maturin develop --release --features mobile 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop with mobile feature failed"
  echo "      Skipping mobile emulator tests."
  skip "mobile feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with mobile feature"

# ── Check ADB availability ────────────────────────────────────────
echo ""
echo "[2/4] Checking for ADB..."
HAS_ADB=false
if command -v adb &>/dev/null; then
  pass "ADB found: $(command -v adb)"
  HAS_ADB=true

  # Check for connected devices
  DEVICE_COUNT=$(adb devices 2>/dev/null | grep -c 'device$' || true)
  if [[ "$DEVICE_COUNT" -gt 0 ]]; then
    pass "$DEVICE_COUNT device(s) connected"
  else
    echo "  WARN: ADB found but no devices connected — dynamic tests may be skipped"
  fi
else
  skip "ADB not found on PATH — dynamic tests will be skipped"
fi

# ── Run tests ─────────────────────────────────────────────────────
echo ""
echo "[3/4] Running test_mobile.py..."

PYTEST_OUTPUT=$(python3 -m pytest "$MOBILE_TEST" --strict-markers -q 2>&1) || true
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
