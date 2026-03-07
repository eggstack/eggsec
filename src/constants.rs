//! Centralized constants for Slapper
//!
//! This module contains all magic numbers, strings, and default values
//! used throughout the codebase. Using constants improves maintainability
//! by making it easy to find and update hardcoded values.

pub const DEFAULT_REMOTE_PORT: u16 = 7890;
pub const DEFAULT_TRACEROUTE_PORT: u16 = 33434;
pub const DEFAULT_PROXY_TIMEOUT_MS: u64 = 10000;
pub const DEFAULT_HEALTH_CHECK_INTERVAL_SECS: u64 = 60;
pub const DEFAULT_MAX_HEALTH_CHECK_FAILURES: u32 = 3;

pub const WAYBACK_SNAPSHOT_LIMIT: usize = 100;
pub const DEFAULT_ICMP_PAYLOAD_SIZE: usize = 56;

pub const DEFAULT_CONFIG_FILE: &str = "slapper.toml";
pub const DEFAULT_WORDLIST: &str = "wordlists/directories.txt";

pub const SUPPORTED_WAF_COUNT: usize = 30;

pub mod severity {
    pub const CRITICAL: &str = "critical";
    pub const HIGH: &str = "high";
    pub const MEDIUM: &str = "medium";
    pub const LOW: &str = "low";
    pub const INFO: &str = "info";
}

pub mod http {
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
    pub const DEFAULT_MAX_REDIRECTS: u32 = 5;
    pub const DEFAULT_CONCURRENCY: usize = 10;
}

pub mod scan {
    pub const DEFAULT_PORT_RANGE: &str = "1-1024";
    pub const DEFAULT_PORT_CONCURRENCY: usize = 100;
    pub const DEFAULT_ENDPOINT_CONCURRENCY: usize = 20;
}

pub mod cache {
    pub const DEFAULT_TTL_SECS: u64 = 3600;
    pub const DEFAULT_MAX_ENTRIES: usize = 10000;
}

pub mod nvd {
    pub const DEFAULT_RATE_LIMIT_DELAY_MS: u64 = 6000;
}

pub mod ui {
    pub const CHECK_MARK: &str = "✓";
    pub const CROSS_MARK: &str = "✗";
    pub const ARROW: &str = "→";

    pub const WIDTH_DEFAULT: usize = 58;
}

pub mod errors {
    pub const FAILED_TO_CREATE_CLIENT: &str = "Failed to create HTTP client";
    pub const TARGET_NOT_IN_SCOPE: &str = "Target is not in allowed scope";
    pub const FAILED_TO_CONNECT: &str = "Failed to connect";
    pub const FAILED_TO_RESOLVE: &str = "Failed to resolve host";
    pub const FAILED_TO_SEND_REQUEST: &str = "Failed to send request";
    pub const TIMEOUT_EXCEEDED: &str = "Request timed out";
    pub const INVALID_URL: &str = "Invalid URL";
    pub const INVALID_TARGET: &str = "Invalid target";
    pub const FILE_NOT_FOUND: &str = "File not found";
    pub const PARSE_ERROR: &str = "Failed to parse";
    pub const SERIALIZE_ERROR: &str = "Failed to serialize";
    pub const CONFIG_ERROR: &str = "Configuration error";
    pub const SCAN_FAILED: &str = "Scan failed";
    pub const AUTH_REQUIRED: &str = "Authentication required";
    pub const RATE_LIMITED: &str = "Rate limited";
}
