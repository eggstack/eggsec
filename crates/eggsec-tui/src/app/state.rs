use crate::app::confirmation::{PendingAction, PendingPolicyConfirmation};
use crate::app::notifications::Notification;
use crate::search::GlobalSearch;
use crate::tabs::history::HistoryEntry;
use crate::tabs::Tab;
use crate::theme::install::ThemeInstallReport;
use std::collections::VecDeque;
use std::sync::mpsc::Receiver;
use std::thread::JoinHandle;

/// Overlay UI state (help, HTTP options, search, confirm popup, policy confirmation, notifications)
pub struct OverlayState {
    pub show_help: bool,
    pub help_tab: Option<Tab>,
    pub show_http_options: bool,
    pub show_search: bool,
    pub pending_action: Option<PendingAction>,
    /// Policy enforcement confirmation (RequireConfirmation from EnforcementContext).
    /// Highest precedence overlay; mirrors CLI ManualOverride discretion.
    pub pending_policy: Option<PendingPolicyConfirmation>,
    pub notification: Option<Notification>,
    pub help_scroll_offset: usize,
}

impl Default for OverlayState {
    fn default() -> Self {
        Self {
            show_help: false,
            help_tab: None,
            show_http_options: false,
            show_search: false,
            pending_action: None,
            pending_policy: None,
            notification: None,
            help_scroll_offset: 0,
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
    pub started_at: Option<std::time::Instant>,
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
            started_at: None,
        }
    }
}

/// Runtime state for best-effort packaged/user theme loading.
/// The TUI must remain usable with built-in themes even if this loader fails.
pub struct ThemeLoadState {
    pub rx: Option<Receiver<ThemeInstallReport>>,
    pub handle: Option<JoinHandle<()>>,
    pub deferred_theme_name: Option<String>,
    pub changed_by_user: bool,
}

impl Default for ThemeLoadState {
    fn default() -> Self {
        Self {
            rx: None,
            handle: None,
            deferred_theme_name: None,
            changed_by_user: false,
        }
    }
}

impl ThemeLoadState {
    pub fn is_running(&self) -> bool {
        self.rx.is_some() || self.handle.is_some()
    }

    pub fn mark_user_changed(&mut self) {
        self.changed_by_user = true;
        self.deferred_theme_name = None;
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
        assert!(state.pending_policy.is_none());
        assert!(state.notification.is_none());
    }

    #[test]
    fn overlay_state_pending_policy_default_none() {
        let state = OverlayState::default();
        assert!(state.pending_policy.is_none());
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
        assert!(state.started_at.is_none());
    }

    #[test]
    fn theme_load_state_defaults() {
        let state = ThemeLoadState::default();
        assert!(state.rx.is_none());
        assert!(state.handle.is_none());
        assert!(state.deferred_theme_name.is_none());
        assert!(!state.changed_by_user);
    }

    #[test]
    fn theme_load_state_is_running_checks_receiver_and_handle() {
        let state = ThemeLoadState::default();
        assert!(!state.is_running());

        let (_tx, rx) = std::sync::mpsc::channel::<ThemeInstallReport>();
        let state = ThemeLoadState {
            rx: Some(rx),
            handle: None,
            deferred_theme_name: None,
            changed_by_user: false,
        };
        assert!(state.is_running());

        let handle = std::thread::spawn(|| {});
        let mut state = ThemeLoadState {
            rx: None,
            handle: Some(handle),
            deferred_theme_name: None,
            changed_by_user: false,
        };
        assert!(state.is_running());
        state.handle.take().unwrap().join().unwrap();
    }

    #[test]
    fn theme_load_state_mark_user_changed_clears_deferred_theme() {
        let mut state = ThemeLoadState::default();
        state.deferred_theme_name = Some("catppuccin-mocha".to_string());

        state.mark_user_changed();

        assert!(state.deferred_theme_name.is_none());
        assert!(state.changed_by_user);
    }
}
