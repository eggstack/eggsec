use thiserror::Error;

/// The primary error type for eggsec operations.
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
/// use eggsec::error::{EggsecError, Result};
///
/// fn validate_target(host: &str) -> Result<()> {
///     if host.is_empty() {
///         return Err(EggsecError::InvalidTarget("empty host".into()));
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
/// use eggsec::error::{EggsecError, Result};
///
/// fn check_timeout() -> Result<()> {
///     Err(EggsecError::Timeout {
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
pub enum EggsecError {
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

    #[error("Internal error: {0}")]
    Internal(String),

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

impl EggsecError {
    /// Returns true if this error is a timeout error.
    pub fn is_timeout(&self) -> bool {
        matches!(self, EggsecError::Timeout { .. })
    }

    /// Returns true if this error is a network/connection error.
    pub fn is_network(&self) -> bool {
        matches!(self, EggsecError::Network(_))
    }

    /// Returns the HTTP status code if this is an HTTP error.
    pub fn http_status(&self) -> Option<u16> {
        match self {
            EggsecError::HttpStatus { status, .. } => Some(*status),
            _ => None,
        }
    }

    /// Sets the timeout_ms value for Timeout errors, preserving the operation name.
    /// Returns self for chaining. For non-Timeout errors, returns self unchanged.
    ///
    /// # Example
    ///
    /// ```
    /// use eggsec::error::{EggsecError, Result};
    ///
    /// fn make_request_with_timeout(timeout_ms: u64) -> Result<()> {
    ///     Err(EggsecError::Timeout {
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
        if let EggsecError::Timeout { operation, .. } = self {
            EggsecError::Timeout {
                timeout_ms,
                operation,
            }
        } else {
            self
        }
    }
}

pub type Result<T> = std::result::Result<T, EggsecError>;

