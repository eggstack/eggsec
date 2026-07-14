use std::future::Future;
use std::pin::Pin;

use crate::dispatch::executor::{ExecutionOutput, OperationExecutor};

/// Executor adapter for NSE (Nmap Scripting Engine) operations.
///
/// Only compiled when the `nse` feature is enabled.
/// Delegates to existing `dispatch::api::run_nse` function.
#[cfg(feature = "nse")]
pub struct NseExecutor;

#[cfg(feature = "nse")]
impl OperationExecutor for NseExecutor {
    fn operation_ids(&self) -> &[&str] {
        &["nse"]
    }

    fn metadata(&self) -> &[&crate::config::OperationMetadata] {
        &[]
    }

    fn can_handle(&self, operation_id: &str) -> bool {
        operation_id == "nse"
    }

    fn execute_async<'a>(
        &'a self,
        task: &'a crate::config::OperationDescriptor,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    ) -> Pin<Box<dyn Future<Output = ExecutionOutput> + Send + 'a>> {
        Box::pin(async move {
            let target = task.target.clone().unwrap_or_default();

            let result = crate::dispatch::api::run_nse(
                target,
                "default".to_string(),
                None,
                None,
                progress_tx,
            )
            .await;

            match result {
                Ok(task_result) => ExecutionOutput::Success(Box::new(task_result)),
                Err(e) => ExecutionOutput::Failed(e.to_string()),
            }
        })
    }
}
