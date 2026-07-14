use std::future::Future;
use std::pin::Pin;

use crate::dispatch::executor::{ExecutionOutput, OperationExecutor};

/// Executor adapter for network operations:
/// `load-test`, `stress-test`, `packet`, `auth-test`.
///
/// Delegates to existing `dispatch::network` and `dispatch::auth` functions.
pub struct NetworkExecutor;

impl OperationExecutor for NetworkExecutor {
    fn operation_ids(&self) -> &[&str] {
        &["load-test", "stress-test", "packet", "auth-test"]
    }

    fn metadata(&self) -> &[&crate::config::OperationMetadata] {
        &[]
    }

    fn can_handle(&self, operation_id: &str) -> bool {
        matches!(
            operation_id,
            "load-test" | "stress-test" | "packet" | "auth-test"
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
                "load-test" => {
                    let timeout = std::time::Duration::from_secs(30);
                    crate::dispatch::network::run_load_test(target, 100, 10, timeout, progress_tx)
                        .await
                }
                "stress-test" => {
                    crate::dispatch::network::run_stress_test(
                        target,
                        "syn".to_string(),
                        1000,
                        60,
                        10,
                        progress_tx,
                    )
                    .await
                }
                "packet" => {
                    crate::dispatch::network::run_packet_capture(
                        "eth0".to_string(),
                        String::new(),
                        1000,
                        None,
                        progress_tx,
                    )
                    .await
                }
                "auth-test" => {
                    crate::dispatch::auth::run_auth_task(
                        target,
                        None,
                        None,
                        None,
                        100,
                        1,
                        30,
                        progress_tx,
                    )
                    .await
                }
                _ => {
                    return ExecutionOutput::Failed(format!(
                        "NetworkExecutor cannot handle operation: {}",
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
