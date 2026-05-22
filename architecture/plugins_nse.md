# Plugins & NSE Integration

Slapper is designed to be highly extensible through its plugin system and full integration with the Nmap Scripting Engine (NSE).

## Plugin System (`slapper-plugin` & `slapper-ruby`)

The plugin system allows developers to extend Slapper's capabilities using high-level languages like Python and Ruby.

### Python Plugins

- **Integration**: Uses `pyo3` to bridge between Rust and Python.
- **Capabilities**: Python plugins can implement custom scanners, fuzzer mutators, or output formatters.
- **Example**: See `examples/plugins/example_scanner.py`.

### Ruby Plugins

- **Integration**: Managed via the `slapper-ruby` crate.
- **Metasploit Integration**: Provides a bridge to Metasploit RPC, allowing Slapper to trigger Metasploit modules directly.
- **Example**: See `examples/plugins/metasploit_example.rb`.

### Plugin Security

Both Python and Ruby plugins are validated for suspicious patterns before loading:

**Python Security Checks (`slapper-plugin/src/security.rs`):**
- Regex patterns for dangerous constructs: `os.system`, `subprocess`, `socket`, `eval`, `fork`, `__import__`, `open(`, `pty.spawn`, `ctypes`, etc.
- AST-based analysis via `ast_scanner.rs` for deeper inspection
- Maximum plugin size: 1MB

**Ruby Security Checks (`slapper-ruby/src/security.rs`):**
- Regex patterns for dangerous constructs: `eval`, `exec`, `system`, backticks, `IO.popen`, `Process.spawn`, `File.read/write/open`, `Net::HTTP`, `Socket`, etc.

**Configuration:**
```rust
pub struct PluginConfig {
    pub enabled: bool,
    pub config: HashMap<String, serde_json::Value>,
    pub block_suspicious_plugins: bool,  // default: true
    pub timeout_secs: u64,               // default: 300
    pub max_file_size_bytes: usize,      // default: 1_000_000
}
```

## NSE (Nmap Scripting Engine) Integration (`slapper-nse`)

Slapper includes a full-featured Lua interpreter (via `mlua`) that can run standard Nmap NSE scripts.

### Core Features

- **Compatibility**: Supports a vast majority of existing NSE scripts.
- **Sandbox**: Optionally restricts dangerous Lua operations (e.g., file system access, network connections) for safer execution of untrusted scripts.
- **NSE Tool**: Provides a high-level API for running NSE scripts against targets discovered by Slapper.

### Sandbox Configuration

```rust
pub struct SandboxConfig {
    pub enabled: bool,                    // Controlled by `sandbox` feature
    pub allowed_dir: Option<PathBuf>,     // Restrict file ops to directory (default: /tmp/slapper-nse)
    pub allowed_commands: Vec<String>,   // Whitelist for io.popen
    pub log_violations: bool,             // Log instead of block
    pub allowed_networks: Vec<IpNetwork>, // CIDR allowlist for sockets
}
```

### Sandboxed Operations

| Library | Operations | Sandbox Enforcement |
|---------|------------|---------------------|
| `io` | `open()`, `lines()`, `popen()`, `tmpfile()` | Path canonicalization, command allowlist |
| `lfs` | All file operations | Path validation against `allowed_dir` |
| `os` | `getenv()`, `setenv()` | Blocked in sandbox |
| `socket` | `connect()`, `tcp_connect()`, `sendto()` | Host validation against `allowed_networks` |

### Benefits

- **Instant Capability**: Access to thousands of community-developed security checks from day one.
- **Lua Scripting**: Simple and familiar scripting language for custom security logic.
- **Seamless Integration**: NSE results are integrated into Slapper's finding management and reporting system.

### NSE Libraries

164 NSE-style library modules implemented including: `stdnse`, `nmap`, `http`, `socket`, `io`, `os`, `lfs`, `dns`, `ssl`, `ssh`, `mysql`, `postgres`, `redis`, `mongodb`, `ldap`, `snmp`, `smb`, `smb2`, `vulns`, and many more. All located in `crates/slapper-nse/src/libraries/`.

### CVE Integration

The `vulns` library provides access to CVE databases:
- **NVD** (National Vulnerability Database) - `https://services.nvd.nist.gov/rest/json/cves/2.0`
- **OSV** (Open Source Vulnerabilities)
- **CISA KEV** (Known Exploited Vulnerabilities)

## Recent Bug Fixes

| Issue | Fix |
|-------|-----|
| Ruby `load_plugin()` had no timeout | Added `recv_timeout()` with 300s default |
| Python plugin result truncation was silent | Now logs count of truncated findings |
| UDP `sendto()` didn't validate sandbox | `connect_udp()` now checks host via `is_host_allowed()` |
| Duplicate `getenv` registration in `os.rs` | Removed duplicate `getenv_fn2` at line 295-302 |
| `output.rs` multiple `unwrap()` on `writeln!` calls | Changed to use `let _ = writeln!()` pattern |
| `CveCache` used `HashMap` instead of `FxHashMap` | Changed to `FxHashMap` for performance |
| `CveAggregator` used `HashSet` instead of `FxHashSet` | Changed to `FxHashSet` for performance |
| Path traversal check bypass via `..` string check | Removed simple string check; rely on `is_path_allowed()` |
| `async_executor.rs` Default impl panicked | Changed to `unwrap_or_else` panic with descriptive message |
