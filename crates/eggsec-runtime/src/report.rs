//! Bounded retention of completed-task report documents.
//!
//! The runtime owns task lifecycle but not report *semantics*: the engine
//! decides what a completed result means, and the daemon serves what the
//! engine produced. What lives here is only the bounded store between them —
//! a key/value map from task id to the report document the engine derived
//! from that task's result.
//!
//! Retention is bounded on purpose. A long-lived daemon would otherwise
//! accumulate one report document per task for the life of the process, and
//! report documents are the largest payloads the runtime ever holds. The
//! bound is small (see [`MAX_RETAINED_REPORTS`]) and the eviction order is
//! oldest-first, so a burst of tasks cannot grow the store without limit.
//!
//! The payload is `serde_json::Value` rather than a report-model type on
//! purpose: the runtime crate deliberately carries no domain types, and a
//! report document crosses the wire as JSON anyway. The consumer — the
//! desktop GUI — deserializes it into the canonical report model.

use std::collections::{HashMap, VecDeque};
use std::sync::Mutex;

use crate::ids::TaskId;

/// The maximum number of completed-task reports a runtime retains.
///
/// Oldest-first eviction applies once the store is full, so the bound is a
/// hard ceiling on retained report documents rather than a soft target.
pub const MAX_RETAINED_REPORTS: usize = 64;

/// A bounded, in-memory store of completed-task report documents.
///
/// The store is cheap to clone-share: it is used behind an `Arc` and every
/// method takes `&self`, so a producer and a consumer can hold the same
/// store without either owning it.
///
/// The internal lock is a `std::sync::Mutex` and is never held across an
/// await point — every method is synchronous and its critical section is a
/// map operation. That keeps the store usable from the synchronous parts of
/// an executor without forcing an async signature on the read path.
#[derive(Debug)]
pub struct RuntimeReportStore {
    inner: Mutex<ReportStoreInner>,
}

#[derive(Debug)]
struct ReportStoreInner {
    reports: HashMap<TaskId, serde_json::Value>,
    /// Insertion order, used for oldest-first eviction.
    order: VecDeque<TaskId>,
}

impl RuntimeReportStore {
    /// Create an empty store.
    pub fn new() -> Self {
        Self {
            inner: Mutex::new(ReportStoreInner {
                reports: HashMap::new(),
                order: VecDeque::new(),
            }),
        }
    }

    /// Record the report document for a completed task.
    ///
    /// Re-inserting an existing task id replaces its document and keeps its
    /// original position in the eviction order; a task is inserted once, so
    /// this is only a defensive path.
    ///
    /// When the store is at [`MAX_RETAINED_REPORTS`], the oldest entry is
    /// evicted to make room.
    pub fn insert(&self, task_id: TaskId, report: serde_json::Value) {
        let mut inner = self.lock();
        if inner.reports.insert(task_id, report).is_none() {
            inner.order.push_back(task_id);
        }
        while inner.order.len() > MAX_RETAINED_REPORTS {
            if let Some(evicted) = inner.order.pop_front() {
                inner.reports.remove(&evicted);
            }
        }
    }

    /// The report document recorded for a task, if one is retained.
    pub fn get(&self, task_id: &TaskId) -> Option<serde_json::Value> {
        let inner = self.lock();
        inner.reports.get(task_id).cloned()
    }

    /// The number of retained report documents.
    pub fn len(&self) -> usize {
        self.lock().reports.len()
    }

    /// Whether the store holds no report documents.
    pub fn is_empty(&self) -> bool {
        self.lock().reports.is_empty()
    }

    /// Drop every retained report document.
    pub fn clear(&self) {
        let mut inner = self.lock();
        inner.reports.clear();
        inner.order.clear();
    }

    /// Take the store's lock, recovering from poisoning rather than
    /// propagating it.
    ///
    /// A poisoned lock means some other holder panicked while holding it.
    /// The store's invariants are still intact — the critical sections are
    /// map operations that cannot leave the map half-updated — so the
    /// poisoned state is recovered and the event is logged rather than
    /// turned into a panic in every subsequent caller.
    fn lock(&self) -> std::sync::MutexGuard<'_, ReportStoreInner> {
        self.inner.lock().unwrap_or_else(|poisoned| {
            tracing::warn!("the runtime report store lock was poisoned; recovering state");
            poisoned.into_inner()
        })
    }
}

impl Default for RuntimeReportStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task() -> TaskId {
        TaskId::new()
    }

    fn doc(marker: &str) -> serde_json::Value {
        serde_json::json!({ "marker": marker })
    }

    #[test]
    fn a_fresh_store_is_empty() {
        let store = RuntimeReportStore::new();
        assert!(store.is_empty());
        assert_eq!(store.len(), 0);
    }

    #[test]
    fn an_inserted_report_is_readable_back() {
        let store = RuntimeReportStore::new();
        let id = task();
        store.insert(id, doc("first"));
        assert_eq!(store.len(), 1);
        assert_eq!(
            store.get(&id),
            Some(serde_json::json!({ "marker": "first" })),
            "the stored document comes back unchanged"
        );
    }

    #[test]
    fn an_unknown_task_has_no_report() {
        let store = RuntimeReportStore::new();
        assert!(store.get(&task()).is_none());
    }

    #[test]
    fn retention_is_bounded_and_evicts_the_oldest_first() {
        let store = RuntimeReportStore::new();
        let ids: Vec<TaskId> = (0..MAX_RETAINED_REPORTS + 5).map(|_| task()).collect();
        for (index, id) in ids.iter().enumerate() {
            store.insert(*id, doc(&index.to_string()));
        }
        assert_eq!(
            store.len(),
            MAX_RETAINED_REPORTS,
            "the store never grows past its bound"
        );
        for id in &ids[..5] {
            assert!(
                store.get(id).is_none(),
                "the oldest entries are evicted first"
            );
        }
        for id in &ids[5..] {
            assert!(
                store.get(id).is_some(),
                "the most recent entries are retained"
            );
        }
    }

    #[test]
    fn reinserting_a_task_replaces_its_document() {
        let store = RuntimeReportStore::new();
        let id = task();
        store.insert(id, doc("first"));
        store.insert(id, doc("second"));
        assert_eq!(store.len(), 1, "a replacement is not a second entry");
        assert_eq!(
            store.get(&id),
            Some(serde_json::json!({ "marker": "second" })),
            "the newer document replaces the older one"
        );
    }

    #[test]
    fn clearing_drops_every_document() {
        let store = RuntimeReportStore::new();
        let id = task();
        store.insert(id, doc("first"));
        store.clear();
        assert!(store.is_empty());
        assert!(store.get(&id).is_none());
    }
}
