//! Runtime event reducer for TUI.
//!
//! Maps `RuntimeEvent` values from `eggsec-runtime` to TUI tab state mutations.
//! This is the Phase 4 canonical lifecycle path — runtime events drive
//! progress, completion, failure, and cancellation, while typed `TaskResult`
//! values continue through the legacy `result_rx` channel as a compatibility
//! bridge.
//!
//! Task-to-tab mapping is explicit and local to the TUI. Each task is
//! registered with its originating tab via `register_task()`.
//!
//! ## Architecture
//!
//! The adapter uses a two-phase reduce/apply pattern to work within Rust's
//! borrow rules:
//!
//! 1. **Reduce**: `reduce(event)` reads the event, updates internal mapping
//!    (register/unregister), and returns `Vec<TuiAction>` — instructions
//!    for the caller to apply to tab state.
//! 2. **Apply**: `apply_actions(actions, app)` executes the actions against
//!    `&mut App`, updating tab state.
//!
//! This separation is necessary because the adapter lives inside `App`,
//! so we cannot pass `&mut App` into the adapter's reduce methods.

use rustc_hash::FxHashMap;

use crate::tabs::Tab;
use eggsec_runtime::{RuntimeEvent, TaskId};

/// Actions produced by the reducer for the caller to apply to App state.
#[derive(Debug, Clone)]
pub(crate) enum TuiAction {
    /// Tab progress update: (tab, completed, total).
    UpdateProgress(Tab, u64, u64),
    /// Tab failed with error message: (tab, message).
    TabError(Tab, String),
    /// Tab cancelled: (tab, reason).
    TabCancelled(Tab, Option<String>),
    /// Tab completed: (tab, outcome).
    TabCompleted(Tab, eggsec_runtime::TaskOutcome),
    /// Tab started: (tab, task_id).
    TabStarted(Tab, TaskId),
}

/// Reducer that maps runtime lifecycle events to TUI actions.
///
/// Maintains a `TaskId → Tab` mapping so events for any task can be routed
/// to the correct tab even when the user has navigated away.
pub(crate) struct TuiRuntimeAdapter {
    /// Runtime task ID → originating tab.
    task_to_tab: FxHashMap<TaskId, Tab>,
}

impl TuiRuntimeAdapter {
    pub fn new() -> Self {
        Self {
            task_to_tab: FxHashMap::default(),
        }
    }

    /// Register a task with the tab that initiated it.
    pub fn register_task(&mut self, task_id: TaskId, tab: Tab) {
        self.task_to_tab.insert(task_id, tab);
    }

    /// Unregister a task (called on terminal events).
    fn unregister_task(&mut self, task_id: &TaskId) -> Option<Tab> {
        self.task_to_tab.remove(task_id)
    }

    /// Look up the tab that owns a given task.
    fn tab_for_task(&self, task_id: &TaskId) -> Option<Tab> {
        self.task_to_tab.get(task_id).copied()
    }

