//! Frontend-neutral runtime client contract for Eggsec's interactive frontends.
//!
//! This crate owns the small contract a frontend uses to drive the Eggsec
//! runtime, plus the two reference implementations:
//!
//! - [`EmbeddedRuntimeClient`] wraps an in-process `eggsec_runtime::Runtime`.
//! - [`DaemonRuntimeClient`] speaks the daemon's Unix-socket JSON-lines
//!   protocol.
//!
//! It exists so a second interactive frontend (a desktop GUI) can reuse the
//! exact contract the terminal UI uses today without depending on
//! `eggsec-tui` — and therefore without pulling in ratatui, crossterm, or any
//! other terminal-frontend dependency. The crate depends on no frontend
//! framework: only on `eggsec-runtime`, `eggsec-daemon-protocol`, and a
//! narrow Tokio feature set.
//!
//! It is **not** an enforcement surface. The contract carries no authorization
//! decisions of its own: the daemon applies its own access controls and the
//! embedded runtime applies the engine's, exactly as before the move.
//!
//! The trait is intentionally coarse — its error type is `String` — because
//! that is the contract the existing frontends already bind to.

use std::future::Future;
use std::pin::Pin;

use eggsec_runtime::{
    RunRequest, RuntimeCapabilities, RuntimeEvent, SessionId, SessionSnapshot, SessionSummary,
    TaskId,
};

/// Re-exported so a frontend can name its own client kind without taking a
/// direct dependency on the daemon protocol crate.
pub use eggsec_daemon_protocol::client_registry::ClientKind;

/// A future that resolves to a result.
pub type RuntimeClientFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>;

/// How a frontend identifies itself to the daemon.
///
/// The crate is frontend-neutral, so it cannot hardcode any one frontend's
/// identity: the kind recorded in the daemon's client registry, the label the
/// daemon shows in its listings, and the namespace for this client's request
/// ids are all the caller's to choose.
#[derive(Debug, Clone)]
pub struct ClientIdentity {
    /// The kind recorded in the daemon's client registry.
    pub kind: ClientKind,
    /// Human-readable label shown in daemon-side listings.
    pub label: String,
    /// Prefix for this client's request ids.
    pub request_prefix: String,
}

impl ClientIdentity {
    /// Build a request id in this client's namespace.
    ///
    /// The prefix is supplied verbatim, so a caller that wants a separator
    /// includes it in the prefix itself (`"gui-"` yields `gui-0`).
    pub fn request_id(&self, counter: u64) -> String {
        format!("{}{}", self.request_prefix, counter)
    }
}

/// Trait abstracting runtime backends for an interactive frontend (embedded vs daemon).
pub trait RuntimeClient: Send + Sync + 'static {
    /// Get runtime capabilities.
    fn capabilities(&self) -> RuntimeClientFuture<RuntimeCapabilities>;
    /// Create a new session.
    fn create_session(
        &self,
        surface: eggsec_runtime::request::RuntimeSurface,
        scope: Option<eggsec_runtime::session::SessionScope>,
        labels: Vec<String>,
    ) -> RuntimeClientFuture<SessionId>;
    /// List all sessions.
    fn list_sessions(&self) -> RuntimeClientFuture<Vec<SessionSummary>>;
    /// Get a snapshot of a session.
    fn snapshot(&self, session_id: SessionId) -> RuntimeClientFuture<SessionSnapshot>;
    /// Submit a task to a session.
    fn submit(&self, session_id: SessionId, request: RunRequest) -> RuntimeClientFuture<TaskId>;
    /// Cancel a specific task.
    fn cancel(&self, session_id: SessionId, task_id: TaskId) -> RuntimeClientFuture<()>;
    /// Cancel the active task in a session.
    fn cancel_active(&self, session_id: SessionId) -> RuntimeClientFuture<()>;
    /// Subscribe to runtime events for a session.
    fn subscribe(&self, session_id: SessionId) -> RuntimeClientFuture<RuntimeEventReceiverHandle>;
}

/// Handle for receiving runtime events from either embedded or daemon backend.
/// Wraps a tokio mpsc receiver.
pub struct RuntimeEventReceiverHandle {
    rx: tokio::sync::mpsc::UnboundedReceiver<RuntimeEvent>,
}

impl RuntimeEventReceiverHandle {
    pub fn new(rx: tokio::sync::mpsc::UnboundedReceiver<RuntimeEvent>) -> Self {
        Self { rx }
    }

    pub async fn recv(&mut self) -> Option<RuntimeEvent> {
        self.rx.recv().await
    }

    pub fn try_recv(&mut self) -> Option<RuntimeEvent> {
        self.rx.try_recv().ok()
    }
}

pub mod daemon;
pub mod embedded;

pub use daemon::{DaemonClientError, DaemonHealth, DaemonRuntimeClient};
pub use embedded::EmbeddedRuntimeClient;

#[cfg(test)]
mod tests {
    use std::future::Future;
    use std::pin::Pin;
    use std::sync::Arc;

    use eggsec_runtime::{
        CancellationToken, Runtime, RuntimeCapabilities, RuntimeConfig, RuntimeError, RuntimeEvent,
        RuntimeEventSink, RuntimeExecutionContext, RuntimeTaskExecutor, SessionId, TaskId,
        TaskOutcome,
    };

    use crate::{DaemonClientError, EmbeddedRuntimeClient, RuntimeClient};

    /// The crate declares no `macros` Tokio feature, so `#[tokio::test]` is
    /// unavailable. A current-thread runtime is enough for every assertion
    /// here.
    fn block_on<F: Future>(future: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("a current-thread runtime")
            .block_on(future)
    }

    struct NoopExecutor;

