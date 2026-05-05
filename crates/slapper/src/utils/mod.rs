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

pub mod auth;
pub mod cache;
pub mod circuit_breaker;
pub mod client_pool;
pub mod error;
pub mod formatting;
pub mod http;
pub mod logging;
pub mod network;
pub mod output;
pub mod parsing;
pub mod progress;
pub mod rate_limiter;
pub mod scope;
pub mod service_detection;
pub mod stealth;
pub mod target;
pub mod urlencoding;
pub mod validation;

#[cfg(any(feature = "stress-testing", feature = "packet-inspection"))]
pub mod privilege;

pub use auth::constant_time_eq;
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use client_pool::{ClientPool, OptimizedClientPool};
pub use formatting::{preserve_all, strip_controls};
pub use http::{
    create_http_client, create_http_client_with_options, create_http_client_with_proxy,
    create_insecure_client_with_options, create_insecure_http_client, get_shared_http_client,
    get_shared_insecure_http_client,
};
pub use logging::sanitize_for_logging;
pub use network::{connect_with_nodelay, connect_with_nodelay_timeout};
pub use output::{
    print_error, print_info, print_json, print_json_compact, print_success, print_warning,
};
pub use parsing::{contains_ignore_case, parse_headers, parse_url_validated};
pub use scope::{check_scope, check_scope_from_url};
pub use target::{
    extract_domain, extract_host_port, extract_target_from_url, is_ip_address, normalize_url,
    parse_host_port, parse_socket_addr, strip_url_protocol,
};
pub use validation::{
    validate_concurrency, validate_git_repo_path, validate_path, validate_path_string,
    validate_rate_limit, validate_timeout, validate_url,
};

#[cfg(any(feature = "stress-testing", feature = "packet-inspection"))]
pub use privilege::{check_privileged, is_root, require_root};
