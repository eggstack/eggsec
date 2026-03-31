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

impl From<serde_yaml::Error> for SlapperError {
    fn from(e: serde_yaml::Error) -> Self {
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

impl From<hickory_resolver::error::ResolveError> for SlapperError {
    fn from(e: hickory_resolver::error::ResolveError) -> Self {
        SlapperError::Network(format!("DNS resolution failed: {}", e))
    }
}

impl From<anyhow::Error> for SlapperError {
    fn from(e: anyhow::Error) -> Self {
        SlapperError::Runtime(e.to_string())
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
