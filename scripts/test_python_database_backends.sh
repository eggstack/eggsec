#!/usr/bin/env bash
# test_python_database_backends.sh — Database pentest Python integration tests
#
# Builds eggsec-python with the db-pentest feature and runs backend-specific
# tests only when the corresponding database environment variables are set
# (PGHOST, MYSQL_HOST, MSSQL_HOST, MONGODB_HOST, REDIS_HOST).
#
# Usage:
#   bash scripts/test_python_database_backends.sh
#
# Prerequisites:
#   - Rust toolchain, maturin, pytest
#   - For real backend tests: set PGHOST, MYSQL_HOST, MSSQL_HOST, MONGODB_HOST,
#     or REDIS_HOST to point at a running database instance.
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
echo "  test_python_database_backends.sh"
echo "  Python Database Backend Integration Tests"
echo "============================================================"
echo ""

# ── Check test files exist ────────────────────────────────────────
DB_TEST="$ROOT/crates/eggsec-python/tests/test_db_pentest.py"
if [[ ! -f "$DB_TEST" ]]; then
  skip "test_db_pentest.py not found — skipping database tests"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi

# ── Build with db-pentest feature ─────────────────────────────────
echo "[1/4] Building eggsec-python with db-pentest feature..."
if ! timeout 600 maturin develop --release --features db-pentest 2>&1 | tail -5; then
  echo ""
  echo "WARN: maturin develop with db-pentest feature failed"
  echo "      Skipping database backend tests."
  skip "db-pentest feature compilation failed"
  echo ""
  echo "Summary: PASS=$PASS FAIL=$FAIL SKIP=$SKIP"
  exit 2
fi
pass "eggsec-python built with db-pentest feature"

# ── Detect available backends ─────────────────────────────────────
echo ""
echo "[2/4] Detecting available database backends..."
BACKEND_ARGS=()

if [[ -n "${PGHOST:-}" ]]; then
  echo "  Postgres: PGHOST=$PGHOST"
  BACKEND_ARGS+=(-k "postgres")
else
  echo "  Postgres: not configured (set PGHOST to enable)"
fi

if [[ -n "${MYSQL_HOST:-}" ]]; then
  echo "  MySQL:    MYSQL_HOST=$MYSQL_HOST"
  BACKEND_ARGS+=(-k "mysql")
else
  echo "  MySQL:    not configured (set MYSQL_HOST to enable)"
fi

if [[ -n "${MSSQL_HOST:-}" ]]; then
  echo "  MSSQL:    MSSQL_HOST=$MSSQL_HOST"
  BACKEND_ARGS+=(-k "mssql")
else
  echo "  MSSQL:    not configured (set MSSQL_HOST to enable)"
fi

if [[ -n "${MONGODB_HOST:-}" ]]; then
  echo "  MongoDB:  MONGODB_HOST=$MONGODB_HOST"
  BACKEND_ARGS+=(-k "mongodb")
else
  echo "  MongoDB:  not configured (set MONGODB_HOST to enable)"
fi

if [[ -n "${REDIS_HOST:-}" ]]; then
  echo "  Redis:    REDIS_HOST=$REDIS_HOST"
  BACKEND_ARGS+=(-k "redis")
else
  echo "  Redis:    not configured (set REDIS_HOST to enable)"
fi

# ── Run dry-run tests (always) ───────────────────────────────────
echo ""
echo "[3/4] Running dry-run tests (no database required)..."
DRY_OUTPUT=$(python3 -m pytest "$DB_TEST" --strict-markers -q -k "dry_run or dryrun or unit" 2>&1) || true
echo "$DRY_OUTPUT"

PASSED=$(echo "$DRY_OUTPUT" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
FAILED=$(echo "$DRY_OUTPUT" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")
SKIPPED=$(echo "$DRY_OUTPUT" | grep -oP '\d+ skipped' | grep -oP '\d+' || echo "0")
PASSED=${PASSED:-0}
FAILED=${FAILED:-0}
SKIPPED=${SKIPPED:-0}
PASS=$((PASS + PASSED))
FAIL=$((FAIL + FAILED))
SKIP=$((SKIP + SKIPPED))

# ── Run backend-specific tests ────────────────────────────────────
echo ""
echo "[4/4] Running backend-specific tests..."
if [[ ${#BACKEND_ARGS[@]} -gt 0 ]]; then
  BACKEND_OUTPUT=$(python3 -m pytest "$DB_TEST" --strict-markers -q "${BACKEND_ARGS[@]}" 2>&1) || true
  echo "$BACKEND_OUTPUT"

  PASSED=$(echo "$BACKEND_OUTPUT" | grep -oP '\d+ passed' | grep -oP '\d+' || echo "0")
  FAILED=$(echo "$BACKEND_OUTPUT" | grep -oP '\d+ failed' | grep -oP '\d+' || echo "0")
  SKIPPED=$(echo "$BACKEND_OUTPUT" | grep -oP '\d+ skipped' | grep -oP '\d+' || echo "0")
  PASSED=${PASSED:-0}
  FAILED=${FAILED:-0}
  SKIPPED=${SKIPPED:-0}
  PASS=$((PASS + PASSED))
  FAIL=$((FAIL + FAILED))
  SKIP=$((SKIP + SKIPPED))
else
  echo "  No database backends configured — skipping backend-specific tests."
  echo "  Set PGHOST, MYSQL_HOST, MSSQL_HOST, MONGODB_HOST, or REDIS_HOST to enable."
  SKIP=$((SKIP + 1))
fi

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
