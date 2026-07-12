use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use pyo3::prelude::*;

use crate::event_protocol::EventEnvelope;

/// Observable accounting for event delivery and backpressure.
#[pyclass(frozen)]
#[derive(Debug, Clone, Default)]
pub struct EventDeliveryStats {
    #[pyo3(get)]
    pub emitted_count: u64,
    #[pyo3(get)]
    pub delivered_count: u64,
    #[pyo3(get)]
    pub dropped_count: u64,
    #[pyo3(get)]
    pub dropped_by_kind: HashMap<String, u64>,
    #[pyo3(get)]
    pub max_queue_depth: usize,
    #[pyo3(get)]
    pub consumer_lag: usize,
    #[pyo3(get)]
    pub terminal_event_delivery_failures: u64,
}

#[pymethods]
impl EventDeliveryStats {
    fn to_dict(&self, py: Python) -> PyResult<PyObject> {
        let dict = pyo3::types::PyDict::new_bound(py);
        dict.set_item("emitted_count", self.emitted_count)?;
        dict.set_item("delivered_count", self.delivered_count)?;
        dict.set_item("dropped_count", self.dropped_count)?;
        dict.set_item("dropped_by_kind", &self.dropped_by_kind)?;
        dict.set_item("max_queue_depth", self.max_queue_depth)?;
        dict.set_item("consumer_lag", self.consumer_lag)?;
        dict.set_item(
            "terminal_event_delivery_failures",
            self.terminal_event_delivery_failures,
        )?;
        Ok(dict.into())
    }

    fn to_json(&self) -> PyResult<String> {
        serde_json::to_string(self)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __repr__(&self) -> String {
        format!(
            "EventDeliveryStats(emitted={}, delivered={}, dropped={}, max_queue_depth={})",
            self.emitted_count, self.delivered_count, self.dropped_count, self.max_queue_depth
        )
    }
}

impl serde::Serialize for EventDeliveryStats {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serde::Serialize::serialize(
            &serde_json::json!({
                "emitted_count": self.emitted_count,
                "delivered_count": self.delivered_count,
                "dropped_count": self.dropped_count,
                "dropped_by_kind": self.dropped_by_kind,
                "max_queue_depth": self.max_queue_depth,
                "consumer_lag": self.consumer_lag,
                "terminal_event_delivery_failures": self.terminal_event_delivery_failures,
            }),
            serializer,
        )
    }
}

/// A bounded event queue with separate reliable capacity.
///
/// Progress/diagnostic events are best effort and may be dropped when the
/// bounded queue is full. Lifecycle, finding, artifact, failure, cancellation,
/// and completion events use the reliable queue and are never evicted by
/// progress traffic within this process.
pub struct BackpressureChannel {
    buffer: Arc<Mutex<VecDeque<EventEnvelope>>>,
    reliable_buffer: Arc<Mutex<VecDeque<EventEnvelope>>>,
    capacity: usize,
    stats: Arc<Mutex<EventDeliveryStats>>,
    next_sequence: Arc<AtomicU64>,
}

