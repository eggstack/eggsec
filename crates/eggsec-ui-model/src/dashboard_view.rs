use serde::{Deserialize, Serialize};

use eggsec_runtime::session::SessionSummary;

/// Frontend-neutral dashboard summary view.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummaryView {
    pub total_sessions: usize,
    pub active_sessions: usize,
    pub total_active_tasks: usize,
    pub total_completed_tasks: usize,
    pub sessions: Vec<super::session_view::SessionSummaryView>,
}

impl DashboardSummaryView {
    pub fn from_summaries(summaries: &[SessionSummary]) -> Self {
        let sessions: Vec<_> = summaries
            .iter()
            .map(super::session_view::SessionSummaryView::from)
            .collect();
        let active_sessions = sessions.iter().filter(|s| s.active_count > 0).count();
        let total_active_tasks: usize = sessions.iter().map(|s| s.active_count).sum();
        let total_completed_tasks: usize = sessions.iter().map(|s| s.completed_count).sum();
        Self {
            total_sessions: summaries.len(),
            active_sessions,
            total_active_tasks,
            total_completed_tasks,
            sessions,
        }
    }
}
