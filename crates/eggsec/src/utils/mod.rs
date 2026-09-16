//! Engine-internal utility helpers (Phase A ownership cleanup, Phase D closure).
//!
//! Remaining modules are engine infrastructure shared by several domains but
//! not yet stable enough for a crate. Domain-specific helpers have moved to
//! their owners: service tables to `scanner::service_data`, privilege gates to
//! `platform`, cron to `eggsec-agent::cron`. Dead presentation/pool/evasion
//! helpers (`output`, `progress`, `client_pool`, `stealth`) were removed in Phase A; the unused `cache` (`ApiCache`, zero production consumers) was removed in Phase D.
//!
//! ## Key Components
//!
//! - [`http`] - HTTP client creation with various configurations
//! - [`parsing`] - URL and header parsing utilities
//! - [`target`] - Target extraction and normalization
//! - [`formatting`] - String truncation and formatting
//!
//! ## Usage
//!
//! ```rust,no_run
//! use eggsec::utils::{create_http_client, strip_controls};
//!
//! # fn example() -> eggsec::error::Result<()> {
//! // Create HTTP client
//! let client = create_http_client(30)?;
//!
//! // Strip control characters
//! let cleaned = strip_controls("Some text with \x00 control chars", 100);
//! # Ok(())
//! # }
//! ```

pub mod auth;
pub mod circuit_breaker;
pub mod error;
pub mod formatting;
pub mod http;
pub mod logging;
pub mod network;
pub mod parsing;
pub mod rate_limiter;
pub mod redaction;
pub mod target;
pub mod urlencoding;
pub mod validation;

pub use auth::constant_time_eq;
pub use circuit_breaker::{CircuitBreaker, CircuitState};
pub use formatting::{preserve_all, strip_controls};
pub use http::{
    create_http_client, create_http_client_with_options, create_http_client_with_proxy,
    create_insecure_client_with_options, create_insecure_http_client, get_shared_http_client,
    get_shared_insecure_http_client, same_host_redirect_policy, tool_user_agent,
};
pub use logging::sanitize_for_logging;
pub use network::{connect_with_nodelay, connect_with_nodelay_timeout};
pub use parsing::{contains_ignore_case, parse_headers, parse_url_validated};
pub use target::{
    extract_domain, extract_host_port, extract_target_from_url, is_ip_address, normalize_url,
    parse_host_port, parse_socket_addr, strip_url_protocol,
};
pub use validation::{
    validate_concurrency, validate_git_repo_path, validate_path, validate_path_string,
    validate_rate_limit, validate_timeout, validate_url,
};
