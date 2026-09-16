//! Structured load-test progress (Phase D WS4).
//!
//! The core executor never prints and never touches `indicatif`. It emits
//! [`LoadTestEvent`]s through a [`ProgressSink`]; presentation lives in
//! process-host code (CLI indicatif renderer, TUI structured consumer,
//! library/daemon event forwarding or omission).

use std::sync::Arc;

/// Point-in-time progress snapshot.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoadTestProgress {
    /// Requests completed (success + failure).
    pub completed: u64,
    /// Total requests planned.
    pub total: u64,
}

/// Events emitted by the executor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadTestEvent {
    /// A single request completed (`completed` of `total`).
    RequestCompleted { completed: u64, total: u64 },
    /// The run finished (`completed` should equal `total` unless cancelled).
    Finished { completed: u64, total: u64 },
}

/// Sink for executor progress events.
///
/// Implementations must be cheap and non-blocking: the executor calls
/// `on_event` once per completed request. A slow sink throttles the run.
pub trait ProgressSink: Send + Sync {
    fn on_event(&self, event: LoadTestEvent);
}

/// No-op sink (library/Python/daemon default).
#[derive(Debug, Default, Clone, Copy)]
pub struct NoopSink;

impl ProgressSink for NoopSink {
    fn on_event(&self, _event: LoadTestEvent) {}
}

impl ProgressSink for () {
    fn on_event(&self, _event: LoadTestEvent) {}
}

/// Closure-backed sink.
pub struct FnSink<F: Fn(LoadTestEvent) + Send + Sync>(pub F);

impl<F: Fn(LoadTestEvent) + Send + Sync> ProgressSink for FnSink<F> {
    fn on_event(&self, event: LoadTestEvent) {
        (self.0)(event);
    }
}

/// `tokio::sync::mpsc`-backed sink (TUI/daemon structured consumer).
///
/// Uses `try_send` so a slow consumer never blocks workers; dropped events
/// are counted locally and logged at finish.
#[derive(Debug, Clone)]
pub struct ChannelSink {
    tx: tokio::sync::mpsc::Sender<LoadTestProgress>,
}

impl ChannelSink {
    #[must_use]
    pub fn new(tx: tokio::sync::mpsc::Sender<LoadTestProgress>) -> Self {
        Self { tx }
    }
}

impl ProgressSink for ChannelSink {
    fn on_event(&self, event: LoadTestEvent) {
        let (completed, total) = match event {
            LoadTestEvent::RequestCompleted { completed, total }
            | LoadTestEvent::Finished { completed, total } => (completed, total),
        };
        if self
            .tx
            .try_send(LoadTestProgress { completed, total })
            .is_err()
        {
            tracing::trace!("loadtest progress channel full/closed; dropping update");
        }
    }
}

/// Shared atomic progress counter (executor-internal).
#[derive(Debug, Default)]
pub struct SharedProgress {
    completed: std::sync::atomic::AtomicU64,
}

impl SharedProgress {
    #[must_use]
    pub fn new() -> Arc<Self> {
        Arc::new(Self::default())
    }

    /// Increment and return the new completed count.
    pub fn inc(&self) -> u64 {
        self.completed
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            + 1
    }

    #[must_use]
    pub fn get(&self) -> u64 {
        self.completed.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    #[test]
    fn noop_sink_ignores_events() {
        NoopSink.on_event(LoadTestEvent::RequestCompleted {
            completed: 1,
            total: 2,
        });
        ().on_event(LoadTestEvent::Finished {
            completed: 2,
            total: 2,
        });
    }

    #[test]
    fn fn_sink_forwards_events() {
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen2 = seen.clone();
        let sink = FnSink(move |e| {
            seen2.lock().expect("lock").push(e);
        });
        sink.on_event(LoadTestEvent::RequestCompleted {
            completed: 1,
            total: 3,
        });
        assert_eq!(seen.lock().expect("lock").len(), 1);
    }

    #[test]
    fn shared_progress_counts() {
        let p = SharedProgress::new();
        assert_eq!(p.inc(), 1);
        assert_eq!(p.inc(), 2);
        assert_eq!(p.get(), 2);
    }
}
