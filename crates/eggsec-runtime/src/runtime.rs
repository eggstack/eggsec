use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;

use crate::event::{RuntimeErrorInfo, RuntimeEvent, TaskOutcome, TaskProgress, TaskStatus};
use crate::ids::{ClientId, SessionId, TaskId};
use crate::request::{RunRequest, RuntimeSurface};
use crate::session::{
    RuntimeExecutionContext, RuntimeSession, SessionScope, SessionSnapshot, SessionSummary,
};
use crate::RuntimeError;

/// Emit a runtime event best-effort. Logs at trace level if no subscribers
/// are listening. Never panics on channel failure.
fn emit_event(tx: &broadcast::Sender<RuntimeEvent>, event: RuntimeEvent) {
    if tx.receiver_count() == 0 {
        tracing::trace!("no event subscribers; dropping event");
    } else if let Err(e) = tx.send(event) {
        tracing::trace!("event send failed (likely no active receivers): {}", e);
    }
}

/// Emit a runtime event with audit-critical semantics. Logs at warn level
/// on send failure to make policy-relevant event loss observable.
fn emit_event_critical(tx: &broadcast::Sender<RuntimeEvent>, event: RuntimeEvent) {
    if tx.receiver_count() == 0 {
        tracing::warn!("no event subscribers for critical event; event dropped");
    } else if let Err(e) = tx.send(event) {
        tracing::warn!("critical event send failed: {}", e);
    }
}

/// Configuration for the runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Default timeout for tasks. None means no timeout.
    pub default_task_timeout: Option<Duration>,
    /// Maximum active tasks per session.
    pub max_active_tasks_per_session: usize,
    /// Capacity of the event broadcast channel.
    pub event_channel_capacity: usize,
    /// Capabilities advertised by this runtime. Determines which task kinds
    /// sessions report as available. Use `RuntimeCapabilities::noop()` for
    /// daemons without a real executor.
    pub capabilities: crate::capabilities::RuntimeCapabilities,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_task_timeout: Some(Duration::from_secs(300)),
            max_active_tasks_per_session: 1,
            event_channel_capacity: 256,
            capabilities: crate::capabilities::RuntimeCapabilities::full(),
        }
    }
}

/// Options for creating a session.
#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// Override for the default task timeout for this session.
    pub task_timeout: Option<Duration>,
}

/// Event receiver for subscribing to runtime events.
pub struct RuntimeEventReceiver {
    rx: broadcast::Receiver<RuntimeEvent>,
}

impl RuntimeEventReceiver {
    /// Create a receiver from a broadcast channel. Useful for tests.
    pub fn from_broadcast(rx: broadcast::Receiver<RuntimeEvent>) -> Self {
        Self { rx }
    }

    /// Receive the next event. Returns `None` if the channel is closed.
    /// Logs a warning if events were dropped due to broadcast overflow.
    pub async fn recv(&mut self) -> Option<RuntimeEvent> {
        match self.rx.recv().await {
            Ok(event) => Some(event),
            Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                tracing::warn!(
                    dropped = n,
                    "Broadcast event channel overflow, events dropped"
                );
                // Return the next available event after the lag
                self.rx.recv().await.ok()
            }
            Err(tokio::sync::broadcast::error::RecvError::Closed) => None,
        }
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&mut self) -> Option<RuntimeEvent> {
        self.rx.try_recv().ok()
    }
}

/// Sink for task executors to report progress and completion.
#[derive(Clone)]
pub struct RuntimeEventSink {
    task_id: TaskId,
    session_id: SessionId,
    event_tx: broadcast::Sender<RuntimeEvent>,
}

impl RuntimeEventSink {
    fn new(
        task_id: TaskId,
        session_id: SessionId,
        event_tx: broadcast::Sender<RuntimeEvent>,
    ) -> Self {
        Self {
            task_id,
            session_id,
            event_tx,
        }
    }

    /// Return the session ID this sink belongs to.
    pub fn session_id(&self) -> SessionId {
        self.session_id
    }

    /// Emit a progress event.
    pub fn progress(&self, completed: u64, total: Option<u64>, message: Option<String>) {
        emit_event(
            &self.event_tx,
            RuntimeEvent::TaskProgress {
                session_id: self.session_id,
                task_id: self.task_id,
                progress: TaskProgress {
                    completed,
                    total,
                    message,
                },
            },
        );
    }

