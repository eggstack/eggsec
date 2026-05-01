use thiserror::Error;

/// The primary error type for slapper operations.
///
/// Each variant represents a distinct failure domain. Use the corresponding
/// variant when propagating errors from library code; `From` impls handle
/// automatic conversion from common third-party error types.
///
/// # Examples
///
/// Basic usage:
///
/// ```
/// use slapper::error::{SlapperError, Result};
///
/// fn validate_target(host: &str) -> Result<()> {
///     if host.is_empty() {
///         return Err(SlapperError::InvalidTarget("empty host".into()));
///     }
///     Ok(())
/// }
///
/// let err = validate_target("").unwrap_err();
/// assert!(err.to_string().contains("empty host"));
/// ```
///
/// Using helper methods:
///
/// ```
/// use slapper::error::{SlapperError, Result};
///
/// fn check_timeout() -> Result<()> {
///     Err(SlapperError::Timeout {
///         timeout_ms: 5000,
///         operation: "scan".into(),
///     })
/// }
///
/// let err = check_timeout().unwrap_err();
/// assert!(err.is_timeout());
/// assert!(!err.is_network());
/// ```
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

    /// Timeout error. Note: when converted from `reqwest::Error`, the timeout_ms
    /// field will be 0 since reqwest doesn't expose the configured timeout value.
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

    #[error("HTTP error {status}: {message}")]
    HttpStatus { status: u16, message: String },

    #[error("HTTP error: {0}")]
    Http(String),

    #[error("Parse error: {0}")]
    Parse(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Address parse error: {0}")]
    AddressParse(String),

    #[error("Runtime error: {0}")]
    Runtime(String),

    #[error("Cancelled")]
    Cancelled,

    #[error("Proxy error: {0}")]
    Proxy(String),

    #[error("Recon error: {0}")]
    Recon(String),

    #[error("Load test error: {0}")]
    LoadTest(String),

    #[error("Fingerprint error: {0}")]
    Fingerprint(String),
}

impl SlapperError {
    /// Returns true if this error is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, SlapperError::Timeout { .. })
    }

    /// Returns true if this error is a network/connection error.
    pub fn is_network(&self) -> bool {
        matches!(self, SlapperError::Network(_))
    }

    /// Returns the HTTP status code if this is an HTTP error.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            SlapperError::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Sets the timeout_ms value for Timeout errors, preserving the operation name.
    /// Returns self for chaining. For non-Timeout errors, returns self unchanged.
    ///
    /// # Example
    ///
    /// ```
    /// use slapper::error::{SlapperError, Result};
    ///
    /// fn make_request_with_timeout(timeout_ms: u64) -> Result<()> {
    ///     Err(SlapperError::Timeout {
    ///         timeout_ms: 0,
    ///         operation: "scan".into(),
    ///     })
    ///     .map_err(|e| e.with_timeout(timeout_ms))
    /// }
    ///
    /// let result = make_request_with_timeout(5000);
    /// assert!(result.is_err());
    /// let err = result.unwrap_err();
    /// assert!(err.is_timeout());
    /// ```
    pub fn with_timeout(self, timeout_ms: u64) -> Self {
        if let SlapperError::Timeout { operation, .. } = self {
            SlapperError::Timeout {
                timeout_ms,
                operation,
            }
        } else {
            self
        }
    }
}

pub type Result<T> = std::result::Result<T, SlapperError>;

impl From<reqwest::Error> for SlapperError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            SlapperError::Timeout {
                timeout_ms: 0,
                operation: e
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        } else if e.is_connect() {
            SlapperError::Network(format!("Connection failed: {}", e))
        } else if let Some(status) = e.status() {
            SlapperError::HttpStatus {
                status: status.as_u16(),
                message: status.canonical_reason().unwrap_or("Unknown").to_string(),
            }
        } else {
            SlapperError::RequestFailed {
                method: "UNKNOWN".to_string(),
                url: e
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
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

impl From<std::net::AddrParseError> for SlapperError {
    fn from(e: std::net::AddrParseError) -> Self {
        SlapperError::AddressParse(format!("Invalid address: {}", e))
    }
}

impl From<serde_yaml_neo::Error> for SlapperError {
    fn from(e: serde_yaml_neo::Error) -> Self {
        SlapperError::Parse(format!("YAML error: {}", e))
    }
}

impl From<toml::ser::Error> for SlapperError {
    fn from(e: toml::ser::Error) -> Self {
        SlapperError::Parse(format!("TOML serialization error: {}", e))
    }
}

impl From<std::string::FromUtf8Error> for SlapperError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        SlapperError::Parse(format!("UTF-8 error: {}", e))
    }
}

impl From<tokio::time::error::Elapsed> for SlapperError {
    fn from(_e: tokio::time::error::Elapsed) -> Self {
        SlapperError::Timeout {
            timeout_ms: 0,
            operation: "async operation".to_string(),
        }
    }
}

