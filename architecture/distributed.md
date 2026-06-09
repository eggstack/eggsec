# Distributed Module

Eggsec can be deployed in a distributed architecture to perform large-scale security assessments by distributing tasks across multiple worker nodes.

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
- **Task Execution**: Workers receive tasks, execute them locally using the core Eggsec engine, and report results back.

#### Worker Helpers

- **`parse_coordinator_url(url: &str)`** (`worker.rs:11-31`): Parses `host:port` URLs, stripping `http://`/`https://` prefixes. Returns `Result<(&str, u16)>`.
- **`worker_capabilities()`** (`worker.rs:33-35`): Returns `CAPABILITIES` as `Vec<String>` for registration messages.

### Worker Status (`mod.rs:104-109`)

```rust
pub enum WorkerStatus {
    Idle,
    Busy,
    Disconnected,
}
```

Used in `WorkerRegistration.status` and `Heartbeat.status` to report worker state to the coordinator.

### Heartbeat (`mod.rs:111-120`)

```rust
pub struct Heartbeat {
    pub worker_id: String,
    pub status: WorkerStatus,
    pub current_jobs: usize,
    pub completed_jobs: usize,
    pub failed_jobs: usize,
    pub cpu_usage: f32,
    pub memory_usage: f32,
}
```

Workers send periodic heartbeats to the coordinator reporting their current load. The coordinator uses this data for task assignment decisions.

### WorkerConfig (`worker.rs:37-54`)

```rust
pub struct WorkerConfig {
    pub worker_id: String,            // Default: random UUID
    pub coordinator_url: String,      // Default: "http://localhost:8080"
    pub max_concurrency: usize,       // Default: 10
    pub heartbeat_interval_secs: u64, // Default: 30
}
```

Configuration for a worker node. Uses `Default` trait for sensible defaults.

### WorkerStats (`worker.rs:56-63`)

```rust
pub struct WorkerStats {
    pub worker_id: String,
    pub tasks_completed: u64,
    pub tasks_failed: u64,
    pub tasks_in_progress: usize,
    pub last_heartbeat_secs: i64,
}
```

Runtime statistics tracked by the worker and included in heartbeat messages.

### Communication (`remote.rs`, `io.rs`)

Secure and efficient communication between nodes using line-based JSON over TCP (not gRPC or HTTP).

- **Authentication**: PSK-based authentication ensures only authorized workers can join the cluster.
- **Encryption**: TLS encryption support (with `insecure-tls` feature for testing).
- **Line-based Protocol**: Messages are newline-delimited JSON for simple, efficient communication.
- **Real-time Updates**: Status updates and findings are streamed back to the coordinator as they happen.

#### IP Allowlist (`remote.rs:34,70-83`)

`RemoteListener` supports an optional IP allowlist (`ip_allowlist: Option<Vec<String>>`). When set via `with_allowlist()`, only connections from IPs matching the allowlist are accepted. Supports both individual IP addresses and CIDR ranges (via `ipnetwork::IpNetwork`). Non-matching connections are rejected with a warning log before the connection is fully established.

#### Connection Limits (`remote.rs:17,209-213`)

Default max connections: `MAX_CONNECTIONS = 100` (`remote.rs:17`). Configurable via `with_config()`. When the current connection count reaches `max_connections`, new connections are rejected with a warning log. Connections are tracked in `Arc<RwLock<FxHashSet<String>>>` and cleaned up on disconnect via `FxHashSet::remove()`.

#### Rate Limiting (`remote.rs:18-19,121-140`)

Default rate limit: `RATE_LIMIT_PER_MINUTE = 60` per IP (`remote.rs:18`). Window: `RATE_LIMIT_WINDOW_SECS = 60` seconds (`remote.rs:19`). Implemented via `check_rate_limit()` which maintains per-IP timestamp vectors in `FxHashMap<String, Vec<Instant>>`. A periodic cleanup task removes stale entries every 60 seconds (`remote.rs:180-193`).

#### DNS Caching (`remote.rs:514-532`)

`RemoteClient` caches DNS resolutions for 60 seconds (`cached_addr: Option<(SocketAddr, Instant)>`). The `resolve_cached()` method returns a cached address if within TTL, avoiding repeated DNS lookups. Cached addresses are not re-validated for reachability — connection failures are handled by the caller, which falls back to fresh resolution on the next attempt.

### ResponseMessage Type (`command.rs:74-86`)

