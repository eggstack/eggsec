# Container Module Override

## Key Files

| File | Purpose |
|------|---------|
| `crates/slapper/src/container/mod.rs` | Module entry point |
| `crates/slapper/src/container/docker.rs` | Docker container scanning and inspection |
| `crates/slapper/src/container/kubernetes.rs` | Kubernetes cluster scanning |
| `crates/slapper/src/container/escape.rs` | Container escape detection |
| `crates/slapper/src/container/cis.rs` | CIS benchmark compliance checks |

## HIGH Priority Security Issue (Pending Fix)

**Docker Shell Injection Risk at `docker.rs:208-209`:**

```rust
std::process::Command::new("docker")
    .args(["inspect", _image_name])
```

The `_image_name` parameter is passed directly to the shell without sanitization. If `_image_name` contains special characters (e.g., `$(malicious command)` or `| malicious command`), this could lead to command injection.

**Fix required**: Validate image names before passing to shell. Reject or sanitize special characters such as:
- `$()` command substitution
- `|` pipe
- `&&` and `;` command chaining
- Newlines and other control characters

Example sanitization:
```rust
fn is_valid_docker_image_name(name: &str) -> bool {
    !name.chars().any(|c| matches!(c, '$' | '(' | ')' | '|' | ';' | '\n' | '\r' | '\x00'))
}
```

## Known Issues

1. **Kubernetes Scanner Silently Fails**: API calls use `.ok()` on results at `kubernetes.rs:65, 104, 163, 195, 254`, silently ignoring network errors and returning empty results. Log network errors instead of silently ignoring them.

2. **Docker Socket Access Not Checked**: The escape detection in `escape.rs` checks for docker.sock in config strings but doesn't actually verify if the container has access to the Docker socket.

3. **CIS Benchmark Checks Are Simplistic**: CIS checks in `cis.rs` use simple string matching (e.g., `lower.contains("privileged")`) which can produce false positives/negatives.

4. **Node/Namespace Count Always None**: `ClusterInfo::node_count` and `namespace_count` are always `None` despite being part of the struct definition.

## Testing

```bash
cargo test --lib -p slapper container::
cargo clippy --lib -p slapper
```