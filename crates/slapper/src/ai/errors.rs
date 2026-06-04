use reqwest::Error as ReqwestError;
use std::io::Error as IoError;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum AiError {
    #[error("API request failed: {0}")]
    RequestFailed(String),

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

    #[error("Circuit breaker open")]
    CircuitBreakerOpen,
}

impl From<ReqwestError> for AiError {
    fn from(err: ReqwestError) -> Self {
        if err.is_timeout() {
            AiError::Timeout
        } else if err.is_connect() {
            AiError::RequestFailed(err.to_string())
        } else if err.status().map(|s| s.as_u16() == crate::constants::STATUS_RATE_LIMITED).unwrap_or(false) {
            AiError::RateLimited
        } else {
            AiError::RequestFailed(err.to_string())
        }
    }
}

impl From<IoError> for AiError {
    fn from(err: IoError) -> Self {
        AiError::RequestFailed(err.to_string())
    }
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
