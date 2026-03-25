//! Network scanning and service discovery
//!
//! This module provides comprehensive network scanning capabilities including:
//! - TCP port scanning with configurable concurrency and timing
//! - Endpoint discovery using wordlist-based brute forcing
//! - Service fingerprinting through banner grabbing
//! - UDP service detection
//! - ICMP probing (feature-gated)
//!
//! ## Key Components
//!
//! - [`scan_ports`] - Scan TCP ports on target hosts
//! - [`scan_endpoints`] - Discover HTTP endpoints using wordlists
//! - [`fingerprint_services`] - Identify services by banner grabbing
//! - [`SpoofConfig`] - Configuration for IP spoofing and decoy scanning
//! - [`TimingPreset`] - Predefined timing configurations (Paranoid to Insane)
//!
//! ## Feature Flags
//!
//! - `stress-testing` - Enables ICMP probing, IP spoofing, and advanced scanning features
//!
//! ## Usage
//!
//! ### Basic Port Scan
//!
//! ```rust,no_run
//! use slapper::scanner::{scan_ports, SpoofConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let results = scan_ports(
//!     "example.com",
//!     vec![80, 443, 8080],
//!     100,  // concurrency
//!     Duration::from_secs(5),
//!     false,  // tui_mode
//!     SpoofConfig::default(),
//! ).await?;
//!
//! for port in &results.open_ports {
//!     println!("Open: {}/tcp ({})", port.port, port.service);
//! }
//! # Ok(())
//! # }
//! ```
//!
//! ### Endpoint Discovery
//!
//! ```rust,no_run
//! use slapper::scanner::scan_endpoints;
//! use slapper::cli::CommonHttpArgs;
//!
//! # async fn example() -> anyhow::Result<()> {
//! let results = scan_endpoints(
//!     "https://example.com",
//!     "wordlists/common.txt",
//!     20,  // concurrency
//!     30,  // timeout_secs
//!     false,  // tui_mode
//!     CommonHttpArgs::default(),
//! ).await?;
//!
//! println!("Found {} endpoints", results.endpoints.len());
//! # Ok(())
//! # }
//! ```
//!
//! ## Errors
//!
//! Functions return [`anyhow::Result`] and will fail if:
//! - DNS resolution fails for the target host
//! - Network connectivity issues occur
//! - Invalid port ranges are specified
//! - File I/O errors occur (wordlists, output files)

pub mod endpoints;
pub mod fingerprint;
pub mod ports;
pub mod spoof;
pub mod timing;
pub mod udp_fingerprint;

#[cfg(feature = "stress-testing")]
pub mod icmp_probe;

pub use endpoints::{scan_endpoints, EndpointResult, EndpointScanResults};
pub use fingerprint::{fingerprint_services, FingerprintResults, ServiceFingerprint};
pub use ports::{scan_ports, PortResult, PortScanResults};
pub use spoof::{format_spoof_warning, random_ip_from_cidr, SpoofConfig, SpoofStats};
pub use timing::{PortPriority, TimingConfig, TimingPreset};
pub use udp_fingerprint::{
    fingerprint_udp_services, get_default_udp_ports, UdpFingerprintResults, UdpServiceFingerprint,
};

#[cfg(feature = "stress-testing")]
pub use icmp_probe::{ping_host, PingResult, PingStats};
