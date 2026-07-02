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
use crate::workers::TaskResult;

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
        _sink: RuntimeEventSink,
        _cancel: CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static,
        >,
    > {
        let ctx = self.context.load();
        let progress_tx = ctx.progress_tx.clone();
        let result_tx = ctx.result_tx.clone();

        Box::pin(async move {
            let dispatcher = TuiTaskDispatcher::new(progress_tx, result_tx);
            dispatcher.dispatch(request).await
        })
    }
}

impl super::App {
    pub fn has_active_task(&self) -> bool {
        self.task_state.task_id.is_some()
            || self.task_state.tab.is_some()
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
        } else if self.task_state.task_id.is_some() {
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
        if let (Some(session_id), Some(task_id)) =
            (self.runtime_session_id, self.task_state.task_id)
        {
            let runtime = self.runtime.clone();
            let sid = session_id;
            let tid = task_id;
            tokio::spawn(async move {
                if let Err(e) = runtime.cancel(sid, tid).await {
                    tracing::debug!("Runtime cancel failed (may already be completed): {}", e);
                }
            });
        }

        self.task_state.task_id = None;
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

            // Update the executor context with new channel senders.
            // The executor reads this via ArcSwap before dispatching.
            let ctx = TuiDispatcherContext {
                progress_tx,
                result_tx,
            };
            self.executor_context.store(Arc::new(ctx));

            // Submit to runtime for lifecycle tracking + execution.
            let pending_task_id = Arc::new(std::sync::Mutex::new(None));
            self.runtime_pending_task_id = Some(pending_task_id.clone());

            let runtime = self.runtime.clone();
            let session_id = self.runtime_session_id;
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
                        .create_session(eggsec_runtime::SessionOptions::default())
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
                        *pending_task_id.lock().unwrap() = Some(task_id);
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
}
