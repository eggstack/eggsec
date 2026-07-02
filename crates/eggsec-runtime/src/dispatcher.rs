use std::pin::Pin;

use crate::event::TaskOutcome;
use crate::request::RunRequest;
use crate::RuntimeError;

/// Frontend-neutral task dispatcher trait.
///
/// Implementations map `RunRequest` to actual tool execution and return a
/// `TaskOutcome` for lifecycle tracking. The trait is dependency-free —
/// concrete implementations live in frontend crates (TUI, CLI, agent) that
/// have access to the engine functions.
///
/// # Architecture
///
/// The runtime owns task lifecycle (timeout, cancellation, events). The
/// dispatcher owns task execution logic. The executor bridges the two:
/// it calls the dispatcher and reports the outcome to the runtime.
pub trait TaskDispatcher: Send + Sync + 'static {
    /// Dispatch a task and return its outcome.
    ///
    /// The dispatcher should execute the tool described by `request` and
    /// return a `TaskOutcome` on success. For frontend-specific result
    /// delivery (e.g., typed channel results for TUI display), the
    /// implementation may perform side effects during dispatch.
    fn dispatch(
        &self,
        request: RunRequest,
    ) -> Pin<Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send>>;
}
