---
name: plugin_security
description: "Plugin security patterns for Slapper including suspicious pattern detection and blocking"
triggers:
  - plugin security
  - block_suspicious_plugins
  - validate_python_plugin
  - validate_ruby_plugin
  - plugin validation
  - python plugin
  - ruby plugin
  - suspicious patterns
metadata:
  category: security
  tools: [plugin]
  scope: internal
---

## Overview

Slapper supports Python and Ruby plugins with built-in security features to prevent malicious plugins from executing dangerous operations.

## Python Plugin Security

### Validation Function

`validate_python_plugin(content: &str, block_suspicious_plugins: bool)` in `crates/slapper-plugin/src/python.rs`:

Uses regex-based pattern detection with word-boundary awareness for more robust matching:

**Suspicious Patterns Detected:**
- `r"\bos\.system\b"` - arbitrary command execution
- `r"\bsubprocess\b"` - process spawning
- `r"\bsocket\b"` - network connections
- `r"\beval\("` - dynamic code execution
- `r"\bexec\b"` - dynamic code execution
- `r"\bfork\b"` - process forking
- `r"\b__import__\b"` - dynamic import
- `r"\bopen\s*\("` - file access
- `r"pty\.spawn"` - PTY spawning
- `r"os\.popen"` - OS command pipe
- `r"multiprocessing\.Process"` - Process threads
- `r"\bctypes\b"` - C FFI
- `r"\bimportlib\b"` - dynamic imports
- `r"\bgetattr\("` - attribute access
- `r"\bchr\("` - character encoding
- `r"\\x[0-9a-fA-F]{2}"` - hex escape
- `r"\\u[0-9a-fA-F]{4}"` - unicode escape
- `r"\\[0-7]{3,}"` - octal escape

Uses `LazyLock<Regex>` for compiled patterns to avoid repeated compilation.

### Deserialization DoS Prevention

JSON parsing is size-limited via `MAX_JSON_SIZE_BYTES = 100_000` to prevent DoS attacks during plugin result parsing.

### Configuration

```rust
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
    pub block_suspicious_plugins: bool,  // default: true
    pub timeout_secs: u64,               // default: 300
    pub max_file_size_bytes: usize,      // default: 1,000,000
}
```

### Timeout Enforcement

Plugin execution is time-limited via `timeout_secs` (default: 300 seconds):
- Python: Uses `tokio::time::timeout` wrapper around plugin execution
- Ruby: Uses `rx.recv_timeout()` with the configured duration
- If timeout occurs, execution is cancelled and an error is returned
```

### Usage

```rust
// Create manager with blocking enabled (default)
let manager = PythonPluginManager::new();  // block_suspicious_plugins = true

// Or disable blocking (not recommended)
let manager = PythonPluginManager::with_block_suspicious_plugins(false);

// Load plugins
manager.load_plugins(&plugin_dir)?;
```

## Ruby Plugin Security

### Validation Function

`validate_ruby_plugin(content: &str, block_suspicious_plugins: bool)` in `crates/slapper-ruby/src/bridge.rs`:

**Suspicious Patterns Detected:**
- `eval(` - dynamic code execution
- `exec(` - command execution
- `system(` - command execution
- `` ` `` - command execution
- `IO.popen` - process pipes
- `Process.spawn` - process spawning
- `File.read(` - file reading
- `File.write(` - file writing
- `File.open(` - file access
- `Net::HTTP` - HTTP connections
- `Socket.open` - socket creation
- `TCPSocket` / `UDPSocket` - network sockets
- `Open3.` - process spawning
- `Shellwords.escape` - shell escaping
- `Kernel.exec` - direct exec call
- `\bopen\b` - generic open (matches `open("|cmd")`)
- `\beval\b` - eval without parens

### Configuration

The Ruby bridge stores `block_suspicious_plugins` as a setting:

```rust
pub struct RubyBridge {
    ruby: Ruby,
    loaded: bool,
    block_suspicious_plugins: bool,  // default: true
}
```

## Security Best Practices

1. **Keep blocking enabled** - `block_suspicious_plugins: true` is the default for a reason
2. **Review plugin sources** - Even with blocking disabled, warnings are logged
3. **Use isolated environments** - Run plugins in sandboxed environments when possible

## NSE Sandbox

Slapper's NSE (Nmap Scripting Engine) support includes sandboxing for Lua scripts:

### Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub enabled: bool,           // Default: true
    pub allowed_dir: Option<PathBuf>,  // Restrict file access
    pub allowed_commands: Vec<String>, // Allowed commands for io.popen
    pub log_violations: bool,    // Log instead of block
}
```

### NSE io Library

The NSE `io` library is sandboxed:

- `io.open()` - Path validation with canonicalization
- `io.lines()` - Path validation (blocks traversal attempts)
- `io.popen()` - Command validation via `is_command_allowed()`
- `io.tmpfile()` - Creates files in system temp directory

### Path Validation Pattern

All file operations use `canonicalize()` to resolve symlinks before checking. Symlink cycles and canonicalization failures are blocked (fail-secure):

```rust
if sandbox_enabled {
    let canonical = match path_buf.canonicalize() {
        Ok(c) => c,
        Err(e) => return Err(format!("Path could not be resolved: {} - blocked", e)),
    };
    if !canonical.starts_with(allowed_dir) {
        return Err("Path blocked by sandbox");
    }
}
```

## Feature Flags

- `python-plugins` - Enable Python plugin support
- `ruby-plugins` - Enable Ruby plugin support
- `nse` - Enable NSE script support
- `nse-sandbox` - Enable NSE sandbox mode (default: enabled)

## Triggers

Keywords: plugin security, block_suspicious_plugins, validate_python_plugin, validate_ruby_plugin, plugin validation, python plugin, ruby plugin, suspicious patterns, plugin blocking, plugin allowlist, NSE sandbox, io.lines, io.popen

## References

- `crates/slapper-plugin/src/python.rs` - Python plugin manager
- `crates/slapper-plugin/src/lib.rs` - PluginConfig definition
- `crates/slapper-ruby/src/bridge.rs` - Ruby plugin bridge
- `crates/slapper-nse/src/libraries/io.rs` - NSE io library with sandbox
- `crates/slapper-nse/src/lib.rs` - SandboxConfig definition