    /// Emit a log event.
    pub fn log(&self, level: crate::event::LogLevel, message: String) {
        emit_event(
            &self.event_tx,
            RuntimeEvent::TaskLog {
                session_id: self.session_id,
                task_id: Some(self.task_id),
                level,
                message,
            },
        );
    }

    /// Emit a completion event.
    pub fn completed(&self, outcome: TaskOutcome) {
        emit_event(
            &self.event_tx,
            RuntimeEvent::TaskCompleted {
                session_id: self.session_id,
                task_id: self.task_id,
                outcome,
            },
        );
    }

    /// Emit a failure event.
    pub fn failed(&self, message: String, code: Option<String>) {
        emit_event_critical(
            &self.event_tx,
            RuntimeEvent::TaskFailed {
                session_id: self.session_id,
                task_id: self.task_id,
                error: RuntimeErrorInfo {
                    message,
                    code,
                    details: None,
                },
            },
        );
    }
}

/// Trait for task executors. Implementations bridge the runtime to actual tool
/// execution. In Phase 2 the TUI provides an executor that wraps the existing
/// worker path; in Phase 3 the executor moves into the runtime.
///
/// The executor receives a [`RuntimeExecutionContext`] carrying the session's
/// trust boundary (surface, scope). Executors must use this context — not
/// hardcoded defaults — for enforcement decisions.
pub trait RuntimeTaskExecutor: Send + Sync + 'static {
    /// Execute a task. The executor should report progress via `sink` and
    /// return an outcome on success. The `cancel` token is triggered when the
    /// task is cancelled or times out.
    ///
    /// The `context` carries the session's execution surface and scope metadata,
    /// enabling the executor to make trust-boundary-aware enforcement decisions.
    fn execute(
        &self,
        task_id: TaskId,
        request: RunRequest,
        context: RuntimeExecutionContext,
        sink: RuntimeEventSink,
        cancel: CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    >;
}

struct RuntimeState {
    sessions: HashMap<SessionId, RuntimeSession>,
    config: RuntimeConfig,
    event_tx: broadcast::Sender<RuntimeEvent>,
    executor: Arc<dyn RuntimeTaskExecutor>,
}

/// Frontend-neutral runtime that owns task lifecycle.
///
/// The runtime manages sessions, tasks, timeouts, cancellation, and event
/// delivery. Frontends submit requests and consume events without holding raw
/// task handles.
///
/// Sessions are bound to an execution surface at creation time, establishing
/// the trust boundary for policy enforcement. Each session owns its canonical
/// task state; frontends query via `snapshot()` rather than holding duplicate
/// lifecycle state.
pub struct Runtime {
    state: Arc<Mutex<RuntimeState>>,
}

impl Runtime {
    /// Create a new runtime with the given configuration and executor.
    pub fn new(config: RuntimeConfig, executor: impl RuntimeTaskExecutor) -> Self {
        let (event_tx, _) = broadcast::channel(config.event_channel_capacity);
        Self {
            state: Arc::new(Mutex::new(RuntimeState {
                sessions: HashMap::new(),
                config,
                event_tx,
                executor: Arc::new(executor),
            })),
        }
    }

    /// Create a new session bound to the given execution surface.
    ///
    /// The surface establishes the trust boundary for this session:
    /// - `TuiManual` / `CliManual`: interactive manual surfaces
    /// - `McpServer` / `SecurityAgent` / `RestApi` / `GrpcApi`: strict programmatic surfaces
    /// - `CiStrict`: CI pipeline surface
    pub async fn create_session(
        &self,
        options: SessionOptions,
        surface: RuntimeSurface,
    ) -> Result<SessionId, RuntimeError> {
        self.create_session_with_scope(options, surface, None).await
    }

    /// Create a new session with an explicit scope binding.
    pub async fn create_session_with_scope(
        &self,
        options: SessionOptions,
        surface: RuntimeSurface,
        scope: Option<SessionScope>,
    ) -> Result<SessionId, RuntimeError> {
        let session_id = SessionId::new();
        let mut state = self.state.lock().await;
        let capabilities = state.config.capabilities.clone();
        let session = match scope {
            Some(s) => RuntimeSession::with_scope(session_id, surface, s),
            None => RuntimeSession::new(session_id, surface),
        }
        .with_task_timeout(options.task_timeout)
        .with_capabilities(capabilities);
        state.sessions.insert(session_id, session);
        emit_event(&state.event_tx, RuntimeEvent::SessionCreated { session_id });
        Ok(session_id)
    }

