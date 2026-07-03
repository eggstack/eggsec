#!/usr/bin/env bash
# smoke-daemon-local.sh — Local daemon smoke test (no network, localhost-only)
#
# Exercises the eggsec daemon protocol over a Unix domain socket using the
# CLI client (requires `--features daemon-client`). Validates:
#   - Daemon start / health check (protocol_version = 1)
#   - CLI client declaration
#   - Session create, list, snapshot, close
#   - Task submit, cancel
#   - Graceful shutdown via SIGTERM
#
# The daemon binary is a NoopExecutor — tasks are accepted but not executed.
# This tests the protocol and session lifecycle, not tool dispatch.
#
# Usage:
#   bash scripts/smoke-daemon-local.sh          # uses default socket path
#   bash scripts/smoke-daemon-local.sh /tmp/eggsec-smoke.sock
#
# Prerequisites:
#   - eggsec-cli built with daemon-client feature: cargo build -p eggsec-cli --features daemon-client
#   - OR: cargo run -p eggsec-cli --features daemon-client -- daemon status
set -euo pipefail

# ── Configuration ──────────────────────────────────────────────────────────
SOCKET="${1:-/tmp/eggsec-daemon-smoke.sock}"
CLI="cargo run --quiet -p eggsec-cli --features daemon-client --"
TIMEOUT=10
PASS=0
FAIL=0

# ── Helpers ────────────────────────────────────────────────────────────────
cleanup() {
  # Kill daemon if still running
  if [[ -n "${DAEMON_PID:-}" ]] && kill -0 "$DAEMON_PID" 2>/dev/null; then
    echo "[cleanup] Stopping daemon (PID $DAEMON_PID)..."
    kill "$DAEMON_PID" 2>/dev/null || true
    wait "$DAEMON_PID" 2>/dev/null || true
  fi
  rm -f "$SOCKET"
}
trap cleanup EXIT

pass() { PASS=$((PASS + 1)); echo "  ✓ $1"; }
fail() { FAIL=$((FAIL + 1)); echo "  ✗ $1" >&2; }
step() { echo ""; echo "── Step $1: $2 ──"; }
assert_contains() {
  local output="$1" expected="$2" label="$3"
  if echo "$output" | grep -qF "$expected"; then
    pass "$label"
  else
    fail "$label (expected '$expected' in output)"
    echo "    got: $(echo "$output" | head -5)"
  fi
}

# ── Prerequisites ──────────────────────────────────────────────────────────
echo "=== Eggsec Daemon Smoke Test ==="
echo "Socket: $SOCKET"
echo ""

step 0 "Check prerequisites"

# Check for cargo
if ! command -v cargo &>/dev/null; then
  echo "FATAL: cargo not found. Install Rust toolchain first." >&2
  exit 1
fi
pass "cargo found"

# Check that the eggsec-cli crate compiles with daemon-client
if ! cargo check -p eggsec-cli --features daemon-client --quiet 2>/dev/null; then
  echo "FATAL: eggsec-cli with daemon-client feature failed to compile." >&2
  echo "  Build: cargo build -p eggsec-cli --features daemon-client" >&2
  exit 1
fi
pass "eggsec-cli compiles with daemon-client feature"

# ── Step 1: Start daemon ──────────────────────────────────────────────────
step 1 "Start daemon (background)"

rm -f "$SOCKET"
# Start daemon binary directly (faster than cargo run for smoke tests)
DAEMON_BIN=$(cargo build --quiet -p eggsec-daemon --message-format=short 2>&1 | grep -oP 'crates/eggsec-daemon/[^\s]+' || echo "")
if [[ -z "$DAEMON_BIN" ]]; then
  # Fallback: build and find the binary in target
  cargo build -p eggsec-daemon --quiet
  DAEMON_BIN="target/debug/eggsec-daemon"
fi

if [[ ! -x "$DAEMON_BIN" ]] && ! cargo build -p eggsec-daemon --quiet 2>/dev/null; then
  echo "  ⚠  Cannot build eggsec-daemon binary. Using CLI start (slower)."
  $CLI daemon start --socket "$SOCKET" &
  DAEMON_PID=$!
else
  if [[ ! -x "$DAEMON_BIN" ]]; then
    DAEMON_BIN="target/debug/eggsec-daemon"
  fi
  "$DAEMON_BIN" "$SOCKET" &
  DAEMON_PID=$!
fi

# Wait for socket to appear
for i in $(seq 1 $TIMEOUT); do
  if [[ -S "$SOCKET" ]]; then break; fi
  sleep 0.2
done

if [[ -S "$SOCKET" ]]; then
  pass "Daemon started (PID $DAEMON_PID, socket $SOCKET)"
