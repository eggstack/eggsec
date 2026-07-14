use std::future::Future;
use std::pin::Pin;

use crate::dispatch::executor::{ExecutionOutput, OperationExecutor};

/// Executor adapter for WAF detection and bypass operations:
/// `waf-detect`, `waf-bypass`, `waf-stress`.
///
/// Delegates to existing `dispatch::fuzzer` functions.
pub struct WafExecutor;

impl OperationExecutor for WafExecutor {
    fn operation_ids(&self) -> &[&str] {
        &["waf-detect", "waf-bypass", "waf-stress"]
    }

    fn metadata(&self) -> &[&crate::config::OperationMetadata] {
        &[]
    }

    fn can_handle(&self, operation_id: &str) -> bool {
        matches!(operation_id, "waf-detect" | "waf-bypass" | "waf-stress")
    }

    fn execute_async<'a>(
        &'a self,
        task: &'a crate::config::OperationDescriptor,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    ) -> Pin<Box<dyn Future<Output = ExecutionOutput> + Send + 'a>> {
        Box::pin(async move {
            let target = task.target.clone().unwrap_or_default();

            let result = match task.operation.as_str() {
                "waf-detect" => {
                    crate::dispatch::fuzzer::run_waf(target, false, vec![], progress_tx).await
                }
                "waf-bypass" => {
                    crate::dispatch::fuzzer::run_waf(target, true, vec![], progress_tx).await
                }
                "waf-stress" => {
                    crate::dispatch::fuzzer::run_waf_stress(target, 10, 100, progress_tx).await
                }
                _ => {
                    return ExecutionOutput::Failed(format!(
                        "WafExecutor cannot handle operation: {}",
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
