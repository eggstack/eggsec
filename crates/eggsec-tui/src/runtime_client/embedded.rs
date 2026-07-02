use std::sync::Arc;

use eggsec_runtime::{
    request::RuntimeSurface, session::SessionScope, RunRequest, Runtime, RuntimeCapabilities,
    SessionId, SessionSnapshot, SessionSummary, TaskId,
};

use super::{RuntimeClientFuture, RuntimeEventReceiverHandle, TuiRuntimeClient};

/// Runtime client that wraps an in-process `Runtime` directly.
pub struct EmbeddedRuntimeClient {
    runtime: Arc<Runtime>,
}

impl EmbeddedRuntimeClient {
    pub fn new(runtime: Arc<Runtime>) -> Self {
        Self { runtime }
    }

    pub fn runtime(&self) -> &Arc<Runtime> {
        &self.runtime
    }
}

impl TuiRuntimeClient for EmbeddedRuntimeClient {
    fn capabilities(&self) -> RuntimeClientFuture<RuntimeCapabilities> {
        Box::pin(async move { Ok(RuntimeCapabilities::default()) })
    }

    fn create_session(
        &self,
        surface: RuntimeSurface,
        scope: Option<SessionScope>,
        _labels: Vec<String>,
    ) -> RuntimeClientFuture<SessionId> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .create_session_with_scope(
                    eggsec_runtime::SessionOptions::default(),
                    surface,
                    scope,
                )
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn list_sessions(&self) -> RuntimeClientFuture<Vec<SessionSummary>> {
        let runtime = self.runtime.clone();
        Box::pin(async move { Ok(runtime.list_sessions().await) })
    }

    fn snapshot(&self, session_id: SessionId) -> RuntimeClientFuture<SessionSnapshot> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .snapshot(session_id)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn submit(&self, session_id: SessionId, request: RunRequest) -> RuntimeClientFuture<TaskId> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .submit(session_id, request)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn cancel(&self, session_id: SessionId, task_id: TaskId) -> RuntimeClientFuture<()> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .cancel(session_id, task_id)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn cancel_active(&self, session_id: SessionId) -> RuntimeClientFuture<()> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            runtime
                .cancel_active(session_id)
                .await
                .map_err(|e| e.to_string())
        })
    }

    fn subscribe(&self, _session_id: SessionId) -> RuntimeClientFuture<RuntimeEventReceiverHandle> {
        let runtime = self.runtime.clone();
        Box::pin(async move {
            let mut broadcast_rx = runtime.subscribe().await;
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            tokio::spawn(async move {
                while let Some(event) = broadcast_rx.recv().await {
                    if tx.send(event).is_err() {
                        break;
                    }
                }
            });
            Ok(RuntimeEventReceiverHandle::new(rx))
        })
    }
}
