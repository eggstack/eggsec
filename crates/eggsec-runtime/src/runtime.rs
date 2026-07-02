use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{broadcast, Mutex};
use tokio_util::sync::CancellationToken;

use crate::event::{RuntimeErrorInfo, RuntimeEvent, TaskOutcome, TaskProgress, TaskStatus};
use crate::ids::{SessionId, TaskId};
use crate::request::{RunRequest, RuntimeSurface};
use crate::session::{RuntimeSession, SessionSnapshot};
use crate::RuntimeError;

/// Configuration for the runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Default timeout for tasks. None means no timeout.
    pub default_task_timeout: Option<Duration>,
    /// Maximum active tasks per session.
    pub max_active_tasks_per_session: usize,
    /// Capacity of the event broadcast channel.
    pub event_channel_capacity: usize,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_task_timeout: Some(Duration::from_secs(300)),
            max_active_tasks_per_session: 1,
            event_channel_capacity: 256,
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
    /// Receive the next event. Returns `None` if the channel is closed.
    pub async fn recv(&mut self) -> Option<RuntimeEvent> {
        self.rx.recv().await.ok()
    }

    /// Try to receive an event without blocking.
    pub fn try_recv(&mut self) -> Option<RuntimeEvent> {
        self.rx.try_recv().ok()
    }
}

/// Sink for task executors to report progress and completion.
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

    /// Emit a progress event.
    pub fn progress(&self, completed: u64, total: Option<u64>, message: Option<String>) {
        let _ = self.event_tx.send(RuntimeEvent::TaskProgress {
            session_id: self.session_id,
            task_id: self.task_id,
            progress: TaskProgress {
                completed,
                total,
                message,
            },
        });
    }

    /// Emit a log event.
    pub fn log(&self, level: crate::event::LogLevel, message: String) {
        let _ = self.event_tx.send(RuntimeEvent::TaskLog {
            session_id: self.session_id,
            task_id: Some(self.task_id),
            level,
            message,
        });
    }

    /// Emit a completion event.
    pub fn completed(&self, outcome: TaskOutcome) {
        let _ = self.event_tx.send(RuntimeEvent::TaskCompleted {
            session_id: self.session_id,
            task_id: self.task_id,
            outcome,
        });
    }

    /// Emit a failure event.
    pub fn failed(&self, message: String, code: Option<String>) {
        let _ = self.event_tx.send(RuntimeEvent::TaskFailed {
            session_id: self.session_id,
            task_id: self.task_id,
            error: RuntimeErrorInfo {
                message,
                code,
                details: None,
            },
        });
    }
}