    /// Get the execution surface for a session.
    pub async fn session_surface(
        &self,
        session_id: SessionId,
    ) -> Result<RuntimeSurface, RuntimeError> {
        let state = self.state.lock().await;
        let session = state
            .sessions
            .get(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;
        Ok(session.execution_surface())
    }

    /// Get the scope metadata for a session, if bound.
    pub async fn session_scope(
        &self,
        session_id: SessionId,
    ) -> Result<Option<SessionScope>, RuntimeError> {
        let state = self.state.lock().await;
        let session = state
            .sessions
            .get(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;
        Ok(session.scope().cloned())
    }

    /// Set the owner client for a session.
    ///
    /// Used by the daemon to record which client created the session,
    /// enabling owner-filtered persisted session access after restart.
    pub async fn set_session_owner(
        &self,
        session_id: SessionId,
        owner: ClientId,
    ) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().await;
        let session = state
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;
        session.set_owner(owner);
        Ok(())
    }

    /// List summaries for all active (non-closed) sessions.
    pub async fn list_sessions(&self) -> Vec<SessionSummary> {
        let state = self.state.lock().await;
        state
            .sessions
            .iter()
            .filter(|(_, session)| !session.is_closed())
            .map(|(id, session)| SessionSummary {
                session_id: *id,
                surface: session.execution_surface(),
                scope: session.scope().cloned(),
                active_count: session.active_tasks().len(),
                completed_count: session.completed_tasks().len(),
                created_at_epoch_secs: session.created_at_secs(),
                owner_client_id: session.owner_client_id(),
            })
            .collect()
    }

    /// Submit a task to a session. Returns the task ID.
    pub async fn submit(
        &self,
        session_id: SessionId,
        request: RunRequest,
    ) -> Result<TaskId, RuntimeError> {
        let task_id = TaskId::new();

        let (cancel_token, timeout_duration, executor_clone, event_tx_clone, context) = {
            let mut state = self.state.lock().await;

            // Validate session exists
            if !state.sessions.contains_key(&session_id) {
                return Err(RuntimeError::SessionNotFound(session_id.to_string()));
            }

            // Reject submissions to closed sessions
            if state
                .sessions
                .get(&session_id)
                .is_some_and(|s| s.is_closed())
            {
                return Err(RuntimeError::SessionNotFound(session_id.to_string()));
            }

            // Single-active-task policy: terminalize any existing active tasks
            let active_ids: Vec<TaskId> = state
                .sessions
                .get(&session_id)
                .unwrap()
                .tasks
                .iter()
                .filter(|(_, t)| !t.status.is_terminal())
                .map(|(id, _)| *id)
                .collect();
            for existing_id in active_ids {
                if let Some(task) = state
                    .sessions
                    .get_mut(&session_id)
                    .unwrap()
                    .tasks
                    .get_mut(&existing_id)
                {
                    task.status = TaskStatus::Cancelled;
                    task.last_error = Some("replaced by new task".into());
                    if let Some(cancel) = task.abort.take() {
                        cancel.cancel();
                    }
                    task._handle = None;
                }
                emit_event(
                    &state.event_tx,
                    RuntimeEvent::TaskCancelled {
                        session_id,
                        task_id: existing_id,
                        reason: Some("replaced by new task".into()),
                    },
                );
            }
            if let Some(session) = state.sessions.get_mut(&session_id) {
                session.increment_generation();
            }

            let cancel_token = CancellationToken::new();
            // Session timeout overrides runtime default. None means "use runtime default".
            let timeout_duration = state
                .sessions
                .get(&session_id)
                .and_then(|s| s.task_timeout_override())
                .or(state.config.default_task_timeout);

            // Build execution context from session state (trust boundary).
            let context = {
                let session = state.sessions.get(&session_id).unwrap();
                RuntimeExecutionContext {
                    session_id,
                    surface: session.execution_surface(),
                    scope: session.scope().cloned(),
                }
            };

            state.sessions.get_mut(&session_id).unwrap().tasks.insert(
                task_id,
                crate::session::TaskRecord {
                    request: request.clone(),
                    status: TaskStatus::Queued,
                    progress: None,
                    last_error: None,
                    outcome: None,
                    abort: Some(cancel_token.clone()),
                    _handle: None,
                },
            );

            emit_event(
                &state.event_tx,
                RuntimeEvent::TaskQueued {
                    session_id,
                    task_id,
                    request: request.clone(),
                },
            );

            (
                cancel_token,
                timeout_duration,
                state.executor.clone(),
                state.event_tx.clone(),
                context,
            )
        };

        // Spawn the task outside the lock
        let state_arc = self.state.clone();
        let sink = RuntimeEventSink::new(task_id, session_id, event_tx_clone.clone());

        let handle = {
            let state_arc = state_arc.clone();
            let cancel_for_spawn = cancel_token.clone();
            let request_for_spawn = request.clone();

            tokio::spawn(async move {
                // Signal TaskStarted — but only if the task hasn't been
                // terminalized (cancelled/timed out/superseded) while queued.
                let already_terminal = {
                    let mut state = state_arc.lock().await;
                    if let Some(session) = state.sessions.get_mut(&session_id) {
                        if let Some(task) = session.tasks.get_mut(&task_id) {
                            if task.status.is_terminal() {
                                true
                            } else {
                                task.status = TaskStatus::Running;
                                false
                            }
                        } else {
                            true
                        }
                    } else {
                        true
                    }
                };
                if already_terminal {
                    return;
                }
                emit_event(
                    &event_tx_clone,
                    RuntimeEvent::TaskStarted {
                        session_id,
                        task_id,
                    },
                );

                // Execute with timeout
                let result = if let Some(timeout) = timeout_duration {
                    tokio::time::timeout(
                        timeout,
                        executor_clone.execute(
                            task_id,
                            request_for_spawn,
                            context,
                            sink,
                            cancel_for_spawn.clone(),
                        ),
                    )
                    .await
                } else {
                    Ok(executor_clone
                        .execute(
                            task_id,
                            request_for_spawn,
                            context,
                            sink,
                            cancel_for_spawn.clone(),
                        )
                        .await)
                };

                // Determine final status from executor result
                let (final_status, error_msg, outcome_value, terminal_event) = match result {
                    Ok(Ok(outcome)) => {
                        let event = RuntimeEvent::TaskCompleted {
                            session_id,
                            task_id,
                            outcome: outcome.clone(),
                        };
                        (TaskStatus::Completed, None, Some(outcome), Some(event))
                    }
                    Ok(Err(e)) => {
                        let msg = e.to_string();
                        let event = RuntimeEvent::TaskFailed {
                            session_id,
                            task_id,
                            error: RuntimeErrorInfo {
                                message: msg.clone(),
                                code: None,
                                details: None,
                            },
                        };
                        (TaskStatus::Failed, Some(msg), None, Some(event))
                    }
                    Err(_elapsed) => {
                        cancel_for_spawn.cancel();
                        let event = RuntimeEvent::TaskCancelled {
                            session_id,
                            task_id,
                            reason: Some("timed out".into()),
                        };
                        (
                            TaskStatus::TimedOut,
                            Some("task timed out".into()),
                            None,
                            Some(event),
                        )
                    }
                };

                // Update task record FIRST — before emitting terminal events.
                // This ensures snapshot() sees the terminal state before any
                // subscriber observes the terminal event, preventing the persistence
                // worker from capturing a stale snapshot.
                let mut state = state_arc.lock().await;
                if let Some(session) = state.sessions.get_mut(&session_id) {
                    if let Some(task) = session.tasks.get_mut(&task_id) {
                        if task.status.is_terminal() {
                            // Task was already terminal (cancelled, timed out, or
                            // superseded). Discard the late result.
                            return;
                        }
                        task.status = final_status;
                        task.last_error = error_msg;
                        task.outcome = outcome_value;
                        task.abort = None;
                        task._handle = None;
                    }
                    session.increment_generation();
                }
                drop(state);

                // NOW emit terminal event — state is already updated, so any
                // subscriber (including persistence worker) will see consistent state.
                if let Some(event) = terminal_event {
                    let is_critical = matches!(
                        &event,
                        RuntimeEvent::TaskFailed { .. } | RuntimeEvent::TaskCancelled { .. }
                    );
                    if is_critical {
                        emit_event_critical(&event_tx_clone, event);
                    } else {
                        emit_event(&event_tx_clone, event);
                    }
                }
            })
        };

        // Store the handle in the task record
        {
            let mut state = self.state.lock().await;
            if let Some(session) = state.sessions.get_mut(&session_id) {
                if let Some(task) = session.tasks.get_mut(&task_id) {
                    task._handle = Some(handle);
                }
            }
        }

        Ok(task_id)
    }

    /// Cancel a task.
    pub async fn cancel(&self, session_id: SessionId, task_id: TaskId) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().await;
        let session = state
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;

        {
            let task = session
                .tasks
                .get_mut(&task_id)
                .ok_or_else(|| RuntimeError::TaskNotFound(task_id.to_string()))?;

            if task.status.is_terminal() {
                return Err(RuntimeError::TaskAlreadyCompleted(task_id.to_string()));
            }

            task.status = TaskStatus::Cancelled;
            if let Some(cancel) = task.abort.take() {
                cancel.cancel();
            }
        }

        session.increment_generation();

        emit_event_critical(
            &state.event_tx,
            RuntimeEvent::TaskCancelled {
                session_id,
                task_id,
                reason: Some("cancelled by user".into()),
            },
        );

        Ok(())
    }

