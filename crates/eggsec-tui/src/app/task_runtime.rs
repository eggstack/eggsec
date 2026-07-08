use std::sync::Arc;

use arc_swap::ArcSwap;
use eggsec::config::ApprovedOperation;
use eggsec_runtime::dispatcher::TaskDispatcher;
use eggsec_runtime::event::TaskOutcome;
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::{RuntimeError, RuntimeEventSink, RuntimeTaskExecutor, TaskId};
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use crate::app::task_dispatcher::TuiTaskDispatcher;
use eggsec::dispatch::TaskResult;

/// Per-task context for the TUI executor.
///
/// Holds the channel senders for a single task submission. The executor
/// loads this via `ArcSwap` before dispatching, ensuring it uses the
/// channels for the current task.
pub(crate) struct TuiDispatcherContext {
    pub progress_tx: mpsc::Sender<(u64, u64)>,
    pub result_tx: mpsc::Sender<TaskResult>,
}

/// Real executor for `eggsec_runtime::Runtime`.
///
/// Replaces the Phase 2 `TuiStubExecutor`. Uses a `TuiTaskDispatcher`
/// to map `RunRequest` to engine calls, sending typed `TaskResult`
/// through channels for TUI consumption.
pub(crate) struct TuiExecutor {
    context: Arc<ArcSwap<TuiDispatcherContext>>,
}

impl TuiExecutor {
    pub fn new(context: Arc<ArcSwap<TuiDispatcherContext>>) -> Self {
        Self { context }
    }
}

impl RuntimeTaskExecutor for TuiExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        request: RunRequest,
        _context: eggsec_runtime::RuntimeExecutionContext,
        _sink: RuntimeEventSink,
        _cancel: CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        let context = self.context.clone();

        Box::pin(async move {
            let dispatcher = TuiTaskDispatcher::new(context);
            dispatcher.dispatch(request).await
        })
    }
}

impl super::App {
    /// Check if there is an active (non-completed) task.
    ///
    /// Uses channel liveness and session binding rather than storing a
    /// redundant `TaskId` — the canonical task identity lives in the
    /// runtime session.
    pub fn has_active_task(&self) -> bool {
        self.task_state.tab.is_some()
            || self.task_state.progress_rx.is_some()
            || self.task_state.result_rx.is_some()
    }

    pub fn active_task_tab(&self) -> Option<super::tabs::Tab> {
        self.task_state.tab
    }

    pub fn active_task_elapsed_secs(&self) -> Option<u64> {
        self.task_state.started_at.map(|start| {
            let elapsed = std::time::Instant::now().saturating_duration_since(start);
            elapsed.as_secs()
        })
    }

    pub fn task_status_summary(&self) -> Option<String> {
        if !self.has_active_task() {
            return None;
        }
        let tab_name = self.task_state.tab.map(|t| t.title()).unwrap_or("Task");
        let state = if self.task_state.paused {
            "paused"
        } else if self.task_state.result_rx.is_some() || self.task_state.progress_rx.is_some() {
            "running"
        } else {
            "stopping"
        };
        let elapsed = self
            .active_task_elapsed_secs()
            .map(|s| format!(" {s}s"))
            .unwrap_or_default();
        let hints = if self.task_state.paused {
            " [Ctrl-Y resume]"
        } else {
            " [Ctrl-C stop] [Ctrl-Z pause]"
        };
        Some(format!("Task: {tab_name} ({state}{elapsed}){hints}"))
    }

    fn stop_tab_state(&mut self, tab: super::tabs::Tab) {
        let mut tab = tab;
        tab.as_tab_input(self).stop();
    }

