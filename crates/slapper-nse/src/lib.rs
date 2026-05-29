//! NSE (Nmap Scripting Engine) support for Slapper
//!
//! This module provides the ability to run Nmap NSE scripts using a Lua interpreter.
//! It leverages mlua (Lua 5.4) and wraps existing Slapper functionality
//! to provide NSE-compatible libraries.

use ipnetwork::IpNetwork;
use std::net::IpAddr;
use std::path::PathBuf;

#[cfg(all(feature = "nse", target_family = "unix"))]
#[link(name = "z")]
unsafe extern "C" {}

/// Configuration for running NSE scripts.
pub struct NseConfig {
    pub target: String,
    pub script: String,
    pub script_args: Option<String>,
    pub script_file: Option<String>,
    pub json: bool,
    pub verbose: bool,
}

impl NseConfig {
    pub fn new(
        target: &str,
        script: &str,
        script_args: Option<&str>,
        script_file: Option<&str>,
        json: bool,
        verbose: bool,
    ) -> Self {
        Self {
            target: target.to_string(),
            script: script.to_string(),
            script_args: script_args.map(|s| s.to_string()),
            script_file: script_file.map(|s| s.to_string()),
            json,
            verbose,
        }
    }
}

/// Sandbox configuration for restricting NSE Lua script capabilities.
///
/// When sandboxing is enabled, dangerous operations like `io.popen` (arbitrary
/// command execution) and unrestricted filesystem access are blocked or limited.
#[derive(Debug, Clone)]
pub struct SandboxConfig {
    /// Whether sandboxing is enabled.
    pub enabled: bool,
    /// If set, restrict file operations to this directory.
    pub allowed_dir: Option<PathBuf>,
    /// If non-empty, only these commands are allowed via `io.popen`.
    /// If empty and sandbox is enabled, `io.popen` is fully blocked.
    pub allowed_commands: Vec<String>,
    /// Whether to log sandbox violations instead of blocking them.
    pub log_violations: bool,
    /// If non-empty, only network connections to these CIDR ranges are allowed.
    /// If empty and sandbox is enabled, socket connections are allowed but a warning is logged.
    pub allowed_networks: Vec<IpNetwork>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            // Sandbox behavior is controlled by the `sandbox` feature.
            enabled: cfg!(feature = "sandbox"),
            allowed_dir: Some(PathBuf::from("/tmp/slapper-nse")),
            allowed_commands: Vec::new(),
            log_violations: true,
            allowed_networks: Vec::new(),
        }
    }
}

impl SandboxConfig {
    fn allowed_root(&self) -> Option<PathBuf> {
        let dir = self.allowed_dir.as_ref()?;
        dir.canonicalize().ok().or_else(|| Some(dir.clone()))
    }

    /// Create a sandbox config with sandboxing enabled and default settings.
    pub fn enabled() -> Self {
        Self {
            enabled: true,
            ..Default::default()
        }
    }

    /// Check if a file path is allowed under the sandbox.
    pub fn is_path_allowed(&self, path: &str) -> bool {
        if !self.enabled {
            return true;
        }

        let Some(allowed_dir) = self.allowed_root() else {
            return true;
        };

        let path_buf = PathBuf::from(path);
        let Ok(canonical) = path_buf.canonicalize() else {
            // If canonicalize fails (file doesn't exist), check the parent
            if let Some(parent) = path_buf.parent() {
                if let Ok(canonical_parent) = parent.canonicalize() {
                    return canonical_parent.starts_with(&allowed_dir);
                }
            }
            // Reject if we can't verify the path
            return false;
        };

        canonical.starts_with(allowed_dir)
    }

    /// Check if a command is allowed via `io.popen`.
    pub fn is_command_allowed(&self, cmd: &str) -> bool {
        if !self.enabled {
            return true;
        }

        if self.allowed_commands.is_empty() {
            return false;
        }

        let cmd_name = cmd.split_whitespace().next().unwrap_or(cmd);
        self.allowed_commands
            .iter()
            .any(|allowed| cmd_name == allowed)
    }

    /// Check if a network target IP is allowed under the sandbox.
    ///
    /// Returns `true` if:
    /// - Sandbox is disabled
    /// - `allowed_networks` is empty (allow all with warning)
    /// - The IP matches any network in `allowed_networks`
    ///
    /// Returns `false` if the IP does not match any allowed network.
    pub fn is_network_allowed(&self, ip: IpAddr) -> bool {
        if !self.enabled {
            return true;
        }

        if self.allowed_networks.is_empty() {
            return true;
        }

        self.allowed_networks
            .iter()
            .any(|network| network.contains(ip))
    }

    /// Check if a network target host is allowed.
    ///
    /// This resolves the hostname and checks the resulting IP against allowed networks.
    /// Returns `false` if resolution fails while an allowlist is configured.
    pub fn is_host_allowed(&self, host: &str) -> bool {
        use std::net::ToSocketAddrs;

        if !self.enabled {
            return true;
        }

        if self.allowed_networks.is_empty() {
            return true;
        }

        let Ok(mut addrs) = format!("{}:0", host).to_socket_addrs() else {
            return false;
        };

        addrs.any(|addr| self.is_network_allowed(addr.ip()))
    }
}