    /// Reduce a runtime event into zero or more TuiActions.
    ///
    /// `current_task_tab` is the TUI's knowledge of which tab initiated the
    /// current task (from `task_state.tab`). On `TaskStarted`, if no mapping
    /// exists yet, the adapter auto-registers using this tab.
    ///
    /// This method only borrows `&mut self` (the adapter), not `App`,
    /// so it avoids borrow-checker conflicts.
    pub fn reduce(&mut self, event: RuntimeEvent, current_task_tab: Option<Tab>) -> Vec<TuiAction> {
        match event {
            RuntimeEvent::TaskStarted { task_id, .. } => {
                // Auto-register on first TaskStarted if no mapping exists.
                if self.tab_for_task(&task_id).is_none() {
                    if let Some(tab) = current_task_tab {
                        self.register_task(task_id, tab);
                    }
                }
                if let Some(tab) = self.tab_for_task(&task_id) {
                    tracing::debug!(
                        task_id = %task_id,
                        tab = %tab.title(),
                        "Runtime task started"
                    );
                    vec![TuiAction::TabStarted(tab, task_id)]
                } else {
                    tracing::debug!(
                        task_id = %task_id,
                        "Runtime task started (no tab mapping)"
                    );
                    vec![]
                }
            }
            RuntimeEvent::TaskProgress {
                task_id, progress, ..
            } => {
                if let Some(tab) = self.tab_for_task(&task_id) {
                    let completed = progress.completed;
                    let total = progress.total.unwrap_or(0);
                    if total > 0 {
                        return vec![TuiAction::UpdateProgress(tab, completed, total)];
                    }
                }
                vec![]
            }
            RuntimeEvent::TaskCompleted {
                task_id, outcome, ..
            } => {
                let tab = self.unregister_task(&task_id);
                if let Some(tab) = tab {
                    tracing::debug!(
                        task_id = %task_id,
                        tab = %tab.title(),
                        "Runtime task completed"
                    );
                    vec![TuiAction::TabCompleted(tab, outcome)]
                } else {
                    tracing::debug!(
                        task_id = %task_id,
                        "Runtime task completed (no tab mapping)"
                    );
                    vec![]
                }
            }
            RuntimeEvent::TaskFailed { task_id, error, .. } => {
                let tab = self.unregister_task(&task_id);
                if let Some(tab) = tab {
                    tracing::warn!(
                        task_id = %task_id,
                        tab = %tab.title(),
                        message = %error.message,
                        "Runtime task failed"
                    );
                    vec![TuiAction::TabError(tab, error.message)]
                } else {
                    tracing::warn!(
                        task_id = %task_id,
                        message = %error.message,
                        "Runtime task failed (no tab mapping)"
                    );
                    vec![]
                }
            }
            RuntimeEvent::TaskCancelled {
                task_id, reason, ..
            } => {
                let tab = self.unregister_task(&task_id);
                if let Some(tab) = tab {
                    tracing::debug!(
                        task_id = %task_id,
                        tab = %tab.title(),
                        reason = ?reason,
                        "Runtime task cancelled"
                    );
                    vec![TuiAction::TabCancelled(tab, reason)]
                } else {
                    tracing::debug!(
                        task_id = %task_id,
                        reason = ?reason,
                        "Runtime task cancelled (no tab mapping)"
                    );
                    vec![]
                }
            }
            RuntimeEvent::TaskLog {
                task_id, message, ..
            } => {
                if let Some(tid) = task_id {
                    if let Some(tab) = self.tab_for_task(&tid) {
                        tracing::debug!(
                            task_id = %tid,
                            tab = %tab.title(),
                            message = %message,
                            "Task log"
                        );
                    } else {
                        tracing::debug!(
                            task_id = %tid,
                            message = %message,
                            "Task log (no tab)"
                        );
                    }
                } else {
                    tracing::debug!(message = %message, "Task log (no task)");
                }
                vec![]
            }
            RuntimeEvent::TaskQueued { .. }
            | RuntimeEvent::SessionCreated { .. }
            | RuntimeEvent::Snapshot { .. }
            | RuntimeEvent::PolicyDecisionRequired { .. }
            | RuntimeEvent::Audit { .. } => vec![],
        }
    }

    /// Apply a batch of actions to the App.
    ///
    /// This is a free function to avoid borrow conflicts — the adapter
    /// is a field of App, so `&self` + `&mut App` cannot coexist in a method.
    pub fn apply_actions(actions: Vec<TuiAction>, app: &mut super::App) -> bool {
        let mut dirty = false;
        for action in actions {
            match action {
                TuiAction::UpdateProgress(tab, completed, total) => {
                    tab.update_progress_in_app(app, completed, total);
                    dirty = true;
                }
                TuiAction::TabError(mut tab, message) => {
                    tab.as_tab_state_mut(app)
                        .set_error(crate::app::tab_error::TabError::Target(message));
                    dirty = true;
                }
                TuiAction::TabCancelled(mut tab, _reason) => {
                    tab.as_tab_state_mut(app).reset();
                    dirty = true;
                }
                TuiAction::TabCompleted(_tab, _outcome) => {
                    // Typed TaskResult continues through result_rx as compatibility bridge.
                    dirty = true;
                }
                TuiAction::TabStarted(_tab, _task_id) => {
                    dirty = true;
                }
            }
        }
        dirty
    }

    /// Drain all events from the receiver and reduce them to actions.
    ///
    /// This method only borrows the adapter and the receiver (not App),
    /// so it can be called without borrow conflicts.
    pub fn drain_and_reduce(
        &mut self,
        rx: &mut Option<eggsec_runtime::RuntimeEventReceiver>,
        current_task_tab: Option<Tab>,
    ) -> Vec<TuiAction> {
        let mut all_actions = Vec::new();

        // Take the receiver temporarily to avoid borrow conflicts.
        let mut rx_owned = rx.take();
        if let Some(ref mut rx_inner) = rx_owned {
            while let Some(event) = rx_inner.try_recv() {
                let actions = self.reduce(event, current_task_tab);
                all_actions.extend(actions);
            }
        }
        *rx = rx_owned;

        all_actions
    }

