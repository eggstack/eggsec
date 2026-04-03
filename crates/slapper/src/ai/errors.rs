use thiserror::Error;

#[derive(Error, Debug)]
pub enum AiError {
    #[error("API request failed: {0}")]
    RequestFailed(#[from] reqwest::Error),

    #[error("Authentication failed: missing API key")]
    MissingApiKey,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("API returned error response: {0}")]
    ApiError(String),

    #[error("Parse error: {0}")]
    ParseError(String),

    #[error("Timeout error")]
    Timeout,

    #[error("Rate limit exceeded")]
    RateLimited,

    #[error("Invalid response format from API")]
    InvalidResponse,

    #[error("Circuit breaker open for endpoint: {0}")]
    CircuitBreakerOpen(String),
}

impl AiError {
    pub fn invalid_config(msg: impl Into<String>) -> Self {
        AiError::InvalidConfig(msg.into())
    }

    pub fn api_error(msg: impl Into<String>) -> Self {
        AiError::ApiError(msg.into())
    }

    pub fn parse_error(msg: impl Into<String>) -> Self {
        AiError::ParseError(msg.into())
    }
}

pub type Result<T> = std::result::Result<T, AiError>;
