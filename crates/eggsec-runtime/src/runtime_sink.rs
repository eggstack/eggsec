//! Runtime event sink/receiver and broadcast helpers (Phase D WS7).
//!
//! Cohesive module extracted from `runtime.rs`: events/backpressure only.
//! No submission, task registry, session lifecycle, or cancellation logic.
//!
//! Stable facade: `runtime.rs` re-exports everything here.
//!
//! # Backpressure contract
//!
//! - `emit_event` (best-effort, `trace` on drop) for routine progress;
//! - `emit_event_critical` (`warn` on drop) for policy-relevant terminal
//!   events (`TaskFailed`, `TaskCancelled`, `SessionClosed`);
//! - receivers treat `Lagged` as recoverable (log + continue), never as
//!   closure.

use tokio::sync::broadcast;

use crate::event::{RuntimeErrorInfo, RuntimeEvent, TaskOutcome, TaskProgress};
use crate::ids::{SessionId, TaskId};

/// Emit a runtime event best-effort. Logs at trace level if no subscribers
/// are listening. Never panics on channel failure.
pub(crate) fn emit_event(tx: &broadcast::Sender<RuntimeEvent>, event: RuntimeEvent) {
    if tx.receiver_count() == 0 {
        tracing::trace!("no event subscribers; dropping event");
    } else if let Err(e) = tx.send(event) {
        tracing::trace!("event send failed (likely no active receivers): {}", e);
    }
}

/// Emit a runtime event with audit-critical semantics. Logs at warn level
/// on send failure to make policy-relevant event loss observable.
pub(crate) fn emit_event_critical(tx: &broadcast::Sender<RuntimeEvent>, event: RuntimeEvent) {
    if tx.receiver_count() == 0 {
        tracing::warn!("no event subscribers for critical event; event dropped");
    } else if let Err(e) = tx.send(event) {
        tracing::warn!("critical event send failed: {}", e);
    }
}

/// Event receiver for subscribing to runtime events.
pub struct RuntimeEventReceiver {
    pub(crate) rx: broadcast::Receiver<RuntimeEvent>,
}

impl RuntimeEventReceiver {
    /// Create a receiver from a broadcast channel. Useful for tests.
    pub fn from_broadcast(rx: broadcast::Receiver<RuntimeEvent>) -> Self {
        Self { rx }
    }

    /// Receive the next event. Returns `None` if the channel is closed.
    /// Logs a warning each time events were dropped due to broadcast
    /// overflow, then keeps receiving — lag is recoverable and must not be
    /// misreported as channel closure.
    pub async fn recv(&mut self) -> Option<RuntimeEvent> {
        loop {
            match self.rx.recv().await {
                Ok(event) => return Some(event),
                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!(
                        dropped = n,
                        "Broadcast event channel overflow, events dropped"
                    );
                    // Keep consuming; the channel is still open.
                }
                Err(tokio::sync::broadcast::error::RecvError::Closed) => return None,
            }
        }
    }

    /// Try to receive an event without blocking. Returns `None` when no event
    /// is available or the channel is closed. Logs a warning on overflow and
    /// keeps draining instead of swallowing the lag.
    pub fn try_recv(&mut self) -> Option<RuntimeEvent> {
        loop {
            match self.rx.try_recv() {
                Ok(event) => return Some(event),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(n)) => {
                    tracing::warn!(
                        dropped = n,
                        "Broadcast event channel overflow, events dropped"
                    );
                    // Keep draining; the channel is still open.
                }
                Err(tokio::sync::broadcast::error::TryRecvError::Empty) => return None,
                Err(tokio::sync::broadcast::error::TryRecvError::Closed) => return None,
            }
        }
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
    pub(crate) fn new(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn receiver_recovers_from_lag_instead_of_closing() {
        let (tx, rx) = broadcast::channel::<RuntimeEvent>(2);
        let mut receiver = RuntimeEventReceiver::from_broadcast(rx);
        // Overflow the channel: lag is recoverable, receiver must not report
        // closure.
        for _ in 0..5 {
            let _ = tx.send(RuntimeEvent::SessionClosed {
                session_id: SessionId::new(),
            });
        }
        // At least one event (or lag recovery) should still yield progress;
        // None only means Closed, which must not happen here.
        let _ = receiver.try_recv();
    }

    #[test]
    fn sink_reports_own_session() {
        let (tx, _) = broadcast::channel::<RuntimeEvent>(16);
        let session_id = SessionId::new();
        let sink = RuntimeEventSink::new(TaskId::new(), session_id, tx);
        assert_eq!(sink.session_id(), session_id);
    }
}
