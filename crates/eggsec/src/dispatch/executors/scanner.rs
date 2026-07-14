use std::future::Future;
use std::pin::Pin;

use crate::dispatch::executor::{ExecutionOutput, OperationExecutor};

/// Executor adapter for scanner operations:
/// `scan-ports`, `scan-endpoints`, `fingerprint`.
///
/// Delegates to existing `dispatch::scanner` functions.
pub struct ScannerExecutor;

impl OperationExecutor for ScannerExecutor {
    fn operation_ids(&self) -> &[&str] {
        &["scan-ports", "scan-endpoints", "fingerprint"]
    }

    fn metadata(&self) -> &[&crate::config::OperationMetadata] {
        &[]
    }

    fn can_handle(&self, operation_id: &str) -> bool {
        matches!(
            operation_id,
            "scan-ports" | "scan-endpoints" | "fingerprint"
        )
    }

    fn execute_async<'a>(
        &'a self,
        task: &'a crate::config::OperationDescriptor,
        progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    ) -> Pin<Box<dyn Future<Output = ExecutionOutput> + Send + 'a>> {
        Box::pin(async move {
            let target = task.target.clone().unwrap_or_default();

            let result = match task.operation.as_str() {
                "scan-ports" => {
                    let timeout = std::time::Duration::from_secs(60);
                    crate::dispatch::scanner::run_port_scan(
                        target,
                        "1-1024".to_string(),
                        100,
                        timeout,
                        progress_tx,
                    )
                    .await
                }
                "scan-endpoints" => {
                    let timeout = std::time::Duration::from_secs(60);
                    crate::dispatch::scanner::run_endpoint_scan(
                        target,
                        10,
                        timeout,
                        None,
                        progress_tx,
                    )
                    .await
                }
                "fingerprint" => {
                    let timeout = std::time::Duration::from_secs(60);
                    crate::dispatch::scanner::run_fingerprint(
                        target,
                        "1-1024".to_string(),
                        timeout,
                        progress_tx,
                    )
                    .await
                }
                _ => {
                    return ExecutionOutput::Failed(format!(
                        "ScannerExecutor cannot handle operation: {}",
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
