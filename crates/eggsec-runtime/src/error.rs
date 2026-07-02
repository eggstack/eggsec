use thiserror::Error;

/// Errors specific to the eggsec-runtime crate.
#[derive(Debug, Error)]
pub enum RuntimeError {
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    #[error("invalid session ID")]
    InvalidSessionId,

    #[error("invalid task ID")]
    InvalidTaskId,

    #[error("session not found: {0}")]
    SessionNotFound(String),

    #[error("task not found: {0}")]
    TaskNotFound(String),

    #[error("task already completed: {0}")]
    TaskAlreadyCompleted(String),

    #[error("unsupported task kind for this runtime")]
    UnsupportedTaskKind,

    #[error("dispatch failed: {0}")]
    DispatchFailed(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = RuntimeError::SessionNotFound("s-123".into());
        assert_eq!(format!("{}", err), "session not found: s-123");
    }
}
