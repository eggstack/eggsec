# Distributed Module Architecture Review

Review date: 2026-05-24
Architecture document: `architecture/distributed.md`

---

## Summary Statistics

| Metric | Count |
|--------|-------|
| Verified Claims | 23 |
| Discrepancies | 9 |
| Bugs Found | 4 |
| Improvement Opportunities | 7 |

---

## Verified Claims

### Core Data Structures

| Claim | Location | Status |
|-------|----------|--------|
| TaskType enum with 7 task types | `mod.rs:59-67` | ✅ VERIFIED |
| Task struct | `queue.rs:7-18` | ✅ VERIFIED |
| TaskResult struct | `queue.rs:20-27` | ✅ VERIFIED |
| TaskQueue with thread-safe operations | `queue.rs:29-152` | ✅ VERIFIED |
| QueueError enum | `queue.rs:154-158` | ✅ VERIFIED (minor line offset) |
| RemoteListener | `remote.rs:27-398` | ✅ VERIFIED (line offset) |
| RemoteClient | `remote.rs:405-765` | ✅ VERIFIED (line offset) |
| CommandExecutor | `command.rs:108-229` | ✅ VERIFIED |
| CommandMessage | `command.rs:28-49` | ✅ VERIFIED |
| Worker | `worker.rs:64-188` | ✅ VERIFIED (line offset) |
| TlsServer | `io.rs:110-161` | ✅ VERIFIED |
| TlsClient | `io.rs:163-225` | ✅ VERIFIED |
| StreamWrapper | `io.rs:19-108` | ✅ VERIFIED |
| LineWriter | `io.rs:306-340` | ✅ VERIFIED |

### Protocol & Security

| Claim | Location | Status |
|-------|----------|--------|
| PSK-based authentication | `remote.rs:254-269` | ✅ VERIFIED |
| TLS encryption support | `remote.rs:233-250` | ✅ VERIFIED |
| `insecure-tls` feature for testing | `io.rs:172-191` | ✅ VERIFIED |
| Line-based JSON protocol | `io.rs:315-340` | ✅ VERIFIED |

### Bug Fixes Documented

| Claim | Location | Status |
|-------|----------|--------|
| `dequeue()` sets worker_id and assigned_at_secs | `queue.rs:65-66` | ✅ VERIFIED |
| `dequeue()` returns Result for error handling | `queue.rs:57` | ✅ VERIFIED |
| Heartbeat uses TCP via `RemoteClient::send_heartbeat()` | `remote.rs:591-635` | ✅ VERIFIED |

### Performance Improvements

| Claim | Location | Status |
|-------|----------|--------|
| Task.payload uses FxHashMap | `queue.rs:13` | ✅ VERIFIED |
| CommandMessage::Execute.env uses FxHashMap | `command.rs:36-37` | ✅ VERIFIED |
| rate_limits uses FxHashMap | `remote.rs:30-31` | ✅ VERIFIED |

---

## Discrepancies

### Line Number Offsets in Documentation

| Component | Doc Line | Actual Line | Offset |
|-----------|----------|-------------|--------|
| QueueError | 150-154 | 154-158 | +4 |
| RemoteListener struct | 26-388 | 27-398 | +1/+10 |
| RemoteListener impl | N/A | 39-109 | - |
| RemoteClient struct | 395-697 | 405-765 | +10/+68 |
| CommandExecutor impl | 103-220 | 108-229 | +5/+9 |
| Worker struct | 60-558 | 64-558 | +4 |
| generate_psk | 249-254 | 258-263 | +9 |

### Minor Documentation Issues

1. **generate_psk location**: Document says `command.rs:249-254` but actual is `command.rs:258-263`

2. **RemoteListener impl methods**: Documentation references `RemoteListener` struct definition but doesn't mention the impl block with `new()`, `with_config()`, `with_allowlist()`, `with_tls()`, `start()`, etc.

3. **Worker struct missing keyword**: `worker.rs:64` is missing `pub` visibility modifier for the Worker struct (line 64 shows `pub struct Worker` which is correct but documentation references line 60 which has visibility issue)

---

