---
name: nse_sandbox
description: "NSE sandbox configuration and enforcement patterns for Slapper"
triggers:
  - nse sandbox
  - sandbox config
  - allowed_dir
  - symlink cycle
  - lfs library
  - socket library
metadata:
  category: security
  tools: [nse]
  scope: nse-sandbox
---

## Overview

Slapper's NSE (Nmap Scripting Engine) module includes sandboxing features to restrict Lua script capabilities. This skill documents the sandbox patterns and configuration.

## Sandbox Configuration

### SandboxConfig Fields

```rust
pub struct SandboxConfig {
    pub enabled: bool,           // Whether sandboxing is enabled
    pub allowed_dir: Option<PathBuf>,  // Restrict file ops to this directory
    pub allowed_commands: Vec<String>, // Allowed io.popen commands
    pub log_violations: bool,    // Log violations instead of blocking
}
```

### Default Behavior

- `enabled: true` - Sandbox is ON by default (security-first)
- `allowed_dir: Some("/tmp/slapper-nse")` - Restricted to this directory
- `log_violations: true` - Violations are logged but may be allowed

### Secure Directory Setup

When the sandbox initializes with `allowed_dir = Some("/tmp/slapper-nse")`:

1. Directory is created if it doesn't exist
2. Permissions set to `0o700` (owner read/write/execute only)
3. This prevents other users from accessing NSE script files

## Sandboxed Libraries

### io Library (io.rs)

File operations are sandboxed:
- `io.open()` - Restricted to allowed_dir
- `io.lines()` - Restricted to allowed_dir
- `io.popen()` - Command list enforced via `allowed_commands`

### lfs Library (lfs.rs)

Filesystem operations are sandboxed:
- `lfs.mkdir()` - Blocked outside allowed_dir
- `lfs.rmdir()` - Blocked outside allowed_dir
- `lfs.remove()` - Blocked outside allowed_dir
- `lfs.rename()` - Blocked if source or dest outside allowed_dir
- `lfs.chdir()` - Blocked outside allowed_dir
- `lfs.currentdir()` - Fully blocked (information disclosure)
- `lfs.touch()` - Blocked outside allowed_dir
- `lfs.link()` - Blocked if paths outside allowed_dir
- `lfs.set_mode()` - Blocked outside allowed_dir

### socket Library (socket.rs)

Network operations are logged for audit:
- `socket.tcp_connect()` - Logged with host:port
- `socket.connect()` - Logged with host:port
- `socket.sendto()` - Logged for new connections

**Note:** Network restrictions may be intentionally limited. Socket operations are logged for audit but not necessarily blocked.

## Symlink Cycle Detection

The sandbox includes protection against symlink-based attacks:

```rust
const MAX_SYMLINK_DEPTH: usize = 16;

fn safe_canonicalize(path: &PathBuf) -> std::io::Result<PathBuf> {
    // Detects cycles and depth exceeded
    // Returns error if:
    // - Symlink depth > MAX_SYMLINK_DEPTH
    // - Symlink cycle detected (A -> B -> A)
}
```

## Path Validation Pattern

```rust
impl SandboxConfig {
    pub fn is_path_allowed(&self, path: &str) -> bool {
        if !self.enabled {
            return true;
        }

        let Some(ref allowed_dir) = self.allowed_dir else {
            return false;  // Deny if enabled but no allowed_dir
        };

        let path_buf = PathBuf::from(path);
        let Ok(canonical) = safe_canonicalize(&path_buf) else {
            return false;  // Deny if canonicalization fails
        };

        canonical.starts_with(allowed_dir)
    }
}
```

## Registration Pattern

Libraries are registered with sandbox context:

```rust
// In executor_core.rs
crate::libraries::lfs::register_lfs_library(&self.lua, &self.sandbox)?;
crate::libraries::socket::register_socket_library(&self.lua, &self.sandbox)?;
```

## Triggers

Keywords: nse sandbox, sandbox config, allowed_dir, symlink cycle, lfs library, socket library

## Verification Commands

```bash
# Build with sandbox features
cargo build -p slapper --features "nse nse-sandbox"

# Build without sandbox (libraries use defaults)
cargo build -p slapper --features nse

# Run NSE tests
cargo test -p slapper-nse
```

## References

- `crates/slapper-nse/src/lib.rs` - SandboxConfig definition
- `crates/slapper-nse/src/libraries/io.rs` - io library sandboxing
- `crates/slapper-nse/src/libraries/lfs.rs` - lfs library sandboxing
- `crates/slapper-nse/src/libraries/socket.rs` - socket library sandboxing
- `crates/slapper-nse/src/executor_core.rs` - Library registration