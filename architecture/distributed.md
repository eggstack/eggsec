# Distributed Module

Provides a coordinator/worker cluster architecture for distributing security scanning tasks across multiple nodes, with PSK-authenticated TLS connections, a pull-based task queue, heartbeat monitoring, and result aggregation.

## Role & Responsibilities

The distributed module enables horizontal scaling of security assessments:

1. **Coordinator** (`RemoteListener`): Accepts worker connections, authenticates via PSK, manages a shared `TaskQueue`, dispatches tasks on pull request, aggregates results, and tracks worker liveness.
2. **Worker** (`Worker`): Self-registers with the coordinator, periodically requests tasks, processes them locally through the Eggsec engine, and reports results back.
3. **Communication** (`RemoteClient`/`RemoteListener`): Line-based JSON over TCP with optional TLS, including rate limiting, connection limits, and IP allowlisting.

## Location & Feature Gating

| Item | Path | Feature gate |
|------|------|--------------|
| Module root | `crates/eggsec/src/distributed/mod.rs` | None (always compiled) |
| Protocol messages | `crates/eggsec/src/distributed/command.rs` | None |
| Task queue | `crates/eggsec/src/distributed/queue.rs` | None |
| Coordinator server | `crates/eggsec/src/distributed/remote.rs` | None |
| I/O (TLS, line protocol) | `crates/eggsec/src/distributed/io.rs` | None |
| Worker node | `crates/eggsec/src/distributed/worker.rs` | None |
| CLI cluster command | `crates/eggsec/src/commands/handlers/cluster.rs` | `cli` |

**No feature gate**: the entire distributed module compiles unconditionally. Task processing in `worker.rs` uses `EnforcementContext` and `EnforcedDispatcher` only when `tool-api`, `rest-api`, or `grpc-api` is enabled; without those features, `process_task()` returns an error (`worker.rs:517-523`).

## Architecture

### Protocol Message Inventory

#### CommandMessage Variants (`command.rs:28-70`)

8 variants (tagged union via `#[serde(tag = "type")]`):

| # | Variant | Direction | Fields |
|---|---------|-----------|--------|
| 1 | `Execute` | Coordinator → Worker | `id`, `command: Vec<String>`, `timeout: Option<u64>`, `env: Option<FxHashMap>` |
| 2 | `Register` | Worker → Coordinator | `id`, `hostname`, `capabilities: Vec<String>` |
| 3 | `Heartbeat` | Worker → Coordinator | `id`, `status: String` |
| 4 | `Result` | Worker → Coordinator | `id`, `result: TaskResult` |
| 5 | `RequestTasks` | Worker → Coordinator | `id`, `worker_id`, `max_tasks: usize` |
| 6 | `AssignTasks` | Coordinator → Worker | `id`, `tasks: Vec<Task>` |
| 7 | `EnqueueTask` | Client → Coordinator | `id`, `task: Task` |
| 8 | `StatusRequest` | Client → Coordinator | `id` |

#### ResponseMessage (`command.rs:72-125`)

8 `msg_type` values:

| msg_type | Set by | Context |
|----------|--------|---------|
| `"response"` | `success()` / `error()` | Generic success/error |
| `"authenticated"` | `handle_connection()` | Welcome after PSK auth |
| `"registered"` | `registration()` | Worker registration confirmation |
| `"heartbeat_ack"` | Heartbeat handler | Heartbeat acknowledgment |
| `"result_ack"` | Result handler | Task result acknowledgment |
| `"tasks_assigned"` | RequestTasks handler | Task assignment response |
| `"enqueue_ack"` | EnqueueTask handler | Task enqueue confirmation |
| `"status"` | StatusRequest handler | Status query response |

### Component Inventory

#### RemoteListener (`remote.rs:27-39`)