impl BackpressureChannel {
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.min(4096)))),
            reliable_buffer: Arc::new(Mutex::new(VecDeque::new())),
            capacity: capacity.max(1),
            stats: Arc::new(Mutex::new(EventDeliveryStats::default())),
            next_sequence: Arc::new(AtomicU64::new(1)),
        }
    }

    pub fn send(&self, mut event: EventEnvelope) {
        if event.sequence == 0 {
            event.sequence = self.next_sequence.fetch_add(1, Ordering::Relaxed);
        }
        let reliable = is_reliable(&event.event_type);
        if let Ok(mut stats) = self.stats.lock() {
            stats.emitted_count += 1;
        }

        if reliable {
            if let Ok(mut queue) = self.reliable_buffer.lock() {
                queue.push_back(event);
                self.update_depth(queue.len());
            } else {
                self.record_drop("reliable", true);
            }
            return;
        }

        let mut buffer = match self.buffer.lock() {
            Ok(buffer) => buffer,
            Err(_) => {
                self.record_drop("unknown", false);
                return;
            }
        };
        if buffer.len() >= self.capacity {
            if let Some(dropped) = buffer.pop_front() {
                self.record_drop(&dropped.event_type, false);
            }
        }
        buffer.push_back(event);
        self.update_depth(buffer.len());
    }

    pub fn try_recv(&self) -> Option<EventEnvelope> {
        let event = self
            .reliable_buffer
            .lock()
            .ok()
            .and_then(|mut queue| queue.pop_front())
            .or_else(|| self.buffer.lock().ok()?.pop_front());
        if event.is_some() {
            if let Ok(mut stats) = self.stats.lock() {
                stats.delivered_count += 1;
                stats.consumer_lag = self.len();
            }
        }
        event
    }

    pub fn len(&self) -> usize {
        self.buffer.lock().map(|q| q.len()).unwrap_or(0)
            + self.reliable_buffer.lock().map(|q| q.len()).unwrap_or(0)
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn total_dropped(&self) -> u64 {
        self.stats.lock().map(|s| s.dropped_count).unwrap_or(0)
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    pub fn stats(&self) -> EventDeliveryStats {
        self.stats.lock().map(|s| s.clone()).unwrap_or_default()
    }

    fn update_depth(&self, depth: usize) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.max_queue_depth = stats.max_queue_depth.max(depth);
            stats.consumer_lag = depth;
        }
    }

    fn record_drop(&self, kind: &str, reliable: bool) {
        if let Ok(mut stats) = self.stats.lock() {
            stats.dropped_count += 1;
            *stats.dropped_by_kind.entry(kind.to_string()).or_default() += 1;
            if reliable {
                stats.terminal_event_delivery_failures += 1;
            }
        }
        tracing::warn!(
            event_kind = kind,
            reliable,
            "event delivery accounting recorded a drop"
        );
    }
}

pub(crate) fn is_reliable(event_type: &str) -> bool {
    event_type.contains("planning")
        || event_type.contains("preflight")
        || event_type.starts_with("stage.")
        || event_type.contains("finding")
        || event_type.contains("artifact")
        || event_type.contains("cancel")
        || event_type.contains("failed")
        || event_type.contains("failure")
        || event_type.contains("completed")
        || event_type.contains("completion")
}

/// Python-visible wrapper for `BackpressureChannel`.
#[pyclass]
pub struct PyBackpressureChannel {
    inner: BackpressureChannel,
}

#[pymethods]
impl PyBackpressureChannel {
    #[new]
    #[pyo3(signature = (capacity=256,))]
    fn new(capacity: usize) -> Self {
        Self {
            inner: BackpressureChannel::new(capacity),
        }
    }

    fn send(&self, event: EventEnvelope) {
        self.inner.send(event);
    }

    fn try_recv(&self) -> Option<EventEnvelope> {
        self.inner.try_recv()
    }

    fn len(&self) -> usize {
        self.inner.len()
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn total_dropped(&self) -> u64 {
        self.inner.total_dropped()
    }

    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn stats(&self) -> EventDeliveryStats {
        self.inner.stats()
    }

    fn __repr__(&self) -> String {
        let stats = self.inner.stats();
        format!(
            "BackpressureChannel(len={}, capacity={}, dropped={})",
            self.inner.len(),
            self.inner.capacity(),
            stats.dropped_count
        )
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn event(kind: &str) -> EventEnvelope {
        Python::with_gil(|py| {
            EventEnvelope::create(kind.to_string(), py.None(), None, None, None, None)
        })
    }

    #[test]
    fn progress_is_accounted_when_queue_is_saturated() {
        let channel = BackpressureChannel::new(1);
        channel.send(event("progress"));
        channel.send(event("progress"));
        let stats = channel.stats();
        assert_eq!(stats.emitted_count, 2);
        assert_eq!(stats.dropped_count, 1);
        assert_eq!(stats.dropped_by_kind.get("progress"), Some(&1));
    }

    #[test]
    fn reliable_event_survives_progress_saturation() {
        let channel = BackpressureChannel::new(1);
        channel.send(event("progress"));
        channel.send(event("operation.completed"));
        assert_eq!(
            channel.try_recv().unwrap().event_type,
            "operation.completed"
        );
    }
}
