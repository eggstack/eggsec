//! Daemon persistence bridging (Phase D WS7).
//!
//! Cohesive module extracted from `host.rs`: fire-and-forget persistence
//! fan-out helpers only. No session lifecycle, RBAC, request handling, or
//! recovery orchestration here.
//!
//! Stable facade: `host.rs` re-exports the timeout const and helpers.

use crate::store::{DaemonStore, PersistedAuditEvent};

/// Upper bound for fire-and-forget persistence fan-out tasks so a stalled
/// store cannot leak long-lived tasks (project invariant: 30-300s timeouts).
pub const PERSISTENCE_TASK_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

/// Best-effort audit persistence for non-`&self` contexts (startup, helpers).
pub async fn record_audit_event_logged(store: &dyn DaemonStore, event: PersistedAuditEvent) {
    if let Err(error) = store.record_audit_event(&event).await {
        tracing::warn!(
            ?error,
            action = %event.action,
            "failed to persist daemon audit event"
        );
    }
}

/// Run a persistence future with a bounded timeout so stalled stores cannot
/// leak long-lived tasks.
pub async fn persistence_with_timeout(label: &str, fut: impl std::future::Future<Output = ()>) {
    if tokio::time::timeout(PERSISTENCE_TASK_TIMEOUT, fut)
        .await
        .is_err()
    {
        tracing::warn!(action = label, "daemon persistence task timed out");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn persistence_timeout_bounds_stalled_stores() {
        let completed = Arc::new(AtomicBool::new(false));
        let flag = completed.clone();
        persistence_with_timeout("test", async move {
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            flag.store(true, Ordering::SeqCst);
        })
        .await;
        assert!(completed.load(Ordering::SeqCst));
    }

    #[test]
    fn persistence_timeout_is_within_project_invariant() {
        assert!(PERSISTENCE_TASK_TIMEOUT.as_secs() >= 30);
        assert!(PERSISTENCE_TASK_TIMEOUT.as_secs() <= 300);
    }
}