```rust
pub struct RemoteListener {
    psk: String,
    shutdown_tx: broadcast::Sender<()>,
    connections: Arc<RwLock<FxHashSet<String>>>,
    rate_limits: Arc<RwLock<FxHashMap<String, Vec<Instant>>>>,
    max_connections: usize,
    rate_limit: u32,
    ip_allowlist: Option<Vec<String>>,
    tls_server: Option<Arc<TlsServer>>,
    plaintext_allowed: bool,
    task_queue: Arc<TaskQueue>,
    workers: Arc<RwLock<FxHashMap<String, WorkerRegistration>>>,
}
```

| Method | Description |
|--------|-------------|
| `new(psk)` | Default: 100 max connections, 60/min rate limit |
| `with_config(psk, max_connections, rate_limit)` | Custom limits |
| `with_allowlist(psk, allowlist)` | IP allowlist (individual IPs + CIDR) |
| `with_tls(psk, tls_config)` | TLS from PEM cert/key |
| `new_plaintext(psk)` | Plaintext with warning log |
| `start(port)` | Blocking accept loop |
| `shutdown()` | Graceful shutdown via broadcast |
| `get_workers()` | All registered workers |
| `get_queue_counts()` | `(pending, in_progress, completed)` |
| `connection_count()` | Current active connections |
| `is_tls()` | Whether TLS is enabled |

#### RemoteClient (`remote.rs:650-1206`)

```rust
pub struct RemoteClient {
    psk: String,
    tls: Option<TlsClient>,
    cached_addr: Option<(SocketAddr, Instant)>,
    plaintext_allowed: bool,
}
```

| Method | Description |
|--------|-------------|
| `new(psk)` | Plaintext client |
| `with_tls(psk, domain)` | TLS client (insecure-tls feature for NoVerifier) |
| `new_plaintext(psk)` | Plaintext with warning |
| `register_worker(host, port, worker_id, hostname, capabilities)` | Register with coordinator |
| `send_heartbeat(host, port, worker_id, status)` | Send heartbeat |
| `send_result(host, port, result)` | Submit task result |
| `request_tasks(host, port, worker_id, max_tasks)` | Pull tasks from queue |
| `execute(host, port, command, timeout)` | Remote command execution |
| `request_status(host, port)` | Query coordinator status |
| `enqueue_task(host, port, task)` | Push task to queue |

#### Worker (`worker.rs:77-91`)

```rust
pub struct Worker {
    config: WorkerConfig,
    stats: Arc<Mutex<WorkerStats>>,
    sender: Option<mpsc::Sender<Task>>,
    receiver: Option<mpsc::Receiver<Task>>,
    heartbeat_handle: Option<JoinHandle<()>>,
    task_request_handle: Option<JoinHandle<()>>,
    task_processor_handle: Option<JoinHandle<()>>,
    psk: String,
    enforcement: Arc<EnforcementContext>,          // tool-api/rest-api/grpc-api
    dispatcher: EnforcedDispatcher,                // tool-api/rest-api/grpc-api
    shutdown_tx: watch::Sender<bool>,
}
```

#### WorkerConfig (`worker.rs:46-54`)

| Field | Default | Description |
|-------|---------|-------------|
| `worker_id` | Random UUID | Unique worker identifier |
| `coordinator_url` | `"http://localhost:8080"` | Coordinator address |
| `max_concurrency` | 10 | Max concurrent tasks |
| `heartbeat_interval_secs` | 30 | Heartbeat interval |
| `tls_domain` | `Some("localhost")` | TLS domain for verification |

#### WorkerStats (`worker.rs:68-75`)

| Field | Type | Description |
|-------|------|-------------|
| `worker_id` | `String` | Worker identifier |
| `tasks_completed` | `u64` | Successfully completed tasks |
| `tasks_failed` | `u64` | Failed tasks |
| `tasks_in_progress` | `usize` | Currently processing |
| `last_heartbeat_secs` | `i64` | Last heartbeat timestamp |

#### WorkerRegistration (`mod.rs:99-108`)

```rust
pub struct WorkerRegistration {
    pub worker_id: String,
    pub hostname: String,
    pub capabilities: Vec<TaskType>,
    pub max_concurrency: usize,
    pub status: WorkerStatus,
    pub last_heartbeat_secs: Option<i64>,
}
```

