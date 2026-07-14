use crate::config::OperationMetadata;
use crate::dispatch::types::TaskResult;
use std::future::Future;
use std::pin::Pin;

/// Result of executing an operation through an executor adapter.
pub enum ExecutionOutput {
    /// Operation completed successfully.
    Success(Box<TaskResult>),
    /// Required feature is not compiled in.
    FeatureUnavailable { operation_id: String },
    /// Execution failed with an error message.
    Failed(String),
}

/// Domain executor adapter trait.
///
/// Each domain implements this to handle its operations. The trait decouples
/// operation dispatch from the monolithic `dispatch_inner()` match, enabling
/// per-domain ownership of request conversion, execution, and result mapping.
///
/// # Object safety
///
/// The trait is object-safe: no generic self parameters, no associated types
/// with generic bounds. Executors are stored as `Box<dyn OperationExecutor>`.
pub trait OperationExecutor: Send + Sync {
    /// Returns the canonical operation IDs this executor handles.
    ///
    /// Must be non-empty and match IDs in `ALL_OPERATION_METADATA`.
    fn operation_ids(&self) -> &[&str];

    /// Returns metadata for the operations this executor handles.
    ///
    /// The returned slice must have the same length as `operation_ids()`
    /// and be in the same order.
    fn metadata(&self) -> &[&OperationMetadata];

    /// Execute the operation synchronously (blocking).
    ///
    /// Called from `dispatch_inner()` for operations that do not require
    /// async I/O. Default implementation returns `Failed`.
    fn execute_sync(&self, _task: &crate::config::OperationDescriptor) -> ExecutionOutput {
        ExecutionOutput::Failed("sync execution not implemented".into())
    }

    /// Execute the operation asynchronously.
    ///
    /// This is the primary execution path. The default implementation
    /// returns `Failed` so executors only need to override one method.
    fn execute_async<'a>(
        &'a self,
        _task: &'a crate::config::OperationDescriptor,
        _progress_tx: tokio::sync::mpsc::Sender<(u64, u64)>,
    ) -> Pin<Box<dyn Future<Output = ExecutionOutput> + Send + 'a>> {
        Box::pin(async { ExecutionOutput::Failed("async execution not implemented".into()) })
    }

    /// Check if this executor can handle the given operation ID.
    ///
    /// Default implementation checks membership in `operation_ids()`.
    fn can_handle(&self, operation_id: &str) -> bool {
        self.operation_ids().contains(&operation_id)
    }
}
