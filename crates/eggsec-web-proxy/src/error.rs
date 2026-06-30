use std::fmt;

/// Error type for the web-proxy domain crate.
#[derive(Debug)]
pub enum WebProxyError {
    /// Proxy connection or protocol error.
    Proxy(String),
    /// Network error (bind, connect, timeout).
    Network(String),
    /// Configuration error.
    Config(String),
    /// IO error.
    Io(std::io::Error),
    /// TLS/certificate error.
    Tls(String),
    /// Intercept engine error.
    Intercept(String),
    /// Rule engine error.
    Rule(String),
    /// Protocol detection or handling error.
    Protocol(String),
    /// Timeout with details.
    Timeout { timeout_ms: u64, operation: String },
}

impl fmt::Display for WebProxyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Proxy(msg) => write!(f, "Proxy error: {}", msg),
            Self::Network(msg) => write!(f, "Network error: {}", msg),
            Self::Config(msg) => write!(f, "Config error: {}", msg),
            Self::Io(e) => write!(f, "IO error: {}", e),
            Self::Tls(msg) => write!(f, "TLS error: {}", msg),
            Self::Intercept(msg) => write!(f, "Intercept error: {}", msg),
            Self::Rule(msg) => write!(f, "Rule error: {}", msg),
            Self::Protocol(msg) => write!(f, "Protocol error: {}", msg),
            Self::Timeout {
                timeout_ms,
                operation,
            } => {
                write!(f, "Timeout after {}ms: {}", timeout_ms, operation)
            }
        }
    }
}

impl std::error::Error for WebProxyError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for WebProxyError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for WebProxyError {
    fn from(e: serde_json::Error) -> Self {
        Self::Intercept(format!("JSON error: {}", e))
    }
}

impl From<serde_yaml_neo::Error> for WebProxyError {
    fn from(e: serde_yaml_neo::Error) -> Self {
        Self::Config(format!("YAML error: {}", e))
    }
}

impl From<reqwest::Error> for WebProxyError {
    fn from(e: reqwest::Error) -> Self {
        Self::Network(format!("HTTP client error: {}", e))
    }
}

impl From<std::num::ParseIntError> for WebProxyError {
    fn from(e: std::num::ParseIntError) -> Self {
        Self::Proxy(format!("Parse integer error: {}", e))
    }
}

impl From<tokio::sync::AcquireError> for WebProxyError {
    fn from(e: tokio::sync::AcquireError) -> Self {
        Self::Network(format!("Semaphore acquire error: {}", e))
    }
}

/// Convenience Result type for the web-proxy domain crate.
pub type Result<T> = std::result::Result<T, WebProxyError>;
