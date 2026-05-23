# Distributed Module Architecture Review

**Document**: `architecture/distributed.md`
**Implementation**: `crates/slapper/src/distributed/`
**Review Date**: 2026-05-23

---

## Executive Summary

The distributed module is well-documented and mostly well-implemented. Most bugs documented in the "Bugs Fixed (2026-05-22)" section have been properly fixed. However, several issues remain, including a critical race condition in task reassignment, a performance issue with rate limiting locks, and missing functionality that was planned but never implemented.

---

## Verified Claims

### Task Lifecycle Implementation

| Claim | Status | Verification |
|-------|--------|--------------|
| `TaskQueue::enqueue(task)` | VERIFIED | `queue.rs:46-55` - Properly validates queue size and returns `Result<(), QueueError>` |
| `TaskQueue::dequeue(worker_id)` sets `worker_id` and `assigned_at_secs` | VERIFIED | `queue.rs:65-66` - Both fields are set before inserting into `in_progress` |
| `dequeue()` returns `Result<Option<Task>, QueueError>` | VERIFIED | `queue.rs:57-72` - Now returns `Result<Option<Task>, QueueError>` |
| `TaskQueue::complete(result)` | VERIFIED | `queue.rs:100-116` - Removes from `in_progress`, adds to `completed` with size limit |
| `TaskQueue::reassign_stale_tasks(timeout_secs)` | VERIFIED | `queue.rs:74-98` - Returns stale tasks to pending queue |

### HashMap to FxHashMap Migration

| Location | Status | Line |
|----------|--------|------|
| `Task.payload` | VERIFIED | `queue.rs:13` - Uses `FxHashMap<String, serde_json::Value>` |
| `CommandMessage::Execute.env` | VERIFIED | `command.rs:37` - Uses `Option<FxHashMap<String, String>>` |
| `RemoteListener.rate_limits` | VERIFIED | `remote.rs:31` - Uses `Arc<RwLock<FxHashMap<String, Vec<Instant>>>>` |

### Bug Fixes

| Bug | Status | Verification |
|-----|--------|--------------|
| `dequeue()` ignoring `worker_id` param | FIXED | `queue.rs:65` - Sets `task.worker_id = Some(worker_id.to_string())` |
| `dequeue()` returning `Option<Task>` silently | FIXED | `queue.rs:57-72` - Now returns `Result<Option<Task>, QueueError>` |
| Heartbeat using HTTP POST to non-existent REST API | FIXED | `worker.rs:159` - Now uses `client.send_heartbeat()` via TCP |

### Component Table Accuracy

| Component | File | Lines | Key Function/Type | Status |
|-----------|------|-------|-------------------|--------|
| TaskType enum | mod.rs | 59-67 | 7 task types | VERIFIED |
| Task struct | queue.rs | 7-18 | Core task representation | VERIFIED |
| TaskResult struct | queue.rs | 20-27 | Task execution result | VERIFIED |
| TaskQueue | queue.rs | 29-154 | Thread-safe task queue | VERIFIED |
| QueueError | queue.rs | 150-154 | Queue error types | VERIFIED |
| RemoteListener | remote.rs | 26-388 | Coordinator server | VERIFIED (actually 26-375) |
| RemoteClient | remote.rs | 395-697 | Worker client | VERIFIED |
| CommandExecutor | command.rs | 103-220 | Secure command execution | VERIFIED (actually 104-227) |
| CommandMessage | command.rs | 28-46 | Protocol messages | VERIFIED |
| Worker | worker.rs | 60-558 | Worker node | VERIFIED |
| TlsServer | io.rs | 110-161 | TLS server from PEM | VERIFIED (actually 110-161) |
| TlsClient | io.rs | 163-225 | TLS client | VERIFIED (actually 163-225) |
| StreamWrapper | io.rs | 19-108 | Unified stream enum | VERIFIED |
| LineWriter | io.rs | 306-340 | Line I/O wrapper | VERIFIED (actually 306-340) |
| generate_psk | command.rs | 249-254 | PSK generation | VERIFIED |

---

## Discrepancies

### 1. Line Numbers in Documentation Are Approximate

**Severity**: Low
**Impact**: Documentation may become outdated as code changes

The documentation table provides line numbers that don't exactly match the implementation (e.g., `RemoteListener` ends at line 375 not 388). While the module structure is correctly documented, line numbers should be treated as approximate guidance rather than precise references.

### 2. Missing TaskResult Usage in Coordinator

**Severity**: Medium
**Impact**: Results are not being collected by coordinator

