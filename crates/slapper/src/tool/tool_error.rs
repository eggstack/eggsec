use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolError {
    pub code: String,
    pub message: String,
    pub details: Option<serde_json::Value>,
    pub target: Option<String>,
    pub recoverable: bool,
    pub error_type: ToolErrorType,
    pub retry_after_ms: Option<u64>,
}

impl ToolError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            details: None,
            target: None,
            recoverable: false,
            error_type: ToolErrorType::Internal,
            retry_after_ms: None,
        }
    }

    pub fn with_details(mut self, details: serde_json::Value) -> Self {
        self.details = Some(details);
        self
    }

    pub fn at_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    pub fn with_error_type(mut self, error_type: ToolErrorType) -> Self {
        self.error_type = error_type;
        self.recoverable = error_type.is_recoverable();
        self
    }

    pub fn with_retry_after(mut self, ms: u64) -> Self {
        self.retry_after_ms = Some(ms);
        self.recoverable = true;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ToolErrorType {
    Validation,
    Authentication,
    Authorization,
    RateLimit,
    Network,
    Timeout,
    ScopeViolation,
    NotFound,
    Configuration,
    Internal,
    ToolNotFound,
}

impl ToolErrorType {
    pub fn is_recoverable(&self) -> bool {
        matches!(
            self,
            ToolErrorType::RateLimit
                | ToolErrorType::Timeout
                | ToolErrorType::Network
                | ToolErrorType::Internal
        )
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            ToolErrorType::Validation => "validation",
            ToolErrorType::Authentication => "authentication",
            ToolErrorType::Authorization => "authorization",
            ToolErrorType::RateLimit => "rate_limit",
            ToolErrorType::Network => "network",
            ToolErrorType::Timeout => "timeout",
            ToolErrorType::ScopeViolation => "scope_violation",
            ToolErrorType::NotFound => "not_found",
            ToolErrorType::Configuration => "configuration",
            ToolErrorType::Internal => "internal",
            ToolErrorType::ToolNotFound => "tool_not_found",
        }
    }
}

impl std::fmt::Display for ToolErrorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}