```rust
pub struct ResponseMessage {
    pub id: String,
    #[serde(rename = "type")]
    pub msg_type: String,          // see table below
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "duration_ms")]
    pub duration_ms: Option<u64>,
    pub hostname: Option<String>,
    pub capabilities: Option<Vec<String>>,
}
```

**msg_type values:**

| Value | Context | Set by |
|-------|---------|--------|
| `"response"` | Generic success/error response | `success()` and `error()` constructors |
| `"authenticated"` | Welcome after PSK auth | `handle_connection()` |
| `"registered"` | Confirmation after worker registration | `registration()` constructor |
| `"heartbeat_ack"` | Heartbeat acknowledgment | Heartbeat handler |
| `"result_ack"` | Task result acknowledgment | Result handler |
| `"tasks_assigned"` | Task assignment response | RequestTasks handler |
| `"enqueue_ack"` | Task enqueue confirmation | EnqueueTask handler |
| `"status"` | Status query response | StatusRequest handler |

Constructors: `success(id, output, duration_ms)`, `error(id, error, duration_ms)`, `registration(id, hostname, capabilities)`.

## Worker Capabilities (`mod.rs:83-91`)

The `CAPABILITIES` constant defines the set of task types that distributed workers can advertise support for:

```rust
pub const CAPABILITIES: &[&str] = &[
    "PortScan",
    "ServiceFingerprint",
    "EndpointDiscovery",
    "Fuzz",
    "WafTest",
    "LoadTest",
    "Recon",
];
```

When workers register with the coordinator via `CommandMessage::Register`, they include their capabilities. The coordinator uses these capabilities to assign appropriate tasks to each worker based on what the worker is capable of performing. The `WorkerRegistration` struct at `mod.rs:93-102` stores the worker's reported capabilities alongside its `worker_id`, `hostname`, `max_concurrency`, `status`, and `last_heartbeat_secs`.

### Capability Matching

| Capability | TaskType | Description |
|------------|----------|-------------|
| `PortScan` | `PortScan` | TCP/UDP port scanning |
| `ServiceFingerprint` | `ServiceFingerprint` | Service/version detection |
| `EndpointDiscovery` | `EndpointDiscovery` | HTTP endpoint discovery |
| `Fuzz` | `Fuzz` | Fuzzing engine |
| `WafTest` | `WafTest` | WAF detection and bypass |
| `LoadTest` | `LoadTest` | HTTP load testing |
| `Recon` | `Recon` | Reconnaissance |

## Key Components

| Component | File | Lines | Key Function/Type |
|-----------|------|-------|-------------------|
| TaskType enum | mod.rs | 58-67 | 7 task types |
| CAPABILITIES constant | mod.rs | 83-91 | 7 capability strings |
| WorkerRegistration | mod.rs | 93-102 | Worker metadata with capabilities |
| WorkerStatus enum | mod.rs | 104-109 | Idle, Busy, Disconnected |
| Heartbeat | mod.rs | 111-120 | Worker liveness and load report |
| Task struct | queue.rs | 7-18 | Core task representation |
| TaskResult struct | queue.rs | 20-27 | Task execution result |
| TaskQueue | queue.rs | 29-152 | Thread-safe task queue |
| QueueError | queue.rs | 154-169 | Queue error types |
| RemoteListener | remote.rs | 27-615 | Coordinator server |
| RemoteClient | remote.rs | 622-1166 | Worker client |
| CommandExecutor | command.rs | 129-252 | Sandboxed command execution |
| CommandMessage | command.rs | 28-72 | Protocol messages (8 variants) |
| ResponseMessage | command.rs | 74-127 | Coordinator responses |
| RemoteResult | command.rs | 254-279 | Typed remote execution result |
| Worker | worker.rs | 65-338 | Worker node with task processing |
| WorkerConfig | worker.rs | 37-54 | Worker configuration with defaults |
| WorkerStats | worker.rs | 56-63 | Runtime statistics |
| parse_coordinator_url | worker.rs | 11-31 | URL parsing helper |
| worker_capabilities | worker.rs | 33-35 | Returns CAPABILITIES as Vec<String> |
| TlsServer | io.rs | 110-161 | TLS server from PEM |
| TlsClient | io.rs | 163-225 | TLS client (insecure-tls feature) |
| StreamWrapper | io.rs | 19-108 | Unified stream enum |
| LineWriter | io.rs | 306-340 | Line I/O wrapper |
| generate_psk | command.rs | 281-286 | 32-byte hex PSK generation |

