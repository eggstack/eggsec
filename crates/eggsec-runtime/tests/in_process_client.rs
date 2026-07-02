//! In-process client contract tests for `eggsec-runtime`.
//!
//! These tests prove that the runtime can be used independently of the TUI:
//! - Create runtime with a test executor
//! - Create sessions bound to various surfaces
//! - Submit tasks and observe lifecycle events
//! - Cancel tasks and verify terminal status
//! - Inspect snapshots for active/completed tasks
//! - Bind session scope and verify snapshot scope
//!
//! No `eggsec-tui` dependency is required.

use std::sync::Arc;
use std::time::Duration;

use eggsec_runtime::event::{LogLevel, RuntimeEvent, TaskOutcome, TaskStatus};
use eggsec_runtime::ids::TaskId;
use eggsec_runtime::request::{PortScanParams, RunRequest, RuntimeSurface, TaskKind};
use eggsec_runtime::runtime::{Runtime, RuntimeConfig, RuntimeEventSink, RuntimeTaskExecutor};
use eggsec_runtime::session::SessionScope;
use eggsec_runtime::{RuntimeError, SessionId, SessionOptions};

/// A test executor that completes immediately with a structured result.
struct ImmediateExecutor;

impl RuntimeTaskExecutor for ImmediateExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        _request: RunRequest,
        sink: RuntimeEventSink,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        Box::pin(async move {
            sink.log(LogLevel::Info, "task started".into());
            sink.progress(50, Some(100), Some("halfway".into()));
            sink.progress(100, Some(100), None);
            Ok(TaskOutcome::Result(eggsec_runtime::TaskResultEnvelope {
                kind: "port-scan".into(),
                summary: Some("42 ports scanned".into()),
                payload: serde_json::json!({}),
                artifacts: vec![],
            }))
        })
    }
}

/// A test executor that sleeps until cancelled.
struct SlowExecutor;

impl RuntimeTaskExecutor for SlowExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        _request: RunRequest,
        _sink: RuntimeEventSink,
        cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
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
        _cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        Box::pin(async { Err(RuntimeError::UnsupportedTaskKind) })
    }
}

/// A test executor that emits progress and produces a text outcome.
struct TextOutcomeExecutor;

impl RuntimeTaskExecutor for TextOutcomeExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        _request: RunRequest,
        sink: RuntimeEventSink,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        Box::pin(async move {
            sink.progress(10, Some(10), Some("done".into()));
            Ok(TaskOutcome::Text("scan complete".into()))
        })
    }
}

/// A test executor that emits a JSON outcome.
struct JsonOutcomeExecutor;

impl RuntimeTaskExecutor for JsonOutcomeExecutor {
    fn execute(
        &self,
        _task_id: TaskId,
        _request: RunRequest,
        _sink: RuntimeEventSink,
        _cancel: tokio_util::sync::CancellationToken,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>,
    > {
        Box::pin(async {
            Ok(TaskOutcome::Json(serde_json::json!({
                "host": "10.0.0.1",
                "open_ports": [80, 443]
            })))
        })
    }
}

fn port_scan_request() -> RunRequest {
    RunRequest {
        task_kind: TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: Some("80,443".into()),
            scan_type: Some("syn".into()),
            timeout_ms: Some(5000),
        }),
        requested_by: None,
        surface: RuntimeSurface::CliManual,
        labels: vec!["test".into()],
    }
}

fn endpoint_scan_request() -> RunRequest {
    RunRequest {
        task_kind: TaskKind::EndpointScan(eggsec_runtime::request::EndpointScanParams {
            target: "http://api.example.com".into(),
            methods: Some(vec!["GET".into(), "POST".into()]),
            wordlist: None,
        }),
        requested_by: None,
        surface: RuntimeSurface::McpServer,
        labels: vec![],
    }
}

// ---------------------------------------------------------------------------
// Contract tests: runtime works without TUI
// ---------------------------------------------------------------------------

#[tokio::test]
async fn create_runtime_and_session() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let surface = runtime.session_surface(session_id).await.unwrap();
    assert_eq!(surface, RuntimeSurface::CliManual);
}

#[tokio::test]
async fn create_session_with_mcp_surface() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::McpServer)
        .await
        .unwrap();

    let surface = runtime.session_surface(session_id).await.unwrap();
    assert_eq!(surface, RuntimeSurface::McpServer);
}

#[tokio::test]
async fn create_session_with_rest_surface() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::RestApi)
        .await
        .unwrap();

    let surface = runtime.session_surface(session_id).await.unwrap();
    assert_eq!(surface, RuntimeSurface::RestApi);
}

#[tokio::test]
async fn create_session_with_agent_surface() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::SecurityAgent)
        .await
        .unwrap();

    let surface = runtime.session_surface(session_id).await.unwrap();
    assert_eq!(surface, RuntimeSurface::SecurityAgent);
}