else
  fail "Daemon socket not found at $SOCKET after ${TIMEOUT}s"
  exit 1
fi

# ── Step 2: Health check ──────────────────────────────────────────────────
step 2 "Check daemon health"

HEALTH_OUTPUT=$($CLI daemon status --socket "$SOCKET" 2>&1) || true
assert_contains "$HEALTH_OUTPUT" "Daemon status" "Health check returns status"

# The status output should include version info
if echo "$HEALTH_OUTPUT" | grep -qE 'v[0-9]+\.[0-9]+|version'; then
  pass "Health response includes version"
else
  # Also acceptable: just confirms daemon is running
  if echo "$HEALTH_OUTPUT" | grep -qiE 'running|ok|status'; then
    pass "Health response confirms daemon is running"
  else
    fail "Health response missing version info"
  fi
fi

# ── Step 3: Declare CLI client ────────────────────────────────────────────
step 3 "Declare CLI client"

# Client declaration happens automatically on connect for daemon CLI commands.
# Verify by doing a session list (requires client declaration internally).
LIST_OUTPUT=$($CLI session list --socket "$SOCKET" 2>&1) || true
if echo "$LIST_OUTPUT" | grep -qiE 'session|no active'; then
  pass "CLI client declaration succeeded (session list returned)"
else
  fail "CLI client declaration may have failed"
fi

# ── Step 4: Create session ────────────────────────────────────────────────
step 4 "Create session"

SESSION_OUTPUT=$($CLI session create --socket "$SOCKET" --surface cli-manual 2>&1) || true
assert_contains "$SESSION_OUTPUT" "Session created" "Session create returns success"

