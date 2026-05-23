# Distributed Module Architecture Review

## Summary

The distributed module implementation aligns with `architecture/distributed.md`. Key components (TaskQueue, RemoteListener, RemoteClient, CommandExecutor, Worker) are all present. FxHashMap usage is correctly implemented for performance. However, there are some discrepancies and issues to note.

## Implementation Verification

### Key Components

| Component | File | Lines | Status |
|-----------|------|-------|--------|
| TaskType enum | mod.rs | 58-67 | ✅ 7 task types present |
| Task struct | queue.rs | 7-18 | ✅ Correct |
| TaskResult struct | queue.rs | 20-27 | ✅ Correct |
| TaskQueue | queue.rs | 29-148 | ✅ Thread-safe implementation |
| QueueError | queue.rs | 150-154 | ✅ Error types present |
| RemoteListener | remote.rs | 26-388 | ✅ Implementation present |
| RemoteClient | remote.rs | 395-697 | ✅ Implementation present |
| CommandExecutor | command.rs | 104-220 | ✅ Secure command execution |
| CommandMessage | command.rs | 28-46 | ✅ Protocol messages |
| Worker | worker.rs | 60-558 | ✅ Worker node implementation |
| TlsServer | io.rs | 110-161 | ✅ TLS server from PEM |
| TlsClient | io.rs | 163-225 | ✅ TLS client |
| StreamWrapper | io.rs | 19-108 | ✅ Unified stream enum |
| LineWriter | io.rs | 306-340 | ✅ Line I/O wrapper |
| generate_psk | command.rs | 250-255 | ✅ PSK generation |

### FxHashMap Usage (as documented)

| Location | Type | Reason | Status |
|----------|------|--------|--------|
| `queue.rs:13` | `Task.payload` | Performance | ✅ Correct |
| `command.rs:37` | `CommandMessage::Execute.env` | Performance | ✅ Correct |
| `remote.rs:30` | `RemoteListener.rate_limits` | Performance | ✅ Correct |

### Bug Fixes Verification

**queue.rs:57 - dequeue()**: ✅ Fixed. Now properly:
- Uses `worker_id` parameter to set `task.worker_id`
- Sets `assigned_at_secs` to current timestamp
- Returns `Result<Option<Task>, QueueError>` for explicit error handling

**queue.rs:57 - dequeue() error handling**: ✅ Fixed. Returns `Result<Option<Task>, QueueError>` instead of silently dropping errors.

**worker.rs:132-161 - Heartbeat**: ✅ Fixed. Now uses `RemoteClient::send_heartbeat()` via TCP instead of HTTP POST to non-existent REST API endpoint.

### Task Lifecycle

1. **Enqueue** (`queue.rs:46-55`): ✅ `TaskQueue::enqueue(task)`
2. **Dequeue** (`queue.rs:57-72`): ✅ `TaskQueue::dequeue(worker_id)` sets worker_id and assigned_at_secs
3. **Execute**: ✅ Workers execute tasks locally
4. **Complete** (`queue.rs:100-116`): ✅ `TaskQueue::complete(result)`
5. **Reassign** (`queue.rs:74-98`): ✅ `TaskQueue::reassign_stale_tasks(timeout_secs)`

## Discrepancies

### 1. CommandMessage env field type

**Architecture doc** says `CommandMessage::Execute.env` is `FxHashMap` (line 36).

**Implementation** (`command.rs:37`): ✅ `Option<FxHashMap<String, String>>` - correct.

### 2. Worker line count discrepancy

**Architecture doc** says Worker is at `worker.rs:60-558` (499 lines). Actual implementation is 558 lines total, but struct definition starts at line 60 with `pub struct Worker {`. This matches.

### 3. TlsServer::acceptor vs clone_acceptor

**Architecture doc** (`distributed.md:46`): Says "TLS server from PEM" - `TlsServer` lines 110-161.

**Implementation** (`io.rs:154-160`): `acceptor()` returns reference, `clone_acceptor()` returns cloned acceptor. This is correct - the architecture document is simply describing the struct, not specifying exact method signatures.

## Issues Found

### 1. CommandExecutor validates but ignores env parameter

**Location**: `command.rs:146-149`

```rust
// Security: Do not allow custom environment variables
if env.is_some() {
    return Err("Custom environment variables are not allowed".to_string());
}
```

**Issue**: The `env` field in `CommandMessage::Execute` is accepted in the protocol, but then rejected at execution time. This is inconsistent - the PSK generation at `command.rs:250-255` doesn't use `env` either.

**Impact**: Low - it's a security measure, but wastes bandwidth sending env that will be rejected.

### 2. Worker registration capabilities mismatch

**Location**: `worker.rs:115-123`

Worker advertises capabilities as:
```rust
vec![
    "PortScan".to_string(),
    "ServiceFingerprint".to_string(),
    ...
]
```

But `TaskType` enum (`mod.rs:58-67`) uses:
```rust
pub enum TaskType {
    PortScan,
    ServiceFingerprint,
    ...
}
```

These don't match - the string "PortScan" doesn't match the enum variant `TaskType::PortScan`. The capabilities should probably be derived from `TaskType` variants.

**Impact**: Medium - registration appears to work but capabilities advertised to coordinator don't match actual task types.

### 3. No Arc::try_unwrap usage observed

The architecture document mentions using `map_err` instead of `expect()` for `Arc::try_unwrap`. However, I didn't find any `Arc::try_unwrap` usage in the distributed module. This is fine if there's no need for this pattern, but it's worth noting for consistency.

### 4. remote.rs rate_limits locking

**Location**: `remote.rs:127-146`

```rust
async fn check_rate_limit(
    rate_limits: &Arc<RwLock<FxHashMap<String, Vec<Instant>>>>,
    ip: &str,
    limit: u32,
) -> bool {
    let mut limits = rate_limits.write().await;
    ...
}
```

**Issue**: This holds the write lock for the entire rate limit check and update. Under high load, this could cause lock contention.

**Impact**: Medium under high connection rates.

## Recommendations

1. **CommandMessage env**: Either remove the `env` field from `CommandMessage::Execute` or allow specific safe environment variables (like `RUST_LOG`).

2. **Worker capabilities**: Derive string capabilities from `TaskType` enum to ensure consistency.

3. **Rate limiting**: Consider using a more efficient rate limiting approach (like token bucket with atomic operations) if lock contention becomes an issue.

## Conclusion

The distributed module is largely well-implemented and matches the architecture document. The key fixes from 2026-05-22 (queue.rs dequeue and heartbeat issues) are correctly applied. The main concerns are the Worker capabilities mismatch and the env handling in CommandExecutor.