## Bugs Found

### BUG 1: WorkerStats Never Updated (High Priority)

**File**: `worker.rs:78-82`

**Issue**: WorkerStats fields `tasks_completed`, `tasks_failed`, and `tasks_in_progress` are initialized to 0 and never updated during task processing.

```rust
stats: WorkerStats {
    worker_id: config.worker_id.clone(),
    tasks_completed: 0,      // Never updated
    tasks_failed: 0,         // Never updated
    tasks_in_progress: 0,    // Never updated
    last_heartbeat_secs: chrono::Utc::now().timestamp(),
},
```

**Impact**: Workers report incorrect statistics to the coordinator. The coordinator cannot make informed scheduling decisions based on worker load.

**Fix**: Update `process_task()` or `Worker::start_task_processing_loop()` to update stats when tasks complete/fail.

---

### BUG 2: Heartbeat Reports Static Zero Values (High Priority)

**File**: `worker.rs:151-157`

**Issue**: Heartbeat sends hardcoded static values regardless of actual worker state:

```rust
let status = serde_json::json!({
    "worker_id": worker_id,
    "status": "idle",        // Hardcoded!
    "current_jobs": 0,       // Always 0!
    "completed_jobs": 0,     // Always 0!
    "failed_jobs": 0,        // Always 0!
});
```

**Impact**: Coordinator receives no useful information about worker load or activity. Task distribution cannot be optimized based on actual worker utilization.

**Fix**: Track in-progress tasks and report actual counts in heartbeat.

---

### BUG 3: Task Results Never Sent to Coordinator (High Priority)

**File**: `worker.rs:169-183`

**Issue**: `start_task_processing_loop()` spawns tasks but never sends `TaskResult` back to the coordinator via `CommandMessage::Result`.

```rust
while let Some(task) = receiver.recv().await {
    tokio::spawn(async move {
        let result = process_task(task).await;
        if let Err(e) = result {
            tracing::error!("Task processing error: {}", e);
        }
        // Result is dropped here - never sent to coordinator!
    });
}
```

**Impact**: All task results are lost. The coordinator's `TaskQueue::complete()` is never called, and completed tasks are never removed from `in_progress`. The entire distributed task result aggregation system is non-functional.

**Fix**: Send results back via `RemoteClient::report_result()` (which maps to `CommandMessage::Result`).

---

### BUG 4: Heartbeat Status Always "idle" (Medium Priority)

**File**: `worker.rs:152`

**Issue**: Even though the Worker has a status field (`WorkerStats.status` would be appropriate), heartbeat always reports `"status": "idle"`:

```rust
"status": "idle",  // Should reflect actual worker status
```

**Impact**: Coordinator cannot determine if a worker is idle, busy, or disconnected based on heartbeat messages.

**Fix**: Add actual `WorkerStatus` tracking and report it in heartbeat.

---

## Improvement Opportunities

### IMPROVEMENT 1: Implement Worker Registration Flow (High Priority)

**Location**: `worker.rs:106-124`

**Issue**: `register_with_coordinator()` sends registration but the coordinator's `CommandMessage::Register` handler (`remote.rs:345-354`) only responds with capabilities - it doesn't store worker information for tracking or health monitoring.

**Impact**: No way to track which workers are registered, their capabilities, or their health status.

**Suggestion**: Store `WorkerRegistration` in a map on the coordinator side and use it for task assignment based on capabilities.

---

### IMPROVEMENT 2: Task Assignment from Coordinator (Medium Priority)

**Location**: `remote.rs` and `worker.rs`

**Issue**: The coordinator has a `TaskQueue` but there's no mechanism for workers to request tasks from the coordinator. Workers only receive tasks via internal `mpsc::channel`, not from the coordinator.

**Impact**: The distributed task queue system is one-directional (coordinator→queue, but no pull mechanism).

**Suggestion**: Add `CommandMessage::RequestTask` and corresponding `TaskQueue::dequeue()` call from worker.

---

### IMPROVEMENT 3: Graceful Worker Shutdown (Medium Priority)

**Location**: `worker.rs:64-104`

