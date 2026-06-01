# Distributed Module

Slapper can be deployed in a distributed architecture to perform large-scale security assessments by distributing tasks across multiple worker nodes.

## Cluster Architecture (`src/distributed/`)

### Coordinator

The central node that manages the cluster, assigns tasks, and aggregates results.

- **Queue Management (`queue.rs`)**: A reliable task queue that ensures each task is assigned and completed successfully.
- **Worker Management**: Tracks the health and capacity of all registered worker nodes.
- **Command Dispatch (`command.rs`)**: Sends high-level instructions to workers.

### Worker (`worker.rs`)

Independent nodes that perform the actual scanning and fuzzing tasks.

- **Self-Registration**: Workers automatically register with the coordinator on startup.
- **Resource Monitoring**: Workers report their current load and availability to the coordinator.
- **Task Execution**: Workers receive tasks, execute them locally using the core Slapper engine, and report results back.

### Communication (`remote.rs`, `io.rs`)

Secure and efficient communication between nodes using line-based JSON over TCP (not gRPC or HTTP).

- **Authentication**: PSK-based authentication ensures only authorized workers can join the cluster.
- **Encryption**: TLS encryption support (with `insecure-tls` feature for testing).
- **Line-based Protocol**: Messages are newline-delimited JSON for simple, efficient communication.
- **Real-time Updates**: Status updates and findings are streamed back to the coordinator as they happen.

#### IP Allowlist (`remote.rs:34,70-83`)

`RemoteListener` supports an optional IP allowlist (`ip_allowlist: Option<Vec<String>>`). When set via `with_allowlist()`, only connections from IPs matching the allowlist are accepted. Supports both individual IP addresses and CIDR ranges (via `ipnetwork::IpNetwork`). Non-matching connections are rejected with a warning log before the connection is fully established.

#### Connection Limits (`remote.rs:17,209-213`)

Default max connections: `MAX_CONNECTIONS = 100` (`remote.rs:17`). Configurable via `with_config()`. When the current connection count reaches `max_connections`, new connections are rejected with a warning log. Connections are tracked in `Arc<RwLock<Vec<String>>>` and cleaned up on disconnect.

#### Rate Limiting (`remote.rs:18-19,121-140`)

Default rate limit: `RATE_LIMIT_PER_MINUTE = 60` per IP (`remote.rs:18`). Window: `RATE_LIMIT_WINDOW_SECS = 60` seconds (`remote.rs:19`). Implemented via `check_rate_limit()` which maintains per-IP timestamp vectors in `FxHashMap<String, Vec<Instant>>`. A periodic cleanup task removes stale entries every 60 seconds (`remote.rs:180-193`).

#### DNS Caching (`remote.rs:514-532`)

`RemoteClient` caches DNS resolutions for 60 seconds (`cached_addr: Option<(SocketAddr, Instant)>`). The `resolve_cached()` method returns a cached address if within TTL, avoiding repeated DNS lookups. Cached addresses are not re-validated for reachability — connection failures are handled by the caller, which falls back to fresh resolution on the next attempt.

### ResponseMessage Type (`command.rs:65-118`)

```rust
pub struct ResponseMessage {
    pub id: String,
    pub msg_type: String,          // "response", "authenticated", "registered", "heartbeat_ack", "result_ack", "tasks_assigned"
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    pub duration_ms: Option<u64>,
    pub hostname: Option<String>,
    pub capabilities: Option<Vec<String>>,
}
```

Constructors: `success(id, output, duration_ms)`, `error(id, error, duration_ms)`, `registration(id, hostname, capabilities)`.

## Key Components

| Component | File | Lines | Key Function/Type |
|-----------|------|-------|-------------------|
| TaskType enum | mod.rs | 59-67 | 7 task types |
| Task struct | queue.rs | 8-18 | Core task representation |
| TaskResult struct | queue.rs | 21-27 | Task execution result |
| TaskQueue | queue.rs | 29-154 | Thread-safe task queue |
| QueueError | queue.rs | 155-169 | Queue error types |
| RemoteListener | remote.rs | 27-467 | Coordinator server |
| RemoteClient | remote.rs | 474-941 | Worker client |
| CommandExecutor | command.rs | 120-243 | Secure command execution |
| CommandMessage | command.rs | 30-63 | Protocol messages (6 variants) |
| Worker | worker.rs | 65-708 | Worker node |
| TlsServer | io.rs | 110-161 | TLS server from PEM |
| TlsClient | io.rs | 163-225 | TLS client |
| StreamWrapper | io.rs | 19-108 | Unified stream enum |
| LineWriter | io.rs | 306-340 | Line I/O wrapper |
| generate_psk | command.rs | 272-277 | PSK generation |

### CommandMessage Variants

| Variant | Fields | Direction | Purpose |
|---------|--------|-----------|---------|
| `Execute` | `id`, `command`, `timeout`, `env` | Coordinator → Worker | Execute a slapper command on the worker |
| `Register` | `id`, `hostname`, `capabilities` | Worker → Coordinator | Worker self-registration on startup |
| `Heartbeat` | `id`, `status` | Worker → Coordinator | Periodic liveness and status report |
| `Result` | `id`, `result` | Worker → Coordinator | Task execution result with output/error |
| `RequestTasks` | `id`, `worker_id`, `max_tasks` | Worker → Coordinator | Worker requests tasks from the queue |
| `AssignTasks` | `id`, `tasks` | Coordinator → Worker | Coordinator assigns tasks to the worker |

## Task Lifecycle

1. **Enqueue**: Tasks are added via `TaskQueue::enqueue(task)`
2. **Dequeue**: Workers claim tasks via `TaskQueue::dequeue(worker_id)` which sets `worker_id` and `assigned_at_secs`
3. **Execute**: Workers execute tasks locally
4. **Complete**: Results are submitted via `TaskQueue::complete(result)`
5. **Reassign**: Stale tasks (timeout exceeded) are returned to pending via `TaskQueue::reassign_stale_tasks(timeout_secs)`

## Benefits

- **Scalability**: Easily handle thousands of targets by adding more worker nodes.
- **Resilience**: If a worker fails, its tasks are automatically reassigned to other nodes.
- **Geographic Distribution**: Deploy workers in different regions to test from multiple perspectives.

## Bugs Fixed (2026-05-22)

| File | Issue | Fix |
|------|-------|-----|
| `queue.rs:57` | `dequeue()` ignored `worker_id` param and didn't set `assigned_at_secs` | Now properly tracks which worker owns task and when |
| `queue.rs:57` | `dequeue()` returned `Option<Task>` silently dropped errors | Changed to return `Result<Option<Task>, QueueError>` for explicit error handling |
| `worker.rs:132-161` | Heartbeat used HTTP POST to non-existent REST API endpoint | Changed to use `RemoteClient::send_heartbeat()` via TCP |

## Performance Improvements (2026-05-22)

| File | HashMap Type | Reason |
|------|-------------|--------|
| `queue.rs:13` | `Task.payload` | Changed from `std::collections::HashMap` to `FxHashMap` for performance |
| `command.rs:36` | `CommandMessage::Execute.env` | Changed from `HashMap` to `FxHashMap` for performance |
| `remote.rs:30` | `RemoteListener.rate_limits` | Changed from `HashMap` to `FxHashMap` for performance |

Note: `Task::payload` uses `#[serde(default)]` for backward compatibility with serialized data.
