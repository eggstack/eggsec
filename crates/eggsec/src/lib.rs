//! Eggsec - High-performance security testing toolkit
//!
//! A comprehensive, async-first security testing toolkit built in Rust.
//! Provides reconnaissance, port scanning, endpoint discovery, service
//! fingerprinting, WAF detection/bypass, security fuzzing, load testing,
//! and distributed scanning capabilities.
//!
//! ## Workspace Crates
//!
//! - `eggsec-core`: dependency-light shared types and constants.
//! - `eggsec-tool-core`: protocol-neutral tool request/response/error/history types.
//! - `eggsec-output`: report formatting and output adapters.
//! - `eggsec-nse`: optional Nmap NSE compatibility support.
//! - `eggsec-tui`: terminal UI adapter crate.
//! - `eggsec-cli`: user-facing binary package; binary name is `eggsec`.
//! - `eggsec-agent`: agent coordination primitives (extracted from `tool/agents/`).
//!
//! The main `eggsec` crate owns the assessment engine, command dispatch,
//! scope/config loading, and feature-gated integrations.
//!
//! ## Architecture
//!
//! The crate is organized into these main module groups:
//!
//! - **`cli`** - Command-line argument parsing (clap-based)
//! - **`commands`** - Command dispatch and handler implementations
//! - **`config`** - Configuration loading (TOML/YAML), scope enforcement
//! - **`scanner`** - Port scanning, endpoint discovery, service fingerprinting
//! - **`fuzzer`** - Security fuzzing engine with 40 payload types
//! - **`waf`** - WAF detection (34 products) and bypass techniques
//! - **`recon`** - Passive reconnaissance (DNS, WHOIS, SSL, tech detection, CVE mapping)
//! - **`loadtest`** - HTTP load testing with metrics
//! - **`pipeline`** - Chained security assessment profiles
//! - **`tool`** - Tool registry/execution framework; core DTOs live in `eggsec-tool-core` (feature-gated)
//! - **`output`** - Compatibility facade over `eggsec-output` plus engine-coupled report modules
//! - **`distributed`** - Worker/coordinator cluster architecture
//! - **`proxy`** - SOCKS/HTTP/Tor proxy pool management
//! - **`packet`** - Packet capture and crafting (feature-gated)
//! - **`stress`** - Network stress testing (feature-gated)
//!
//! ## Feature Flags
//!
//! - `default` - Core scanning, fuzzing, WAF, load testing
//! - `tool-api` - Tool abstraction layer (always enabled internally)
//! - `insecure-tls` - Disables TLS certificate verification (testing only)
//! - `stress-testing` - DoS tools, proxy management, raw sockets
//! - `packet-inspection` - Live packet capture, traceroute
//! - `rest-api` / `grpc-api` - API server integration
//! - `ws-api` - WebSocket API server support
//! - `nse` - Nmap NSE script support
//! - `nse-ssh2` - NSE with full SSH2/libssh2-backed support
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
//! - `git-secrets` - Git secrets scanning
//! - `wireless` - Wireless security testing (WiFi scanning (passive reconnaissance and basic security analysis))
//! - `pdf` - PDF report generation
//! - `full` - All features combined
//!
//! ## Error Handling
//!
//! Core library modules use [`EggsecError`] (via [`Result`]) as the canonical
//! error type. Each variant maps to a failure domain (network, config, scan, etc.).
//! Command handlers and binary entry points use `anyhow::Result` for convenience;
//! `.map_err()` bridges convert between the two at call-site boundaries.
//!
//! Prefer `EggsecError` variants over `anyhow!()` in library code. Use
//! `From` impls (e.g., `From<std::io::Error>`) for automatic conversion from
//! third-party error types.

#[cfg(feature = "api-schema")]
pub mod api_schema;
pub mod auth;
pub mod auth_context;
#[cfg(feature = "headless-browser")]
pub mod browser;
pub mod cli;
pub mod commands;
#[cfg(feature = "compliance")]
pub mod compliance;
#[cfg(not(feature = "compliance"))]
#[allow(dead_code)]
mod compliance;
pub mod config;
pub mod constants;
#[cfg(feature = "container")]
pub mod container;
#[cfg(not(feature = "container"))]
#[allow(dead_code)]
mod container;
pub mod distributed;
pub mod error;
pub mod findings;
pub mod fuzzer;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
#[cfg(not(feature = "advanced-hunting"))]
#[allow(dead_code)]
mod hunt;
#[cfg(feature = "external-integrations")]
pub mod integrations;
#[cfg(not(feature = "external-integrations"))]
#[allow(dead_code)]
mod integrations;
pub mod loadtest;
pub mod logging;
pub mod notify;
pub mod output;
pub mod pipeline;
pub mod probe;
pub mod proxy;
pub mod recon;
pub mod scanner;
#[cfg(feature = "database")]
pub mod storage;
#[cfg(not(feature = "database"))]
#[allow(dead_code)]
mod storage;
#[cfg(feature = "stress-testing")]
pub mod stress;
#[cfg(feature = "sbom")]
pub mod supply_chain;
#[cfg(not(feature = "sbom"))]
#[allow(dead_code)]
mod supply_chain;
pub mod types;
pub mod utils;
#[cfg(feature = "vuln-management")]
pub mod vuln;
#[cfg(not(feature = "vuln-management"))]
#[allow(dead_code)]
mod vuln;
pub mod waf;
#[cfg(feature = "websocket")]
pub mod websocket;
#[cfg(feature = "finding-workflow")]
pub mod workflow;
#[cfg(not(feature = "finding-workflow"))]
#[allow(dead_code)]
mod workflow;

#[cfg(any(feature = "tool-api", feature = "rest-api", feature = "grpc-api"))]
pub mod tool;

#[cfg(feature = "ai-integration")]
pub mod ai;

#[cfg(feature = "rest-api")]
pub mod agent;

#[cfg(feature = "nse")]
pub use eggsec_nse as nse;

#[cfg(all(feature = "nse", feature = "tool-api"))]
pub mod nse_tool;

#[cfg(any(feature = "packet-inspection", feature = "stress-testing"))]
pub mod packet;

#[cfg(feature = "wireless")]
pub mod wireless;

#[cfg(feature = "mobile")]
pub mod mobile;
#[cfg(not(feature = "mobile"))]
#[allow(dead_code)]
mod mobile {
    // Stub module so `crate::mobile` name resolves for cfg-gated call sites.
    // Real implementation (and its optional `zip`/`plist` deps + cli::MobileArgs)
    // is only compiled when the `mobile` feature is enabled.
}

#[cfg(feature = "db-pentest")]
pub mod db_pentest;

pub use config::{load_config, load_scope, EggsecConfig, Scope};
pub use error::{EggsecError, Result};
pub use types::Severity;
