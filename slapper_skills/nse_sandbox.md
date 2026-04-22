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

### socket Library NOT Sandboxed

**The `socket` library is NOT sandboxed** even when `nse-sandbox` is enabled. Scripts can still make arbitrary TCP/UDP connections.

**Affected functions:**
- `socket.tcp()` - TCP socket creation
- `socket.connect(host, port)` - TCP connection
- `socket.udp()` - UDP socket creation
- `socket.send(sock, data)` - Send data
- `socket.recv(sock, size)` - Receive data

**Current behavior:** Socket operations only log when sandbox is enabled but proceed unconditionally. This is a documented limitation.

### Symlink Cycles (RESOLVED)

Previously, symlink cycle detection was incomplete - if `canonicalize()` failed, it fell back to the unresolved path. **This is now fixed** - canonicalization failures result in the path being blocked.

## Security Best Practices

1. **Enable sandbox by default** - `SandboxConfig { enabled: true, ... }`
2. **Set specific allowed_dir** - Restrict filesystem access to minimal directory
3. **Use allowlist for io.popen** - Only permit known-safe commands
4. **Monitor socket usage** - The socket library is NOT sandboxed; consider network filtering at infrastructure level

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