    /// Cancel the active (non-terminal) task in a session.
    ///
    /// This is a convenience method for single-active-task sessions (the
    /// common case). Returns `Ok(())` if a task was cancelled, or
    /// `Ok(())` with no side effects if no active task exists.
    pub async fn cancel_active(&self, session_id: SessionId) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().await;
        let session = state
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;

        let active_id = session
            .tasks
            .iter()
            .find(|(_, t)| !t.status.is_terminal())
            .map(|(id, _)| *id);

        if let Some(task_id) = active_id {
            if let Some(task) = session.tasks.get_mut(&task_id) {
                task.status = TaskStatus::Cancelled;
                if let Some(cancel) = task.abort.take() {
                    cancel.cancel();
                }
            }
            session.increment_generation();
            emit_event_critical(
                &state.event_tx,
                RuntimeEvent::TaskCancelled {
                    session_id,
                    task_id,
                    reason: Some("cancelled by user".into()),
                },
            );
        }

        Ok(())
    }

    /// Get a snapshot of a session's state.
    pub async fn snapshot(&self, session_id: SessionId) -> Result<SessionSnapshot, RuntimeError> {
        let state = self.state.lock().await;
        let session = state
            .sessions
            .get(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;

        Ok(session.snapshot())
    }

    /// Close a session, marking it as closed and rejecting future task submissions.
    ///
    /// Returns `Ok(())` on success. Returns `SessionNotFound` if the session
    /// does not exist or is already closed.
    ///
    /// All active tasks in the session are cancelled before closing.
    pub async fn close_session(&self, session_id: SessionId) -> Result<(), RuntimeError> {
        let mut state = self.state.lock().await;
        let session = state
            .sessions
            .get_mut(&session_id)
            .ok_or_else(|| RuntimeError::SessionNotFound(session_id.to_string()))?;

        if session.is_closed() {
            return Err(RuntimeError::SessionNotFound(session_id.to_string()));
        }

        // Cancel all active tasks before closing
        let active_task_ids: Vec<TaskId> = session
            .tasks
            .iter()
            .filter(|(_, t)| !t.status.is_terminal())
            .map(|(id, _)| *id)
            .collect();

        for task_id in &active_task_ids {
            if let Some(task) = session.tasks.get_mut(task_id) {
                if let Some(abort) = task.abort.take() {
                    abort.cancel();
                }
                task.status = TaskStatus::Cancelled;
                task.last_error = Some("session closed".into());
            }
        }

        session.close();
        session.increment_generation();

        // Emit cancellation events for each active task, then the session closed event
        for task_id in &active_task_ids {
            emit_event_critical(
                &state.event_tx,
                RuntimeEvent::TaskCancelled {
                    session_id,
                    task_id: *task_id,
                    reason: Some("session closed".into()),
                },
            );
        }

        emit_event_critical(&state.event_tx, RuntimeEvent::SessionClosed { session_id });

        Ok(())
    }

    /// Hydrate a session from a snapshot (for daemon recovery on startup).
    ///
    /// Reconstructs a `RuntimeSession` from a persisted snapshot and inserts
    /// it into the runtime's session map. Active tasks from the snapshot are
    /// not restored (they held runtime handles), but completed task records
    /// are preserved for history and audit querying.
    pub async fn hydrate_session(
        &self,
        snapshot: SessionSnapshot,
    ) -> Result<SessionId, RuntimeError> {
        let session_id = snapshot.session_id;
        let session = RuntimeSession::hydrate_from_snapshot(snapshot);
        let mut state = self.state.lock().await;
        state.sessions.insert(session_id, session);
        emit_event(&state.event_tx, RuntimeEvent::SessionCreated { session_id });
        Ok(session_id)
    }

    /// Subscribe to runtime events.
    pub async fn subscribe(&self) -> RuntimeEventReceiver {
        let state = self.state.lock().await;
        RuntimeEventReceiver {
            rx: state.event_tx.subscribe(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::LogLevel;
    use crate::request::{PortScanParams, RuntimeSurface, TaskKind};

    /// A test executor that immediately completes with a text outcome.
    struct ImmediateExecutor;

    impl RuntimeTaskExecutor for ImmediateExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: RunRequest,
            _context: RuntimeExecutionContext,
            sink: RuntimeEventSink,
            _cancel: CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>>
                    + Send
                    + 'static,
            >,
        > {
            Box::pin(async move {
                sink.log(LogLevel::Info, "test task started".into());
                sink.progress(50, Some(100), Some("halfway".into()));
                sink.progress(100, Some(100), None);
                Ok(TaskOutcome::Text("done".into()))
            })
        }
    }

    /// A test executor that sleeps then completes.
    struct SlowExecutor;

    impl RuntimeTaskExecutor for SlowExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: RunRequest,
            _context: RuntimeExecutionContext,
            _sink: RuntimeEventSink,
            cancel: CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>>
                    + Send
                    + 'static,
            >,
        > {
            Box::pin(async move {
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(60)) => {},
                    _ = cancel.cancelled() => {},
                }
                Ok(TaskOutcome::Empty)
            })
        }
    }

    /// A test executor that always fails.
    struct FailingExecutor;

    impl RuntimeTaskExecutor for FailingExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: RunRequest,
            _context: RuntimeExecutionContext,
            _sink: RuntimeEventSink,
            _cancel: CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>>
                    + Send
                    + 'static,
            >,
        > {
            Box::pin(async { Err(RuntimeError::UnsupportedTaskKind) })
        }
    }

    /// A test executor that sleeps longer than the timeout/cancellation, then
    /// returns a late result. Used to verify stale completion guards.
    struct DelayedReturnExecutor;

    impl RuntimeTaskExecutor for DelayedReturnExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: RunRequest,
            _context: RuntimeExecutionContext,
            _sink: RuntimeEventSink,
            cancel: CancellationToken,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>>
                    + Send
                    + 'static,
            >,
        > {
            Box::pin(async move {
                // Sleep well past timeout/cancellation
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(5)) => {},
                    _ = cancel.cancelled() => {},
                }
                // Return a result even after cancellation
                Ok(TaskOutcome::Text("late result".into()))
            })
        }
    }

    fn test_request() -> RunRequest {
        RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: Some("80".into()),
                scan_type: None,
                timeout_ms: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: vec![],
        }
    }

    #[tokio::test]
    async fn create_session_and_submit() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let task_id = runtime.submit(session_id, test_request()).await.unwrap();

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(!snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.active_tasks[0].task_id, task_id);
    }

    #[tokio::test]
    async fn task_completes_immediately() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Wait briefly for the spawned task to complete
        tokio::time::sleep(Duration::from_millis(50)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Completed);
    }

    #[tokio::test]
    async fn cancel_task() {
        let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Give the task a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        runtime.cancel(session_id, task_id).await.unwrap();

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn task_timeout() {
        let config = RuntimeConfig {
            default_task_timeout: Some(Duration::from_millis(50)),
            ..Default::default()
        };
        let runtime = Runtime::new(config, SlowExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Wait for timeout to fire
        tokio::time::sleep(Duration::from_millis(200)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::TimedOut);
    }

    #[tokio::test]
    async fn failed_task() {
        let runtime = Runtime::new(RuntimeConfig::default(), FailingExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Failed);
        assert!(snapshot.completed_tasks[0].last_error.is_some());
    }

    #[tokio::test]
    async fn events_are_emitted() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let mut rx = runtime.subscribe().await;
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Collect events
        let mut events = Vec::new();
        tokio::time::timeout(Duration::from_millis(500), async {
            loop {
                if let Some(event) = rx.recv().await {
                    events.push(event);
                    if matches!(events.last(), Some(RuntimeEvent::TaskCompleted { .. })) {
                        break;
                    }
                }
            }
        })
        .await
        .ok();

        assert!(events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::SessionCreated { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TaskQueued { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TaskStarted { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TaskProgress { .. })));
        assert!(events
            .iter()
            .any(|e| matches!(e, RuntimeEvent::TaskCompleted { .. })));
    }

    #[tokio::test]
    async fn cancel_replaces_existing_task() {
        let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let task1 = runtime.submit(session_id, test_request()).await.unwrap();
        let task2 = runtime.submit(session_id, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        // task1 should have been cancelled by task2 submission and remain in history
        let snapshot = runtime.snapshot(session_id).await.unwrap();
        // task2 is the only active one
        assert_eq!(snapshot.active_tasks.len(), 1);
        assert_eq!(snapshot.active_tasks[0].task_id, task2);
        // task1 should be in completed tasks with Cancelled status
        let cancelled = snapshot
            .completed_tasks
            .iter()
            .find(|t| t.task_id == task1)
            .expect("cancelled task should remain in history");
        assert_eq!(cancelled.status, TaskStatus::Cancelled);
        assert!(cancelled.last_error.is_some());
        // Cancelled task preserves task kind and request summary
        assert_eq!(cancelled.task_kind, test_request().task_kind);
        assert!(!cancelled.request_summary.is_empty());
    }

    #[tokio::test]
    async fn session_not_found() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let fake = SessionId::new();
        let result = runtime.submit(fake, test_request()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn stale_completion_does_not_overwrite_terminal() {
        // The executor returns "late result" after 5s, but the task should be
        // cancelled by the user before that. The stale completion guard must
        // prevent the late result from overwriting the Cancelled status.
        let runtime = Runtime::new(RuntimeConfig::default(), DelayedReturnExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Give task a moment to start
        tokio::time::sleep(Duration::from_millis(10)).await;

        // Cancel the task
        runtime.cancel(session_id, task_id).await.unwrap();

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Cancelled);
        // The late result must NOT overwrite the outcome
        assert!(snapshot.completed_tasks[0].outcome.is_none());
    }

    #[tokio::test]
    async fn stale_timeout_does_not_overwrite_terminal() {
        // Task times out, then executor returns late. TimedOut must persist.
        let config = RuntimeConfig {
            default_task_timeout: Some(Duration::from_millis(50)),
            ..Default::default()
        };
        let runtime = Runtime::new(config, DelayedReturnExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(200)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::TimedOut);
        assert!(snapshot.completed_tasks[0].outcome.is_none());
    }

    #[tokio::test]
    async fn cancel_completed_task_errors() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let task_id = runtime.submit(session_id, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let result = runtime.cancel(session_id, task_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn session_surface_query() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::McpServer)
            .await
            .unwrap();

        let surface = runtime.session_surface(session_id).await.unwrap();
        assert_eq!(surface, RuntimeSurface::McpServer);
    }

    #[tokio::test]
    async fn snapshot_includes_surface() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::RestApi)
            .await
            .unwrap();

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert_eq!(snapshot.surface, RuntimeSurface::RestApi);
    }

    #[tokio::test]
    async fn sessions_without_tui() {
        // Verify that runtime sessions can be created, submitted, and
        // queried entirely without any TUI dependency.
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
            .await
            .unwrap();

        assert_eq!(
            runtime.session_surface(session_id).await.unwrap(),
            RuntimeSurface::CliManual
        );

        let task_id = runtime.submit(session_id, test_request()).await.unwrap();
        tokio::time::sleep(Duration::from_millis(50)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].task_id, task_id);
        assert_eq!(snapshot.surface, RuntimeSurface::CliManual);
    }

    #[tokio::test]
    async fn multiple_sessions_independent() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let s1 = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let s2 = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::McpServer)
            .await
            .unwrap();

        runtime.submit(s1, test_request()).await.unwrap();
        runtime.submit(s2, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(50)).await;

        let snap1 = runtime.snapshot(s1).await.unwrap();
        let snap2 = runtime.snapshot(s2).await.unwrap();

        assert_eq!(snap1.surface, RuntimeSurface::TuiManual);
        assert_eq!(snap2.surface, RuntimeSurface::McpServer);
        assert_eq!(snap1.completed_tasks.len(), 1);
        assert_eq!(snap2.completed_tasks.len(), 1);
    }

    #[tokio::test]
    async fn cancel_active_cancels_running_task() {
        let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        runtime.cancel_active(session_id).await.unwrap();

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Cancelled);
    }

    #[tokio::test]
    async fn cancel_active_noop_when_no_active_task() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();

        // No tasks submitted — cancel_active should succeed without error.
        runtime.cancel_active(session_id).await.unwrap();

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert!(snapshot.completed_tasks.is_empty());
    }

    #[tokio::test]
    async fn session_timeout_overrides_runtime_default() {
        // Runtime default is 300s, but session sets 50ms timeout.
        let config = RuntimeConfig {
            default_task_timeout: Some(Duration::from_secs(300)),
            ..Default::default()
        };
        let runtime = Runtime::new(config, SlowExecutor);
        let session_id = runtime
            .create_session(
                SessionOptions {
                    task_timeout: Some(Duration::from_millis(50)),
                },
                RuntimeSurface::TuiManual,
            )
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Wait for session timeout to fire (50ms)
        tokio::time::sleep(Duration::from_millis(200)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::TimedOut);
    }

    #[tokio::test]
    async fn session_without_override_uses_runtime_default() {
        let config = RuntimeConfig {
            default_task_timeout: Some(Duration::from_millis(50)),
            ..Default::default()
        };
        let runtime = Runtime::new(config, SlowExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Wait for runtime default timeout (50ms)
        tokio::time::sleep(Duration::from_millis(200)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.active_tasks.is_empty());
        assert_eq!(snapshot.completed_tasks.len(), 1);
        assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::TimedOut);
    }

    #[tokio::test]
    async fn two_sessions_different_timeout_behavior() {
        let config = RuntimeConfig {
            default_task_timeout: Some(Duration::from_millis(50)),
            ..Default::default()
        };
        let runtime = Runtime::new(config, SlowExecutor);

        // Session with no override — uses 50ms default
        let s1 = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();

        // Session with explicit longer timeout — should NOT time out in 200ms
        let s2 = runtime
            .create_session(
                SessionOptions {
                    task_timeout: Some(Duration::from_secs(300)),
                },
                RuntimeSurface::McpServer,
            )
            .await
            .unwrap();

        runtime.submit(s1, test_request()).await.unwrap();
        runtime.submit(s2, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(200)).await;

        let snap1 = runtime.snapshot(s1).await.unwrap();
        let snap2 = runtime.snapshot(s2).await.unwrap();

        // s1 should have timed out
        assert!(snap1.active_tasks.is_empty());
        assert_eq!(snap1.completed_tasks.len(), 1);
        assert_eq!(snap1.completed_tasks[0].status, TaskStatus::TimedOut);

        // s2 should still be active (or running)
        assert_eq!(snap2.active_tasks.len(), 1);
        assert_eq!(snap2.active_tasks[0].status, TaskStatus::Running);
    }

    #[tokio::test]
    async fn event_emission_no_receivers_does_not_panic() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        // Create session and submit WITHOUT subscribing — no receivers
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _task_id = runtime.submit(session_id, test_request()).await.unwrap();

        // Wait for task to complete — should not panic even without subscribers
        tokio::time::sleep(Duration::from_millis(50)).await;

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert_eq!(snapshot.completed_tasks.len(), 1);
    }

    #[tokio::test]
    async fn close_session_marks_session_closed() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();

        assert!(runtime.close_session(session_id).await.is_ok());

        let snapshot = runtime.snapshot(session_id).await.unwrap();
        assert!(snapshot.closed);
        assert!(snapshot.closed_at.is_some());
    }

    #[tokio::test]
    async fn close_session_not_found() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let fake = SessionId::new();
        let result = runtime.close_session(fake).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn close_session_already_closed_errors() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();

        runtime.close_session(session_id).await.unwrap();
        let result = runtime.close_session(session_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn submit_to_closed_session_errors() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();

        runtime.close_session(session_id).await.unwrap();

        let result = runtime.submit(session_id, test_request()).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn list_sessions_excludes_closed() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let s1 = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();
        let _s2 = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::McpServer)
            .await
            .unwrap();

        let sessions = runtime.list_sessions().await;
        assert_eq!(sessions.len(), 2);

        runtime.close_session(s1).await.unwrap();

        let sessions = runtime.list_sessions().await;
        assert_eq!(sessions.len(), 1);
        assert!(sessions.iter().all(|s| s.session_id != s1));
    }

    #[tokio::test]
    async fn close_session_emits_event() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let mut rx = runtime.subscribe().await;
        let session_id = runtime
            .create_session(SessionOptions::default(), RuntimeSurface::TuiManual)
            .await
            .unwrap();

        runtime.close_session(session_id).await.unwrap();

        // Drain events looking for SessionClosed
        let mut found = false;
        tokio::time::timeout(Duration::from_millis(200), async {
            while let Some(event) = rx.recv().await {
                if matches!(event, RuntimeEvent::SessionClosed { session_id: sid } if sid == session_id) {
                    found = true;
                    break;
                }
            }
        })
        .await
        .ok();
        assert!(found);
    }
}
