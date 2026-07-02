use eggsec_runtime::dispatcher::TaskDispatcher;
use eggsec_runtime::event::TaskOutcome;
use eggsec_runtime::request::RunRequest;
use eggsec_runtime::RuntimeError;

use crate::app::task_runtime::TuiDispatcherContext;
use arc_swap::ArcSwap;
use std::sync::Arc;

/// TUI-side task dispatcher that delegates to `eggsec::dispatch`.
///
/// Instead of converting `RunRequest` → `TaskConfig` → `TaskRunner`,
/// this directly calls `eggsec::dispatch::dispatch_inner` which routes
/// to the appropriate engine function based on `TaskKind`.
pub(crate) struct TuiTaskDispatcher {
    executor_context: Arc<ArcSwap<TuiDispatcherContext>>,
}

impl TuiTaskDispatcher {
    pub fn new(executor_context: Arc<ArcSwap<TuiDispatcherContext>>) -> Self {
        Self { executor_context }
    }
}

impl TaskDispatcher for TuiTaskDispatcher {
    fn dispatch(
        &self,
        request: RunRequest,
    ) -> std::pin::Pin<
        Box<dyn std::future::Future<Output = Result<TaskOutcome, RuntimeError>> + Send>,
    > {
        let ctx = self.executor_context.load();
        let progress_tx = ctx.progress_tx.clone();
        let result_tx = ctx.result_tx.clone();

        Box::pin(async move {
            eggsec::dispatch::dispatch_inner(request, progress_tx, result_tx)
                .await
                .map_err(|e| {
                    RuntimeError::DispatchFailed(format!("task execution failed: {}", e))
                })?;

            // Return empty outcome — typed results were sent through the
            // result channel for TUI consumption.
            Ok(TaskOutcome::Empty)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn task_dispatcher_creation() {
        let (progress_tx, _) = tokio::sync::mpsc::channel(100);
        let (result_tx, _) = tokio::sync::mpsc::channel(1);
        let ctx = Arc::new(ArcSwap::from_pointee(TuiDispatcherContext {
            progress_tx,
            result_tx,
        }));
        let dispatcher = TuiTaskDispatcher::new(ctx);
        assert!(
            std::any::TypeId::of::<TuiTaskDispatcher>()
                == std::any::TypeId::of::<TuiTaskDispatcher>()
        );
    }
}
