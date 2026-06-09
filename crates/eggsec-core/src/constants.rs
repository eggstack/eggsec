//! Centralized constants for Eggsec
//!
//! This module contains all magic numbers, strings, and default values
//! used throughout the codebase. Using constants improves maintainability
//! by making it easy to find and update hardcoded values.

pub const PROJECT_QUALIFIER: &str = "tools";
pub const PROJECT_NAME: &str = "eggsec";

pub const DEFAULT_EXPORT_DIR: &str = "./exports";

pub const DEFAULT_REMOTE_PORT: u16 = 7890;

pub const DEFAULT_CONFIG_FILE: &str = "eggsec.toml";

pub const DEFAULT_MAX_RETRIES: u32 = 3;
pub const DEFAULT_RETRY_DELAY_MS: u64 = 1000;

pub const DEFAULT_POOL_IDLE_TIMEOUT_SECS: u64 = 30;
pub const DEFAULT_POOL_MAX_IDLE_PER_HOST: usize = 20;

pub const DEFAULT_TOOL_TIMEOUT_MS: u64 = 30000;
pub const DEFAULT_BROWSER_TIMEOUT_MS: u64 = 60000;
pub const BROWSER_TIMEOUT_BUFFER_MS: u64 = 10000;
pub const DEFAULT_PROXY_TIMEOUT_MS: u64 = 10000;

pub const DEFAULT_TASK_QUEUE_CAPACITY: usize = 10000;
pub const WORKER_STALE_TIMEOUT_SECS: i64 = 90;
pub const DEFAULT_LEASE_DURATION_MS: u64 = 300000;
pub const DEFAULT_SCHEDULER_RETRY_DELAY_MS: u64 = 30000;

pub const MAX_REQUESTS_PER_SECOND_LIMIT: u32 = 10000;

pub const STATUS_RATE_LIMITED: u16 = 429;
pub const STATUS_FORBIDDEN: u16 = 403;
pub const STATUS_LOCKED: u16 = 423;
pub const STATUS_SERVER_ERROR: u16 = 503;

pub const SUPPORTED_WAF_COUNT: usize = 34;

pub mod http {
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
    pub const DEFAULT_MAX_REDIRECTS: u32 = 10;
    pub const DEFAULT_CONCURRENCY: usize = 10;
}

pub mod scan {
    pub const DEFAULT_PORT_CONCURRENCY: usize = 100;
}

pub mod cache {
    pub const DEFAULT_TTL_SECS: u64 = 3600;
}

pub mod waf {
    pub const MAX_REDIRECTS: usize = 5;
    pub const HEADER_MATCH_SCORE: u16 = 25;
    pub const COOKIE_MATCH_SCORE: u16 = 20;
    pub const BODY_MATCH_SCORE: u16 = 15;
    pub const IP_MATCH_SCORE: u16 = 20;
    pub const UNKNOWN_WAF_CONFIDENCE: u16 = 30;
    pub const LENGTH_DIFF_THRESHOLD: usize = 100;
    pub const HIGH_CONFIDENCE_EXIT: u16 = 90;
    pub const BLOCKED_STATUS_CODES: [u16; 4] = [403, 406, 429, 503];
    pub const BLOCKED_PATTERNS: [&str; 8] = [
        "access denied",
        "request blocked",
        "your request has been blocked",
        "malicious request",
        "security policy violation",
        "forbidden",
        "waf",
        "firewall",
    ];
    pub const WEAK_BLOCK_INDICATOR_PATTERNS: [&str; 4] =
        ["security", "unauthorized", "suspicious", "rate limit"];
    pub const UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD: usize = 2;
    pub const SMUGGLING_TIMEOUT_SECS: u64 = 15;
    pub const SMUGGLING_TIMEOUT_MS: u64 = 15_000;
}
