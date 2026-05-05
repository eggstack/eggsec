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

Secure and efficient communication between nodes using gRPC or a custom binary protocol.

- **Authentication**: Ensures only authorized workers can join the cluster.
- **Encryption**: All data in transit is encrypted.
- **Real-time Updates**: Status updates and findings are streamed back to the coordinator as they happen.

## Benefits

- **Scalability**: Easily handle thousands of targets by adding more worker nodes.
- **Resilience**: If a worker fails, its tasks are automatically reassigned to other nodes.
- **Geographic Distribution**: Deploy workers in different regions to test from multiple perspectives.