    impl RuntimeTaskExecutor for NoopExecutor {
        fn execute(
            &self,
            _task_id: TaskId,
            _request: eggsec_runtime::RunRequest,
            _context: RuntimeExecutionContext,
            _sink: RuntimeEventSink,
            _cancel: CancellationToken,
        ) -> Pin<Box<dyn Future<Output = Result<TaskOutcome, RuntimeError>> + Send + 'static>>
        {
            Box::pin(async { Ok(TaskOutcome::Text("ok".into())) })
        }
    }

    fn test_runtime() -> Arc<Runtime> {
        Arc::new(Runtime::new(RuntimeConfig::default(), NoopExecutor))
    }

    /// The contract must stay object-safe: every frontend holds the client as
    /// a trait object, so `Arc<dyn RuntimeClient + Send + Sync>` has to
    /// compile.
    #[test]
    fn runtime_client_is_object_safe() {
        let client: Arc<dyn RuntimeClient + Send + Sync> =
            Arc::new(EmbeddedRuntimeClient::new(test_runtime()));
        let capabilities = block_on(client.capabilities());
        assert!(capabilities.is_ok());
    }

    /// The embedded client drives the in-process runtime.
    #[test]
    fn embedded_client_creates_and_lists_sessions() {
        let client = EmbeddedRuntimeClient::new(test_runtime());
        let session_id = block_on(client.create_session(
            eggsec_runtime::request::RuntimeSurface::CliManual,
            None,
            vec![],
        ))
        .expect("a session must be created");
        let sessions = block_on(client.list_sessions()).expect("sessions must list");
        assert!(sessions
            .iter()
            .any(|summary| summary.session_id == session_id));
        let snapshot = block_on(client.snapshot(session_id)).expect("a snapshot must resolve");
        assert_eq!(snapshot.session_id, session_id);
    }

    /// An unknown session is an error, not a panic — the contract's error type
    /// is a `String`, so a frontend can surface it directly.
    #[test]
    fn embedded_client_reports_unknown_sessions_as_errors() {
        let client = EmbeddedRuntimeClient::new(test_runtime());
        let result = block_on(client.snapshot(SessionId::new()));
        assert!(result.is_err());
    }

    /// The event receiver handle is the frontend's view of the runtime's
    /// broadcast stream.
    #[test]
    fn event_receiver_handle_delivers_events() {
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        let mut handle = crate::RuntimeEventReceiverHandle::new(rx);
        let session_id = SessionId::new();
        tx.send(RuntimeEvent::SessionCreated { session_id })
            .expect("the event must be sent");
        match block_on(handle.recv()) {
            Some(RuntimeEvent::SessionCreated {
                session_id: received,
                ..
            }) => assert_eq!(received, session_id),
            other => panic!("expected SessionCreated, got {:?}", other),
        }
        assert!(handle.try_recv().is_none());
    }

    /// The daemon client's error type is part of the public contract: a
    /// frontend must be able to tell a closed connection from a protocol
    /// error.
    #[test]
    fn daemon_client_error_is_publicly_nameable() {
        let error = DaemonClientError::Protocol("unexpected reply".into());
        assert!(error.to_string().contains("unexpected reply"));
        let error = DaemonClientError::ConnectionClosed;
        assert!(!error.to_string().is_empty());
    }

    /// The capabilities the embedded client reports are the runtime's own.
    #[test]
    fn embedded_client_reports_runtime_capabilities() {
        let client = EmbeddedRuntimeClient::new(test_runtime());
        let capabilities: RuntimeCapabilities =
            block_on(client.capabilities()).expect("capabilities must resolve");
        assert!(capabilities.supports_cancellation);
        assert!(capabilities.supports_multiple_sessions);
    }

    /// The request-id namespace belongs to the caller. This crate is
    /// frontend-neutral, so it must not stamp one frontend's prefix onto
    /// every client that connects through it.
    #[test]
    fn client_identity_namespaces_request_ids() {
        let tui = crate::ClientIdentity {
            kind: crate::ClientKind::Tui,
            label: "eggsec-tui".to_owned(),
            request_prefix: "tui-".to_owned(),
        };
        assert_eq!(tui.request_id(0), "tui-0");
        assert_eq!(tui.request_id(7), "tui-7");

        let gui = crate::ClientIdentity {
            kind: crate::ClientKind::Gui,
            label: "eggsec-gui".to_owned(),
            request_prefix: "gui-".to_owned(),
        };
        assert_eq!(gui.request_id(0), "gui-0");
        assert_eq!(gui.request_id(7), "gui-7");
    }

    /// The identity carries the kind and the label the daemon records for the
    /// client, so two frontends sharing this crate remain distinguishable in
    /// the daemon's registry.
    #[test]
    fn client_identity_carries_kind_and_label() {
        let identity = crate::ClientIdentity {
            kind: crate::ClientKind::Gui,
            label: "eggsec-gui".to_owned(),
            request_prefix: "gui-".to_owned(),
        };
        assert_eq!(identity.kind, crate::ClientKind::Gui);
        assert_eq!(identity.label, "eggsec-gui");
    }

    /// The crate must not hardcode any one frontend's identity: a TUI-shaped
    /// literal anywhere in the non-test source is a regression toward the
    /// frontend-specific behavior this crate was extracted to remove.
    #[test]
    fn crate_carries_no_hardcoded_frontend_identity() {
        let source = include_str!("daemon.rs");
        // Strip the test module before scanning, so the assertions above can
        // name the TUI's literals.
        let production = source
            .split("#[cfg(test)]")
            .next()
            .expect("the module has a production half");
        for forbidden in ["tui-", "ClientKind::Tui", "eggsec-tui"] {
            assert!(
                !production.contains(forbidden),
                "the daemon client must not hardcode {forbidden}"
            );
        }
    }
}