impl From<crate::config::ScopeError> for SlapperError {
    fn from(e: crate::config::ScopeError) -> Self {
        SlapperError::ScopeViolation(e.to_string())
    }
}

impl From<hickory_resolver::error::ResolveError> for SlapperError {
    fn from(e: hickory_resolver::error::ResolveError) -> Self {
        SlapperError::Network(format!("DNS resolution failed: {}", e))
    }
}

impl From<anyhow::Error> for SlapperError {
    fn from(e: anyhow::Error) -> Self {
        SlapperError::RequestFailed {
            method: "UNKNOWN".to_string(),
            url: "unknown".to_string(),
            error: e.to_string(),
        }
    }
}

#[cfg(feature = "ai-integration")]
impl From<crate::ai::AiError> for SlapperError {
    fn from(e: crate::ai::AiError) -> Self {
        match e {
            crate::ai::AiError::RequestFailed(msg) => SlapperError::RequestFailed {
                method: "AI".to_string(),
                url: "ai-api".to_string(),
                error: msg,
            },
            crate::ai::AiError::MissingApiKey => {
                SlapperError::Config("Missing AI API key".to_string())
            }
            crate::ai::AiError::InvalidConfig(msg) => {
                SlapperError::Config(format!("AI config error: {}", msg))
            }
            crate::ai::AiError::ApiError(msg) => SlapperError::RequestFailed {
                method: "AI".to_string(),
                url: "ai-api".to_string(),
                error: msg,
            },
            crate::ai::AiError::ParseError(msg) => {
                SlapperError::Parse(format!("AI parse error: {}", msg))
            }
            crate::ai::AiError::Timeout => SlapperError::Timeout {
                timeout_ms: 0,
                operation: "ai-request".to_string(),
            },
            crate::ai::AiError::RateLimited => {
                SlapperError::RateLimited("AI rate limit exceeded".to_string())
            }
            crate::ai::AiError::InvalidResponse => {
                SlapperError::Parse("Invalid AI response".to_string())
            }
            crate::ai::AiError::CircuitBreakerOpen => {
                SlapperError::RateLimited("AI circuit breaker open".to_string())
            }
        }
    }
}

#[cfg(feature = "packet-inspection")]
impl From<crate::packet::CaptureError> for SlapperError {
    fn from(e: crate::packet::CaptureError) -> Self {
        SlapperError::Network(format!("Packet capture error: {}", e))
    }
}

#[cfg(any(feature = "packet-inspection", feature = "stress-testing"))]
impl From<crate::packet::TracerouteError> for SlapperError {
    fn from(e: crate::packet::TracerouteError) -> Self {
        SlapperError::Network(format!("Traceroute error: {}", e))
    }
}

impl From<std::num::ParseIntError> for SlapperError {
    fn from(e: std::num::ParseIntError) -> Self {
        SlapperError::Parse(format!("Integer parse error: {}", e))
    }
}

impl From<tokio::sync::AcquireError> for SlapperError {
    fn from(e: tokio::sync::AcquireError) -> Self {
        SlapperError::Runtime(format!("Semaphore acquire error: {}", e))
    }
}

impl From<quick_xml::Error> for SlapperError {
    fn from(e: quick_xml::Error) -> Self {
        SlapperError::Output(format!("XML error: {}", e))
    }
}

impl From<maxminddb::MaxMindDbError> for SlapperError {
    fn from(e: maxminddb::MaxMindDbError) -> Self {
        SlapperError::Io(std::io::Error::other(format!("MaxMind DB error: {}", e)))
    }
}

impl From<reqwest::header::InvalidHeaderValue> for SlapperError {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        SlapperError::Http(format!("Invalid header value: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_timeout() {
        let err = SlapperError::Timeout {
            timeout_ms: 5000,
            operation: "http request".to_string(),
        };
        assert!(err.is_timeout());
        assert!(!err.is_network());
    }

    #[test]
    fn test_error_is_network() {
        let err = SlapperError::Network("connection refused".to_string());
        assert!(err.is_network());
        assert!(!err.is_timeout());
    }

    #[test]
    fn test_error_http_status() {
        let err = SlapperError::HttpStatus {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(err.http_status(), Some(404));

        let err = SlapperError::Config("test".to_string());
        assert_eq!(err.http_status(), None);
    }

    #[test]
    fn test_error_display() {
        let err = SlapperError::InvalidTarget("empty host".to_string());
        assert_eq!(err.to_string(), "Invalid target: empty host");
    }

    #[test]
    fn test_result_type() {
        fn example() -> Result<String> {
            Err(SlapperError::Runtime("something went wrong".to_string()))
        }
        let err = example().unwrap_err();
        assert_eq!(err.to_string(), "Runtime error: something went wrong");
    }
}
