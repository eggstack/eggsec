use std::future::Future;
use std::pin::Pin;

use crate::dispatch::executor::{ExecutionOutput, OperationExecutor};

/// Executor adapter for fuzzing operations: `fuzz`, `graphql`, `oauth`.
///
/// Delegates to existing `dispatch::fuzzer` and `dispatch::api` functions.
pub struct FuzzExecutor;

impl OperationExecutor for FuzzExecutor {
    fn operation_ids(&self) -> &[&str] {
        &["fuzz", "graphql", "oauth"]
    }

    fn metadata(&self) -> &[&crate::config::OperationMetadata] {
        &[]
    }

    fn can_handle(&self, operation_id: &str) -> bool {
        matches!(operation_id, "fuzz" | "graphql" | "oauth")
    }

    fn execute_async<'a>(
        &'a self,
        task: &'a crate::config::OperationDescriptor,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    ) -> Pin<Box<dyn Future<Output = ExecutionOutput> + Send + 'a>> {
        Box::pin(async move {
            let target = task.target.clone().unwrap_or_default();

            let result = match task.operation.as_str() {
                "fuzz" => {
                    crate::dispatch::fuzzer::run_fuzz(
                        target,
                        "xss".to_string(),
                        "smart".to_string(),
                        false,
                        0,
                        "GET".to_string(),
                        None,
                        10,
                        60,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        false,
                        progress_tx,
                    )
                    .await
                }
                "graphql" => {
                    crate::dispatch::api::run_graphql(
                        target,
                        true,
                        false,
                        false,
                        false,
                        10,
                        300,
                        progress_tx,
                    )
                    .await
                }
                "oauth" => {
                    crate::dispatch::api::run_oauth(
                        target,
                        None,
                        None,
                        false,
                        false,
                        false,
                        false,
                        10,
                        300,
                        progress_tx,
                    )
                    .await
                }
                _ => {
                    return ExecutionOutput::Failed(format!(
                        "FuzzExecutor cannot handle operation: {}",
                        task.operation
                    ))
                }
            };

            match result {
                Ok(task_result) => ExecutionOutput::Success(Box::new(task_result)),
                Err(e) => ExecutionOutput::Failed(e.to_string()),
            }
        })
    }
}
