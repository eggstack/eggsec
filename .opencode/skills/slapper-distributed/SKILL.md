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

## Resources
- `crates/slapper/src/distributed/AGENTS.override.md` - Detailed distributed patterns
- `AGENTS.md` - General project guidelines
- `ARCHITECTURE.md` - Overall design
