# Slapper Distributed Skill

Distributed computing module workflows and patterns for cluster-based testing.

## Key Types and Patterns

### Worker Configuration
`Worker::new(config, psk)` requires both `WorkerConfig` and a PSK string:
```rust
let worker = Worker::new(config, "your-secret-psk".to_string());
let mut worker = worker;
worker.start().await?;
```

### TLS
`distributed/io.rs` has `StreamWrapper` enum:
- `Plain` - Unencrypted stream
- `TlsClient` - TLS client connection
- `TlsServer` - TLS server connection

### TlsServer
`TlsServer::from_pem(cert_path, key_path)` loads PEM cert + key files.

### TlsClient
`TlsClient::new(domain)` creates client with `NoVerifier` (insecure, for internal use).

### Worker Registration Protocol

Workers register with coordinators using TCP line-based JSON (NOT HTTP):

```rust
// Worker side
let client = RemoteClient::new_plaintext(psk);
client.register_worker(host, port, worker_id, hostname, capabilities).await?;

// Coordinator expects CommandMessage::Register { id, hostname, capabilities }
```

Heartbeats also use the same protocol:
```rust
client.send_heartbeat(host, port, worker_id, status).await?;
```

**Important**: Coordinator URL format is `host:port` (no http:// prefix).

## Bugs Fixed

### 2026-05-28 (Wave 1 & 2)

| File | Issue | Fix |
|------|-------|-----|
| `worker.rs:166-182` | Task results never sent to coordinator (CRITICAL) | Added `RemoteClient::send_result()` to send `CommandMessage::Result` back |
| `worker.rs:55-82,150-156` | WorkerStats never updated, heartbeat hardcoded zeros | Changed to `Arc<Mutex<WorkerStats>>`, heartbeat reports actual values |
| `worker.rs:93-104` | Worker registration flow incomplete | Verified registration works correctly |
| `worker.rs:64-104` | No graceful worker shutdown | Added `watch::Sender<bool>` channel and `shutdown()` method |
| `remote.rs:207-211` | Connection panics silently lost | Captured `JoinHandle` and log panics |
| `remote.rs:121-140` | Rate limit entries never cleaned | Added periodic cleanup task |

### 2026-05-28

| File | Issue | Fix |
|------|-------|-----|
| `worker.rs:115-123` | Worker advertised hardcoded string capabilities | Created `worker_capabilities()` helper deriving from `TaskType` enum |
| `command.rs:146-149` | `env` field rejected without explanation | Added clarifying comment for intentional security rejection |

### 2026-05-22

| File | Issue | Fix |
|------|-------|-----|
| `queue.rs:57` | `dequeue()` ignored `worker_id` param and didn't set `assigned_at_secs` | Now properly tracks which worker owns task and when |
| `worker.rs:132-161` | Heartbeat used HTTP POST to non-existent API endpoint | Changed to use `RemoteClient::send_heartbeat()` via TCP |

## Performance Improvements (2026-05-22)

| File | HashMap Type | Reason |
|------|-------------|--------|
| `queue.rs:13` | `Task.payload` | Changed from `std::collections::HashMap` to `FxHashMap` for performance |
| `command.rs:36` | `CommandMessage::Execute.env` | Changed from `HashMap` to `FxHashMap` for performance |
| `remote.rs:30` | `RemoteListener.rate_limits` | Changed from `HashMap` to `FxHashMap` for performance |

Note: `Task::payload` is serializable with `#[serde(default)]` for backward compatibility.

## Task Lifecycle

1. `TaskQueue::enqueue(task)` - Add task to pending queue
2. Worker sends `CommandMessage::RequestTasks` when idle (every 5s)
3. Coordinator handles by calling `TaskQueue::dequeue(worker_id)` for each requested task
4. Coordinator responds with `CommandMessage::AssignTasks { tasks }`
5. Worker feeds tasks into internal processing channel
6. `TaskQueue::complete(result)` - Moves task to completed, removes from in_progress
7. `TaskQueue::reassign_stale_tasks(timeout_secs)` - Returns tasks stale > timeout to pending

## Message Protocol

| Message | Direction | Purpose |
|---------|-----------|---------|
| `Register` | Worker → Coordinator | Register worker with capabilities |
| `Heartbeat` | Worker → Coordinator | Periodic status update |
| `RequestTasks` | Worker → Coordinator | Worker requests available tasks |
| `AssignTasks` | Coordinator → Worker | Response with dequeued tasks |
| `Result` | Worker → Coordinator | Task completion result |
| `Execute` | Coordinator → Worker | Remote command execution |

## Command Execution Security

`CommandExecutor` in `command.rs` enforces:
- Only `slapper` binary allowed
- Max 50 arguments, 1000 chars each
- Forbidden patterns: `../`, path traversal, sensitive paths (`/etc/`, `~/.ssh/`, `.pem`, `.key`)
- No custom environment variables
- 10MB output limit

## Testing

### Running Distributed Tests
```bash
cargo test --lib -p slapper distributed::
```

### Writing Tests
Follow existing test patterns in `distributed/` modules, testing TLS stream handling and cluster communication.

## Common Tasks

### Adding TLS Support for New Stream Type
1. Update `StreamWrapper` enum in `distributed/io.rs` if needed
2. Implement TLS logic using `TlsServer` or `TlsClient`
3. Use `NoVerifier` only for internal, insecure connections
4. Add tests for new stream type

### Implementing Worker Registration
1. Parse coordinator URL to get host:port
2. Create `RemoteClient::new_plaintext(psk)`
3. Call `register_worker()` with worker_id, hostname, capabilities
4. Call `send_heartbeat()` periodically with status updates

## Resources
- `crates/slapper/src/distributed/AGENTS.override.md` - Detailed distributed patterns
- `AGENTS.md` - General project guidelines
- `architecture/distributed.md` - Module architecture docs