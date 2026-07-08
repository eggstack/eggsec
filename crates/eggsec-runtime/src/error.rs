use thiserror::Error;

use crate::request::RuntimeSurface;

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

    #[error("session closed: {0}")]
    SessionClosed(String),

    #[error("surface mismatch: session surface {session}, request surface {request}")]
    SurfaceMismatch {
        session: RuntimeSurface,
        request: RuntimeSurface,
    },

    #[error("enforcement denied: {0}")]
    EnforcementDenied(String),

    #[error("scope unavailable: {0}")]
    ScopeUnavailable(String),

    #[error("cancelled")]
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_display() {
        let err = RuntimeError::SessionNotFound("s-123".into());
        assert_eq!(format!("{}", err), "session not found: s-123");
    }

    #[test]
    fn error_display_session_closed() {
        let err = RuntimeError::SessionClosed("s-456".into());
        assert_eq!(format!("{}", err), "session closed: s-456");
    }

    #[test]
    fn error_display_surface_mismatch() {
        let err = RuntimeError::SurfaceMismatch {
            session: RuntimeSurface::TuiManual,
            request: RuntimeSurface::RestApi,
        };
        let msg = format!("{}", err);
        assert!(msg.contains("tui-manual"));
        assert!(msg.contains("rest-api"));
    }

    #[test]
    fn error_display_enforcement_denied() {
        let err = RuntimeError::EnforcementDenied("not allowed".into());
        assert_eq!(format!("{}", err), "enforcement denied: not allowed");
    }

    #[test]
    fn error_display_scope_unavailable() {
        let err = RuntimeError::ScopeUnavailable("no scope".into());
        assert_eq!(format!("{}", err), "scope unavailable: no scope");
    }

    #[test]
    fn error_display_cancelled() {
        let err = RuntimeError::Cancelled;
        assert_eq!(format!("{}", err), "cancelled");
    }
}
