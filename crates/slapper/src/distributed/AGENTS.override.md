# Distributed Module Override

Specialized guidance for the distributed computing module.

## Worker Configuration

`Worker::new(config, psk)` requires both `WorkerConfig` and a PSK string:
```rust
let worker = Worker::new(config, "your-secret-psk".to_string());
let mut worker = worker;
worker.start().await?;
```

## TLS

`distributed/io.rs` has `StreamWrapper` enum:
- `Plain` - Unencrypted stream
- `TlsClient` - TLS client connection
- `TlsServer` - TLS server connection

## TlsServer

`TlsServer::from_pem(cert_path, key_path)` loads PEM cert + key files.

## TlsClient

`TlsClient::new(domain)` creates client with `NoVerifier` (insecure, for internal use).

## Worker Registration Protocol

Workers use `RemoteClient` to register with the coordinator via TCP (not HTTP):

```rust
let client = RemoteClient::new_plaintext(psk);
client.register_worker(host, port, worker_id, hostname, capabilities).await?;
```

The coordinator expects line-based JSON messages via `CommandMessage::Register` and `CommandMessage::Heartbeat`, not HTTP POST requests.

## URL Parsing

Worker coordinator URLs should be `host:port` format (no http:// prefix):
```rust
fn parse_coordinator_url(url: &str) -> Result<(&str, u16)> {
    let url = url
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches('/');

    let parts: Vec<&str> = url.split(':').collect();
    if parts.len() != 2 {
        return Err(SlapperError::Config(format!(
            "Invalid coordinator URL format: {} (expected host:port)",
            url
        )));
    }

    let host = parts[0];
    let port: u16 = parts[1]
        .parse()
        .map_err(|_| SlapperError::Config(format!("Invalid port in coordinator URL: {}", url)))?;

    Ok((host, port))
}
```

## Bugs Fixed (2026-05-28)

| File | Issue | Fix |
|------|-------|-----|
| `queue.rs:150-154` | `QueueError` missing Display and Error traits | Added `impl Display` and `impl Error` for `?` operator support |
| `mod.rs`, `worker.rs`, `remote.rs` | Worker and coordinator used different capability naming schemes | Created shared `CAPABILITIES` constant in `mod.rs` |

## Bugs Fixed (2026-05-22)

| File | Issue | Fix |
|------|-------|-----|
| `queue.rs:57` | `dequeue()` ignored `worker_id` param and didn't set `assigned_at_secs` | Now properly tracks which worker owns task and when assigned |
| `queue.rs:57` | `dequeue()` returned `Option<Task>` silently dropping errors | Changed to return `Result<Option<Task>, QueueError>` for explicit error handling |
| `worker.rs:132-161` | Heartbeat used HTTP POST to non-existent REST API endpoint | Changed to use `RemoteClient::send_heartbeat()` via TCP line-based JSON |

## Performance Improvements (2026-05-22)

| File | HashMap Type | Reason |
|------|-------------|--------|
| `queue.rs:13` | `Task.payload` | Changed from `std::collections::HashMap` to `FxHashMap` for performance |
| `command.rs:36` | `CommandMessage::Execute.env` | Changed from `HashMap` to `FxHashMap` for performance |
| `remote.rs:30` | `RemoteListener.rate_limits` | Changed from `HashMap` to `FxHashMap` for performance |

Note: `Task::payload` uses `#[serde(default)]` for backward compatibility with serialized data.

## Key Patterns

### Task Tracking
- `Task::worker_id` - Set by `dequeue()` when a worker claims a task
- `Task::assigned_at_secs` - Timestamp when task was assigned (for stale task detection)
- `TaskQueue::dequeue(worker_id)` returns `Result<Option<Task>, QueueError>` - handle errors explicitly
- Use `TaskQueue::reassign_stale_tasks(timeout_secs)` to recover tasks from dead workers
- `QueueError` enum: `QueueFull`, `TaskNotFound`

### Critical Issue (2026-06-09 Review)
**Task results never sent to coordinator** - The result system is broken. Workers execute tasks but results are never communicated back. This is a HIGH severity bug identified in the architecture review. See `plans/distributed_review.md` for details.

### PSK Authentication
- PSK is sent as first message after TCP connect: `AuthMessage { psk }`
- Server validates using constant-time comparison: `bool::from(psk.as_bytes().ct_eq(server_psk.as_bytes()))`
- On failure, server sends error and closes connection

### Line-Based Protocol
Messages are newline-delimited JSON. Use `LineWriter`:
```rust
let mut writer = LineWriter::new(stream);
writer.write_line(&serde_json::to_string(&msg)?).await?;
let response: ResponseMessage = serde_json::from_str(&writer.read_line().await?)?;
```



(End file - 132 lines)