use std::time::Duration;

use crate::workers;
use eggsec::config::ApprovedOperation;

/// Stub executor for `eggsec_runtime::Runtime` initialization.
///
/// The TUI does not use the runtime's executor directly — it uses a local
/// compatibility bridge in `spawn_task` that wraps the existing `TaskRunner`.
/// This stub satisfies the `Runtime::new()` requirement for an executor.
/// Phase 3 will replace this with the real executor that moves into the
/// runtime/engine crate.
pub(crate) struct TuiStubExecutor;

impl eggsec_runtime::RuntimeTaskExecutor for TuiStubExecutor {
    fn execute(
        &self,
        _task_id: eggsec_runtime::TaskId,
        _request: eggsec_runtime::RunRequest,
        _sink: eggsec_runtime::RuntimeEventSink,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<
            dyn std::future::Future<
                    Output = Result<eggsec_runtime::TaskOutcome, eggsec_runtime::RuntimeError>,
                > + Send
                + 'static,
        >,
    > {
        Box::pin(async {
            // The TUI never dispatches through the runtime's executor.
            // Actual task execution goes through the local compatibility bridge.
            Err(eggsec_runtime::RuntimeError::UnsupportedTaskKind)
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
    ///
    /// Phase 2 bridge: calls `runtime.cancel()` to record the lifecycle event,
    /// then drops channel receivers to stop the progress forwarder. The runtime
    /// tracks the canonical cancellation; the TUI cleans up local state.
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

    /// Submit a task to the runtime and spawn a local compatibility executor.
    ///
    /// Phase 2 bridge: the runtime records the task lifecycle (ID, timeout,
    /// cancellation), while the local executor wraps the existing `TaskRunner`
    /// worker path. Channels (`progress_rx`/`result_rx`) remain as the TUI's
    /// consumption path until Phase 4 migrates to `RuntimeEventReceiver`.
    pub(crate) fn spawn_task(
        &mut self,
        config: Option<workers::TaskConfig>,
        _approved: Option<ApprovedOperation>,
    ) {
        if let Some(config) = config {
            if self.has_active_task() {
                tracing::warn!(
                    "A task is already running. Aborting previous task before starting new one."
                );
                self.clear_task_runtime();
            }

            let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(100);
            let (result_tx, result_rx) = tokio::sync::mpsc::channel(1);

            self.task_state.progress_rx = Some(progress_rx);
            self.task_state.result_rx = Some(result_rx);

            self.task_state.tab = Some(self.current_tab);
            self.task_state.started_at = Some(std::time::Instant::now());

            // Submit to runtime for lifecycle tracking (best-effort).
            // Use a shared holder to sync task_id back to TaskState on next update().
            let pending_task_id = std::sync::Arc::new(std::sync::Mutex::new(None));
            self.runtime_pending_task_id = Some(pending_task_id.clone());

            let runtime = self.runtime.clone();
            let session_id = self.runtime_session_id;
            // Shared holder for session_id so spawned task can store it back.
            let pending_session_id =
                std::sync::Arc::new(std::sync::Mutex::new(None::<eggsec_runtime::SessionId>));
            let pending_session_id_clone = pending_session_id.clone();
            // Shared holder for event receiver subscription.
            let pending_event_rx = std::sync::Arc::new(tokio::sync::Mutex::new(None));
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
                            // Store session_id back so App can reuse it for next task.
                            *pending_session_id_clone.lock().unwrap() = Some(sid);
                            sid
                        }
                        Err(e) => {
                            tracing::error!("Failed to create runtime session: {}", e);
                            return;
                        }
                    },
                };

                // Subscribe to runtime events before task submission so we
                // capture TaskQueued and TaskStarted.
                let event_rx = runtime.subscribe().await;
                *pending_event_rx_clone.lock().await = Some(event_rx);

                let request = eggsec_runtime::RunRequest {
                    task_kind: eggsec_runtime::TaskKind::PortScan(
                        eggsec_runtime::request::PortScanParams {
                            target: String::new(),
                            ports: None,
                            scan_type: None,
                            timeout_ms: None,
                        },
                    ),
                    requested_by: None,
                    surface: eggsec_runtime::RuntimeSurface::TuiManual,
                    labels: vec![],
                };

                match runtime.submit(session_id, request).await {
                    Ok(task_id) => {
                        // Store task_id for sync to TaskState on next update().
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

            // Spawn the local compatibility executor with existing worker path
            let runner = workers::TaskRunner::new(config, progress_tx, result_tx.clone());
            let error_tx = result_tx;

            let inner_handle = tokio::spawn(async move { runner.run().await });
            let inner_abort = inner_handle.abort_handle();
            let handle_to_abort = inner_abort.clone();

            let handle = tokio::spawn(async move {
                match tokio::time::timeout(Duration::from_secs(300), inner_handle).await {
                    Ok(Ok(Ok(()))) => {}
                    Ok(Ok(Err(e))) => {
                        let friendly_error = super::make_friendly_error(&e);
                        tracing::error!("Task failed: {}", friendly_error);
                        if let Err(e) = error_tx
                            .send(workers::TaskResult::Error(friendly_error))
                            .await
                        {
                            tracing::warn!("Failed to send task error result: {:?}", e);
                        }
                    }
                    Ok(Err(join_error)) => {
                        if join_error.is_cancelled() {
                            tracing::error!("Task was cancelled");
                            if let Err(e) = error_tx
                                .send(workers::TaskResult::Error("Task was cancelled".to_string()))
                                .await
                            {
                                tracing::warn!("Failed to send task error result: {:?}", e);
                            }
                        } else if join_error.is_panic() {
                            tracing::error!("Task panicked");
                            if let Err(e) = error_tx
                                .send(workers::TaskResult::Error("Task panicked".to_string()))
                                .await
                            {
                                tracing::warn!("Failed to send task error result: {:?}", e);
                            }
                        } else {
                            tracing::error!("Task failed: {}", join_error);
                            if let Err(e) = error_tx
                                .send(workers::TaskResult::Error("Task failed".to_string()))
                                .await
                            {
                                tracing::warn!("Failed to send task error result: {:?}", e);
                            }
                        }
                    }
                    Err(_) => {
                        tracing::error!("Task timed out after 300s - aborting task");
                        handle_to_abort.abort();
                        if let Err(e) = error_tx
                            .send(workers::TaskResult::Error(
                                "Task timed out after 300 seconds".to_string(),
                            ))
                            .await
                        {
                            tracing::warn!("Failed to send task error result: {:?}", e);
                        }
                    }
                }
            });

            // Phase 2 bridge: drop the outer handle (local executor runs independently).
            // The runtime tracks canonical lifecycle; this handle is retained only for
            // the local timeout/cancellation path via inner_abort.
            // TODO(phase-3): remove raw handle, rely on runtime.cancel() + channel drop
            drop(handle);
            let _ = inner_abort; // Retained for emergency cleanup; not stored on App.
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
