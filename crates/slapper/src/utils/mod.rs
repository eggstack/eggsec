//! Utility functions and helpers
//!
//! Common utilities used throughout the codebase for HTTP handling,
//! URL parsing, scope checking, and output formatting.
//!
//! ## Key Components
//!
//! - [`http`] - HTTP client creation with various configurations
//! - [`scope`] - Target scope validation
//! - [`parsing`] - URL and header parsing utilities
//! - [`target`] - Target extraction and normalization
//! - [`formatting`] - String truncation and formatting
//! - [`output`] - Terminal output helpers (colors, icons)
//!
//! ## Usage
//!
//! ```rust,no_run
//! use slapper::utils::{check_scope, create_http_client, strip_controls};
//!
//! # fn example() -> slapper::error::Result<()> {
//! // Create HTTP client
//! let client = create_http_client(30)?;
//!
//! // Strip control characters
//! let cleaned = strip_controls("Some text with \x00 control chars", 100);
//! # Ok(())
//! # }
//! ```

pub mod cache;
pub mod circuit_breaker;
pub mod client_pool;
pub mod formatting;
pub mod http;
pub mod output;
pub mod parsing;
pub mod rate_limiter;
pub mod scope;
pub mod service_detection;
pub mod stealth;
pub mod target;
pub mod urlencoding;
pub mod validation;

#[cfg(any(feature = "stress-testing", feature = "packet-inspection"))]
pub mod privilege;

pub use formatting::{preserve_all, strip_controls};
pub use circuit_breaker::{CircuitBreaker, CircuitBreakerRegistry, CircuitState};
pub use client_pool::{ClientPool, OptimizedClientPool};
pub use http::{
    create_http_client, create_http_client_with_options, create_http_client_with_proxy,
    create_insecure_client_with_options, create_insecure_http_client,
};
pub use output::{
    print_error, print_info, print_json, print_json_compact, print_success, print_warning,
};
pub use parsing::{parse_headers, parse_url_validated};
pub use scope::{check_scope, check_scope_from_url};
pub use target::{
    extract_domain, extract_host_port, extract_target_from_url, is_ip_address, normalize_url,
    parse_host_port, parse_socket_addr,
};

#[cfg(any(feature = "stress-testing", feature = "packet-inspection"))]
pub use privilege::{check_privileged, is_root, require_root};
