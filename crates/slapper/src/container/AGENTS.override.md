# Container Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/slapper/src/container/mod.rs` | Module entry point |
| `crates/slapper/src/container/docker.rs` | Docker container scanning and inspection |
| `crates/slapper/src/container/kubernetes.rs` | Kubernetes cluster scanning |
| `crates/slapper/src/container/escape.rs` | Container escape detection |
| `crates/slapper/src/container/cis.rs` | CIS benchmark compliance checks |

## Known Issues

1. **Docker Socket Access Not Checked**: The escape detection in `escape.rs` checks for docker.sock in config strings but doesn't actually verify if the container has access to the Docker socket.

2. **CIS Benchmark Checks Are Simplistic**: CIS checks in `cis.rs` use simple string matching (e.g., `lower.contains("privileged")`) which can produce false positives/negatives.

## Testing

```bash
cargo test --lib -p slapper container::
cargo clippy --lib -p slapper
```