    /// Cancel the active task via the runtime and clear TUI state.
    fn clear_task_runtime(&mut self) {
        // Cancel via runtime client (daemon mode) or embedded runtime.
        if let Some(session_id) = self.runtime_binding.session_id {
            if let Some(ref client) = self.runtime_client {
                let client = client.clone();
                let sid = session_id;
                tokio::spawn(async move {
                    if let Err(e) = client.cancel_active(sid).await {
                        tracing::debug!(
                            "Daemon cancel_active failed (may already be completed): {}",
                            e
                        );
                    }
                });
            } else {
                let runtime = self.runtime_binding.runtime.clone();
                let sid = session_id;
                tokio::spawn(async move {
                    if let Err(e) = runtime.cancel_active(sid).await {
                        tracing::debug!(
                            "Runtime cancel_active failed (may already be completed): {}",
                            e
                        );
                    }
                });
            }
        }

        self.task_state.tab = None;
        if let Some(rx) = self.task_state.progress_rx.take() {
            drop(rx);
        }
        if let Some(rx) = self.task_state.result_rx.take() {
            drop(rx);
        }
        self.task_state.started_at = None;
    }

    pub fn stop(&mut self) {
        let tab = self.task_state.tab.unwrap_or(self.current_tab);
        self.stop_tab_state(tab);
        self.clear_task_runtime();
    }

    pub fn stop_with_message(&mut self, message: &str) {
        let tab = self.task_state.tab.unwrap_or(self.current_tab);
        self.stop_tab_state(tab);
        self.clear_task_runtime();

        // Reuse current tab-targeted error plumbing by temporarily scoping task_tab.
        self.task_state.tab = Some(tab);
        self.set_error_for_current_tab(crate::app::tab_error::TabError::Target(
            message.to_string(),
        ));
        self.task_state.tab = None;
    }