    /// Clear all task mappings (used on app reset).
    #[allow(dead_code)]
    pub fn clear(&mut self) {
        self.task_to_tab.clear();
    }

    /// Returns true if any tasks are registered.
    #[allow(dead_code)]
    pub fn has_tasks(&self) -> bool {
        !self.task_to_tab.is_empty()
    }

    /// Number of registered tasks.
    #[allow(dead_code)]
    pub fn task_count(&self) -> usize {
        self.task_to_tab.len()
    }
}

/// Trait for routing progress updates to specific tabs.
pub(super) trait TabProgressRouter {
    fn update_progress_in_app(&self, app: &mut super::App, completed: u64, total: u64);
}

impl TabProgressRouter for Tab {
    fn update_progress_in_app(&self, app: &mut super::App, completed: u64, total: u64) {
        match self {
            Tab::Recon => app.tabs.recon.update_progress(completed, total),
            Tab::Load => app.tabs.load.update_progress(completed, total),
            Tab::ScanPorts => app.tabs.scan_ports.update_progress(completed, total),
            Tab::ScanEndpoints => app.tabs.scan_endpoints.update_progress(completed, total),
            Tab::Fingerprint => app.tabs.fingerprint.update_progress(completed, total),
            Tab::Fuzz => app.tabs.fuzz.core.update_progress(completed, total),
            Tab::Waf => app.tabs.waf.update_progress(completed, total),
            Tab::WafStress => app.tabs.waf_stress.update_progress(completed, total),
            Tab::Scan => {
                let total_stages = app.tabs.scan.stages.len() as u64;
                if total_stages == 0 {
                    return;
                }
                let completed_stages = app
                    .tabs
                    .scan
                    .stages
                    .iter()
                    .filter(|s| matches!(s.status, crate::tabs::StageStatus::Completed))
                    .count() as u64;
                app.tabs
                    .scan
                    .update_progress(completed_stages, total_stages);
            }
            #[cfg(feature = "wireless")]
            Tab::Wireless => {
                app.tabs.wireless.update_progress(completed, total);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::create_test_app;

    #[test]
    fn adapter_new_is_empty() {
        let adapter = TuiRuntimeAdapter::new();
        assert!(!adapter.has_tasks());
        assert_eq!(adapter.task_count(), 0);
    }

    #[test]
    fn register_and_lookup_task() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        assert!(adapter.has_tasks());
        assert_eq!(adapter.task_count(), 1);
        assert_eq!(adapter.tab_for_task(&task_id), Some(Tab::Recon));
    }

    #[test]
    fn unregister_task_removes_mapping() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);
        adapter.unregister_task(&task_id);

        assert!(!adapter.has_tasks());
        assert_eq!(adapter.tab_for_task(&task_id), None);
    }

    #[test]
    fn reduce_task_started_with_mapping() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskStarted {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
        };

