use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

use crate::event_protocol::EventEnvelope;

/// Channel-based backpressure for event delivery.
///
/// A bounded, in-process channel that applies backpressure when
/// the consumer cannot keep up with the producer. When the channel
/// is full, the oldest event is dropped and a warning is logged.
///
/// This is a pure-Rust implementation (no Python dependency) to
/// keep the backpressure layer lightweight and testable.
pub struct BackpressureChannel {
    buffer: Arc<Mutex<VecDeque<EventEnvelope>>>,
    capacity: usize,
    total_dropped: Arc<Mutex<u64>>,
}

impl BackpressureChannel {
    /// Create a new bounded backpressure channel.
    pub fn new(capacity: usize) -> Self {
        Self {
            buffer: Arc::new(Mutex::new(VecDeque::with_capacity(capacity.min(4096)))),
            capacity,
            total_dropped: Arc::new(Mutex::new(0)),
        }
    }

    /// Send an event into the channel. If the channel is full,
    /// the oldest event is dropped to make room.
    pub fn send(&self, event: EventEnvelope) {
        let mut buf = match self.buffer.lock() {
            Ok(b) => b,
            Err(_) => return,
        };
        if buf.len() >= self.capacity {
            if buf.pop_front().is_some() {
                let mut dropped = match self.total_dropped.lock() {
                    Ok(d) => d,
                    Err(_) => return,
                };
                *dropped += 1;
                tracing::warn!(
                    "BackpressureChannel: dropped oldest event (total dropped: {})",
                    *dropped
                );
            }
        }
        buf.push_back(event);
    }

    /// Try to receive an event. Returns `None` if the channel is empty.
    pub fn try_recv(&self) -> Option<EventEnvelope> {
        self.buffer.lock().ok()?.pop_front()
    }

    /// Number of events currently buffered.
    pub fn len(&self) -> usize {
        self.buffer.lock().map(|b| b.len()).unwrap_or(0)
    }

    /// Whether the channel is empty.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Number of events dropped due to backpressure.
    pub fn total_dropped(&self) -> u64 {
        self.total_dropped.lock().map(|d| *d).unwrap_or(0)
    }

    /// Maximum capacity of the channel.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Python-visible wrapper for BackpressureChannel.
#[pyo3::pyclass]
pub struct PyBackpressureChannel {
    inner: BackpressureChannel,
}

#[pyo3::pymethods]
impl PyBackpressureChannel {
    /// Create a new bounded backpressure channel.
    ///
    /// Args:
    ///     capacity: Maximum number of events before backpressure kicks in.
    #[new]
    #[pyo3(signature = (capacity=256,))]
    fn new(capacity: usize) -> Self {
        Self {
            inner: BackpressureChannel::new(capacity),
        }
    }

    /// Send an event into the channel.
    fn send(&self, event: EventEnvelope) {
        self.inner.send(event);
    }

    /// Try to receive an event. Returns None if empty.
    fn try_recv(&self) -> Option<EventEnvelope> {
        self.inner.try_recv()
    }

    /// Number of events currently buffered.
    fn len(&self) -> usize {
        self.inner.len()
    }

    /// Whether the channel is empty.
    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Number of events dropped due to backpressure.
    fn total_dropped(&self) -> u64 {
        self.inner.total_dropped()
    }

    /// Maximum capacity of the channel.
    #[getter]
    fn capacity(&self) -> usize {
        self.inner.capacity()
    }

    fn __repr__(&self) -> String {
        format!(
            "BackpressureChannel(len={}, capacity={}, dropped={})",
            self.inner.len(),
            self.inner.capacity(),
            self.inner.total_dropped(),
        )
    }

    fn __len__(&self) -> usize {
        self.inner.len()
    }
}
