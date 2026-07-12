# WS 6: Daemon Schema Parity — Implementation Plan

## Status: Planning

## Architecture Summary

The daemon uses JSON lines protocol over Unix socket. Wire format is defined by `eggsec-daemon::protocol`:
- `ClientCommand` — `#[serde(tag = "type")]` enum (14 variants)
- `ServerMessage` — `#[serde(tag = "type")]` enum (13 variants)
- Runtime types: `RunRequest` (`TaskKind` tagged `{"kind":"...","params":{...}}`), `RuntimeSurface`, `SessionSnapshot`, `SessionSummary`, `RuntimeEvent`

Python client (`daemon.rs`) delegates to `eggsec_daemon::client::DaemonClient` for all wire serialization. Simplified Python DTOs (`DaemonResponsePy`, `SessionSummaryPy`, etc.) are intentional UX adaptations, not compatibility issues.

## Findings

### Already Compatible
All operations the Python client currently supports are wire-compatible because they delegate to the Rust daemon client which constructs `ClientCommand` directly.

### Missing Operations (protocol supports them, Python doesn't expose them)
1. `async_daemon_submit_task` — **critical gap**, cannot submit tasks from Python
2. `async_daemon_subscribe` — cannot receive events
3. `async_daemon_cancel_task` — cannot cancel tasks
4. `async_daemon_list_persisted_sessions` — missing from Python
5. `async_daemon_get_persisted_snapshot` — missing from Python
6. `async_daemon_approve_policy` — missing (no daemon client method either)
7. `async_daemon_cancel_active` — missing (no daemon client method either)

### Missing Rust daemon client methods
- `cancel_active()` — protocol variant exists but no convenience method
- `approve_policy()` — protocol variant exists but no convenience method

## Implementation Steps

### Step 1: Add missing daemon client methods

**File:** `crates/eggsec-daemon/src/client.rs`

Add two methods to `DaemonClient`:

```rust
pub async fn cancel_active(&mut self, session_id: SessionId) -> Result<ServerMessage>
pub async fn approve_policy(&mut self, session_id: SessionId, task_id: TaskId, approved: bool, reason: Option<String>) -> Result<ServerMessage>
```

Both follow the existing `send_command()` pattern.

### Step 2: Add Python async daemon functions

**File:** `crates/eggsec-python/src/daemon.rs`

Add 7 new `#[pyfunction]` entries following the existing pattern:

| Function | Signature | Delegates to |
|----------|-----------|--------------|
| `async_daemon_submit_task` | `(client, session_id, task_kind_json, surface="cli_manual", labels=None)` | `inner.submit_task(sid, RunRequest { task_kind: serde_json::from_str(json), ... })` |
| `async_daemon_subscribe` | `(client, session_id)` | `inner.subscribe(sid)` — returns event receiver |
| `async_daemon_cancel_task` | `(client, session_id, task_id)` | `inner.cancel_task(sid, tid)` |
| `async_daemon_cancel_active` | `(client, session_id)` | `inner.cancel_active(sid)` |
| `async_daemon_approve_policy` | `(client, session_id, task_id, approved, reason=None)` | `inner.approve_policy(sid, tid, approved, reason)` |
| `async_daemon_list_persisted_sessions` | `(client)` | `inner.list_persisted_sessions()` |
| `async_daemon_get_persisted_snapshot` | `(client, session_id)` | `inner.get_persisted_snapshot(sid)` |

For `submit_task`, the `task_kind_json` parameter accepts a JSON string in `eggsec_runtime::TaskKind` serde format: `{"kind": "PortScan", "params": {"target": "10.0.0.1"}}`. The function deserializes it via `serde_json::from_str::<TaskKind>()`.

### Step 3: Register new functions

**File:** `crates/eggsec-python/src/lib.rs`

Add `add_function` calls under the `#[cfg(feature = "daemon-client")]` block and in the `api_surface()` list.

### Step 4: Add Rust-side wire format tests

**File:** `crates/eggsec-python/src/daemon.rs` (new `#[cfg(test)]` module)

Tests:
1. `TaskKind` JSON round-trip for PortScan, Fingerprint, Recon, LoadTest, Fuzz variants
2. `RunRequest` JSON structure matches expected daemon wire format
3. `ClientCommand::SubmitTask` JSON serialization produces correct `type` tag
4. `SessionSnapshot` round-trip
5. `SessionSummary` round-trip

### Step 5: Add Python-side compatibility tests

**File:** `crates/eggsec-python/tests/test_daemon_serialization.py`

Tests:
1. `DaemonResponsePy` JSON structure: has `ok`, `request_id`, `message`, `error_code`
2. `SessionSummaryPy` JSON structure
3. `DaemonCapabilitiesPy` JSON structure
4. `TaskStatusPy` JSON structure
5. `DaemonEventPy` JSON structure
6. `TransportMetadataPy` JSON structure
7. Task kind JSON helpers produce valid `TaskKind`-compatible JSON
8. `PortScanRequest` JSON structure matches `PortScanParams` serde format

### Step 6: Document compatibility

**File:** `crates/eggsec-python/README.md`

Add section:
- Wire format is JSON lines (same as daemon native protocol)
- Python client delegates to Rust daemon client — serialization is identical
- Task kinds use `eggsec_runtime::TaskKind` serde format
- Protocol version constant: `DAEMON_PROTOCOL_VERSION = 1`
- Simplified Python DTOs are API convenience, not wire format

## Files Modified

| File | Change |
|------|--------|
| `crates/eggsec-daemon/src/client.rs` | Add `cancel_active()` and `approve_policy()` methods |
| `crates/eggsec-python/src/daemon.rs` | Add 7 async functions, Rust-side wire tests |
| `crates/eggsec-python/src/lib.rs` | Register new functions in module and API surface |
| `crates/eggsec-python/tests/test_daemon_serialization.py` | New test file for wire compatibility |
| `crates/eggsec-python/README.md` | Add wire compatibility section |

## Verification

```bash
cargo check -p eggsec-daemon
cargo check -p eggsec-python --features daemon-client
cargo test -p eggsec-daemon
cargo test -p eggsec-python
pytest crates/eggsec-python/tests/test_daemon_serialization.py
```