#### Heartbeat (`mod.rs:117-126`)

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

Note: The `Heartbeat` struct is defined but not directly serialized for the wire protocol. Workers send heartbeat data as a JSON string in `CommandMessage::Heartbeat.status`, containing `worker_id`, `status` (idle/busy), `current_jobs`, `completed_jobs`, and `failed_jobs` (`worker.rs:222-228`).

#### WorkerStatus (`mod.rs:110-115`)

3 variants: `Idle`, `Busy`, `Disconnected`.

#### TaskType (`mod.rs:64-73`)

7 variants: `PortScan`, `ServiceFingerprint`, `EndpointDiscovery`, `Fuzz`, `WafTest`, `LoadTest`, `Recon`.

#### CAPABILITIES (`mod.rs:89-97`)

```rust
pub const CAPABILITIES: &[&str] = &[
    "PortScan", "ServiceFingerprint", "EndpointDiscovery",
    "Fuzz", "WafTest", "LoadTest", "Recon",
];
```

#### Task (`queue.rs:7-18`)

```rust
pub struct Task {
    pub id: String,
    pub job_id: String,
    pub task_type: TaskType,
    pub target: String,
    pub payload: FxHashMap<String, serde_json::Value>,
    pub worker_id: Option<String>,         // set by dequeue()
    pub assigned_at_secs: Option<i64>,     // set by dequeue()
}
```

#### TaskResult (`queue.rs:20-27`)

```rust
pub struct TaskResult {
    pub task_id: String,
    pub success: bool,
    pub output: String,
    pub error: Option<String>,
    pub duration_millis: u64,
}
```

#### TaskQueue (`queue.rs:29-153`)

```rust
pub struct TaskQueue {
    pending: Arc<RwLock<VecDeque<Task>>>,
    in_progress: Arc<RwLock<FxHashMap<String, Task>>>,
    completed: Arc<RwLock<VecDeque<TaskResult>>>,
    max_size: usize,
}
```

#### QueueError (`queue.rs:155-169`)

2 variants: `QueueFull`, `TaskNotFound`.

### I/O Layer (`io.rs`)

#### StreamWrapper (`io.rs:19-108`)

Enum wrapping TCP/TLS streams:
- `Plain(TcpStream)` — unencrypted
- `TlsClient(tokio_rustls::client::TlsStream<TcpStream>)` — client TLS
- `TlsServer(ServerTlsStream<TcpStream>)` — server TLS

Implements `AsyncRead` + `AsyncWrite` by delegating to the inner stream.

#### TlsServer (`io.rs:110-161`)

