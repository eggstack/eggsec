---
name: plugin_security
description: "Plugin security patterns - validation and suspicious pattern detection"
triggers:
  - plugin security
  - suspicious patterns
  - plugin validation
  - plugin loading
metadata:
  category: Security
  tools: [plugin, python, ruby]
  scope: targets
---

## Overview

Slapper's plugin system includes security validation to prevent malicious or dangerous plugins from executing. Both Python and Ruby plugins are validated against suspicious patterns.

## Python Plugin Security

The `validate_python_plugin()` function in `slapper-plugin/src/security.rs` checks for:

**Dangerous Patterns Detected:**
1. `os.system` - Shell command execution
2. `subprocess` - Process spawning
3. `socket` - Network connections
4. `eval(` - Code evaluation
5. `\bexec\b` - Command execution (regex)
6. `\bfork\b` - Process forking
7. `__import__` - Dynamic module loading
8. `\bopen\(` - File access
9. `pty.spawn` - PTY spawning
10. `os.popen` - Shell command execution
11. `multiprocessing.Process` - Process creation
12. `ctypes` - C library bindings
13. `importlib` - Dynamic imports
14. `getattr(` - Attribute access
15. `chr(` - Character encoding
16. Hex escapes (`\x[0-9a-fA-F]{2}`)
17. Unicode escapes (`\u[0-9a-fA-F]{4}`)
18. Octal escapes (`\[0-7]{3,}`)

Patterns use case-insensitive matching (`(?i)` flag).

## Ruby Plugin Security

The `validate_ruby_plugin()` function checks for:

**Dangerous Patterns Detected:**
1. `\beval\(` - Code evaluation
2. `\bexec\(` - Command execution
3. `\bsystem\(` - Shell commands
4. `` ` `` - Backtick command execution
5. `IO.popen` - Process I/O
6. `Process.spawn` - Process spawning
7. `File.read(` - File reading
8. `File.write(` - File writing
9. `File.open(` - File operations
10. `Net::HTTP` - HTTP requests
11. `Socket.open` - Network sockets
12. `TCPSocket` - TCP connections
13. `UDPSocket` - UDP connections
14. `Open3.` - Process capture
15. `Shellwords.escape` - Shell argument manipulation
16. `Kernel.exec` - Direct command execution
17. `\bopen\b` - File/popen access
18. `\beval\b` - Eval without parentheses

## Configuration

Plugin security is controlled by `block_suspicious_plugins` in `PluginConfig`:

```rust
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
    pub block_suspicious_plugins: bool,  // default: true
}
```

When `true` (default), plugins matching suspicious patterns are rejected.

## Shared Security Module

`slapper-plugin/src/security.rs` provides consolidated security validation:
- `validate_python_plugin(content, block_suspicious_plugins) -> Result<(), String>`
- `get_max_plugin_size_bytes() -> usize` (default 1MB)

## Safe Ruby APIs

After the sandbox security fix (Wave 2.1), Ruby plugins only have safe reporting methods:
- `Slapper::Report.finding()`
- `Slapper::Report.vulnerability()`
- `Slapper::Report.info()`
- `Slapper::Report.success()`
- `Slapper::Report.warning()`
- `Slapper::Report.error()`

Dangerous APIs removed: HTTP, Scanner, Fuzzer, Metasploit, Encoder, Session

## Implementation

- `slapper-plugin/src/security.rs` - Shared security patterns
- `slapper-plugin/src/python.rs` - Python plugin validation
- `slapper-ruby/src/bridge.rs` - Ruby plugin validation

## Verification

```bash
cargo test --lib -p slapper-plugin --features python-plugins
cargo test --lib -p slapper-ruby --features ruby-plugins
```