impl From<reqwest::Error> for EggsecError {
    fn from(e: reqwest::Error) -> Self {
        if e.is_timeout() {
            EggsecError::Timeout {
                timeout_ms: 0,
                operation: e
                    .url()
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
            }
        } else if e.is_connect() {
            EggsecError::Network(format!("Connection failed: {}", e))
        } else if let Some(status) = e.status() {
            EggsecError::HttpStatus {
                status: status.as_u16(),
                message: status.canonical_reason().unwrap_or("Unknown").to_string(),
            }
        } else {
            EggsecError::RequestFailed {
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

impl From<toml::de::Error> for EggsecError {
    fn from(e: toml::de::Error) -> Self {
        EggsecError::Parse(format!("TOML parse error: {}", e))
    }
}

impl From<serde_json::Error> for EggsecError {
    fn from(e: serde_json::Error) -> Self {
        EggsecError::Parse(format!("JSON error: {}", e))
    }
}

impl From<url::ParseError> for EggsecError {
    fn from(e: url::ParseError) -> Self {
        EggsecError::Parse(format!("URL parse error: {}", e))
    }
}

impl From<std::net::AddrParseError> for EggsecError {
    fn from(e: std::net::AddrParseError) -> Self {
        EggsecError::AddressParse(format!("Invalid address: {}", e))
    }
}

impl From<serde_yaml_neo::Error> for EggsecError {
    fn from(e: serde_yaml_neo::Error) -> Self {
        EggsecError::Parse(format!("YAML error: {}", e))
    }
}

impl From<toml::ser::Error> for EggsecError {
    fn from(e: toml::ser::Error) -> Self {
        EggsecError::Parse(format!("TOML serialization error: {}", e))
    }
}

impl From<std::string::FromUtf8Error> for EggsecError {
    fn from(e: std::string::FromUtf8Error) -> Self {
        EggsecError::Parse(format!("UTF-8 error: {}", e))
    }
}

impl From<tokio::time::error::Elapsed> for EggsecError {
    fn from(_e: tokio::time::error::Elapsed) -> Self {
        EggsecError::Timeout {
            timeout_ms: 0,
            operation: "async operation".to_string(),
        }
    }
}

impl From<crate::config::ScopeError> for EggsecError {
    fn from(e: crate::config::ScopeError) -> Self {
        EggsecError::ScopeViolation(e.to_string())
    }
}

impl From<hickory_resolver::net::NetError> for EggsecError {
    fn from(e: hickory_resolver::net::NetError) -> Self {
        EggsecError::Network(format!("DNS resolution failed: {}", e))
    }
}

impl From<anyhow::Error> for EggsecError {
    fn from(e: anyhow::Error) -> Self {
        let msg = if let Some(src) = e.source() {
            format!("{}: {}", e, src)
        } else {
            e.to_string()
        };
        EggsecError::Internal(msg)
    }
}

#[cfg(feature = "ai-integration")]
impl From<crate::ai::AiError> for EggsecError {
    fn from(e: crate::ai::AiError) -> Self {
        match e {
            crate::ai::AiError::RequestFailed(msg) => EggsecError::RequestFailed {
                method: "AI".to_string(),
                url: "ai-api".to_string(),
                error: msg,
            },
            crate::ai::AiError::MissingApiKey => {
                EggsecError::Config("Missing AI API key".to_string())
            }
            crate::ai::AiError::InvalidConfig(msg) => {
                EggsecError::Config(format!("AI config error: {}", msg))
            }
            crate::ai::AiError::ApiError(msg) => EggsecError::RequestFailed {
                method: "AI".to_string(),
                url: "ai-api".to_string(),
                error: msg,
            },
            crate::ai::AiError::ParseError(msg) => {
                EggsecError::Parse(format!("AI parse error: {}", msg))
            }
            crate::ai::AiError::Timeout => EggsecError::Timeout {
                timeout_ms: 0,
                operation: "ai-request".to_string(),
            },
            crate::ai::AiError::RateLimited => {
                EggsecError::RateLimited("AI rate limit exceeded".to_string())
            }
            crate::ai::AiError::InvalidResponse => {
                EggsecError::Parse("Invalid AI response".to_string())
            }
            crate::ai::AiError::CircuitBreakerOpen => {
                EggsecError::RateLimited("AI circuit breaker open".to_string())
            }
        }
    }
}

#[cfg(feature = "packet-inspection")]
impl From<crate::packet::CaptureError> for EggsecError {
    fn from(e: crate::packet::CaptureError) -> Self {
        EggsecError::Network(format!("Packet capture error: {}", e))
    }
}

#[cfg(any(feature = "packet-inspection", feature = "stress-testing"))]
impl From<crate::packet::TracerouteError> for EggsecError {
    fn from(e: crate::packet::TracerouteError) -> Self {
        EggsecError::Network(format!("Traceroute error: {}", e))
    }
}

impl From<std::num::ParseIntError> for EggsecError {
    fn from(e: std::num::ParseIntError) -> Self {
        EggsecError::Parse(format!("Integer parse error: {}", e))
    }
}

impl From<tokio::sync::AcquireError> for EggsecError {
    fn from(e: tokio::sync::AcquireError) -> Self {
        EggsecError::Runtime(format!("Semaphore acquire error: {}", e))
    }
}

impl From<quick_xml::Error> for EggsecError {
    fn from(e: quick_xml::Error) -> Self {
        EggsecError::Output(format!("XML error: {}", e))
    }
}

impl From<maxminddb::MaxMindDbError> for EggsecError {
    fn from(e: maxminddb::MaxMindDbError) -> Self {
        EggsecError::Io(std::io::Error::other(format!("MaxMind DB error: {}", e)))
    }
}

impl From<reqwest::header::InvalidHeaderValue> for EggsecError {
    fn from(e: reqwest::header::InvalidHeaderValue) -> Self {
        EggsecError::Http(format!("Invalid header value: {}", e))
    }
}

#[cfg(feature = "web-proxy")]
impl From<eggsec_web_proxy::WebProxyError> for EggsecError {
    fn from(e: eggsec_web_proxy::WebProxyError) -> Self {
        EggsecError::Network(format!("Web proxy error: {}", e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_timeout() {
        let err = EggsecError::Timeout {
            timeout_ms: 5000,
            operation: "http request".to_string(),
        };
        assert!(err.is_timeout());
        assert!(!err.is_network());
    }

    #[test]
    fn test_error_is_network() {
        let err = EggsecError::Network("connection refused".to_string());
        assert!(err.is_network());
        assert!(!err.is_timeout());
    }

    #[test]
    fn test_error_http_status() {
        let err = EggsecError::HttpStatus {
            status: 404,
            message: "Not Found".to_string(),
        };
        assert_eq!(err.http_status(), Some(404));

        let err = EggsecError::Config("test".to_string());
        assert_eq!(err.http_status(), None);
    }

    #[test]
    fn test_error_display() {
        let err = EggsecError::InvalidTarget("empty host".to_string());
        assert_eq!(err.to_string(), "Invalid target: empty host");
    }

    #[test]
    fn test_result_type() {
        fn example() -> Result<String> {
            Err(EggsecError::Runtime("something went wrong".to_string()))
        }
        let err = example().unwrap_err();
        assert_eq!(err.to_string(), "Runtime error: something went wrong");
    }
}