**Issue**: `Worker::start()` spawns heartbeat and task processor loops but there's no `shutdown()` or `stop()` method to cleanly terminate them.

**Impact**: Dropping a Worker leaves orphaned async tasks running.

**Suggestion**: Add shutdown channel and proper cancellation in heartbeat/task processor loops.

---

### IMPROVEMENT 4: Connection Cleanup on Panic (Medium Priority)

**Location**: `remote.rs:207-211`

**Issue**: If `handle_connection()` panics, the connection won't be removed from `connections` list since cleanup happens at line 390 after the loop exits (via EOF).

```rust
tokio::spawn(async move {
    if let Err(e) = Self::handle_connection(...) {
        tracing::error!("Connection error: {}", e);
    }
});
```

**Impact**: Panic causes connection leak in `connections` Vec.

**Suggestion**: Use `Arc<Mutex<>>` or catch panic in a wrapper.

---

### IMPROVEMENT 5: Rate Limit Cleanup (Medium Priority)

**Location**: `remote.rs:119-138`

**Issue**: `check_rate_limit()` cleans old entries from the `rate_limits` HashMap but there's no periodic cleanup for IPs that no longer connect. The HashMap grows unbounded.

**Impact**: Memory grows over time as old IPs accumulate.

**Suggestion**: Add periodic cleanup of stale rate limit entries (entries older than 2x RATE_LIMIT_WINDOW_SECS).

---

### IMPROVEMENT 6: DNS Rebinding Protection (Low Priority)

**Location**: `remote.rs:445-453`

**Issue**: `resolve_cached()` caches DNS resolution for 60 seconds but doesn't validate the cached `SocketAddr` is still valid or hasn't been recycled.

**Impact**: Could connect to wrong IP if DNS changes during cache period.

**Suggestion**: Add validation that cached address still resolves correctly before use.

---

### IMPROVEMENT 7: Worker Registration Capabilities Not Validated (Low Priority)

**Location**: `remote.rs:345-354`

**Issue**: `CommandMessage::Register` accepts any capabilities list from workers without validation against `CAPABILITIES` constant.

```rust
CommandMessage::Register {
    id,
    hostname,
    capabilities,  // Not validated against CAPABILITIES
} => { ... }
```

**Impact**: Workers could register with arbitrary capability strings.

**Suggestion**: Validate `capabilities` against `CAPABILITIES` constant.

---

## Priority Summary

| ID | Finding | Priority |
|----|---------|----------|
| BUG 3 | Task results never sent to coordinator | HIGH |
| BUG 1 | WorkerStats never updated | HIGH |
| BUG 2 | Heartbeat reports static zero values | HIGH |
| IMP 1 | Worker registration flow incomplete | HIGH |
| IMP 2 | No task assignment pull mechanism | MEDIUM |
| IMP 3 | No graceful worker shutdown | MEDIUM |
| IMP 4 | Connection cleanup on panic | MEDIUM |
| IMP 5 | Rate limit cleanup unbounded | MEDIUM |
| BUG 4 | Heartbeat status always "idle" | MEDIUM |
| IMP 6 | DNS rebinding protection | LOW |
| IMP 7 | Capabilities not validated | LOW |

---

## Risk Assessment

**Critical**: 1 bug (BUG 3 - task results lost, distributed system fundamentally broken for result collection)

**High**: 2 bugs + 1 improvement (BUG 1, BUG 2 - stats reporting broken; IMP 1 - registration incomplete)

**Medium**: 5 issues (worker lifecycle, cleanup, connection handling)

**Low**: 2 issues (validation, DNS caching)

---

## Recommendations

1. **Immediate**: Fix BUG 3 to send task results back to coordinator - this is core functionality
2. **Immediate**: Fix BUG 1 and BUG 2 to update and report actual worker statistics
3. **Short-term**: Implement proper worker registration tracking on coordinator
4. **Short-term**: Add task request/pull mechanism for workers
5. **Medium-term**: Add graceful shutdown to worker
6. **Medium-term**: Fix connection cleanup to handle panics
7. **Long-term**: Consider adding connection pooling for workers (currently new connection per heartbeat)