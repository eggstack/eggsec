use std::future::Future;
use std::pin::Pin;

use crate::dispatch::executor::{ExecutionOutput, OperationExecutor};

/// Executor adapter for reconnaissance operations:
/// `recon`, `pipeline`.
///
/// Delegates to existing `dispatch::recon` functions.
pub struct ReconExecutor;

impl OperationExecutor for ReconExecutor {
    fn operation_ids(&self) -> &[&str] {
        &["recon", "pipeline"]
    }

    fn metadata(&self) -> &[&crate::config::OperationMetadata] {
        &[]
    }

    fn can_handle(&self, operation_id: &str) -> bool {
        matches!(operation_id, "recon" | "pipeline")
    }

    fn execute_async<'a>(
        &'a self,
        task: &'a crate::config::OperationDescriptor,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    ) -> Pin<Box<dyn Future<Output = ExecutionOutput> + Send + 'a>> {
        Box::pin(async move {
            let target = task.target.clone().unwrap_or_default();

            let result = match task.operation.as_str() {
                "recon" => {
                    crate::dispatch::recon::run_recon(
                        target,
                        20,
                        crate::dispatch::types::ReconOptions::default(),
                        progress_tx,
                    )
                    .await
                }
                "pipeline" => {
                    let profile = crate::types::ScanProfile::Quick;
                    // No report output: this executor receives only an
                    // `OperationDescriptor` with no request params, so there
                    // is no caller-supplied destination to honour. See the
                    // canonical `pipeline` arm for the parameterized path.
                    crate::dispatch::recon::run_pipeline(target, profile, None, progress_tx).await
                }
                _ => {
                    return ExecutionOutput::Failed(format!(
                        "ReconExecutor cannot handle operation: {}",
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
