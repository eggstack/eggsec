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
    /// Enforcement-scope snapshot for scope-sensitive tasks (load-test).
    /// `None` fails closed for those tasks; attach the manual enforcement
    /// scope at submission time.
    pub scope: Option<eggsec::config::Scope>,
}

/// Real executor for `eggsec_runtime::Runtime`.
///
/// Uses a `TuiTaskDispatcher` to map `RunRequest` to engine calls, sending
/// typed `TaskResult` through channels for TUI consumption. Cancellation
/// races through the shared `eggsec_runtime::race_with_cancel` primitive —
/// the same primitive as the daemon-backed `EggsecRuntimeExecutor` — so
/// embedded and daemon modes share terminal/cancel semantics by construction.
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
        cancel: CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        let context = self.context.clone();

        Box::pin(async move {
            // Shared cancellation primitive (same as the daemon-backed
            // adapter): pre-cancelled tasks never start detached work, and
            // mid-execution cancellation drops the dispatch future, releasing
            // senders so forwarders drain instead of leaking.
            let dispatcher = TuiTaskDispatcher::new(context);
            eggsec_runtime::race_with_cancel(dispatcher.dispatch(request), cancel).await
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

    /// Release active-task state when an authoritative terminal lifecycle
    /// event (`TaskCompleted` / `TaskFailed` / `TaskCancelled`) arrives for the
    /// tab that was running.
    ///
    /// The receivers are retired later, by `update()`, once they are empty —
    /// see [`super::state::TaskState::finished`]. The typed `TaskResult` travels
    /// on a separate channel and may still be in flight when the completion
    /// event is observed, so dropping it here would swallow the result.
    pub(crate) fn clear_active_task_state_for(&mut self, tab: super::tabs::Tab) {
        if self.task_state.tab == Some(tab) {
            self.task_state.tab = None;
            self.task_state.started_at = None;
            self.task_state.paused = false;
            self.task_state.finished = true;
        }
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
        //
        // Both awaits are wrapped in a timeout: an unresponsive engine or
        // daemon socket would otherwise leak the spawned task and, because the
        // cancel is best-effort, leave the engine work running with no TUI
        // handle on it.
        const CANCEL_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);
        if let Some(session_id) = self.runtime_binding.session_id {
            if let Some(ref client) = self.runtime_client {
                let client = client.clone();
                let sid = session_id;
                tokio::spawn(async move {
                    match tokio::time::timeout(CANCEL_TIMEOUT, client.cancel_active(sid)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => {
                            tracing::debug!(
                                "Daemon cancel_active failed (may already be completed): {}",
                                e
                            );
                        }
                        Err(_) => {
                            tracing::warn!(
                                "Daemon cancel_active timed out after {}s; task may still be running",
                                CANCEL_TIMEOUT.as_secs()
                            );
                        }
                    }
                });
            } else {
                let runtime = self.runtime_binding.runtime.clone();
                let sid = session_id;
                tokio::spawn(async move {
                    match tokio::time::timeout(CANCEL_TIMEOUT, runtime.cancel_active(sid)).await {
                        Ok(Ok(())) => {}
                        Ok(Err(e)) => {
                            tracing::debug!(
                                "Runtime cancel_active failed (may already be completed): {}",
                                e
                            );
                        }
                        Err(_) => {
                            tracing::warn!(
                                "Runtime cancel_active timed out after {}s; task may still be running",
                                CANCEL_TIMEOUT.as_secs()
                            );
                        }
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
        self.task_state.paused = false;
        self.task_state.finished = false;
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
            // A task always starts unpaused. `update()` returns before draining the
            // progress/result channels while `paused` is set, so inheriting a stale
            // pause would silently suppress this task's entire output.
            self.task_state.paused = false;
            self.task_state.finished = false;

            // Task-tab mapping lives in the runtime adapter: lifecycle events
            // (progress, completion, failure) route to the originating tab
            // regardless of which tab is currently focused.

            // Update the executor context with new channel senders.
            // The executor reads this via ArcSwap before dispatching.
            // Carry the manual enforcement-scope snapshot for scope-sensitive
            // tasks (load-test per-hop authorization).
            let ctx = TuiDispatcherContext {
                progress_tx,
                result_tx,
                scope: Some(self.enforcement_state.loaded_scope().scope.clone()),
            };
            self.executor_context.store(Arc::new(ctx));

            // Submit to runtime for lifecycle tracking + execution.
            let runtime = self.runtime_binding.runtime.clone();
            let session_id = self.runtime_binding.session_id;
            let session_scope: eggsec_runtime::SessionScope =
                eggsec::config::session_scope_from_loaded(self.enforcement_state.loaded_scope());
            let pending_session_id =
                Arc::new(std::sync::Mutex::new(None::<eggsec_runtime::SessionId>));
            let pending_session_id_clone = pending_session_id.clone();
            let pending_event_rx = Arc::new(tokio::sync::Mutex::new(None));
            let pending_event_rx_clone = pending_event_rx.clone();
            self.runtime_pending_event_rx = Some(pending_event_rx);

            // Daemon mode must submit through the client that owns the attached
            // session. `runtime_binding.runtime` is always the local embedded
            // runtime, so submitting there with a daemon session id addressed a
            // session the local runtime never created and every task failed.
            // `runtime_client` is `Some` exactly when a daemon client is connected.
            let daemon_client = self.runtime_client.clone();
            let pending_error = Arc::new(std::sync::Mutex::new(None::<String>));
            self.runtime_pending_error = Some(pending_error.clone());
            let pending_error_clone = pending_error.clone();
            let surface = eggsec_runtime::request::RuntimeSurface::TuiManual;

            // Submission is a single RPC; a hang here would wedge the TUI
            // indefinitely, so it is bounded.
            const SUBMIT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);

            tokio::spawn(async move {
                let set_error = move |msg: String| {
                    tracing::error!("{msg}");
                    *pending_error_clone.lock().unwrap_or_else(|poisoned| {
                        tracing::warn!("TUI pending-error mutex was poisoned; recovering state");
                        poisoned.into_inner()
                    }) = Some(msg);
                };

                let submitted = tokio::time::timeout(SUBMIT_TIMEOUT, async {
                    match daemon_client {
                        // Daemon mode: the session is created/subscribed during
                        // attach, and results arrive on the daemon event handle.
                        Some(client) => {
                            let sid = match session_id {
                                Some(sid) => sid,
                                None => match client
                                    .create_session(surface, Some(session_scope), Vec::new())
                                    .await
                                {
                                    Ok(sid) => {
                                        *pending_session_id_clone
                                            .lock()
                                            .unwrap_or_else(|poisoned| {
                                                tracing::warn!(
                                                    "TUI pending-session-id mutex was poisoned; recovering state"
                                                );
                                                poisoned.into_inner()
                                            }) = Some(sid);
                                        sid
                                    }
                                    Err(e) => {
                                        return Err(format!("Failed to create daemon session: {e}"))
                                    }
                                },
                            };
                            client
                                .submit(sid, request)
                                .await
                                .map(|task_id| {
                                    tracing::debug!("Task submitted to daemon: {task_id}");
                                })
                                .map_err(|e| format!("Failed to submit task to daemon: {e}"))
                        }
                        None => {
                            let session_id = match session_id {
                                Some(sid) => sid,
                                None => match runtime
                                    .create_session_with_scope(
                                        eggsec_runtime::SessionOptions::default(),
                                        surface,
                                        Some(session_scope),
                                    )
                                    .await
                                {
                                    Ok(sid) => {
                                        *pending_session_id_clone
                                            .lock()
                                            .unwrap_or_else(|poisoned| {
                                                tracing::warn!(
                                                    "TUI pending-session-id mutex was poisoned; recovering state"
                                                );
                                                poisoned.into_inner()
                                            }) = Some(sid);
                                        sid
                                    }
                                    Err(e) => {
                                        return Err(format!(
                                            "Failed to create runtime session: {e}"
                                        ))
                                    }
                                },
                            };

                            // Subscribe to runtime events before task submission.
                            let event_rx = runtime.subscribe().await;
                            *pending_event_rx_clone.lock().await = Some(event_rx);

                            runtime
                                .submit(session_id, request)
                                .await
                                .map(|task_id| {
                                    tracing::debug!("Task submitted to runtime: {task_id}");
                                })
                                .map_err(|e| format!("Failed to submit task: {e}"))
                        }
                    }
                })
                .await;

                match submitted {
                    Ok(Ok(())) => {}
                    Ok(Err(msg)) => set_error(msg),
                    Err(_) => set_error(format!(
                        "Task submission timed out after {}s; the runtime did not respond",
                        SUBMIT_TIMEOUT.as_secs()
                    )),
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
    fn terminal_lifecycle_event_releases_active_task_state() {
        // Regression: `result_rx` is never closed (its sender is held by the
        // App-owned `executor_context` ArcSwap), so channel closure could not
        // signal completion. Without the authoritative lifecycle event the tab
        // stayed "running" forever and `q` never quit.
        let mut app = create_test_app();
        app.task_state.tab = Some(Tab::ScanPorts);
        app.task_state.started_at = Some(std::time::Instant::now());
        app.task_state.paused = true;
        assert!(app.has_active_task());

        let actions = vec![super::super::runtime_adapter::TuiAction::TabCompleted(
            Tab::ScanPorts,
            eggsec_runtime::TaskOutcome::Empty,
        )];
        super::super::runtime_adapter::TuiRuntimeAdapter::apply_actions(actions, &mut app);

        assert!(
            app.task_state.tab.is_none(),
            "active task tab must be released"
        );
        assert!(app.task_state.started_at.is_none());
        assert!(
            !app.task_state.paused,
            "terminal event must clear a stuck pause"
        );
    }

    #[test]
    fn terminal_lifecycle_event_for_other_tab_leaves_active_task_alone() {
        let mut app = create_test_app();
        app.task_state.tab = Some(Tab::ScanPorts);

        let actions = vec![super::super::runtime_adapter::TuiAction::TabCompleted(
            Tab::Fuzz,
            eggsec_runtime::TaskOutcome::Empty,
        )];
        super::super::runtime_adapter::TuiRuntimeAdapter::apply_actions(actions, &mut app);

        assert_eq!(app.task_state.tab, Some(Tab::ScanPorts));
    }

    #[test]
    fn terminal_event_then_update_retires_channels_so_quit_is_unblocked() {
        // Regression: the receivers are never closed (their sender is held by the
        // App-owned `executor_context` ArcSwap), so `has_active_task()` stayed
        // true forever — the task strip stayed on screen and `q` never quit.
        let mut app = create_test_app();
        let (result_tx, result_rx) = tokio::sync::mpsc::channel(1);
        let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(4);
        app.task_state.tab = Some(Tab::ScanPorts);
        app.task_state.started_at = Some(std::time::Instant::now());
        app.task_state.result_rx = Some(result_rx);
        app.task_state.progress_rx = Some(progress_rx);
        // Sender kept alive for the whole "session", as in production.
        let _keep = (result_tx, progress_tx);

        let actions = vec![super::super::runtime_adapter::TuiAction::TabCompleted(
            Tab::ScanPorts,
            eggsec_runtime::TaskOutcome::Empty,
        )];
        super::super::runtime_adapter::TuiRuntimeAdapter::apply_actions(actions, &mut app);
        assert!(app.task_state.finished);

        // One update retires the drained channels and releases the task.
        app.update();
        assert!(
            !app.has_active_task(),
            "task must be fully released after the terminal event"
        );
        assert!(app.task_state.result_rx.is_none());
        assert!(app.task_state.progress_rx.is_none());
    }

    #[test]
    fn a_queued_progress_update_is_applied_not_dropped_by_retirement() {
        // Regression guard for the retirement path: it must be gated on the
        // receivers being *empty*, probed without consuming. A consuming probe
        // (`try_recv().is_err()`) would silently discard a queued update.
        let mut app = create_test_app();
        app.current_tab = Tab::ScanPorts;
        let (result_tx, result_rx) = tokio::sync::mpsc::channel(1);
        let (progress_tx, progress_rx) = tokio::sync::mpsc::channel(4);
        app.task_state.tab = Some(Tab::ScanPorts);
        app.task_state.result_rx = Some(result_rx);
        app.task_state.progress_rx = Some(progress_rx);
        // Senders stay alive for the whole "session", as in production.
        let _keep_tx = result_tx;

        // Queued before the terminal event is observed.
        progress_tx
            .try_send((7, 9))
            .expect("progress channel accepts");

        let actions = vec![super::super::runtime_adapter::TuiAction::TabCompleted(
            Tab::ScanPorts,
            eggsec_runtime::TaskOutcome::Empty,
        )];
        super::super::runtime_adapter::TuiRuntimeAdapter::apply_actions(actions, &mut app);

        app.update();

        // The queued update reached the tab (7 of 9 -> ~78%), and only then were
        // the drained receivers retired.
        let progress = app.current_tab.as_tab_state(&app).progress();
        let expected = 7.0 / 9.0 * 100.0;
        assert!(
            (progress - expected).abs() < 1.0,
            "queued progress update was dropped by retirement (progress={progress}, expected~{expected})"
        );
        assert!(app.task_state.progress_rx.is_none());
        assert!(app.task_state.result_rx.is_none());
        assert!(!app.has_active_task());
    }

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
        use eggsec_runtime::{Runtime, RuntimeConfig, RuntimeSurface};

        // Create a runtime and a session outside of the TUI.
        let runtime = std::sync::Arc::new(Runtime::new(
            RuntimeConfig::default(),
            crate::app::task_runtime::TuiExecutor::new(std::sync::Arc::new(
                arc_swap::ArcSwap::new(std::sync::Arc::new(
                    crate::app::task_runtime::TuiDispatcherContext {
                        progress_tx: tokio::sync::mpsc::channel(1).0,
                        result_tx: tokio::sync::mpsc::channel(1).0,
                        scope: None,
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
                        scope: None,
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

    /// Embedded adapter cancellation contract (Phase 1).
    ///
    /// `TuiExecutor` races dispatch against the runtime cancellation token
    /// with the same shared primitive as the daemon-backed adapter
    /// (`eggsec_runtime::race_with_cancel`): pre-cancelled tasks never start
    /// detached work, and mid-execution cancellation drops the future,
    /// releasing senders so forwarders drain instead of leaking.
    #[tokio::test]
    async fn embedded_adapter_cancels_before_detached_work() {
        use eggsec_runtime::{race_with_cancel, RuntimeError};

        let cancel = eggsec_runtime::CancellationToken::new();
        cancel.cancel();
        let result =
            race_with_cancel(async { Ok::<_, RuntimeError>("must not run") }, cancel).await;
        assert!(matches!(result, Err(RuntimeError::DispatchFailed(_))));
        if let Err(RuntimeError::DispatchFailed(msg)) = result {
            assert!(
                msg.contains("cancel"),
                "cancel error must mention cancel: {msg}"
            );
        }
    }

    #[tokio::test]
    async fn embedded_cancel_releases_senders() {
        use eggsec_runtime::{race_with_cancel, RuntimeError};

        let (tx, mut rx) = tokio::sync::mpsc::channel::<u32>(4);
        let cancel = eggsec_runtime::CancellationToken::new();
        let cancel_clone = cancel.clone();
        let dispatch = async move {
            let _held = tx;
            std::future::pending::<()>().await;
            #[allow(unreachable_code)]
            Ok::<_, RuntimeError>("never")
        };
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            cancel_clone.cancel();
        });
        let result = race_with_cancel(dispatch, cancel).await;
        assert!(matches!(result, Err(RuntimeError::DispatchFailed(_))));
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
                .await
                .map(|v| v.is_none())
                .unwrap_or(false),
            "channel must close after cancellation (no detached sender)"
        );
    }
}