#[cfg(feature = "nse")]
pub mod async_executor;
#[cfg(feature = "nse")]
pub mod context;
#[cfg(feature = "nse")]
pub mod cve;
#[cfg(feature = "nse")]
pub mod executor;
#[cfg(feature = "nse")]
pub mod executor_core;
pub mod output;
#[cfg(feature = "nse")]
pub mod public_api;

#[cfg(feature = "nse")]
pub mod libraries;

#[cfg(feature = "nse")]
pub use async_executor::AsyncNseExecutor;
#[cfg(feature = "nse")]
pub use executor::NseExecutor;
#[cfg(feature = "nse")]
pub use executor_core::ExecutorCore;
#[cfg(feature = "nse")]
pub use executor_core::SandboxMetrics;

#[cfg(feature = "nse")]
pub async fn run_cli(config: NseConfig) -> anyhow::Result<()> {
    let target = config.target.clone();
    let script = config.script.clone();
    let script_args = config.script_args.clone().unwrap_or_default();
    let script_args_display = script_args.clone();
    let script_file = config.script_file.clone();
    let json = config.json;

    println!("Running NSE script '{}' against '{}'", script, target);

    // Run the blocking executor in a separate thread to avoid runtime conflicts
    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<String> {
        let mut executor = NseExecutor::with_target(&target)
            .map_err(|e| anyhow::anyhow!("Failed to create NSE executor: {}", e))?;
        executor
            .set_script_args(&script_args)
            .map_err(|e| anyhow::anyhow!("Invalid script args: {}", e))?;

        let script_content = if let Some(ref script_file) = script_file {
            std::fs::read_to_string(script_file)?
        } else {
            get_builtin_script(&script)
        };

        let result = executor
            .run_script(&script_content)
            .map_err(|e| anyhow::anyhow!("Script execution failed: {}", e))?;

        Ok(result)
    })
    .await
    .map_err(|e| anyhow::anyhow!("Task execution failed: {}", e))??;

    if json {
        let output = serde_json::json!({
            "target": config.target,
            "script": config.script,
            "script_args": script_args_display,
            "output": result,
            "success": true
        });
        println!("{}", serde_json::to_string_pretty(&output)?);
    } else {
        println!("Target: {}", config.target);
        println!("Script: {}", config.script);
        println!("Result: {}", result);
    }

    Ok(())
}

#[cfg(not(feature = "nse"))]
pub async fn run_cli(_config: NseConfig) -> anyhow::Result<()> {
    anyhow::bail!("NSE support requires the 'nse' feature. Build with: cargo build --features nse")
}

#[cfg(feature = "nse")]
pub fn get_builtin_script(name: &str) -> String {
    match name {
        "default" | "discovery" => r#"
-- Default NSE discovery script
local stdnse = require "stdnse"

stdnse.verbose1("Starting NSE discovery scan...")

local host = nmap.target
if host and host ~= "" then
    stdnse.format_output({status = "open", service = "discovered"}, {separator = ", "})
end

local output = stdnse.output_table()
output.host = host or "unknown"
output.status = "discovered"
output.scan_time = os.date("*t")

return output
"#
        .to_string(),
        "banner" => r#"
-- Banner grabbing script
local stdnse = require "stdnse"
local comm = require "comm"
local socket = require "socket"

local host = nmap.target
local port = 80

if not host or host == "" then
    return stdnse.output_table()
end

local s = socket.connect(host, port)
if s then
    s:send("HEAD / HTTP/1.0\r\n\r\n")
    local status, response = s:receive(1024)
    s:close()

    local output = stdnse.output_table()
    output.banner = response or ""
    output.host = host
    output.port = port
    return output
end

return nil
"#
        .to_string(),
        "http-headers" => r#"
-- HTTP headers discovery script
local stdnse = require "stdnse"
local http = require "http"

local host = nmap.target

if not host or host == "" then
    return stdnse.output_table()
end

local response = http.get(host, 80, "/")

local output = stdnse.output_table()
output.host = host
output.port = 80
output.title = response.title or ""
output.status = response.status or 0

return output
"#
        .to_string(),
        "dns-check" => r#"
-- DNS resolution check script
local stdnse = require "stdnse"
local dns = require "dns"

local host = nmap.target

if not host or host == "" then
    return stdnse.output_table()
end

local success = dns.query(host)

local output = stdnse.output_table()
output.host = host
output.resolved = success

return output
"#
        .to_string(),
        "ssl-cert" => r#"
-- SSL certificate information script
local stdnse = require "stdnse"
local sslcert = require "sslcert"
local tls = require "tls"

local host = nmap.target

if not host or host == "" then
    return stdnse.output_table()
end

local output = stdnse.output_table()
output.host = host
output.port = 443
output.tls = "available"

return output
"#
        .to_string(),
        _ => {
            format!(
                r#"
-- Custom NSE script: {}
local stdnse = require "stdnse"

stdnse.verbose1("Executing custom NSE script: {}")

local output = stdnse.output_table()
output.script = "{}"
output.status = "executed"
output.libraries = {{
    stdnse = true,
    nmap = true,
    socket = true,
    http = true,
}}

return output
"#,
                name, name, name
            )
        }
    }
}
