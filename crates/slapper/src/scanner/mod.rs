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
//! # async fn example() -> slapper::error::Result<()> {
//! let results = scan_ports(
//!     "example.com",
//!     vec![80, 443, 8080],
//!     100,  // concurrency
//!     Duration::from_secs(5),
//!     false,  // tui_mode
//!     SpoofConfig::default(),
//!     None,  // progress_tx
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
//! use slapper::scanner::endpoints::{scan_endpoints, EndpointScanConfig};
//! use std::time::Duration;
//!
//! # async fn example() -> slapper::error::Result<()> {
//! let config = EndpointScanConfig {
//!     base_url: "https://example.com".to_string(),
//!     endpoints: vec!["admin".to_string(), "login".to_string()],
//!     concurrency: 20,
//!     timeout_duration: Duration::from_secs(30),
//!     include_404: false,
//!     tui_mode: false,
//!     spoof_config: std::sync::Arc::new(Default::default()),
//!     verify_tls: true,
//!     progress_tx: None,
//! };
//!
//! let results = scan_endpoints(config).await?;
//!
//! println!("Found {} endpoints", results.endpoints_found);
//! # Ok(())
//! # }
//! ```
//!
//! ## Errors
//!
//! Functions return [`crate::error::Result`] and will fail if:
//! - DNS resolution fails for the target host
//! - Network connectivity issues occur
//! - Invalid port ranges are specified
//! - File I/O errors occur (wordlists, output files)

pub mod endpoints;
pub mod fingerprint;
pub mod ports;
pub mod spoof;
pub mod templates;
pub mod timing;
pub mod udp_fingerprint;

#[cfg(feature = "stress-testing")]
pub mod icmp_probe;

pub use endpoints::{scan_endpoints, EndpointResult, EndpointScanConfig, EndpointScanResults};
pub use fingerprint::{fingerprint_services, FingerprintResults, ServiceFingerprint};
pub use ports::{
    scan_ports, PortResult, PortScanConfig, PortScanResults, COMMON_PORTS, MAX_SCAN_RESULTS,
};
pub use spoof::{format_spoof_warning, random_ip_from_cidr, SpoofConfig, SpoofStats};
pub use timing::{PortPriority, TimingConfig, TimingPreset};
pub use udp_fingerprint::{
    fingerprint_udp_services, get_default_udp_ports, UdpFingerprintResults, UdpServiceFingerprint,
};

#[cfg(feature = "stress-testing")]
pub use icmp_probe::{ping_host, PingResult, PingStats};
