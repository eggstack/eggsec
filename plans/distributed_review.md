# Architecture Review: distributed.md vs Implementation

**Date:** 2026-05-22
**Reviewer:** Architecture Review Process
**Module:** `distributed/`

---

## 1. Summary

The distributed architecture document (`architecture/distributed.md`) accurately reflects the implementation in `crates/slapper/src/distributed/`. The bugs listed in the "Bugs Fixed (2026-05-22)" section have been correctly applied to the codebase. The FxHashMap performance improvements are also correctly implemented.

### 1.1 What's Implemented Correctly

| Component | Status | Notes |
|-----------|--------|-------|
| Task Lifecycle | ✅ | `dequeue()` properly tracks `worker_id` and `assigned_at_secs` (queue.rs:57-71) |
| Error Handling | ✅ | `dequeue()` returns `Result<Option<Task>, QueueError>` (queue.rs:57) |
| Heartbeat via TCP | ✅ | Worker uses `RemoteClient::send_heartbeat()` via TCP (worker.rs:159) |
| FxHashMap - Task.payload | ✅ | `queue.rs:13` uses `FxHashMap<String, serde_json::Value>` |
| FxHashMap - CommandMessage.env | ✅ | `command.rs:37` uses `Option<FxHashMap<String, String>>` |
| FxHashMap - RemoteListener.rate_limits | ✅ | `remote.rs:30` uses `Arc<RwLock<FxHashMap<String, Vec<Instant>>>>` |
| TaskType enum (7 variants) | ✅ | Correctly defined in `mod.rs:58-67` |
| Line-based JSON protocol | ✅ | Correctly implemented in `io.rs` and `remote.rs` |
| PSK authentication | ✅ | Using constant-time comparison (`remote.rs:266`) |
| TaskQueue structure | ✅ | Has `pending`, `in_progress`, `completed` queues with proper types |

---

## 2. Bugs/Issues Found

### 2.1 Unwrap/Expect in Test Code (Low Severity)

**File:** `io.rs:361-429`

All `unwrap()` calls in `io.rs` are in `#[cfg(test)]` blocks and are acceptable for test code. They do not affect production code paths.

```rust
// io.rs:361 - test code, acceptable
let (stream, _) = listener.accept().await.unwrap();

// io.rs:395 - test code, acceptable  
let response = writer.read_line().await?.unwrap();
```

**Verdict:** No action required. Test code may use `unwrap()` for simplicity.

---

## 3. Recommended Fixes

No bugs or issues requiring fixes were identified in the production code.

The implementation correctly:
- Uses `FxHashMap` instead of `std::collections::HashMap` for performance
- Returns `Result` types for error handling in `dequeue()`
- Properly tracks worker assignment in `dequeue()` 
- Uses TCP-based heartbeat instead of HTTP REST API

---

## 4. Discrepancies Between Arch and Implementation

### 4.1 Key Components Table Line Numbers

The architecture document lists line numbers for key components:

| Component | Arch Document | Actual |
|-----------|---------------|--------|
| TaskType enum | mod.rs:59-67 | mod.rs:58-67 ✅ |
| Task struct | queue.rs:7-18 | queue.rs:7-18 ✅ |
| TaskResult struct | queue.rs:20-27 | queue.rs:20-27 ✅ |
| TaskQueue | queue.rs:29-154 | queue.rs:29-148 ✅ (148 vs 154 - minor diff, table may be slightly off) |
| QueueError | queue.rs:150-154 | queue.rs:150-154 ✅ |
| RemoteListener | remote.rs:26-388 | remote.rs:26-388 ✅ |
| RemoteClient | remote.rs:395-697 | remote.rs:395-697 ✅ |
| CommandExecutor | command.rs:103-220 | command.rs:104-220 ✅ (103 vs 104 - struct def starts at 104) |
| CommandMessage | command.rs:28-46 | command.rs:28-46 ✅ |
| Worker | worker.rs:60-558 | worker.rs:60-558 ✅ |
| TlsServer | io.rs:110-161 | io.rs:110-161 ✅ |
| TlsClient | io.rs:163-225 | io.rs:163-225 ✅ |
| StreamWrapper | io.rs:19-108 | io.rs:19-108 ✅ |
| LineWriter | io.rs:306-340 | io.rs:306-340 ✅ |
| generate_psk | command.rs:249-254 | command.rs:250-255 ✅ |

**Minor discrepancy:** The table is mostly accurate but off by 1-2 lines in a few places. This is acceptable as documentation may become stale.

---

## 5. Verification Checklist

| Item | Verified |
|------|----------|
| `queue.rs:57` dequeue() sets worker_id | ✅ Line 65: `task.worker_id = Some(worker_id.to_string());` |
| `queue.rs:57` dequeue() sets assigned_at_secs | ✅ Line 66: `task.assigned_at_secs = Some(now);` |
| `queue.rs:57` dequeue() returns Result | ✅ Line 57: `pub async fn dequeue(&self, worker_id: &str) -> Result<Option<Task>, QueueError>` |
| `worker.rs:132-161` heartbeat uses TCP | ✅ Line 159: `client.send_heartbeat(host, port, worker_id.clone(), status.to_string()).await` |
| `queue.rs:13` Task.payload uses FxHashMap | ✅ Line 13: `pub payload: FxHashMap<String, serde_json::Value>,` |
| `command.rs:36` env uses FxHashMap | ✅ Line 37: `env: Option<FxHashMap<String, String>>,` |
| `remote.rs:30` rate_limits uses FxHashMap | ✅ Line 30: `rate_limits: Arc<RwLock<FxHashMap<String, Vec<Instant>>>>,` |

---

## 6. Conclusion

**Status:** APPROVED

The distributed module implementation matches the architecture document. All documented bugs have been fixed and all performance improvements have been applied. No issues requiring remediation were found.

The only findings are:
1. Minor documentation line number discrepancies (non-blocking)
2. Test code `unwrap()` usage (acceptable, test-only code)

---

*End of Review*
