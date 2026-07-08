pub mod sqlite;

use async_trait::async_trait;

pub use sqlite::{NoopStore, SqliteStore};

use eggsec_runtime::{SessionId, SessionSnapshot};

/// Audit event recorded for security-relevant daemon actions.
#[derive(Debug, Clone)]
pub struct PersistedAuditEvent {
    pub action: String,
    pub surface: String,
    pub outcome: String,
    pub client_id: Option<String>,
    pub session_id: Option<String>,
    pub timestamp_secs: u64,
}

/// Trait for session snapshot persistence.
///
/// Implementations handle durable storage of session state at lifecycle
/// points (create, submit, cancel, close) and recovery on startup.
#[async_trait]
pub trait DaemonStore: Send + Sync + 'static {
    /// Persist a session snapshot, replacing any existing snapshot for this session.
    async fn save_session_snapshot(&self, snapshot: &SessionSnapshot) -> anyhow::Result<()>;

    /// Load a session snapshot by ID.
    async fn load_session_snapshot(
        &self,
        session_id: SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>>;

    /// Load all persisted session snapshots.
    async fn load_all_sessions(&self) -> anyhow::Result<Vec<SessionSnapshot>>;

    /// Record an audit event.
    async fn record_audit_event(&self, event: &PersistedAuditEvent) -> anyhow::Result<()>;

    /// Delete a session snapshot.
    async fn delete_session(&self, session_id: SessionId) -> anyhow::Result<()>;

    /// Blocking: list all persisted session summaries (for spawn_blocking).
    fn blocking_list_sessions(&self) -> anyhow::Result<Vec<eggsec_runtime::SessionSummary>>;

    /// Blocking: get a persisted snapshot by ID (for spawn_blocking).
    fn blocking_get_snapshot(
        &self,
        session_id: &SessionId,
    ) -> anyhow::Result<Option<SessionSnapshot>>;
}

/// Create a boxed NoopStore for testing.
pub fn noop_store() -> std::sync::Arc<dyn DaemonStore> {
    std::sync::Arc::new(NoopStore)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn noop_store_save_load() {
        let store = NoopStore;
        let snapshot = SessionSnapshot {
            session_id: SessionId::new(),
            surface: eggsec_runtime::RuntimeSurface::TuiManual,
            scope: None,
            created_at_epoch_secs: 0,
            generation: 0,
            active_tasks: vec![],
            completed_tasks: vec![],
            capabilities: eggsec_runtime::RuntimeCapabilities::default(),
            closed: false,
            closed_at: None,
        };
        store.save_session_snapshot(&snapshot).await.unwrap();
        let loaded = store
            .load_session_snapshot(snapshot.session_id)
            .await
            .unwrap();
        assert!(loaded.is_none());
    }

    #[tokio::test]
    async fn noop_store_load_all_empty() {
        let store = NoopStore;
        let sessions = store.load_all_sessions().await.unwrap();
        assert!(sessions.is_empty());
    }

    #[tokio::test]
    async fn noop_store_delete() {
        let store = NoopStore;
        store.delete_session(SessionId::new()).await.unwrap();
    }

    #[tokio::test]
    async fn noop_store_audit_event() {
        let store = NoopStore;
        let event = PersistedAuditEvent {
            action: "test".into(),
            surface: "test".into(),
            outcome: "allow".into(),
            client_id: None,
            session_id: None,
            timestamp_secs: 0,
        };
        store.record_audit_event(&event).await.unwrap();
    }

    #[test]
    fn noop_store_is_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<NoopStore>();
    }
}
