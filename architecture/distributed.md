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

## Key Components

| Component | File | Lines | Key Function/Type |
|-----------|------|-------|-------------------|
| TaskType enum | mod.rs | 59-67 | 7 task types |
| Task struct | queue.rs | 7-18 | Core task representation |
| TaskResult struct | queue.rs | 20-27 | Task execution result |
| TaskQueue | queue.rs | 29-141 | Thread-safe task queue |
| RemoteListener | remote.rs | 26-388 | Coordinator server |
| RemoteClient | remote.rs | 395-697 | Worker client |
| CommandExecutor | command.rs | 103-220 | Secure command execution |
| CommandMessage | command.rs | 28-46 | Protocol messages |
| Worker | worker.rs | 60-553 | Worker node |
| TlsServer | io.rs | 110-161 | TLS server from PEM |
| TlsClient | io.rs | 163-225 | TLS client |
| StreamWrapper | io.rs | 19-108 | Unified stream enum |
| LineWriter | io.rs | 306-340 | Line I/O wrapper |
| generate_psk | command.rs | 249-254 | PSK generation |

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
| `worker.rs:132-161` | Heartbeat used HTTP POST to non-existent REST API endpoint | Changed to use `RemoteClient::send_heartbeat()` via TCP |
