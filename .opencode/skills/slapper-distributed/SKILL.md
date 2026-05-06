# Slapper Distributed Skill

Distributed computing module workflows and patterns for cluster-based testing.

## Key Types and Patterns

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
- `ARCHITECTURE.md` - Overall design