/// Trait for task executors. Implementations bridge the runtime to actual tool
/// execution. In Phase 2 the TUI provides an executor that wraps the existing
/// worker path; in Phase 3 the executor moves into the runtime.
pub trait RuntimeTaskExecutor: Send + Sync + 'static {
    /// Execute a task. The executor should report progress via `sink` and
    /// return an outcome on success. The `cancel` token is triggered when the
    /// task is cancelled or times out.
    fn execute(
        &self,
        task_id: TaskId,
        request: RunRequest,
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
        _options: SessionOptions,
        surface: RuntimeSurface,
    ) -> Result<SessionId, RuntimeError> {
        let session_id = SessionId::new();
        let mut state = self.state.lock().await;
        state
            .sessions
            .insert(session_id, RuntimeSession::new(session_id, surface));
        let _ = state
            .event_tx
            .send(RuntimeEvent::SessionCreated { session_id });
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

    /// Submit a task to a session. Returns the task ID.
    pub async fn submit(
        &self,
        session_id: SessionId,
        request: RunRequest,
    ) -> Result<TaskId, RuntimeError> {
        let task_id = TaskId::new();

        let (cancel_token, timeout_duration, executor_clone, event_tx_clone) = {
            let mut state = self.state.lock().await;

            // Validate session exists
            if !state.sessions.contains_key(&session_id) {
                return Err(RuntimeError::SessionNotFound(session_id.to_string()));
            }

            // Single-active-task policy: abort any existing active task
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
                if let Some(mut old) = state
                    .sessions
                    .get_mut(&session_id)
                    .unwrap()
                    .tasks
                    .remove(&existing_id)
                {
                    if let Some(cancel) = old.abort.take() {
                        cancel.cancel();
                    }
                    old.status = TaskStatus::Cancelled;
                    let _ = state.event_tx.send(RuntimeEvent::TaskCancelled {
                        session_id,
                        task_id: existing_id,
                        reason: Some("replaced by new task".into()),
                    });
                }
            }

            let cancel_token = CancellationToken::new();
            let timeout_duration = state.config.default_task_timeout;

            state.sessions.get_mut(&session_id).unwrap().tasks.insert(
                task_id,
                crate::session::TaskRecord {
                    request: request.clone(),
                    status: TaskStatus::Queued,
                    progress: None,
                    last_error: None,
                    abort: Some(cancel_token.clone()),
                    _handle: None,
                },
            );

            let _ = state.event_tx.send(RuntimeEvent::TaskQueued {
                session_id,
                task_id,
                request: request.clone(),
            });

            (
                cancel_token,
                timeout_duration,
                state.executor.clone(),
                state.event_tx.clone(),
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
                // Signal TaskStarted
                {
                    let mut state = state_arc.lock().await;
                    if let Some(session) = state.sessions.get_mut(&session_id) {
                        if let Some(task) = session.tasks.get_mut(&task_id) {
                            task.status = TaskStatus::Running;
                        }
                    }
                }
                let _ = event_tx_clone.send(RuntimeEvent::TaskStarted {
                    session_id,
                    task_id,
                });

                // Execute with timeout
                let result = if let Some(timeout) = timeout_duration {
                    tokio::time::timeout(
                        timeout,
                        executor_clone.execute(
                            task_id,
                            request_for_spawn,
                            sink,
                            cancel_for_spawn.clone(),
                        ),
                    )
                    .await
                } else {
                    Ok(executor_clone
                        .execute(task_id, request_for_spawn, sink, cancel_for_spawn.clone())
                        .await)
                };

                // Determine final status
                let (final_status, error_msg) = match result {
                    Ok(Ok(outcome)) => {
                        let _ = event_tx_clone.send(RuntimeEvent::TaskCompleted {
                            session_id,
                            task_id,
                            outcome,
                        });
                        (TaskStatus::Completed, None)
                    }
                    Ok(Err(e)) => {
                        let msg = e.to_string();
                        let _ = event_tx_clone.send(RuntimeEvent::TaskFailed {
                            session_id,
                            task_id,
                            error: RuntimeErrorInfo {
                                message: msg.clone(),
                                code: None,
                                details: None,
                            },
                        });
                        (TaskStatus::Failed, Some(msg))
                    }
                    Err(_elapsed) => {
                        cancel_for_spawn.cancel();
                        let _ = event_tx_clone.send(RuntimeEvent::TaskCancelled {
                            session_id,
                            task_id,
                            reason: Some("timed out".into()),
                        });
                        (TaskStatus::TimedOut, Some("task timed out".into()))
                    }
                };

                // Update task record
                let mut state = state_arc.lock().await;
                if let Some(session) = state.sessions.get_mut(&session_id) {
                    if let Some(task) = session.tasks.get_mut(&task_id) {
                        task.status = final_status;
                        task.last_error = error_msg;
                        task.abort = None;
                        task._handle = None;
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

        let _ = state.event_tx.send(RuntimeEvent::TaskCancelled {
            session_id,
            task_id,
            reason: Some("cancelled by user".into()),
        });

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
                let _ = state.event_tx.send(RuntimeEvent::TaskCancelled {
                    session_id,
                    task_id,
                    reason: Some("cancelled by user".into()),
                });
            }
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
        let _task1 = runtime.submit(session_id, test_request()).await.unwrap();
        let task2 = runtime.submit(session_id, test_request()).await.unwrap();

        tokio::time::sleep(Duration::from_millis(10)).await;

        // task1 should have been cancelled by task2 submission
        let snapshot = runtime.snapshot(session_id).await.unwrap();
        // task2 is the only active one
        assert_eq!(snapshot.active_tasks.len(), 1);
        assert_eq!(snapshot.active_tasks[0].task_id, task2);
    }

    #[tokio::test]
    async fn session_not_found() {
        let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
        let fake = SessionId::new();
        let result = runtime.submit(fake, test_request()).await;
        assert!(result.is_err());
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
}