### CommandMessage Variants

| Variant | Fields | Direction | Purpose |
|---------|--------|-----------|---------|
| `Execute` | `id`, `command`, `timeout`, `env` | Coordinator → Worker | Execute a eggsec command on the worker |
| `Register` | `id`, `hostname`, `capabilities` | Worker → Coordinator | Worker self-registration on startup |
| `Heartbeat` | `id`, `status` | Worker → Coordinator | Periodic liveness and status report |
| `Result` | `id`, `result` | Worker → Coordinator | Task execution result with output/error |
| `RequestTasks` | `id`, `worker_id`, `max_tasks` | Worker → Coordinator | Worker requests tasks from the queue |
| `AssignTasks` | `id`, `tasks` | Coordinator → Worker | Coordinator assigns tasks to the worker |
| `EnqueueTask` | `id`, `task` | Client → Coordinator | Push a task into the coordinator's queue |
| `StatusRequest` | `id` | Client → Coordinator | Query worker registry and queue status |

### RemoteListener Public Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `new(psk: String) -> Self` | Create with default settings (100 max conn, 60/min rate limit) |
| `with_config` | `with_config(psk, max_connections, rate_limit) -> Self` | Create with custom connection/rate limits |
| `with_allowlist` | `with_allowlist(psk, allowlist: Vec<String>) -> Self` | Create with IP allowlist |
| `with_tls` | `with_tls(psk, tls_config: TlsConfig) -> Result<Self>` | Create with TLS support |
| `new_plaintext` | `new_plaintext(psk) -> Self` | Alias for `new()` |
| `start` | `start(port: u16) -> Result<()>` | Start listening on port (blocking) |
| `shutdown` | `shutdown(&self)` | Signal graceful shutdown |
| `get_workers` | `get_workers() -> Vec<WorkerRegistration>` | Get all registered workers |
| `get_queue_counts` | `get_queue_counts() -> (usize, usize, usize)` | Returns (pending, in_progress, completed) |
| `connection_count` | `connection_count() -> usize` | Current active connections |
| `is_tls` | `is_tls() -> bool` | Whether TLS is enabled |

### RemoteClient Public Methods

| Method | Signature | Description |
|--------|-----------|-------------|
| `new` | `new(psk: String) -> Self` | Create plaintext client |
| `with_tls` | `with_tls(psk, domain: &str) -> Result<Self>` | Create TLS client (insecure-tls feature) |
| `new_plaintext` | `new_plaintext(psk) -> Self` | Alias for `new()` |
| `register_worker` | `register_worker(host, port, worker_id, hostname, capabilities) -> Result<()>` | Register with coordinator |
| `send_heartbeat` | `send_heartbeat(host, port, worker_id, status) -> Result<()>` | Send heartbeat to coordinator |
| `send_result` | `send_result(host, port, result: TaskResult) -> Result<()>` | Submit task result |
| `request_tasks` | `request_tasks(host, port, worker_id, max_tasks) -> Result<Vec<Task>>` | Request tasks from queue |
| `execute` | `execute(host, port, command, timeout) -> Result<RemoteResult>` | Remote command execution |
| `request_status` | `request_status(host, port) -> Result<serde_json::Value>` | Query coordinator status |
| `enqueue_task` | `enqueue_task(host, port, task: Task) -> Result<()>` | Push task to coordinator queue |

### Worker Task Processors (`worker.rs:340-753`)

The worker dispatches tasks to type-specific processors via `process_task()`:

| Processor | Function | Description |
|-----------|----------|-------------|
| PortScan | `process_port_scan()` | Scans ports using `scanner::ports::scan_ports()` |
| ServiceFingerprint | `process_fingerprint()` | Fingerprints services via `scanner::fingerprint::fingerprint_services()` |
| EndpointDiscovery | `process_endpoints()` | Discovers endpoints via `scanner::endpoints::scan_endpoints()` |
| Fuzz | `process_fuzz()` | Runs fuzzing engine via `fuzzer::engine::FuzzEngine` |
| WafTest | `process_waf()` | Tests WAF bypass via `waf::run_cli()` |
| LoadTest | `process_load_test()` | HTTP load testing via `loadtest::run_cli()` |
| Recon | `process_recon()` | Reconnaissance via `recon::run_cli()` |

Each processor extracts parameters from `Task.payload` (an `FxHashMap<String, serde_json::Value>`) and invokes the corresponding Eggsec engine module.

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
