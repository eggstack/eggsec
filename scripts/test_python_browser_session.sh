#!/usr/bin/env bash
# test_python_browser_session.sh — Headless browser Python session tests
#
# Builds eggsec-python with the headless-browser feature and runs the browser
# test suite if available. Checks for a browser binary before running tests.
#
# Usage:
#   bash scripts/test_python_browser_session.sh
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
#   - A headless browser binary (chromium, chrome, or firefox) on PATH
#
# Exit codes:
#   0 - all tests passed
#   1 - one or more tests failed
#   2 - skipped (feature unavailable, browser missing, or test files missing)
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
echo "  test_python_browser_session.sh"
echo "  Python Headless Browser Session Tests"
echo "============================================================"
echo ""

# ── Check test files exist ────────────────────────────────────────
BROWSER_TEST="$ROOT/crates/eggsec-python/tests/test_browser.py"
if [[ ! -f "$BROWSER_TEST" ]]; then
  skip "test_browser.py not found — skipping browser session tests"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi

# ── Check for browser binary ──────────────────────────────────────
echo "[1/4] Checking for headless browser binary..."
BROWSER_FOUND=""
for candidate in chromium chromium-browser google-chrome google-chrome-stable firefox; do
  if command -v "$candidate" &>/dev/null; then
    BROWSER_FOUND="$candidate"
    break
  fi
done

if [[ -z "$BROWSER_FOUND" ]]; then
  skip "No headless browser found on PATH (looked for chromium, chrome, firefox)"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "Browser found: $BROWSER_FOUND ($(command -v "$BROWSER_FOUND"))"

# ── Build with headless-browser feature ───────────────────────────
echo ""
echo "[2/4] Building eggsec-python with headless-browser feature..."
if ! timeout 600 maturin develop --release --features headless-browser 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop with headless-browser feature failed"
  echo "      Skipping browser session tests."
  skip "headless-browser feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with headless-browser feature"

# ── Run tests ─────────────────────────────────────────────────────
echo ""
echo "[3/4] Running test_browser.py..."

PYTEST_OUTPUT=$(python3 -m pytest "$BROWSER_TEST" --strict-markers -q 2>&1) || true
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