    /// Submit a task to the runtime via the real executor.
    ///
    /// Creates per-task channels, updates the executor context, and
    /// submits a `RunRequest` to the runtime. The runtime's executor
    /// calls `TuiTaskDispatcher::dispatch()` which runs the engine
    /// functions and sends typed `TaskResult` through the channels.
    pub(crate) fn spawn_task(
        &mut self,
        request: Option<RunRequest>,
        _approved: Option<ApprovedOperation>,
    ) {
        if let Some(request) = request {
            if self.has_active_task() {
                tracing::warn!(
                    "A task is already running. Aborting previous task before starting new one."
                );
                self.clear_task_runtime();
            }

            let (progress_tx, progress_rx) = mpsc::channel(100);
            let (result_tx, result_rx) = mpsc::channel(1);

            self.task_state.progress_rx = Some(progress_rx);
            self.task_state.result_rx = Some(result_rx);

            self.task_state.tab = Some(self.current_tab);
            self.task_state.started_at = Some(std::time::Instant::now());

            // Phase 4: register task-tab mapping in the runtime adapter.
            // The adapter routes lifecycle events (progress, completion, failure)
            // to the correct tab regardless of which tab is currently focused.

            // Update the executor context with new channel senders.
            // The executor reads this via ArcSwap before dispatching.
            let ctx = TuiDispatcherContext {
                progress_tx,
                result_tx,
            };
            self.executor_context.store(Arc::new(ctx));

            // Submit to runtime for lifecycle tracking + execution.
            let runtime = self.runtime_binding.runtime.clone();
            let session_id = self.runtime_binding.session_id;
            let session_scope: eggsec_runtime::SessionScope =
                self.enforcement_state.loaded_scope().into();
            let pending_session_id =
                Arc::new(std::sync::Mutex::new(None::<eggsec_runtime::SessionId>));
            let pending_session_id_clone = pending_session_id.clone();
            let pending_event_rx = Arc::new(tokio::sync::Mutex::new(None));
            let pending_event_rx_clone = pending_event_rx.clone();
            self.runtime_pending_event_rx = Some(pending_event_rx);

            tokio::spawn(async move {
                let session_id = match session_id {
                    Some(sid) => sid,
                    None => match runtime
                        .create_session_with_scope(
                            eggsec_runtime::SessionOptions::default(),
                            eggsec_runtime::RuntimeSurface::TuiManual,
                            Some(session_scope),
                        )
                        .await
                    {
                        Ok(sid) => {
                            *pending_session_id_clone.lock().unwrap() = Some(sid);
                            sid
                        }
                        Err(e) => {
                            tracing::error!("Failed to create runtime session: {}", e);
                            return;
                        }
                    },
                };

                // Subscribe to runtime events before task submission.
                let event_rx = runtime.subscribe().await;
                *pending_event_rx_clone.lock().await = Some(event_rx);

                match runtime.submit(session_id, request).await {
                    Ok(task_id) => {
                        tracing::debug!("Task submitted to runtime: {}", task_id);
                    }
                    Err(e) => {
                        tracing::error!("Failed to submit task to runtime: {}", e);
                    }
                }
            });

            // Store session_id holder for sync on next update().
            self.runtime_pending_session_id = Some(pending_session_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::app::create_test_app;
    use crate::tabs::{AppState, Tab};

    #[test]
    fn stop_with_message_targets_task_tab_when_current_tab_differs() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        app.task_state.tab = Some(Tab::Recon);
        app.tabs.recon.core.state = AppState::Running;

        app.stop_with_message("Interrupted by user");

        assert!(
            matches!(app.tabs.recon.core.state, AppState::Error(ref m) if m == "Interrupted by user")
        );
        assert!(app.task_state.tab.is_none());
        assert!(!app.has_active_task());
    }

    #[test]
    fn stop_targets_task_tab_state_when_current_tab_differs() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        app.task_state.tab = Some(Tab::Recon);
        app.tabs.recon.core.state = AppState::Running;

        app.stop();

        assert!(matches!(app.tabs.recon.core.state, AppState::Idle));
        assert!(app.task_state.tab.is_none());
        assert!(!app.has_active_task());
    }

    #[test]
    fn tui_app_binds_to_pre_existing_runtime_session() {
        use crate::app::RuntimeBinding;
        use eggsec_runtime::{Runtime, RuntimeConfig, RuntimeSurface, SessionId};

        // Create a runtime and a session outside of the TUI.
        let runtime = std::sync::Arc::new(Runtime::new(
            RuntimeConfig::default(),
            crate::app::task_runtime::TuiExecutor::new(std::sync::Arc::new(
                arc_swap::ArcSwap::new(std::sync::Arc::new(
                    crate::app::task_runtime::TuiDispatcherContext {
                        progress_tx: tokio::sync::mpsc::channel(1).0,
                        result_tx: tokio::sync::mpsc::channel(1).0,
                    },
                )),
            )),
        ));
        let rt = runtime.clone();
        let session_id = tokio::runtime::Runtime::new().unwrap().block_on(async {
            rt.create_session(
                eggsec_runtime::SessionOptions::default(),
                RuntimeSurface::TuiManual,
            )
            .await
            .unwrap()
        });

        // Bind TUI app to the pre-existing session.
        let mut app = create_test_app();
        app.runtime_binding = RuntimeBinding {
            runtime,
            session_id: Some(session_id),
            events: None,
            daemon_event_handle: None,
        };

        // Verify the TUI can read back the session ID.
        assert_eq!(app.runtime_binding.session_id, Some(session_id));
        assert!(!app.has_active_task());
    }

    #[test]
    fn tui_app_runtime_binding_reflects_session_surface() {
        use crate::app::RuntimeBinding;
        use eggsec_runtime::{Runtime, RuntimeConfig, RuntimeSurface};

        let runtime = std::sync::Arc::new(Runtime::new(
            RuntimeConfig::default(),
            crate::app::task_runtime::TuiExecutor::new(std::sync::Arc::new(
                arc_swap::ArcSwap::new(std::sync::Arc::new(
                    crate::app::task_runtime::TuiDispatcherContext {
                        progress_tx: tokio::sync::mpsc::channel(1).0,
                        result_tx: tokio::sync::mpsc::channel(1).0,
                    },
                )),
            )),
        ));
        let rt = runtime.clone();
        let session_id = tokio::runtime::Runtime::new().unwrap().block_on(async {
            rt.create_session(
                eggsec_runtime::SessionOptions::default(),
                RuntimeSurface::TuiManual,
            )
            .await
            .unwrap()
        });

        let mut app = create_test_app();
        app.runtime_binding = RuntimeBinding {
            runtime,
            session_id: Some(session_id),
            events: None,
            daemon_event_handle: None,
        };

        // The runtime should report the correct surface for the bound session.
        let rt = app.runtime_binding.runtime.clone();
        let sid = app.runtime_binding.session_id.unwrap();
        let surface = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async { rt.session_surface(sid).await.unwrap() });
        assert_eq!(surface, RuntimeSurface::TuiManual);
    }
}
