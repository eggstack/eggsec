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

**Suspicious Patterns Detected:**
- `os.system` - arbitrary command execution
- `subprocess` - process spawning
- `socket` - network connections
- `eval(` - dynamic code execution
- `exec` - dynamic code execution
- `fork` - process forking
- `__import__` - dynamic import
- `open(` - file access

### Configuration

```rust
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
    pub block_suspicious_plugins: bool,  // default: true
}
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

## Feature Flags

- `python-plugins` - Enable Python plugin support
- `ruby-plugins` - Enable Ruby plugin support

Both can be enabled together with `--features python-plugins,ruby-plugins`

## Triggers

Keywords: plugin security, block_suspicious_plugins, validate_python_plugin, validate_ruby_plugin, plugin validation, python plugin, ruby plugin, suspicious patterns, plugin blocking, plugin allowlist

## References

- `crates/slapper-plugin/src/python.rs` - Python plugin manager
- `crates/slapper-plugin/src/lib.rs` - PluginConfig definition
- `crates/slapper-ruby/src/bridge.rs` - Ruby plugin bridge