#[tokio::test]
async fn bind_scope_and_verify_snapshot() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let scope = SessionScope {
        is_explicit: true,
        source: "config".into(),
        path: Some("/etc/eggsec/scope.txt".into()),
    };
    let session_id = runtime
        .create_session_with_scope(
            SessionOptions::default(),
            RuntimeSurface::CliManual,
            Some(scope),
        )
        .await
        .unwrap();

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert!(snapshot.scope.is_some());
    let s = snapshot.scope.unwrap();
    assert!(s.is_explicit);
    assert_eq!(s.source, "config");
    assert_eq!(s.path.as_deref(), Some("/etc/eggsec/scope.txt"));
}

#[tokio::test]
async fn submit_task_and_observe_lifecycle_events() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    // Collect events until TaskCompleted
    let mut events = Vec::new();
    tokio::time::timeout(Duration::from_secs(2), async {
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
    .unwrap();

    // Verify full lifecycle
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

    // Verify terminal event has the task ID
    let completed = events
        .iter()
        .find(|e| matches!(e, RuntimeEvent::TaskCompleted { .. }));
    if let Some(RuntimeEvent::TaskCompleted { task_id: tid, .. }) = completed {
        assert_eq!(*tid, task_id);
    } else {
        panic!("Expected TaskCompleted event");
    }
}

#[tokio::test]
async fn submit_task_and_observe_result_envelope() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    // Wait for completion
    let mut outcome = None;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = rx.recv().await {
                if let RuntimeEvent::TaskCompleted { outcome: o, .. } = event {
                    outcome = Some(o);
                    break;
                }
            }
        }
    })
    .await
    .unwrap();

    let outcome = outcome.expect("Expected TaskCompleted event with outcome");
    match outcome {
        TaskOutcome::Result(envelope) => {
            assert_eq!(envelope.kind, "port-scan");
            assert!(envelope.summary.is_some());
            assert_eq!(envelope.payload, serde_json::json!({}));
            assert!(envelope.artifacts.is_empty());
        }
        other => panic!("Expected TaskOutcome::Result, got: {:?}", other),
    }
}

#[tokio::test]
async fn cancel_task_produces_cancelled_event() {
    let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    // Wait for task to start
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Some(RuntimeEvent::TaskStarted { .. }) = rx.recv().await {
                break;
            }
        }
    })
    .await
    .unwrap();

    // Cancel
    runtime.cancel(session_id, task_id).await.unwrap();

    // Wait for cancellation event
    let mut cancelled = false;
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Some(event) = rx.recv().await {
                if let RuntimeEvent::TaskCancelled { .. } = event {
                    cancelled = true;
                    break;
                }
            }
        }
    })
    .await
    .unwrap();

    assert!(cancelled, "Expected TaskCancelled event");

    // Verify snapshot shows cancelled status
    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert!(snapshot.active_tasks.is_empty());
    assert_eq!(snapshot.completed_tasks.len(), 1);
    assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn snapshot_shows_active_and_completed_tasks() {
    let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert_eq!(snapshot.active_tasks.len(), 1);
    assert_eq!(snapshot.active_tasks[0].task_id, task_id);
    assert_eq!(snapshot.active_tasks[0].status, TaskStatus::Running);
    assert!(snapshot.completed_tasks.is_empty());
}

#[tokio::test]
async fn failed_task_reports_error() {
    let runtime = Runtime::new(RuntimeConfig::default(), FailingExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    // Wait for failure event
    let mut error_message = None;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(event) = rx.recv().await {
                if let RuntimeEvent::TaskFailed { error, .. } = event {
                    error_message = Some(error.message);
                    break;
                }
            }
        }
    })
    .await
    .unwrap();

    assert!(error_message.is_some());
    assert!(error_message.unwrap().contains("unsupported task kind"));

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert!(snapshot.active_tasks.is_empty());
    assert_eq!(snapshot.completed_tasks.len(), 1);
    assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Failed);
    assert!(snapshot.completed_tasks[0].last_error.is_some());
}

#[tokio::test]
async fn task_timeout_produces_timed_out_status() {
    let config = RuntimeConfig {
        default_task_timeout: Some(Duration::from_millis(50)),
        ..Default::default()
    };
    let runtime = Runtime::new(config, SlowExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    // Wait for timeout
    tokio::time::sleep(Duration::from_millis(200)).await;

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert!(snapshot.active_tasks.is_empty());
    assert_eq!(snapshot.completed_tasks.len(), 1);
    assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::TimedOut);
}

#[tokio::test]
async fn multiple_sessions_are_independent() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);

    let s1 = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();
    let s2 = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::McpServer)
        .await
        .unwrap();
    let s3 = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::SecurityAgent)
        .await
        .unwrap();

    runtime.submit(s1, port_scan_request()).await.unwrap();
    runtime.submit(s2, endpoint_scan_request()).await.unwrap();
    runtime.submit(s3, port_scan_request()).await.unwrap();

    tokio::time::sleep(Duration::from_millis(50)).await;

    let snap1 = runtime.snapshot(s1).await.unwrap();
    let snap2 = runtime.snapshot(s2).await.unwrap();
    let snap3 = runtime.snapshot(s3).await.unwrap();

    assert_eq!(snap1.surface, RuntimeSurface::CliManual);
    assert_eq!(snap2.surface, RuntimeSurface::McpServer);
    assert_eq!(snap3.surface, RuntimeSurface::SecurityAgent);

    assert_eq!(snap1.completed_tasks.len(), 1);
    assert_eq!(snap2.completed_tasks.len(), 1);
    assert_eq!(snap3.completed_tasks.len(), 1);
}

