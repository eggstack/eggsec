#![allow(dead_code)]

use thiserror::Error;

#[derive(Debug, Error)]
pub enum SlapperError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Invalid target: {0}")]
    InvalidTarget(String),

    #[error("Network error: {0}")]
    Network(String),

    #[error("Request failed: {method} {url} - {error}")]
    RequestFailed {
        method: String,
        url: String,
        error: String,
    },

    #[error("Timeout after {timeout_ms}ms: {operation}")]
    Timeout { timeout_ms: u64, operation: String },

    #[error("Rate limited: {0}")]
    RateLimited(String),

    #[error("Scan failed: {stage} - {error}")]
    ScanFailed { stage: String, error: String },

    #[error("Payload error: {0}")]
    Payload(String),

    #[error("Output error: {0}")]
    Output(String),

    #[error("Plugin error: {0}")]
    Plugin(String),

    #[error("Scope violation: {0}")]
    ScopeViolation(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),
}

pub type Result<T> = std::result::Result<T, SlapperError>;

impl From<reqwest::Error> for SlapperError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            SlapperError::Timeout {
                timeout_ms: 0,
                operation: e.url().map(|u| u.to_string()).unwrap_or_default(),
            }
        } else if e.is_connect() {
            SlapperError::Network(format!("Connection failed: {}", e))
        } else if e.is_status() {
            SlapperError::Http(format!("HTTP error: {}", e))
        } else {
            SlapperError::RequestFailed {
                method: "UNKNOWN".to_string(),
                url: e.url().map(|u| u.to_string()).unwrap_or_default(),
                error: e.to_string(),
            }
        }
    }
}

impl From<toml::de::Error> for SlapperError {
    fn from(e: toml::de::Error) -> Self {
        SlapperError::Parse(format!("TOML parse error: {}", e))
    }
}

impl From<serde_json::Error> for SlapperError {
    fn from(e: serde_json::Error) -> Self {
        SlapperError::Parse(format!("JSON error: {}", e))
    }
}

impl From<url::ParseError> for SlapperError {
    fn from(e: url::ParseError) -> Self {
        SlapperError::Parse(format!("URL parse error: {}", e))
    }
}

pub trait ErrorContext<T> {
    fn context(self, msg: &str) -> Result<T>;
    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String;
}

impl<T, E: std::fmt::Display> ErrorContext<T> for std::result::Result<T, E> {
    fn context(self, msg: &str) -> Result<T> {
        self.map_err(|e| SlapperError::Parse(format!("{}: {}", msg, e)))
    }

    fn with_context<F>(self, f: F) -> Result<T>
    where
        F: FnOnce() -> String,
    {
        self.map_err(|e| SlapperError::Parse(format!("{}: {}", f(), e)))
    }
}

#[macro_export]
macro_rules! bail {
    ($msg:expr_2021) => {
        return Err($crate::error::SlapperError::from($msg))
    };
    ($fmt:expr_2021, $($arg:expr_2021),+) => {
        return Err($crate::error::SlapperError::from(format!($fmt, $($arg),+)))
    };
}

#[macro_export]
macro_rules! ensure {
    ($cond:expr_2021, $msg:expr_2021) => {
        if !($cond) {
            return Err($crate::error::SlapperError::from($msg));
        }
    };
    ($cond:expr_2021, $fmt:expr_2021, $($arg:expr_2021),+) => {
        if !($cond) {
            return Err($crate::error::SlapperError::from(format!($fmt, $($arg),+)));
        }
    };
}
