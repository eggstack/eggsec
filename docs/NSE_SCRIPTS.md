# NSE Script Development Guide

Slapper supports running Nmap Scripting Engine (NSE) Lua scripts for security scanning.

## Building with NSE Support

```bash
cargo build --release --features nse
# With sandboxing (restricts dangerous Lua operations):
cargo build --release --features nse-sandbox
```

## Script Structure

NSE scripts follow the Nmap convention with categories, rules, and an action function:

```lua
-- Script categories (comma-separated)
categories = {"discovery", "safe"}

-- Port rule: runs against specific ports
portrule = function(host, port)
    return port.number == 80 or port.number == 443
end

-- Host rule: runs against hosts (alternative to portrule)
hostrule = function(host)
    return true
end

-- Action function: the main script logic
action = function(host, port)
    local stdnse = require "stdnse"
    local http = require "http"

    local response = http.get(host, port.number, "/")
    if response then
        return {status = response.status, headers = response.rawheader}
    end
end
```

## Available Lua Libraries

| Library | Description | Dangerous Functions |
|---------|-------------|-------------------|
| `stdnse` | Standard NSE utilities | None |
| `nmap` | Nmap state and functions | None |
| `http` | HTTP client | None |
| `dns` | DNS resolution | None |
| `socket` | TCP/UDP sockets | **NOT sandboxed** - allows network connections |
| `sslcert` | SSL certificate handling | None |
| `shortport` | Port rule helpers | None |
| `lfs` | LuaFileSystem | Path restrictions when sandboxed |
| `io` | File I/O | `io.popen` (command execution) |
| `os` | OS operations | `os.setenv`, `os.remove`, `os.rename` |

## Sandbox Mode

Sandbox restrictions are enabled when built with `--features nse-sandbox`.
With `--features nse` alone, sandbox restrictions are disabled by default.

When sandbox is enabled, dangerous operations are restricted:

- `io.popen`: Blocked by default (returns error). Can allow specific commands via config.
- `io.open`: Path traversal blocked. Can restrict to a specific directory.
- `lfs`: Path restrictions enforced (attributes, dir, mkdir, rmdir, remove, rename, chdir, touch all checked).
- `os.getenv`: Returns empty string (no credential leakage).
- `os.setenv` / `os.unsetenv`: Blocked (uses unsafe code).
- `os.remove` / `os.rename`: Restricted to sandbox directory.
- `os.chdir`: Restricted to sandbox directory.

**Important**: The `socket` library has **conditional network restrictions**. By default, socket operations proceed normally (with a warning log when sandbox is enabled). However, when `allowed_networks` is configured in `SandboxConfig`, connections are validated against the CIDR blocklist and blocked if outside allowed ranges.

### Sandbox Configuration

```rust
use slapper_nse::SandboxConfig;

let sandbox = SandboxConfig {
    enabled: true,
    allowed_dir: Some("/tmp/slapper-nse".into()),
    allowed_commands: vec!["curl".to_string(), "dig".to_string()],
    allowed_networks: vec!["10.0.0.0/8".parse().unwrap(), "192.168.0.0/16".parse().unwrap()], // Optional: restrict socket connections
    log_violations: true,
};
```

## Running NSE Scripts

### CLI

```bash
# Run a built-in script
slapper nse --script default --target example.com

# Run a custom script file
slapper nse --script-file /path/to/script.nse --target example.com

# With script arguments
slapper nse --script http-headers --target example.com --script-args "timeout=10"
```

### Programmatic

```rust
use slapper_nse::{NseExecutor, NseConfig};

let config = NseConfig::new("example.com", "default", None, None, false, false);
slapper_nse::run_cli(config).await?;
```

## Writing Custom Scripts

### HTTP Header Check

```lua
categories = {"discovery", "safe"}
portrule = function(host, port)
    return port.number == 80 or port.number == 443
end

action = function(host, port)
    local http = require "http"
    local stdnse = require "stdnse"

    local response = http.get(host, port.number, "/")
    local output = stdnse.output_table()

    if response then
        output.status = response.status
        output.server = response.header["server"] or "unknown"
        output.has_csp = response.header["content-security-policy"] ~= nil
        output.has_hsts = response.header["strict-transport-security"] ~= nil
    end

    return output
end
```

### Security Considerations

- Scripts run with the same permissions as the Slapper process
- The `sandbox` feature restricts `io.popen` and filesystem access
- Always validate user-provided script arguments
- Use `--sandbox-dir` to limit filesystem access to a specific directory
