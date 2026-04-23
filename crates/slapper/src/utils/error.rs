//! Error message sanitization utilities
//!
//! Provides functions to sanitize error messages to prevent exposure of
//! internal system information, stack traces, or file paths to clients.

use regex::Regex;
use std::sync::LazyLock;

static PATH_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(/[\w\.-]+)+").unwrap()
});

static STACK_TRACE_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(at\s+[\w.$]+\([^)]*\)\s*(in\s+)?[^\n]+)").unwrap()
});

static INTERNAL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(internal|impl|thread\s+'[\w-]+').*").unwrap()
});

static RATE_LIMIT_DETAIL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(RateLimiter|rate_limit|check_rate_limit).*").unwrap()
});

pub fn sanitize_error_message(error: &str) -> String {
    let mut sanitized = error.to_string();

    sanitized = STACK_TRACE_PATTERN
        .replace_all(&sanitized, "[stack trace hidden]")
        .to_string();

    sanitized = INTERNAL_PATTERN
        .replace_all(&sanitized, "[internal details hidden]")
        .to_string();

    sanitized = PATH_PATTERN
        .replace_all(&sanitized, "[path hidden]")
        .to_string();

    if sanitized.len() > 200 {
        sanitized.truncate(197);
        sanitized.push_str("...");
    }

    sanitized
}

pub fn sanitize_rate_limit_error(error: &str) -> String {
    let sanitized = sanitize_error_message(error);
    RATE_LIMIT_DETAIL
        .replace_all(&sanitized, "Rate limit exceeded")
        .to_string()
}

pub fn sanitize_internal_error() -> String {
    "An internal error occurred. Please check logs for details.".to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sanitize_removes_stack_traces() {
        let error = "at com.example.Foo.bar(Foo.java:123) in thread 'main'";
        let result = sanitize_error_message(error);
        assert!(!result.contains("Foo.java"));
    }

    #[test]
    fn test_sanitize_removes_paths() {
        let error = "Failed to read /etc/slapper/config.yaml";
        let result = sanitize_error_message(error);
        assert!(!result.contains("/etc/slapper"));
    }

    #[test]
    fn test_sanitize_truncates_long_errors() {
        let error = "x".repeat(300);
        let result = sanitize_error_message(&error);
        assert!(result.len() <= 200);
    }

    #[test]
    fn test_rate_limit_sanitization() {
        let error = "RateLimiter check_rate_limit failed: too many requests";
        let result = sanitize_rate_limit_error(error);
        assert!(!result.contains("check_rate_limit"));
    }
}
