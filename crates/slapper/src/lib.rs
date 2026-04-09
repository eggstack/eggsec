//! Slapper - High-performance security testing toolkit
//!
//! A comprehensive, async-first security testing toolkit built in Rust.
//! Provides reconnaissance, port scanning, endpoint discovery, service
//! fingerprinting, WAF detection/bypass, security fuzzing, load testing,
//! and distributed scanning capabilities.
//!
//! ## Architecture
//!
//! The crate is organized into these main module groups:
//!
//! - **`cli`** - Command-line argument parsing (clap-based)
//! - **`commands`** - Command dispatch and handler implementations
//! - **`config`** - Configuration loading (TOML/YAML), scope enforcement
//! - **`scanner`** - Port scanning, endpoint discovery, service fingerprinting
//! - **`fuzzer`** - Security fuzzing engine with 22 payload types
//! - **`waf`** - WAF detection (26 products) and bypass techniques
//! - **`recon`** - Passive reconnaissance (DNS, WHOIS, SSL, tech detection, CVE mapping)
//! - **`loadtest`** - HTTP load testing with metrics
//! - **`pipeline`** - Chained security assessment profiles
//! - **`tool`** - REST API/MCP/gRPC integration for AI agents (feature-gated)
//! - **`tui`** - Real-time terminal UI (ratatui-based)
//! - **`output`** - Multiple report formats (JSON, HTML, CSV, SARIF, JUnit)
//! - **`distributed`** - Worker/coordinator cluster architecture
//! - **`proxy`** - SOCKS/HTTP/Tor proxy pool management
//! - **`packet`** - Packet capture and crafting (feature-gated)
//! - **`stress`** - Network stress testing (feature-gated)
//!
//! ## Feature Flags
//!
//! - `default` - Core scanning, fuzzing, WAF, load testing
//! - `tool-api` - Tool abstraction layer (always enabled internally)
//! - `stress-testing` - DoS tools, proxy management, raw sockets
//! - `packet-inspection` - Live packet capture, traceroute
//! - `python-plugins` / `ruby-plugins` - Plugin language support
//! - `all-plugins` - All plugin languages combined
//! - `rest-api` / `grpc-api` - API server integration
//! - `nse` - Nmap NSE script support
//! - `nse-sandbox` - NSE sandbox mode (restricts dangerous operations)
//! - `ai-integration` - AI/LLM analysis and WAF bypass suggestions
//! - `websocket` - WebSocket security testing
//! - `headless-browser` - DOM XSS and SPA crawling
//! - `database` - SQLx-based storage for findings and history
//! - `container` - Kubernetes container security scanning
//! - `sbom` - SBOM generation and vulnerability checking
//! - `advanced-hunting` - Advanced threat hunting techniques
//! - `compliance` - Compliance scanning and reporting
//! - `external-integrations` - Jira, GitHub, GitLab integrations
//! - `finding-workflow` - Finding lifecycle management
//! - `vuln-management` - Vulnerability triage and prioritization
//! - `cloud` - Cloud security scanning (AWS, GCP, Azure)
//! - `api-schema` - API schema-based fuzzing (OpenAPI v3)
//! - `git-secrets` - Git secrets scanning
//! - `pdf` - PDF report generation
//! - `full` - All features combined
//!
//! ## Error Handling
//!
//! Core library modules use [`SlapperError`] (via [`Result`]) as the canonical
//! error type. Each variant maps to a failure domain (network, config, scan, etc.).
//! Command handlers and binary entry points use `anyhow::Result` for convenience;
//! `.map_err()` bridges convert between the two at call-site boundaries.
//!
//! Prefer `SlapperError` variants over `anyhow!()` in library code. Use
//! `From` impls (e.g., `From<std::io::Error>`) for automatic conversion from
//! third-party error types.

pub mod cli;
pub mod commands;
pub mod config;
pub mod constants;
pub mod distributed;
pub mod error;
pub mod fuzzer;
pub mod loadtest;
pub mod logging;
pub mod notify;
pub mod output;
pub mod pipeline;
pub mod proxy;
pub mod recon;
pub mod scanner;
pub mod auth;
#[cfg(feature = "container")]
pub mod container;
#[cfg(not(feature = "container"))]
#[allow(dead_code)]
mod container;
#[cfg(feature = "database")]
pub mod storage;
#[cfg(not(feature = "database"))]
#[allow(dead_code)]
mod storage;
#[cfg(feature = "sbom")]
pub mod supply_chain;
#[cfg(not(feature = "sbom"))]
#[allow(dead_code)]
mod supply_chain;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
#[cfg(not(feature = "advanced-hunting"))]
#[allow(dead_code)]
mod hunt;
#[cfg(feature = "compliance")]
pub mod compliance;
#[cfg(not(feature = "compliance"))]
#[allow(dead_code)]
mod compliance;
#[cfg(feature = "external-integrations")]
pub mod integrations;
#[cfg(not(feature = "external-integrations"))]
#[allow(dead_code)]
mod integrations;
#[cfg(feature = "finding-workflow")]
pub mod workflow;
#[cfg(not(feature = "finding-workflow"))]
#[allow(dead_code)]
mod workflow;
#[cfg(feature = "vuln-management")]
pub mod vuln;
#[cfg(not(feature = "vuln-management"))]
#[allow(dead_code)]
mod vuln;
#[cfg(feature = "websocket")]
pub mod websocket;
#[cfg(feature = "headless-browser")]
pub mod browser;
#[cfg(feature = "stress-testing")]
pub mod stress;
pub mod tui;
pub mod types;
pub mod utils;
pub mod waf;

#[cfg(any(
    feature = "tool-api",
    feature = "rest-api",
    feature = "grpc-api"
))]
pub mod tool;

#[cfg(feature = "ai-integration")]
pub mod ai;

#[cfg(feature = "rest-api")]
pub mod agent;

#[cfg(feature = "nse")]
pub use slapper_nse as nse;

#[cfg(all(feature = "nse", feature = "tool-api"))]
pub mod nse_tool;

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub use slapper_plugin as plugin;

#[cfg(feature = "ruby-plugins")]
pub use slapper_ruby as ruby;

#[cfg(any(feature = "packet-inspection", feature = "stress-testing"))]
pub mod packet;


pub use config::{load_config, load_scope, Scope, SlapperConfig};
pub use error::{Result, SlapperError};
pub use types::Severity;