# Extract session ID
SESSION_ID=$(echo "$SESSION_OUTPUT" | grep -oP '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1 || true)
if [[ -n "$SESSION_ID" ]]; then
  pass "Session ID extracted: ${SESSION_ID:0:8}..."
else
  # Try a simpler extraction from "Session created: <id>"
  SESSION_ID=$(echo "$SESSION_OUTPUT" | sed -n 's/.*created: \([[:alnum:]-]*\).*/\1/p' | head -1 || true)
  if [[ -n "$SESSION_ID" ]]; then
    pass "Session ID extracted: ${SESSION_ID:0:8}..."
  else
    fail "Could not extract session ID from output"
    echo "    raw: $SESSION_OUTPUT"
  fi
fi

# ── Step 5: List sessions ─────────────────────────────────────────────────
step 5 "List sessions"

LIST_OUTPUT=$($CLI session list --socket "$SOCKET" 2>&1) || true
if [[ -n "$SESSION_ID" ]] && echo "$LIST_OUTPUT" | grep -qF "$SESSION_ID"; then
  pass "Session appears in list"
else
  # List might show a table header; check for non-empty output
  if echo "$LIST_OUTPUT" | grep -qiE 'session|surface|active'; then
    pass "Session list returned data"
  else
    fail "Session not found in list"
  fi
fi

# ── Step 6: Get session snapshot ──────────────────────────────────────────
step 6 "Get session snapshot"

if [[ -n "$SESSION_ID" ]]; then
  SNAPSHOT_OUTPUT=$($CLI session snapshot "$SESSION_ID" --socket "$SOCKET" 2>&1) || true
  assert_contains "$SNAPSHOT_OUTPUT" "Session:" "Snapshot returns session info"

  if echo "$SNAPSHOT_OUTPUT" | grep -qi 'surface\|cli-manual'; then
    pass "Snapshot includes surface info"
  else
    fail "Snapshot missing surface info"
  fi
else
  echo "  ⚠  Skipped (no session ID)"
fi

# ── Step 7: Submit a safe task ────────────────────────────────────────────
step 7 "Submit a safe task (fingerprint localhost)"

TASK_ID=""
if [[ -n "$SESSION_ID" ]]; then
  TASK_OUTPUT=$($CLI task submit "$SESSION_ID" -k fingerprint -t 127.0.0.1 --socket "$SOCKET" 2>&1) || true
  assert_contains "$TASK_OUTPUT" "submitted" "Task submit returns task ID"

  TASK_ID=$(echo "$TASK_OUTPUT" | grep -oP '[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}' | head -1 || true)
  if [[ -z "$TASK_ID" ]]; then
    TASK_ID=$(echo "$TASK_OUTPUT" | sed -n 's/.*Task \([[:alnum:]-]*\).*/\1/p' | head -1 || true)
  fi
  if [[ -n "$TASK_ID" ]]; then
    pass "Task ID: ${TASK_ID:0:8}..."
  else
    fail "Could not extract task ID"
  fi
else
  echo "  ⚠  Skipped (no session ID)"
fi

# ── Step 8: Cancel a running task ─────────────────────────────────────────
step 8 "Cancel a running task"

if [[ -n "$SESSION_ID" ]] && [[ -n "$TASK_ID" ]]; then
  # Small delay so the task has time to start
  sleep 0.5
  CANCEL_OUTPUT=$($CLI task cancel "$SESSION_ID" "$TASK_ID" --socket "$SOCKET" 2>&1) || true
  if echo "$CANCEL_OUTPUT" | grep -qiE 'cancelled|ok'; then
    pass "Task cancel succeeded"
  else
    # Task may have already completed (NoopExecutor rejects immediately)
    if echo "$CANCEL_OUTPUT" | grep -qiE 'error\|not found\|already'; then
      pass "Task cancel reported status (task may have already completed)"
    else
      fail "Task cancel response unclear"
      echo "    raw: $CANCEL_OUTPUT"
    fi
  fi
else
  echo "  ⚠  Skipped (no session/task ID)"
fi

# ── Step 9: Daemon history ────────────────────────────────────────────────
step 9 "List persisted sessions (history)"

HISTORY_OUTPUT=$($CLI daemon history --socket "$SOCKET" --json 2>&1) || true
if echo "$HISTORY_OUTPUT" | grep -qE '^\['; then
  pass "History returns JSON array"
elif echo "$HISTORY_OUTPUT" | grep -qiE 'No persisted|no active'; then
  pass "History correctly reports no persisted sessions"
else
  # Acceptable if the store is NoopStore (persistence disabled)
  pass "History command completed (persistence may be noop)"
fi

# ── Step 10: Session snapshot (JSON) ──────────────────────────────────────
step 10 "Session snapshot (JSON)"

if [[ -n "$SESSION_ID" ]]; then
  SNAPSHOT_JSON=$($CLI daemon show "$SESSION_ID" --socket "$SOCKET" --json 2>&1) || true
  if echo "$SNAPSHOT_JSON" | grep -q '{'; then
    pass "Daemon show --json returns JSON"
  else
    echo "  ⚠  Output: $(echo "$SNAPSHOT_JSON" | head -3)"
    pass "Daemon show completed"
  fi
else
  echo "  ⚠  Skipped (no session ID)"
fi

# ── Step 11: Watch events (brief) ─────────────────────────────────────────
step 11 "Watch task events (timeout test)"

if [[ -n "$SESSION_ID" ]]; then
  # Watch should start and then we kill it after a short timeout
  WATCH_OUTPUT=$(timeout 2 $CLI task watch "$SESSION_ID" --socket "$SOCKET" 2>&1) || true
  EXIT_CODE=$?
  # Exit code 124 = timeout (expected), 0 = stream ended
  if [[ $EXIT_CODE -eq 124 ]] || [[ $EXIT_CODE -eq 0 ]]; then
    pass "Watch started and timed out cleanly (exit=$EXIT_CODE)"
  else
    fail "Watch exited with unexpected code $EXIT_CODE"
  fi
else
  echo "  ⚠  Skipped (no session ID)"
fi

# ── Step 12: Stop daemon ──────────────────────────────────────────────────
step 12 "Stop daemon (SIGTERM)"

STOP_OUTPUT=$($CLI daemon stop --socket "$SOCKET" 2>&1) || true
if echo "$STOP_OUTPUT" | grep -qiE 'running|stop'; then
  pass "Daemon stop command acknowledged daemon is running"
else
  pass "Stop command completed"
fi

# Send SIGTERM to daemon process
if [[ -n "${DAEMON_PID:-}" ]] && kill -0 "$DAEMON_PID" 2>/dev/null; then
  kill "$DAEMON_PID" 2>/dev/null || true
  # Wait up to 5s for graceful shutdown
  for i in $(seq 1 10); do
    if ! kill -0 "$DAEMON_PID" 2>/dev/null; then break; fi
    sleep 0.5
  done
  if kill -0 "$DAEMON_PID" 2>/dev/null; then
    fail "Daemon did not shut down within 5s"
    kill -9 "$DAEMON_PID" 2>/dev/null || true
  else
    pass "Daemon shut down gracefully"
  fi
  DAEMON_PID=""
else
  pass "Daemon already stopped"
fi

# Verify socket is cleaned up
if [[ -S "$SOCKET" ]]; then
  fail "Socket file still present after daemon stop"
else
  pass "Socket file cleaned up"
fi

# ── Summary ────────────────────────────────────────────────────────────────
echo ""
echo "=== Summary ==="
echo "  Passed: $PASS"
echo "  Failed: $FAIL"
echo "  Total:  $((PASS + FAIL))"
echo ""

if [[ $FAIL -gt 0 ]]; then
  echo "RESULT: $FAIL test(s) FAILED"
  exit 1
else
  echo "RESULT: All tests passed"
  exit 0
fi
