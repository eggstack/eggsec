use serde::{Deserialize, Serialize};

use eggsec_runtime::event::RuntimeEvent;
use eggsec_runtime::ids::{SessionId, TaskId};

/// Frontend-neutral event view for streaming displays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventView {
    pub session_id: SessionId,
    pub event_type: String,
    pub task_id: Option<TaskId>,
    pub message: Option<String>,
    pub timestamp_hint: Option<String>,
}

impl From<&RuntimeEvent> for EventView {
    fn from(event: &RuntimeEvent) -> Self {
        match event {
            RuntimeEvent::SessionCreated { session_id } => Self {
                session_id: *session_id,
                event_type: "session-created".into(),
                task_id: None,
                message: Some(format!("Session {} created", session_id)),
                timestamp_hint: None,
            },
            RuntimeEvent::Snapshot { session_id, .. } => Self {
                session_id: *session_id,
                event_type: "snapshot".into(),
                task_id: None,
                message: None,
                timestamp_hint: None,
            },
            RuntimeEvent::TaskQueued {
                session_id,
                task_id,
                request,
            } => Self {
                session_id: *session_id,
                event_type: "task-queued".into(),
                task_id: Some(*task_id),
                message: Some(format!("{:?}", request.task_kind)),
                timestamp_hint: None,
            },
            RuntimeEvent::TaskStarted {
                session_id,
                task_id,
                ..
            } => Self {
                session_id: *session_id,
                event_type: "task-started".into(),
                task_id: Some(*task_id),
                message: None,
                timestamp_hint: None,
            },
            RuntimeEvent::TaskProgress {
                session_id,
                task_id,
                progress,
                ..
            } => Self {
                session_id: *session_id,
                event_type: "task-progress".into(),
                task_id: Some(*task_id),
                message: progress.message.clone(),
                timestamp_hint: None,
            },
            RuntimeEvent::TaskLog {
                session_id,
                task_id,
                level,
                message,
            } => Self {
                session_id: *session_id,
                event_type: "task-log".into(),
                task_id: *task_id,
                message: Some(format!("[{:?}] {}", level, message)),
                timestamp_hint: None,
            },
            RuntimeEvent::PolicyDecisionRequired {
                session_id,
                task_id,
                prompt,
            } => Self {
                session_id: *session_id,
                event_type: "policy-decision-required".into(),
                task_id: *task_id,
                message: Some(prompt.message.clone()),
                timestamp_hint: None,
            },
            RuntimeEvent::TaskCompleted {
                session_id,
                task_id,
                ..
            } => Self {
                session_id: *session_id,
                event_type: "task-completed".into(),
                task_id: Some(*task_id),
                message: None,
                timestamp_hint: None,
            },
            RuntimeEvent::TaskFailed {
                session_id,
                task_id,
                error,
                ..
            } => Self {
                session_id: *session_id,
                event_type: "task-failed".into(),
                task_id: Some(*task_id),
                message: Some(error.message.clone()),
                timestamp_hint: None,
            },
            RuntimeEvent::TaskCancelled {
                session_id,
                task_id,
                reason,
                ..
            } => Self {
                session_id: *session_id,
                event_type: "task-cancelled".into(),
                task_id: Some(*task_id),
                message: reason.clone(),
                timestamp_hint: None,
            },
            RuntimeEvent::Audit {
                session_id, event, ..
            } => Self {
                session_id: *session_id,
                event_type: "audit".into(),
                task_id: None,
                message: Some(event.event_type.clone()),
                timestamp_hint: None,
            },
        }
    }
}
