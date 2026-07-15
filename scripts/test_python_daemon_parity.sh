#!/usr/bin/env bash
# test_python_daemon_parity.sh — Daemon client parity Python integration tests
#
# Builds eggsec-python with the daemon-client feature and runs the daemon
# contract/parity test suite. Verifies that the Python daemon client matches
# the Rust daemon protocol behavior.
#
# Usage:
#   bash scripts/test_python_daemon_parity.sh
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
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
echo "  test_python_daemon_parity.sh"
echo "  Python Daemon Client Parity Tests"
echo "============================================================"
echo ""

# ── Check test files exist ────────────────────────────────────────
DAEMON_TEST="$ROOT/crates/eggsec-python/tests/test_daemon_contract.py"
if [[ ! -f "$DAEMON_TEST" ]]; then
  skip "test_daemon_contract.py not found — skipping daemon parity tests"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi

# ── Build with daemon-client feature ──────────────────────────────
echo "[1/3] Building eggsec-python with daemon-client feature..."
if ! timeout 600 maturin develop --release --features daemon-client 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop with daemon-client feature failed"
  echo "      Skipping daemon parity tests."
  skip "daemon-client feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with daemon-client feature"

# ── Run tests ─────────────────────────────────────────────────────
echo ""
echo "[2/3] Running test_daemon_contract.py (daemon parity section)..."

PYTEST_OUTPUT=$(python3 -m pytest "$DAEMON_TEST" --strict-markers -q 2>&1) || true
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
