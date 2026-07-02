use std::future::Future;
use std::pin::Pin;

use eggsec_runtime::{
    RunRequest, RuntimeCapabilities, RuntimeEvent, SessionId, SessionSnapshot, SessionSummary,
    TaskId,
};

/// A future that resolves to a result.
pub type RuntimeClientFuture<T> = Pin<Box<dyn Future<Output = Result<T, String>> + Send + 'static>>;

/// Trait abstracting runtime backends for the TUI (embedded vs daemon).
pub trait TuiRuntimeClient: Send + Sync + 'static {
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

mod daemon;
mod embedded;

pub use daemon::DaemonRuntimeClient;
pub use embedded::EmbeddedRuntimeClient;
