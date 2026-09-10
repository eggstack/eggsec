//! Shared cancellation primitive for runtime task adapters.
//!
//! Both the embedded TUI adapter (`TuiExecutor`) and the daemon-backed engine
//! adapter (`EggsecRuntimeExecutor`) race their dispatch future against the
//! runtime cancellation token through [`race_with_cancel`]. Sharing one
//! primitive guarantees equivalent cancellation semantics: pre-cancelled
//! tasks never start detached work, mid-execution cancellation drops the
//! dispatch future (cooperative async cancellation), and channel senders owned
//! by the future are released so progress forwarders drain instead of leaking.

use tokio_util::sync::CancellationToken;

use crate::error::RuntimeError;

/// Race a dispatch future against the runtime cancellation token.
///
/// - If `cancel` is already cancelled (or fires first), returns
///   `Err(RuntimeError::DispatchFailed("task cancelled..."))` without
///   awaiting the dispatch future to completion. Dropping the future cancels
///   cooperative async work; senders owned by the future are released so
///   forwarders observe channel closure instead of surviving detached.
/// - Otherwise returns the dispatch future's output unchanged.
///
/// Adapters must route ALL task execution through this helper (no ad hoc
/// `tokio::select!` on `cancel.cancelled()` in adapter code) so embedded and
/// daemon-backed paths stay equivalent by construction.
pub async fn race_with_cancel<T>(
    dispatch: impl std::future::Future<Output = Result<T, RuntimeError>> + Send,
    cancel: CancellationToken,
) -> Result<T, RuntimeError> {
    // Fast path: pre-cancelled tasks never start detached work.
    if cancel.is_cancelled() {
        return Err(RuntimeError::DispatchFailed("task cancelled".into()));
    }
    tokio::select! {
        result = dispatch => result,
        _ = cancel.cancelled() => {
            Err(RuntimeError::DispatchFailed(
                "task cancelled during execution".into(),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn pre_cancelled_never_starts_work() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let started = Arc::new(AtomicBool::new(false));
        let started_clone = started.clone();
        let cancel = CancellationToken::new();
        cancel.cancel();

        let result = race_with_cancel(
            async move {
                started_clone.store(true, Ordering::SeqCst);
                Ok::<_, RuntimeError>("should not run")
            },
            cancel,
        )
        .await;
        assert!(matches!(result, Err(RuntimeError::DispatchFailed(_))));
        // The fast path returns before polling the future: no work started.
        assert!(!started.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn cancel_drops_future_releasing_senders() {
        // The dispatch future owns a sender; when cancellation wins, the
        // future is dropped, the sender releases, and the receiver observes
        // closure — proving no detached task survives holding the channel.
        let (tx, mut rx) = tokio::sync::mpsc::channel::<u32>(4);
        let cancel = CancellationToken::new();
        let cancel_clone = cancel.clone();

        let dispatch = async move {
            // Hold the sender across a pending point.
            let _held = tx;
            std::future::pending::<()>().await;
            #[allow(unreachable_code)]
            Ok::<_, RuntimeError>("never")
        };
        // Cancel shortly after the dispatch future starts.
        tokio::spawn(async move {
            tokio::task::yield_now().await;
            cancel_clone.cancel();
        });
        let result = race_with_cancel(dispatch, cancel).await;
        assert!(matches!(result, Err(RuntimeError::DispatchFailed(_))));
        // Sender released by drop: receiver observes closure, not a leak.
        assert!(
            tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
                .await
                .map(|v| v.is_none())
                .unwrap_or(false),
            "channel must close after cancellation (no detached sender)"
        );
    }

    #[tokio::test]
    async fn ready_result_passes_through_unchanged() {
        let cancel = CancellationToken::new();
        let result = race_with_cancel(async { Ok::<_, RuntimeError>(42u32) }, cancel).await;
        assert_eq!(result.unwrap(), 42u32);
    }
}
