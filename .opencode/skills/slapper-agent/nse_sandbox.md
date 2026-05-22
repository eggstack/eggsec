---
name: nse_sandbox
description: "NSE (Nmap Scripting Engine) sandbox configuration and limitations"
triggers:
  - nse sandbox
  - io.popen
  - io.open
  - io.lines
  - sandbox
  - lua sandbox
  - path validation
metadata:
  category: security
  tools: [nse]
  scope: internal
---

## Overview

Slapper's NSE support includes sandboxing for Lua scripts via `SandboxConfig`. The sandbox restricts filesystem access, command execution, and environment modification.

## Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub enabled: bool,              // Default: true (security by default)
    pub allowed_dir: Option<PathBuf>, // Restrict file access to directory
    pub allowed_commands: Vec<String>, // Allowed commands for io.popen
    pub log_violations: bool,        // Log instead of block
}
```

## Sandboxed Libraries

### io Library (Filesystem)

**`io.open(path, mode)`** - File open with sandboxing:
- Path is canonicalized before validation
- Must start with `allowed_dir` if configured
- Blocks `..` path traversal attempts
- Returns error table with `error` field on failure

**`io.lines(path)`** - Read file lines with sandboxing:
- Same path validation as `io.open`
- Returns table with numbered lines on success
- Returns error table on sandbox violation

**`io.popen(cmd, mode)`** - Command execution:
- Validates command against `allowed_commands` list
- If `allowed_commands` is empty, blocks all popen operations
- Returns file handle on success

**`io.tmpfile()`** - Creates temp file:
- Always allowed (uses system temp directory)

### lfs Library (LuaFileSystem)

**lfs library** is **FULLY sandboxed** with path restrictions:
- `lfs.attributes(path)` - File metadata
- `lfs.dir(path)` - Directory listing
- `lfs.mkdir(path)` - Create directory
- `lfs.rmdir(path)` - Remove directory
- `lfs.remove(path)` - Delete file
- `lfs.rename(old, new)` - Rename file
- `lfs.chdir(path)` - Change directory
- `lfs.touch(path)` - Create file
- `lfs.symlinkattributes(path)` - Symlink metadata

All lfs operations validate paths against `allowed_dir`.

## Known Limitations

### socket Library (Network Access)

**The `socket` library has network restrictions when `allowed_networks` is configured:**

| Configuration | Behavior |
|---------------|----------|
| `allowed_networks` empty | Socket operations proceed normally (sandbox still logs connection attempts) |
| `allowed_networks` configured | Connections validated against CIDR allowlist, blocked if outside allowed ranges |

**Affected functions:**
- `socket.tcp()` - TCP socket creation (logs sandbox status)
- `socket.connect(host, port)` - TCP connection (sandbox check)
- `socket.udp()` - UDP socket creation (logs sandbox status)
- `socket.tcp_connect()` - TCP connection (sandbox check)
- `socket.sendto(sock, host, port, data)` - UDP send (sandbox check on host/port)
- `socket.tcp_connect_async(host, port)` - Async TCP (sandbox check)
- `socket.connect_async(host, port)` - Async connection (sandbox check)
- `socket.resolve_async(host)` - Async DNS (sandbox check)

**Sandbox enforcement details:**
- `SocketHandle::is_host_allowed()` checks if host IP is in allowed_networks
- DNS resolution (`resolve_async`) also checks allowed_networks since it reveals internal network info
- `sendto()` for UDP calls `connect_udp()` which validates the new host via `is_host_allowed()`

**Example configuration:**
```rust
pub struct SandboxConfig {
    pub enabled: bool,
    pub allowed_dir: Option<PathBuf>,
    pub allowed_commands: Vec<String>,
    pub allowed_networks: Vec<IpNetwork>, // Optional CIDR allowlist
    pub log_violations: bool,
}
```

### Symlink Cycles (RESOLVED)

Previously, symlink cycle detection was incomplete - if `canonicalize()` failed, it fell back to the unresolved path. **This is now fixed** - canonicalization failures result in the path being blocked.

## Security Best Practices

1. **Enable sandbox by default** - `SandboxConfig { enabled: true, ... }`
2. **Set specific allowed_dir** - Restrict filesystem access to minimal directory
3. **Use allowlist for io.popen** - Only permit known-safe commands
4. **Use allowed_networks** - Configure CIDR blocklist to restrict socket connections to expected network ranges

## Path Validation Pattern

```rust
let canonical = match path_buf.canonicalize() {
    Ok(c) => c,
    Err(e) => return Err(format!("Path could not be resolved: {} - blocked", e)),
};
if !canonical.starts_with(allowed_dir) {
    return Err("Path blocked by sandbox");
}
```

## Triggers

Keywords: nse sandbox, io.popen, io.open, io.lines, sandbox, lua sandbox, path validation, lfs, symlink cycle, socket not sandboxed

## References

- `crates/slapper-nse/src/libraries/io.rs` - io library implementation
- `crates/slapper-nse/src/libraries/lfs.rs` - lfs library implementation
- `crates/slapper-nse/src/libraries/socket.rs` - socket library (not sandboxed)
- `crates/slapper-nse/src/lib.rs` - SandboxConfig definition
- `docs/NSE_SCRIPTS.md` - NSE documentation