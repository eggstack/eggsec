use serde::{Deserialize, Serialize};

use crate::ids::{SessionId, TaskId};
use crate::request::RunRequest;

/// Status of a task in the runtime.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Running,
    Completing,
    Completed,
    Failed,
    Cancelled,
    TimedOut,
}

impl TaskStatus {
    pub fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Completed | Self::Failed | Self::Cancelled | Self::TimedOut
        )
    }
}

/// Progress information for a running task.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskProgress {
    pub completed: u64,
    pub total: Option<u64>,
    pub message: Option<String>,
}

/// Log level for runtime log events.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

/// Reference to an artifact produced by a task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactRef {
    pub id: String,
    pub kind: String,
    pub path: Option<String>,
    pub mime_type: Option<String>,
    pub summary: Option<String>,
}

/// Structured result envelope for task completion.
///
/// Carries a typed kind discriminator, optional summary, structured JSON
/// payload, and artifact references. This is the canonical result path
/// for non-TUI frontends (daemon, CLI, REST, MCP) — the TUI may continue
/// to use typed `TaskResult` channels as a rendering optimization.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResultEnvelope {
    pub kind: String,
    pub summary: Option<String>,
    pub payload: serde_json::Value,
    pub artifacts: Vec<ArtifactRef>,
}

/// Outcome of a completed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TaskOutcome {
    Json(serde_json::Value),
    Text(String),
    Artifact {
        artifact_id: String,
        summary: Option<String>,
    },
    /// Structured result envelope with kind discriminator and artifacts.
    ///
    /// This is the preferred outcome for tasks that produce domain-specific
    /// results. Non-TUI frontends consume this directly; the TUI uses typed
    /// `TaskResult` channels as a rendering optimization.
    Result(TaskResultEnvelope),
    Empty,
}

/// Error information from a failed task.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeErrorInfo {
    pub message: String,
    pub code: Option<String>,
    pub details: Option<serde_json::Value>,
}

/// Policy prompt requiring user confirmation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyPrompt {
    pub message: String,
    pub confirmation_class: Option<String>,
    pub requires_explicit_approval: bool,
}

/// Audit event from the runtime.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeAuditEvent {
    pub event_type: String,
    pub surface: String,
    pub outcome: String,
    pub details: Option<serde_json::Value>,
}

/// A runtime event emitted during task lifecycle.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum RuntimeEvent {
    SessionCreated {
        session_id: SessionId,
    },
    Snapshot {
        session_id: SessionId,
        snapshot: crate::session::SessionSnapshot,
    },
    TaskQueued {
        session_id: SessionId,
        task_id: TaskId,
        request: RunRequest,
    },
    TaskStarted {
        session_id: SessionId,
        task_id: TaskId,
    },
    TaskProgress {
        session_id: SessionId,
        task_id: TaskId,
        progress: TaskProgress,
    },
    TaskLog {
        session_id: SessionId,
        task_id: Option<TaskId>,
        level: LogLevel,
        message: String,
    },
    PolicyDecisionRequired {
        session_id: SessionId,
        task_id: Option<TaskId>,
        prompt: PolicyPrompt,
    },
    TaskCompleted {
        session_id: SessionId,
        task_id: TaskId,
        outcome: TaskOutcome,
    },
    TaskFailed {
        session_id: SessionId,
        task_id: TaskId,
        error: RuntimeErrorInfo,
    },
    TaskCancelled {
        session_id: SessionId,
        task_id: TaskId,
        reason: Option<String>,
    },
    SessionClosed {
        session_id: SessionId,
    },
    Audit {
        session_id: SessionId,
        event: RuntimeAuditEvent,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::{LoadTestParams, PortScanParams, RuntimeSurface, TaskKind};

    #[test]
    fn task_status_is_terminal() {
        assert!(TaskStatus::Completed.is_terminal());
        assert!(TaskStatus::Failed.is_terminal());
        assert!(TaskStatus::Cancelled.is_terminal());
        assert!(TaskStatus::TimedOut.is_terminal());
        assert!(!TaskStatus::Queued.is_terminal());
        assert!(!TaskStatus::Running.is_terminal());
        assert!(!TaskStatus::Completing.is_terminal());
    }

    #[test]
    fn runtime_event_roundtrip() {
        let event = RuntimeEvent::TaskQueued {
            session_id: SessionId::new(),
            task_id: TaskId::new(),
            request: RunRequest {
                task_kind: TaskKind::LoadTest(LoadTestParams {
                    target: "http://example.com".into(),
                    method: "GET".into(),
                    connections: Some(10),
                    duration_secs: Some(30),
                    rate_limit: None,
                }),
                requested_by: None,
                surface: RuntimeSurface::TuiManual,
                labels: vec![],
            },
        };
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: RuntimeEvent = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, RuntimeEvent::TaskQueued { .. }));
    }

    #[test]
    fn task_outcome_roundtrip() {
        let outcome = TaskOutcome::Json(serde_json::json!({"findings": 5}));
        let json = serde_json::to_string(&outcome).unwrap();
        let deserialized: TaskOutcome = serde_json::from_str(&json).unwrap();
        assert!(matches!(deserialized, TaskOutcome::Json(_)));
    }

    #[test]
    fn runtime_error_info_roundtrip() {
        let err = RuntimeErrorInfo {
            message: "connection refused".into(),
            code: Some("ECONNREFUSED".into()),
            details: None,
        };
        let json = serde_json::to_string(&err).unwrap();
        let deserialized: RuntimeErrorInfo = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.message, "connection refused");
    }
}
