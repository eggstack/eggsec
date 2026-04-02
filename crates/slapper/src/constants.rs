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

pub const SUPPORTED_WAF_COUNT: usize = 26;

#[cfg(test)]
mod tests {
    use super::SUPPORTED_WAF_COUNT;

    #[test]
    fn supported_waf_count_matches_actual() {
        let count = crate::waf::waf_patterns::get_waf_signatures().len();
        assert_eq!(
            count, SUPPORTED_WAF_COUNT,
            "SUPPORTED_WAF_COUNT must match actual detector count"
        );
    }
}

pub mod http {
    pub const DEFAULT_TIMEOUT_SECS: u64 = 30;
    pub const DEFAULT_MAX_REDIRECTS: u32 = 10;
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

pub mod waf {
    pub const HEADER_MATCH_SCORE: u8 = 25;
    pub const COOKIE_MATCH_SCORE: u8 = 20;
    pub const BODY_MATCH_SCORE: u8 = 15;
    pub const UNKNOWN_WAF_CONFIDENCE: u8 = 30;
    pub const LENGTH_DIFF_THRESHOLD: usize = 100;
    pub const HIGH_CONFIDENCE_EXIT: u8 = 90;
    pub const BLOCKED_STATUS_CODES: [u16; 4] = [403, 406, 429, 503];
    pub const BLOCKED_PATTERNS: [&str; 5] = ["blocked", "denied", "forbidden", "waf", "firewall"];
}
