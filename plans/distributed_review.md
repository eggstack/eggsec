# Distributed Module Architecture Review

## Summary

The distributed module (`crates/slapper/src/distributed/`) mostly matches the documented architecture in `architecture/distributed.md`. Key components are correctly implemented, but there are issues with the TaskQueue error handling return type and a syntax error in worker.rs.

## Verified Correct

| Claim | Implementation | Status |
|-------|----------------|--------|
| `Task::payload` uses `FxHashMap` | `queue.rs:13` - `FxHashMap<String, serde_json::Value>` | ✅ |
| `CommandMessage::Execute.env` uses `FxHashMap` | `command.rs:37` - `FxHashMap<String, String>` | ✅ |
| `RemoteListener.rate_limits` uses `FxHashMap` | `remote.rs:30` - `FxHashMap<String, Vec<Instant>>` | ✅ |
| `TaskQueue` thread-safe with `Arc<RwLock>` | `queue.rs:29-34` | ✅ |
| `dequeue()` sets `worker_id` and `assigned_at_secs` | `queue.rs:65-66` | ✅ |
| `dequeue()` returns `Result<Option<Task>, QueueError>` | `queue.rs:57` - returns `Result<Option<Task>, QueueError>` | ✅ |
| TaskType enum with 7 task types | `mod.rs:58-67` - PortScan, ServiceFingerprint, EndpointDiscovery, Fuzz, WafTest, LoadTest, Recon | ✅ |
| `TlsServer::from_pem` | `io.rs:115-161` | ✅ |
| `TlsClient::new` | `io.rs:172-191` | ✅ |
| `StreamWrapper` enum | `io.rs:19-23` | ✅ |
| `LineWriter` for line-based I/O | `io.rs:306-340` | ✅ |
| `generate_psk` function | `command.rs:250-255` | ✅ |
| PSK authentication via `ct_eq` | `remote.rs:266` - uses constant-time comparison | ✅ |
| Worker registration via TCP | `worker.rs:102-128` - `RemoteClient::register_worker()` | ✅ |

## Bugs Found

| Priority | Issue | Location |
|----------|-------|----------|
| P1 | Syntax error: stray `pub` keyword before `struct Worker` | `worker.rs:60` |
| P2 | Heartbeat creates new TCP connection per heartbeat | `worker.rs:159` - `RemoteClient::new_plaintext()` called in loop |

## Recommended Fixes

### 1. Fix Syntax Error in worker.rs

```rust
// worker.rs:60 - remove stray `pub`
    pub struct Worker {
```

### 2. Optimize Heartbeat Connection Handling

Currently each heartbeat creates a new TCP connection:

```rust
// worker.rs:158-159
let client = RemoteClient::new_plaintext(psk.clone());
if let Err(e) = client.send_heartbeat(host, port, worker_id.clone(), status.to_string()).await {
```

Consider maintaining a persistent connection for heartbeats or at least document this as a known inefficiency.

## Discrepancies

| Item | Documented | Actual |
|------|-----------|--------|
| `generate_psk` line range | `command.rs:249-254` | `command.rs:250-255` (off-by-one) |
| Bug fix: dequeue returns Result | "Now properly tracks which worker owns task and when" | ✅ Implemented |
| Bug fix: heartbeat via TCP | "Changed to use `RemoteClient::send_heartbeat()` via TCP" | ✅ Implemented |
| `TaskQueue` max_size | Not explicitly documented but used in `new()` | ✅ Implemented |

## Architecture Claims vs Implementation

### Task Lifecycle

| Step | Documented | Implementation | Status |
|------|-----------|----------------|--------|
| Enqueue | `TaskQueue::enqueue(task)` | `queue.rs:46-55` | ✅ |
| Dequeue | `TaskQueue::dequeue(worker_id)` sets `worker_id` and `assigned_at_secs` | `queue.rs:57-72` - properly sets both fields | ✅ |
| Execute | Workers execute tasks locally | `worker.rs:190-220` | ✅ |
| Complete | `TaskQueue::complete(result)` | `queue.rs:100-116` | ✅ |
| Reassign | `TaskQueue::reassign_stale_tasks(timeout_secs)` | `queue.rs:74-98` | ✅ |

### Key Components Table

The architecture document's component table line numbers are mostly accurate with minor discrepancies:

| Component | Documented Lines | Actual Lines | Notes |
|-----------|----------------|--------------|-------|
| TaskType enum | 59-67 | mod.rs:58-67 | ✅ |
| Task struct | 7-18 | queue.rs:7-18 | ✅ |
| TaskResult struct | 20-27 | queue.rs:20-27 | ✅ |
| TaskQueue | 29-154 | queue.rs:29-154 | ✅ |
| RemoteListener | 26-388 | remote.rs:26-388 | ✅ |
| RemoteClient | 395-697 | remote.rs:395-697 | ✅ |
| CommandExecutor | 103-220 | command.rs:103-220 | ✅ |
| TlsServer | 110-161 | io.rs:110-161 | ✅ |
| TlsClient | 163-225 | io.rs:163-225 | ✅ |
| StreamWrapper | 19-108 | io.rs:19-108 | ✅ |
| LineWriter | 306-340 | io.rs:306-340 | ✅ |
| generate_psk | 249-254 | command.rs:250-255 | Off-by-one |

## Notes

- The bug fixes from 2026-05-22 are correctly implemented (queue.rs:57, worker.rs:132-161)
- PSK generation uses `rand::Rng` and `hex::encode` correctly
- Rate limiting implementation in `remote.rs:127-146` is correct
- TLS support via `insecure-tls` feature flag is properly conditional