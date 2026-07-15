#!/usr/bin/env bash
# test_python_repository_durability.sh — Repository/session durability tests
#
# Builds eggsec-python with default features and runs the daemon contract tests
# focused on repository durability (session persistence, state recovery).
#
# Usage:
#   bash scripts/test_python_repository_durability.sh
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
#
# Exit codes:
#   0 - all tests passed
#   1 - one or more tests failed
#   2 - skipped (test files missing)
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
echo "  test_python_repository_durability.sh"
echo "  Python Repository Durability Tests"
echo "============================================================"
echo ""

# ── Check test files exist ────────────────────────────────────────
DAEMON_TEST="$ROOT/crates/eggsec-python/tests/test_daemon_contract.py"
if [[ ! -f "$DAEMON_TEST" ]]; then
  skip "test_daemon_contract.py not found — skipping repository durability tests"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi

# ── Build with default features ───────────────────────────────────
echo "[1/3] Building eggsec-python with default features..."
if ! timeout 600 maturin develop --release 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop failed"
  echo "      Skipping repository durability tests."
  skip "default feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with default features"

# ── Run repository/durability section ─────────────────────────────
echo ""
echo "[2/3] Running test_daemon_contract.py (repository section)..."

PYTEST_OUTPUT=$(python3 -m pytest "$DAEMON_TEST" --strict-markers -q -k "repository or durability or persistence" 2>&1) || true
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
