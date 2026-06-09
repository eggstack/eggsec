use crate::app::confirmation::PendingAction;
use crate::app::notifications::Notification;
use crate::search::GlobalSearch;
use crate::tabs::history::HistoryEntry;
use crate::tabs::Tab;
use std::collections::VecDeque;

/// Overlay UI state (help, HTTP options, search, confirm popup, notifications)
pub struct OverlayState {
    pub show_help: bool,
    pub help_tab: Option<Tab>,
    pub show_http_options: bool,
    pub show_search: bool,
    pub pending_action: Option<PendingAction>,
    pub notification: Option<Notification>,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            show_help: false,
            help_tab: None,
            show_http_options: false,
            show_search: false,
            pending_action: None,
            notification: None,
        }
    }
}

/// Search UI state
pub struct SearchState {
    pub query: String,
    pub is_global: bool,
    pub global_search: Option<GlobalSearch>,
    pub backup: Option<VecDeque<HistoryEntry>>,
}

impl Default for SearchState {
    fn default() -> Self {
        Self {
            query: String::new(),
            is_global: false,
            global_search: Some(GlobalSearch::new()),
            backup: None,
        }
    }
}

/// Quick switch (Ctrl+X tab search) state
pub struct QuickSwitchState {
    pub visible: bool,
    pub query: String,
    pub selected: usize,
}

impl Default for QuickSwitchState {
    fn default() -> Self {
        Self {
            visible: false,
            query: String::new(),
            selected: 0,
        }
    }
}

/// Task runtime state
pub struct TaskState {
    pub handle: Option<tokio::task::JoinHandle<()>>,
    pub inner_abort: Option<tokio::task::AbortHandle>,
    pub tab: Option<Tab>,
    pub progress_rx: Option<tokio::sync::mpsc::Receiver<(u64, u64)>>,
    pub result_rx: Option<tokio::sync::mpsc::Receiver<crate::workers::TaskResult>>,
    pub paused: bool,
}

impl Default for TaskState {
    fn default() -> Self {
        Self {
            handle: None,
            inner_abort: None,
            tab: None,
            progress_rx: None,
            result_rx: None,
            paused: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_state_defaults() {
        let state = OverlayState::default();
        assert!(!state.show_help);
        assert!(state.help_tab.is_none());
        assert!(!state.show_http_options);
        assert!(!state.show_search);
        assert!(state.pending_action.is_none());
        assert!(state.notification.is_none());
    }

    #[test]
    fn search_state_defaults() {
        let state = SearchState::default();
        assert!(state.query.is_empty());
        assert!(!state.is_global);
        assert!(state.global_search.is_some());
        assert!(state.backup.is_none());
    }

    #[test]
    fn quick_switch_state_defaults() {
        let state = QuickSwitchState::default();
        assert!(!state.visible);
        assert!(state.query.is_empty());
        assert_eq!(state.selected, 0);
    }

    #[test]
    fn task_state_defaults() {
        let state = TaskState::default();
        assert!(state.handle.is_none());
        assert!(state.inner_abort.is_none());
        assert!(state.tab.is_none());
        assert!(state.progress_rx.is_none());
        assert!(state.result_rx.is_none());
        assert!(!state.paused);
    }
}