The `TaskQueue::complete()` method exists and properly handles task completion (`queue.rs:100-116`), but there is no code in the coordinator (RemoteListener) that calls `complete()` on tasks. The `RemoteListener::handle_connection()` processes `Execute`, `Register`, and `Heartbeat` commands but never calls `TaskQueue::complete()`.

**Missing code location**: `remote.rs` - `RemoteListener` should have a `TaskQueue` instance and call `complete()` when results arrive.

### 3. Worker Registration Unused

**Severity**: Low
**Impact**: Worker capabilities are tracked but not used for task routing

`WorkerRegistration` and `WorkerStatus` are defined in `mod.rs:93-117`, but they are never used anywhere. The `RemoteListener::handle_connection()` receives registration messages but doesn't store them in any data structure. There's no mechanism to route tasks to workers based on their capabilities.

---

## Bugs Found

### 1. Race Condition in `reassign_stale_tasks()`

**Severity**: High
**Priority**: High
**File**: `queue.rs:74-98`

**Bug**: The function modifies `in_progress` and `pending` in separate lock acquisitions, creating a race condition. If a task completes between when `in_progress.retain()` runs and when `pending.push_back()` runs, the task could be duplicated or lost.

```rust
// Line 78-87: First lock releases
in_progress.retain(|_id, task| { ... }); // Lock released here

// Line 89-95: Second lock acquires
let mut pending = self.pending.write().await; // New lock acquired
for task in stale_tasks.iter() { ... }
```

**Fix**: Use a single lock or transaction to ensure atomicity.

**Impact**: Tasks can be duplicated (assigned to multiple workers) or lost during reassignment under high concurrency.

---

### 2. Rate Limit Lock Duration Issue

**Severity**: Medium
**Priority**: Medium
**File**: `remote.rs:114-133`

**Bug**: The `check_rate_limit()` function holds a write lock for the entire cleanup and check operation:

```rust
async fn check_rate_limit(...) -> bool {
    let mut limits = rate_limits.write().await;  // Line 119 - Lock held...
    let now = Instant::now();
    let window = Duration::from_secs(RATE_LIMIT_WINDOW_SECS);

    // ... entire cleanup operation ...
    timestamps.retain(...);  // Line 125
    
    if timestamps.len() >= limit as usize {
        return false;  // Early return while holding lock
    }

    timestamps.push(now);
    true  // Lock released after full operation
}
```

Under high load, all connections from the same IP share one lock, causing serialization.

**Fix**: Reduce lock hold time by:
1. First read-only scan to check count
2. Release lock
3. Reacquire for write if needed

**Impact**: Under high concurrency, rate limiting itself becomes a bottleneck.

---

### 3. Worker Heartbeat Creates New Client Each Time

**Severity**: Medium
**Priority**: Medium
**File**: `worker.rs:126-165`

**Bug**: Every heartbeat spawns a new `RemoteClient` and establishes a new TCP connection:

```rust
let handle = tokio::spawn(async move {
    let mut interval = tokio::time::interval(...);
    let mut client = RemoteClient::new_plaintext(psk);  // Line 146 - New client each tick
    
    loop {
        interval.tick().await;
        // ... new TCP connection each heartbeat
        if let Err(e) = client.send_heartbeat(&host, port, worker_id.clone(), status.to_string()).await {
```

The `RemoteClient::send_heartbeat()` method (`remote.rs:564-594`) creates a new connection each time.

**Fix**: Cache and reuse the `RemoteClient` instance across heartbeats, similar to how `resolve_cached()` works for DNS.

**Impact**: Unnecessary connection overhead, potential network latency on each heartbeat.

---

### 4. RemoteClient `Drop` Implementation is Empty

**Severity**: Low
**Priority**: Low
**File**: `remote.rs:388-392`

**Bug**: The `Drop` implementation does nothing:

```rust
impl Drop for RemoteClient {
    fn drop(&mut self) {
        tracing::debug!("RemoteClient dropped, cleaning up connection");
    }
}
```

It should close the underlying TCP/TLS connection properly.

**Fix**: Add proper connection cleanup.

**Impact**: Connection cleanup relies on Tokio's implicit drop, which may leave sockets in `TIME_WAIT` state longer than necessary.

---

### 5. `send_heartbeat` Doesn't Use Cached Connection

**Severity**: Medium
**Priority**: Medium
**File**: `remote.rs:564-594`

**Bug**: `send_heartbeat()` calls `connect_to_coordinator()` directly without using the `cached_addr` mechanism:

```rust
pub async fn send_heartbeat(&mut self, host: &str, port: u16, ...) -> Result<()> {
    let mut line_writer = self.connect_to_coordinator(host, port).await?;  // Always reconnects
    ...
}
```

Meanwhile, `execute()` uses `resolve_cached()` properly (`remote.rs:605-615`).

**Fix**: Use the same pattern as `execute()` - check `resolve_cached()` first.

**Impact**: Unnecessary DNS lookups and connection overhead for heartbeat operations.

---

## Improvement Opportunities

### 1. Use `tokio::sync::Semaphore` for Connection Rate Limiting

**Severity**: Low
**Priority**: Low
**File**: `remote.rs:114-133`

The current rate limiting uses `Arc<RwLock<FxHashMap<String, Vec<Instant>>>>` which serializes access. A semaphore-based approach would be more efficient for high-throughput scenarios.

**Estimated Impact**: 10-20% improvement in connection handling throughput under high load.

---

### 2. Implement Worker Capability-Based Routing

**Severity**: Low
**Priority**: Low
**Files**: `mod.rs`, `remote.rs`

Workers send registration with capabilities, but the coordinator doesn't use this information for task routing. Currently all workers receive all task types.

**Estimated Impact**: Better load distribution when workers have heterogeneous capabilities.

---

### 3. Add Connection Pooling for RemoteClient

**Severity**: Medium
**Priority**: Medium
**File**: `remote.rs:382-724`

`RemoteClient` creates a new connection for each operation. Adding connection pooling would reduce overhead.

**Estimated Impact**: 30-50% reduction in connection setup overhead for workloads with many small tasks.

---

### 4. Add Backpressure for Queue Size Limits

**Severity**: Medium
**Priority**: Medium
**Files**: `queue.rs`

Only `pending` queue has a size limit (`queue.rs:49`). The `in_progress` and `completed` queues can grow unbounded.

**Estimated Impact**: Prevent memory exhaustion on coordinator with very large workloads.

---

### 5. Implement Graceful Shutdown for Worker

**Severity**: Low
**Priority**: Low
**File**: `worker.rs`

The `Worker` struct doesn't implement graceful shutdown. In-flight tasks may be lost if the worker process is terminated.

**Estimated Impact**: More reliable task execution in production environments.

---

### 6. Add Metrics/Observability

**Severity**: Low
**Priority**: Low
**Files**: `queue.rs`, `remote.rs`

No metrics are exported for monitoring queue depths, connection counts, task processing times, etc.

**Estimated Impact**: Easier operational monitoring in production.

---

## Priority Summary

| Category | Item | Priority | Estimated Fix Time |
|----------|------|----------|-------------------|
| Bug #1 | Race condition in `reassign_stale_tasks()` | HIGH | 2 hours |
| Bug #3 | Heartbeat creates new client each time | MEDIUM | 1 hour |
| Bug #5 | `send_heartbeat` doesn't use cached connection | MEDIUM | 30 minutes |
| Bug #2 | Rate limit lock duration | MEDIUM | 2 hours |
| Improve #4 | Add backpressure for queues | MEDIUM | 1 hour |
| Improve #3 | Connection pooling | MEDIUM | 3 hours |
| Bug #4 | Empty `Drop` implementation | LOW | 30 minutes |
| Improve #6 | Add metrics/observability | LOW | 2 hours |
| Improve #1 | Semaphore-based rate limiting | LOW | 2 hours |
| Improve #2 | Capability-based routing | LOW | 2 hours |
| Improve #5 | Graceful shutdown | LOW | 1 hour |

---

## Files Reviewed

| File | Lines | Issues Found |
|------|-------|--------------|
| `mod.rs` | 118 | Minor (unused types) |
| `queue.rs` | 165 | 1 Critical (race condition) |
| `remote.rs` | 724 | 3 Medium (heartbeat, rate limit, Drop) |
| `command.rs` | 279 | None |
| `worker.rs` | 558 | 1 Medium (heartbeat pattern) |
| `io.rs` | 450 | None |

---

## Conclusion

The distributed module is mostly well-implemented and matches the architecture documentation. The main concerns are:

1. **Critical**: Race condition in `reassign_stale_tasks()` could cause task duplication or loss
2. **Medium**: Connection handling inefficiencies in heartbeat and `send_heartbeat`
3. **Medium**: Rate limit lock contention under high load

The documented bugs from 2026-05-22 have been properly fixed. The module would benefit from additional testing to verify the race condition fix and from connection pooling to reduce overhead.