        let actions = adapter.reduce(event, None);
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], TuiAction::TabStarted(Tab::Recon, _)));
    }

    #[test]
    fn reduce_task_started_without_mapping_auto_registers() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();

        let event = RuntimeEvent::TaskStarted {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
        };

        // Without fallback tab: no auto-registration.
        let actions = adapter.reduce(event.clone(), None);
        assert!(actions.is_empty());

        // With fallback tab: auto-registers and returns TabStarted.
        let actions = adapter.reduce(event, Some(Tab::Recon));
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], TuiAction::TabStarted(Tab::Recon, _)));
    }

    #[test]
    fn reduce_task_failed_unregisters_and_returns_error() {
        let mut adapter = TuiRuntimeAdapter::new();
        let mut app = create_test_app();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskFailed {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            error: eggsec_runtime::RuntimeErrorInfo {
                message: "Connection refused".to_string(),
                code: Some("ECONNREFUSED".to_string()),
                details: None,
            },
        };

        let actions = adapter.reduce(event, None);
        assert_eq!(actions.len(), 1);
        assert!(
            matches!(&actions[0], TuiAction::TabError(Tab::Recon, msg) if msg == "Connection refused")
        );
        assert!(!adapter.has_tasks());
    }

    #[test]
    fn reduce_task_cancelled_unregisters_and_returns_cancel() {
        let mut adapter = TuiRuntimeAdapter::new();
        let mut app = create_test_app();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskCancelled {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            reason: Some("User cancelled".to_string()),
        };

        let actions = adapter.reduce(event, None);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            TuiAction::TabCancelled(Tab::Recon, Some(_))
        ));
        assert!(!adapter.has_tasks());
    }

    #[test]
    fn reduce_task_completed_unregisters() {
        let mut adapter = TuiRuntimeAdapter::new();
        let mut app = create_test_app();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskCompleted {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            outcome: eggsec_runtime::TaskOutcome::Empty,
        };

        let actions = adapter.reduce(event, None);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            TuiAction::TabCompleted(Tab::Recon, _)
        ));
        assert!(!adapter.has_tasks());
    }

    #[test]
    fn reduce_task_progress_returns_update() {
        let mut adapter = TuiRuntimeAdapter::new();
        let mut app = create_test_app();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskProgress {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            progress: eggsec_runtime::TaskProgress {
                completed: 50,
                total: Some(100),
                message: None,
            },
        };

        let actions = adapter.reduce(event, None);
        assert_eq!(actions.len(), 1);
        assert!(matches!(
            &actions[0],
            TuiAction::UpdateProgress(Tab::Recon, 50, 100)
        ));
    }

    #[test]
    fn reduce_task_log_returns_nothing() {
        let mut adapter = TuiRuntimeAdapter::new();
        let mut app = create_test_app();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskLog {
            session_id: eggsec_runtime::SessionId::new(),
            task_id: Some(task_id),
            level: eggsec_runtime::LogLevel::Info,
            message: "Processing...".to_string(),
        };

        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
    }

    #[test]
    fn reduce_session_created_returns_nothing() {
        let mut adapter = TuiRuntimeAdapter::new();
        let mut app = create_test_app();

        let event = RuntimeEvent::SessionCreated {
            session_id: eggsec_runtime::SessionId::new(),
        };

        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
    }

    #[test]
    fn clear_removes_all_mappings() {
        let mut adapter = TuiRuntimeAdapter::new();
        adapter.register_task(TaskId::new(), Tab::Recon);
        adapter.register_task(TaskId::new(), Tab::Load);

        assert_eq!(adapter.task_count(), 2);
        adapter.clear();
        assert!(!adapter.has_tasks());
    }

    #[test]
    fn multiple_tasks_different_tabs() {
        let mut adapter = TuiRuntimeAdapter::new();
        let tid1 = TaskId::new();
        let tid2 = TaskId::new();
        adapter.register_task(tid1, Tab::Recon);
        adapter.register_task(tid2, Tab::Load);

        assert_eq!(adapter.tab_for_task(&tid1), Some(Tab::Recon));
        assert_eq!(adapter.tab_for_task(&tid2), Some(Tab::Load));
        assert_eq!(adapter.task_count(), 2);

        adapter.unregister_task(&tid1);
        assert_eq!(adapter.task_count(), 1);
        assert_eq!(adapter.tab_for_task(&tid2), Some(Tab::Load));
    }

    #[test]
    fn reduce_unknown_task_id_returns_empty() {
        let mut adapter = TuiRuntimeAdapter::new();
        // Event for a task that was never registered.
        let unknown_id = TaskId::new();

        let event = RuntimeEvent::TaskProgress {
            session_id: eggsec_runtime::SessionId::new(),
            task_id: unknown_id,
            progress: eggsec_runtime::TaskProgress {
                completed: 10,
                total: Some(100),
                message: None,
            },
        };
        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
        assert!(!adapter.has_tasks());
    }

    #[test]
    fn reduce_duplicate_terminal_event_is_idempotent() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskFailed {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            error: eggsec_runtime::RuntimeErrorInfo {
                message: "fail".into(),
                code: None,
                details: None,
            },
        };

        // First terminal event unregisters and returns action.
        let actions = adapter.reduce(event.clone(), None);
        assert_eq!(actions.len(), 1);
        assert!(!adapter.has_tasks());

        // Second terminal event for same task: no mapping, no action.
        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
    }

    #[test]
    fn reduce_policy_decision_required_does_not_panic() {
        let mut adapter = TuiRuntimeAdapter::new();
        let event = RuntimeEvent::PolicyDecisionRequired {
            session_id: eggsec_runtime::SessionId::new(),
            task_id: None,
            prompt: eggsec_runtime::PolicyPrompt {
                message: "Confirm target?".into(),
                confirmation_class: Some("scope-expansion".into()),
                requires_explicit_approval: true,
            },
        };
        // PolicyDecisionRequired is currently unhandled (returns []).
        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
    }

    #[test]
    fn reduce_progress_with_no_total_returns_empty() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Load);

        let event = RuntimeEvent::TaskProgress {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            progress: eggsec_runtime::TaskProgress {
                completed: 50,
                total: None,
                message: Some("indeterminate".into()),
            },
        };
        // Progress with no total: not enough info for a progress bar.
        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
    }

    #[test]
    fn reduce_progress_with_zero_total_returns_empty() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Load);

        let event = RuntimeEvent::TaskProgress {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            progress: eggsec_runtime::TaskProgress {
                completed: 0,
                total: Some(0),
                message: None,
            },
        };
        // Zero total: division guard, no progress update.
        let actions = adapter.reduce(event, None);
        assert!(actions.is_empty());
    }

    #[test]
    fn reduce_task_cancelled_unregisters_exact_once() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::ScanPorts);

        let cancel_event = RuntimeEvent::TaskCancelled {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            reason: Some("user".into()),
        };

        let actions1 = adapter.reduce(cancel_event.clone(), None);
        assert_eq!(actions1.len(), 1);
        assert!(!adapter.has_tasks());

        // Second cancel: no mapping.
        let actions2 = adapter.reduce(cancel_event, None);
        assert!(actions2.is_empty());
    }

    #[test]
    fn reduce_task_started_auto_registers_with_current_tab() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();

        // First: no mapping, no fallback → empty.
        let event = RuntimeEvent::TaskStarted {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
        };
        let actions = adapter.reduce(event.clone(), None);
        assert!(actions.is_empty());

        // Second: with fallback tab → auto-registers.
        let actions = adapter.reduce(event, Some(Tab::Fuzz));
        assert_eq!(actions.len(), 1);
        assert!(matches!(actions[0], TuiAction::TabStarted(Tab::Fuzz, _)));
        assert!(adapter.has_tasks());
    }

    #[test]
    fn reduce_event_after_tab_change_routes_to_original_tab() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        // Task was registered from Recon, but user navigated to Load.
        adapter.register_task(task_id, Tab::Recon);

        let event = RuntimeEvent::TaskCompleted {
            session_id: eggsec_runtime::SessionId::new(),
            task_id,
            outcome: eggsec_runtime::TaskOutcome::Empty,
        };
        // current_task_tab is Load (user navigated), but event routes to Recon.
        let actions = adapter.reduce(event, Some(Tab::Load));
        assert_eq!(actions.len(), 1);
        // The action targets the ORIGINAL tab (Recon), not the current tab.
        assert!(matches!(
            &actions[0],
            TuiAction::TabCompleted(Tab::Recon, _)
        ));
    }

    #[test]
    fn drain_and_reduce_collects_multiple_events() {
        let mut adapter = TuiRuntimeAdapter::new();
        let task_id = TaskId::new();
        adapter.register_task(task_id, Tab::Recon);

        let (tx, rx) = tokio::sync::broadcast::channel::<eggsec_runtime::RuntimeEvent>(10);
        let receiver = eggsec_runtime::RuntimeEventReceiver::from_broadcast(rx);
        let mut rx = Some(receiver);

        // Send events before draining.
        let sid = eggsec_runtime::SessionId::new();
        let _ = tx.send(RuntimeEvent::TaskStarted {
            session_id: sid,
            task_id,
        });
        let _ = tx.send(RuntimeEvent::TaskProgress {
            session_id: sid,
            task_id,
            progress: eggsec_runtime::TaskProgress {
                completed: 25,
                total: Some(100),
                message: None,
            },
        });
        drop(tx);

        let actions = adapter.drain_and_reduce(&mut rx, None);
        // Should get TabStarted + UpdateProgress.
        assert_eq!(actions.len(), 2);
        assert!(matches!(actions[0], TuiAction::TabStarted(_, _)));
        assert!(matches!(actions[1], TuiAction::UpdateProgress(_, 25, 100)));
    }
}