`TlsServer::from_pem(cert_path, key_path)` loads PEM files, extracts certificates and private key (supports PKCS#8 and PKCS#1), builds a `rustls::ServerConfig` with `with_no_client_auth()`.

#### TlsClient (`io.rs:163-313`)

- With `insecure-tls` feature: `NoVerifier` accepts all certificates (for lab use only).
- Without `insecure-tls`: Uses `webpki_roots::TLS_SERVER_ROOTS` for proper certificate verification.
- Tracks `insecure_connection_count` when using `NoVerifier`.

#### LineWriter (`io.rs:315-349`)

Wraps a `StreamWrapper` with newline-delimited JSON framing:
- `write_line(line)`: writes `line + "\n"`, flushes.
- `read_line()`: reads until `\n`, returns `Option<String>`.

### Security Constants (`command.rs:7-26`)

| Constant | Value | Purpose |
|----------|-------|---------|
| `ALLOWED_COMMANDS` | `["eggsec"]` | Only `eggsec` executable permitted |
| `MAX_OUTPUT_SIZE` | 10 MB (10,485,760) | Max command output size |
| `MAX_ARGS` | 50 | Max command arguments |
| `MAX_ARG_LENGTH` | 1000 | Max characters per argument |
| `MAX_TASKS_PER_REQUEST` | 5 | Max tasks per `request_tasks` call (`worker.rs:18`) |
| `FORBIDDEN_PATTERNS` | 13 patterns | `../`, `..\`, `/etc/`, `/root/`, `/proc/`, `/sys/`, `~/.ssh/`, `~/.aws/`, `.pem`, `.key`, `--config`, `--config-file`, `--credentials` |

### Infrastructure Constants (`remote.rs:17-19`, `eggsec-core/src/constants.rs:27-28`)

| Constant | Value | Location |
|----------|-------|----------|
| `MAX_CONNECTIONS` | 100 | `remote.rs:17` |
| `RATE_LIMIT_PER_MINUTE` | 60 | `remote.rs:18` |
| `RATE_LIMIT_WINDOW_SECS` | 60 seconds | `remote.rs:19` |
| `DEFAULT_TASK_QUEUE_CAPACITY` | 10,000 | `eggsec-core/src/constants.rs:27` |
| `WORKER_STALE_TIMEOUT_SECS` | 90 seconds | `eggsec-core/src/constants.rs:28` |
| DNS cache TTL | 60 seconds | `remote.rs:705` |
| Connect timeout | 5 seconds | `remote.rs:742` |
| Auth response timeout | 10 seconds | `remote.rs:788` |
| Heartbeat response timeout | 5 seconds | `remote.rs:883` |
| Result response timeout | 10 seconds | `remote.rs:928` |
| Task request response timeout | 10 seconds | `remote.rs:975` |
| Worker task processing timeout | 300 seconds | `worker.rs:409-410` |
| Task request polling interval | 5 seconds | `worker.rs:269` |
| Heartbeat interval | 30 seconds (configurable) | `worker.rs:62,198` |
| Stale task reassignment interval | 30 seconds | `remote.rs:237` |

## Behavior / Flow

### Coordinator↔Worker Handshake

```
Worker                                          Coordinator
  │                                                │
  │──── TCP connect ──────────────────────────────→│
  │                                                │ check IP allowlist
  │                                                │ check connection limit (100)
  │                                                │ check rate limit (60/min)
  │                                                │
  │──── TLS handshake (if TLS) ──────────────────→│
  │                                                │
  │──── AuthMessage { psk } ─────────────────────→│
  │                                                │ constant-time PSK compare
  │                                                │
  │←─── ResponseMessage { type: "authenticated" } ─│
  │                                                │
  │──── CommandMessage::Register { ... } ─────────→│
  │                                                │ filter capabilities against CAPABILITIES
  │                                                │ store WorkerRegistration
  │←─── ResponseMessage { type: "registered" } ───│
  │                                                │
  │   [connection stays open for command loop]     │
```

### PSK Authentication (`remote.rs:340-355`)

The PSK comparison uses `subtle::ConstantTimeEq`:

```rust
if !bool::from(auth.psk.as_bytes().ct_eq(psk.as_bytes())) {
    // send error response, return Err
}
```

This prevents timing side-channel attacks on the PSK comparison. The PSK is generated via `generate_psk()` (`command.rs:279-287`): 32 random bytes from `OsRng` encoded as 64-character hex.

### Task Lease Lifecycle

```
1. ENQUEUE:    Client sends EnqueueTask → TaskQueue::enqueue()
2. REQUEST:    Worker sends RequestTasks (every 5s when idle)
3. ASSIGN:     Coordinator calls TaskQueue::dequeue(worker_id) up to MAX_TASKS_PER_REQUEST (5)
               → sets task.worker_id and task.assigned_at_secs
               → responds with AssignTasks containing the tasks
4. PROCESS:    Worker feeds tasks into mpsc channel → spawns per-task processing
               → tokio::time::timeout(300s, process_task())
5. COMPLETE:   Worker sends Result → TaskQueue::complete(result)
               → removes from in_progress, pushes to completed
6. REASSIGN:   Background task runs every 30s
               → TaskQueue::reassign_stale_tasks(90)
               → tasks assigned > 90s ago return to pending with cleared worker_id/assigned_at
```

### Worker Start Flow (`worker.rs:140-152`)

```rust
worker.start().await?;
  ├─ register_with_coordinator()      // TLS + Register message
  ├─ mpsc::channel(100)               // internal task channel
  ├─ start_heartbeat_loop()           // tokio::spawn, periodic heartbeat
  ├─ start_task_request_loop()        // tokio::spawn, periodic task pull
  └─ start_task_processing_loop()     // tokio::spawn, per-task dispatch
```

### Worker Shutdown (`worker.rs:434-464`)

`shutdown()` sends `true` via `watch::Sender<bool>`, then aborts all three `JoinHandle`s (heartbeat, task request, task processor). `Drop` performs the same cleanup defensively.

### Worker Task Processing (`worker.rs:332-427`)

Each task is spawned as an independent tokio task with a 300-second timeout:

```rust
tokio::spawn(async move {
    let process = async move {
        let result = process_task(task, enforcement, dispatcher).await;
        // send result back via RemoteClient::send_result()
    };
    tokio::time::timeout(300s, process).await;
});
```

With `tool-api`/`rest-api`/`grpc-api`, `process_task()` (`worker.rs:468-515`) routes through `EnforcedDispatcher::dispatch_checked()` with an `AgentStrict` enforcement context. Without those features, it returns an error.

### Background Tasks

| Task | Interval | Location | Purpose |
|------|----------|----------|---------|
| Rate limit cleanup | 60s | `remote.rs:219-232` | Remove stale per-IP timestamp vectors |
| Stale task reassignment | 30s | `remote.rs:236-250` | Return timed-out tasks to pending queue |

### Capability Matching

When a worker registers, the coordinator filters its claimed capabilities against `CAPABILITIES` (`remote.rs:440-444`). Unknown capabilities are logged and discarded. The coordinator stores only validated capabilities in `WorkerRegistration`.

## Security Model

### PSK + TLS

- **PSK**: 32-byte random hex string (`generate_psk()` at `command.rs:279-287`). Generated via `OsRng` (getrandom), not a fork-reproducible PRNG.
- **Constant-time compare**: `subtle::ConstantTimeEq` prevents timing attacks on PSK validation (`remote.rs:346`).
- **TLS**: Optional, using `rustls`. Server loads PEM cert/key. Client uses `NoVerifier` (insecure-tls feature) or webpki_roots for proper verification.
- **Plaintext fallback**: `new_plaintext()` methods exist but log warnings. Both `start()` and `connect_to_coordinator_with_addr()` reject plaintext unless explicitly opted in.

### What Prevents Rogue Workers

1. **PSK authentication**: Only workers knowing the PSK can connect. The constant-time comparison prevents brute-force timing attacks.
2. **IP allowlist** (optional): `RemoteListener::with_allowlist()` restricts connections to specific IPs/CIDRs.
3. **Rate limiting**: 60 connections/minute per IP prevents rapid brute-force attempts.
4. **Connection limits**: Max 100 concurrent connections prevents resource exhaustion.
5. **Command executor sandboxing** (`command.rs:129-249`): Only `eggsec` binary allowed; forbidden path patterns; max 50 args, 1000 chars each; 10MB output limit; no custom environment variables.
6. **Enforcement context**: When `tool-api` is enabled, workers wrap task processing in `EnforcementContext::agent_strict()`, which validates scope and policy for every dispatched task (`worker.rs:490-498`).

### Command Executor Security (`command.rs:129-249`)

| Check | Value |
|-------|-------|
| Allowed executables | `["eggsec"]` only |
| Max arguments | 50 |
| Max argument length | 1000 chars |
| Max output size | 10 MB |
| Custom environment | Rejected (security: prevents PATH/LD_PRELOAD injection) |
| Forbidden patterns | 13 patterns including `../`, `/etc/`, `~/.ssh/`, `.pem`, `.key`, `--config`, `--credentials` |

## Public API

### Module Re-exports (`mod.rs:58-60`)

```rust
pub use command::{generate_psk, RemoteResult};
pub use queue::{Task, TaskResult};
pub use remote::{RemoteClient, RemoteListener, TlsConfig};
```

### RemoteListener Public Methods

| Method | Signature |
|--------|-----------|
| `new` | `new(psk: String) -> Self` |
| `with_config` | `with_config(psk, max_connections, rate_limit) -> Self` |
| `with_allowlist` | `with_allowlist(psk, allowlist: Vec<String>) -> Self` |
| `with_tls` | `with_tls(psk, tls_config: TlsConfig) -> Result<Self>` |
| `new_plaintext` | `new_plaintext(psk) -> Self` |
| `start` | `start(port: u16) -> Result<()>` |
| `shutdown` | `shutdown(&self)` |
| `get_workers` | `get_workers() -> Vec<WorkerRegistration>` |
| `get_queue_counts` | `get_queue_counts() -> (usize, usize, usize)` |
| `connection_count` | `connection_count() -> usize` |
| `is_tls` | `is_tls() -> bool` |

### RemoteClient Public Methods

| Method | Signature |
|--------|-----------|
| `new` | `new(psk: String) -> Self` |
| `with_tls` | `with_tls(psk, domain: &str) -> Result<Self>` |
| `new_plaintext` | `new_plaintext(psk) -> Self` |
| `register_worker` | `register_worker(host, port, worker_id, hostname, capabilities) -> Result<()>` |
| `send_heartbeat` | `send_heartbeat(host, port, worker_id, status) -> Result<()>` |
| `send_result` | `send_result(host, port, result: TaskResult) -> Result<()>` |
| `request_tasks` | `request_tasks(host, port, worker_id, max_tasks) -> Result<Vec<Task>>` |
| `execute` | `execute(host, port, command, timeout) -> Result<RemoteResult>` |
| `request_status` | `request_status(host, port) -> Result<serde_json::Value>` |
| `enqueue_task` | `enqueue_task(host, port, task: Task) -> Result<()>` |

### Worker Public Methods

| Method | Signature |
|--------|-----------|
| `new` | `new(config: WorkerConfig, psk: String) -> Self` |
| `with_enforcement` | `with_enforcement(config, psk, enforcement) -> Self` |
| `start` | `start(&mut self) -> Result<()>` |
| `get_stats` | `get_stats(&self) -> WorkerStats` |
| `shutdown` | `shutdown(&mut self)` |

### TaskQueue Public Methods

| Method | Signature |
|--------|-----------|
| `new` | `new(max_size: usize) -> Self` |
| `enqueue` | `enqueue(&self, task: Task) -> Result<(), QueueError>` |
| `dequeue` | `dequeue(&self, worker_id: &str) -> Result<Option<Task>, QueueError>` |
| `reassign_stale_tasks` | `reassign_stale_tasks(&self, timeout_secs: i64) -> Vec<Task>` |
| `complete` | `complete(&self, result: TaskResult)` |
| `get_pending_count` | `get_pending_count(&self) -> usize` |
| `get_in_progress_count` | `get_in_progress_count(&self) -> usize` |
| `get_completed_count` | `get_completed_count(&self) -> usize` |
| `get_results` | `get_results(&self) -> Vec<TaskResult>` |
| `clear` | `clear(&self)` |

## Integration Points

| Surface | Entry point | Flow |
|---------|-------------|------|
| CLI `cluster coordinator` | `handle_cluster()` | Generates PSK, creates `RemoteListener`, calls `start(port)` |
| CLI `cluster worker` | `handle_cluster()` | Creates `WorkerConfig`, calls `Worker::with_enforcement()` + `start()` |
| CLI `cluster status` | `handle_cluster()` | Creates `RemoteClient`, calls `request_status()` |
| CLI `cluster enqueue` | `handle_cluster()` | Creates `RemoteClient`, calls `enqueue_task()` |
| CLI `cluster execute` | `handle_cluster()` | Creates `RemoteClient`, calls `execute()` |
| CLI `cluster generate-psk` | `handle_cluster()` | Calls `generate_psk()` |

### Worker Task Processors (`worker.rs:525-870`)

| Processor | Function | Engine call |
|-----------|----------|-------------|
| PortScan | `process_port_scan()` | `scanner::ports::scan_ports()` |
| ServiceFingerprint | `process_fingerprint()` | `scanner::fingerprint::fingerprint_services()` |
| EndpointDiscovery | `process_endpoints()` | `scanner::endpoints::scan_endpoints()` |
| Fuzz | `process_fuzz()` | `fuzzer::engine::FuzzEngine::new().run_return_session()` |
| WafTest | `process_waf()` | `waf::WafEngine::new().run()` |
| LoadTest | `process_load_test()` | `loadtest::LoadTestRunner::from_config_with_engine().run()` |
| Recon | `process_recon()` | `recon::runner::run_full_recon_from_request()` |

Note: The standalone `process_*` functions (`worker.rs:525-870`) are marked `#[allow(dead_code)]`. When `tool-api`/`rest-api`/`grpc-api` is enabled, `process_task()` routes through `EnforcedDispatcher` instead of calling these functions directly.

## Testing

| Test suite | Path | What it covers |
|------------|------|----------------|
| Unit tests | `crates/eggsec/src/distributed/command.rs:289-305` | PSK generation length and uniqueness |
| Unit tests | `crates/eggsec/src/distributed/io.rs:351-459` | StreamWrapper variants, LineWriter roundtrip, TCP plaintext e2e, TLS server invalid PEM |
| Unit tests | `crates/eggsec/src/distributed/worker.rs:876-901` | Worker rejects tasks without explicit scope (requires tool-api) |
| Integration tests | `crates/eggsec/tests/distributed_tests.rs` | Queue operations, FIFO ordering, full queue, complete, evict, serde roundtrip, listener auth (success + invalid PSK), task assignment cycle, heartbeat, connection count, enqueue command, status request, disconnect cleanup, stale task reassignment |

```bash
cargo test --lib -p eggsec distributed::
cargo test -p eggsec --test distributed_tests
```

## Invariants & Gotchas

1. **Each RemoteClient method creates a new TCP connection**: `register_worker`, `send_heartbeat`, `send_result`, `request_tasks`, `execute`, `request_status`, and `enqueue_task` each open a fresh connection, authenticate, send one message, wait for response, and drop the connection. There is no persistent connection pooling.
2. **DNS caching**: `RemoteClient` caches DNS resolution for 60 seconds (`remote.rs:702-715`). Cached addresses are not re-validated for reachability.
3. **Worker registration requires TLS domain**: `WorkerConfig::default()` sets `tls_domain: Some("localhost")`. If `tls_domain` is `None`, registration, heartbeat, and task processing all fail (`worker.rs:159-161,208-211,394-397`).
4. **Completed results eviction**: `TaskQueue::complete()` evicts the oldest results when completed count exceeds `max_size` (`queue.rs:117-119`).
5. **No task cancellation**: Once a task is spawned for processing, there is no mechanism to cancel it. The 300-second timeout is the only abort path.
6. **`process_task()` returns error without `tool-api`**: Without `tool-api`, `rest-api`, or `grpc-api`, the worker's task processor always fails (`worker.rs:517-523`).
7. **Rate limit cleanup is background-only**: The periodic cleanup task (`remote.rs:219-232`) runs every 60 seconds. Burst connections within a window may not be cleaned until the next tick.
8. **`env` field in `CommandMessage::Execute` is accepted but rejected**: The protocol accepts the field for backward compatibility, but `CommandExecutor::execute()` always rejects it with an error (`command.rs:169-178`).

## Links

- [overview.md](overview.md)
- [cli_commands.md](cli_commands.md)
- [dispatch.md](dispatch.md)
- [config.md](config.md)

---

*Last verified against source: 2026-08-25*