#[tokio::test]
async fn new_task_replaces_existing_active_task() {
    let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task1 = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    let task2 = runtime
        .submit(session_id, endpoint_scan_request())
        .await
        .unwrap();

    // Wait for cancellation of task1
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if let Some(RuntimeEvent::TaskCancelled { .. }) = rx.recv().await {
                break;
            }
        }
    })
    .await
    .unwrap();

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert_eq!(snapshot.active_tasks.len(), 1);
    assert_eq!(snapshot.active_tasks[0].task_id, task2);
}

#[tokio::test]
async fn submit_to_nonexistent_session_errors() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let fake = SessionId::new();
    let result = runtime.submit(fake, port_scan_request()).await;
    assert!(result.is_err());
    match result {
        Err(RuntimeError::SessionNotFound(_)) => {}
        other => panic!("Expected SessionNotFound, got: {:?}", other),
    }
}

#[tokio::test]
async fn cancel_completed_task_errors() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let result = runtime.cancel(session_id, task_id).await;
    assert!(result.is_err());
}

#[tokio::test]
async fn cancel_active_noop_when_no_tasks() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    // Should succeed without error
    runtime.cancel_active(session_id).await.unwrap();

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert!(snapshot.active_tasks.is_empty());
    assert!(snapshot.completed_tasks.is_empty());
}

#[tokio::test]
async fn cancel_active_cancels_running_task() {
    let runtime = Runtime::new(RuntimeConfig::default(), SlowExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;

    runtime.cancel_active(session_id).await.unwrap();

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    assert!(snapshot.active_tasks.is_empty());
    assert_eq!(snapshot.completed_tasks.len(), 1);
    assert_eq!(snapshot.completed_tasks[0].status, TaskStatus::Cancelled);
}

#[tokio::test]
async fn text_outcome_executor_delivers_text_outcome() {
    let runtime = Runtime::new(RuntimeConfig::default(), TextOutcomeExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    let mut outcome = None;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(RuntimeEvent::TaskCompleted { outcome: o, .. }) = rx.recv().await {
                outcome = Some(o);
                break;
            }
        }
    })
    .await
    .unwrap();

    match outcome.unwrap() {
        TaskOutcome::Text(text) => assert_eq!(text, "scan complete"),
        other => panic!("Expected Text outcome, got: {:?}", other),
    }
}

#[tokio::test]
async fn json_outcome_executor_delivers_json_outcome() {
    let runtime = Runtime::new(RuntimeConfig::default(), JsonOutcomeExecutor);
    let mut rx = runtime.subscribe().await;
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();

    let mut outcome = None;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Some(RuntimeEvent::TaskCompleted { outcome: o, .. }) = rx.recv().await {
                outcome = Some(o);
                break;
            }
        }
    })
    .await
    .unwrap();

    match outcome.unwrap() {
        TaskOutcome::Json(value) => {
            assert_eq!(value["host"], "10.0.0.1");
            assert!(value["open_ports"].is_array());
        }
        other => panic!("Expected Json outcome, got: {:?}", other),
    }
}

#[tokio::test]
async fn session_snapshot_serializes() {
    let runtime = Runtime::new(RuntimeConfig::default(), ImmediateExecutor);
    let session_id = runtime
        .create_session(SessionOptions::default(), RuntimeSurface::CliManual)
        .await
        .unwrap();

    let _task_id = runtime
        .submit(session_id, port_scan_request())
        .await
        .unwrap();
    tokio::time::sleep(Duration::from_millis(50)).await;

    let snapshot = runtime.snapshot(session_id).await.unwrap();
    let json = serde_json::to_string(&snapshot).unwrap();
    let deserialized: eggsec_runtime::SessionSnapshot = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.session_id, snapshot.session_id);
    assert_eq!(deserialized.surface, RuntimeSurface::CliManual);
    assert_eq!(deserialized.completed_tasks.len(), 1);
}

#[tokio::test]
async fn capabilities_reflect_current_build() {
    let caps = eggsec_runtime::RuntimeCapabilities::default();

    // Only in-process transport is supported
    assert_eq!(caps.transports, vec!["in-process"]);

    // Single active task policy
    assert!(!caps.supports_multiple_active_tasks);

    // Multiple sessions supported
    assert!(caps.supports_multiple_sessions);

    // Cancellation supported
    assert!(caps.supports_cancellation);

    // All 29 task kinds are advertised
    assert_eq!(caps.task_kinds.len(), 29);
}

#[tokio::test]
async fn capabilities_serialize_for_daemon_client() {
    let caps = eggsec_runtime::RuntimeCapabilities::default();
    let json = serde_json::to_string_pretty(&caps).unwrap();

    // Verify it's valid JSON and can be deserialized back
    let deserialized: eggsec_runtime::RuntimeCapabilities = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.task_kinds.len(), caps.task_kinds.len());
    assert_eq!(deserialized.transports, caps.transports);
}
