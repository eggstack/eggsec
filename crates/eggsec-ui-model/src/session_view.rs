use serde::{Deserialize, Serialize};

use eggsec_runtime::ids::SessionId;
use eggsec_runtime::session::{SessionScope, SessionSnapshot, SessionSummary};

/// Frontend-neutral session view for list displays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionSummaryView {
    pub session_id: SessionId,
    pub surface: String,
    pub surface_label: String,
    pub scope_source: Option<String>,
    pub has_explicit_scope: bool,
    pub active_count: usize,
    pub completed_count: usize,
    pub created_at_secs: u64,
}

impl From<&SessionSummary> for SessionSummaryView {
    fn from(s: &SessionSummary) -> Self {
        Self {
            session_id: s.session_id,
            surface: format!("{:?}", s.surface),
            surface_label: s.surface.label().into(),
            scope_source: s.scope.as_ref().map(|sc| sc.source.clone()),
            has_explicit_scope: s.scope.as_ref().map_or(false, |sc| sc.is_explicit),
            active_count: s.active_count,
            completed_count: s.completed_count,
            created_at_secs: s.created_at_secs,
        }
    }
}

/// Frontend-neutral detailed session view for snapshot displays.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionView {
    pub session_id: SessionId,
    pub surface: String,
    pub surface_label: String,
    pub scope: Option<SessionScopeView>,
    pub created_at_secs: u64,
    pub generation: u64,
    pub active_tasks: Vec<super::task_view::TaskView>,
    pub completed_tasks: Vec<super::task_view::TaskView>,
    pub active_count: usize,
    pub completed_count: usize,
    pub capabilities_summary: SessionCapabilitiesSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionScopeView {
    pub is_explicit: bool,
    pub source: String,
    pub path: Option<String>,
}

impl From<&SessionScope> for SessionScopeView {
    fn from(s: &SessionScope) -> Self {
        Self {
            is_explicit: s.is_explicit,
            source: s.source.clone(),
            path: s.path.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SessionCapabilitiesSummary {
    pub task_kind_count: usize,
    pub supports_cancellation: bool,
    pub transports: Vec<String>,
}

impl From<&SessionSnapshot> for SessionView {
    fn from(snapshot: &SessionSnapshot) -> Self {
        let active: Vec<_> = snapshot
            .active_tasks
            .iter()
            .map(super::task_view::TaskView::from)
            .collect();
        let completed: Vec<_> = snapshot
            .completed_tasks
            .iter()
            .map(super::task_view::TaskView::from)
            .collect();
        Self {
            session_id: snapshot.session_id,
            surface: format!("{:?}", snapshot.surface),
            surface_label: snapshot.surface.label().into(),
            scope: snapshot.scope.as_ref().map(SessionScopeView::from),
            created_at_secs: snapshot.created_at_secs,
            generation: snapshot.generation,
            active_count: active.len(),
            completed_count: completed.len(),
            active_tasks: active,
            completed_tasks: completed,
            capabilities_summary: SessionCapabilitiesSummary {
                task_kind_count: snapshot.capabilities.task_kinds.len(),
                supports_cancellation: snapshot.capabilities.supports_cancellation,
                transports: snapshot.capabilities.transports.clone(),
            },
        }
    }
}
