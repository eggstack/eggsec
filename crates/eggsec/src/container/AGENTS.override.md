# Container Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/eggsec/src/container/mod.rs` | Module entry point |
| `crates/eggsec/src/container/docker.rs` | Docker container scanning and inspection |
| `crates/eggsec/src/container/kubernetes.rs` | Kubernetes cluster scanning |
| `crates/eggsec/src/container/escape.rs` | Container escape detection |
| `crates/eggsec/src/container/cis.rs` | CIS benchmark compliance checks |

## Known Issues

1. **Docker Socket Access Not Checked**: The escape detection in `escape.rs` checks for docker.sock in config strings but doesn't actually verify if the container has access to the Docker socket.

2. **CIS Benchmark Checks Are Simplistic**: CIS checks in `cis.rs` use string matching (e.g., `lower.contains("privileged")`) which can produce false positives/negatives. Check 1.1 now uses word-boundary patterns (`"user "` / `"user:"`) to reduce false positives.

## Testing

```bash
cargo test --lib -p eggsec container::
cargo clippy --lib -p eggsec
```