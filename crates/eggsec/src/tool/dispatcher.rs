use crate::config::ApprovedOperation;
use crate::error::EggsecError;
use crate::tool::response::{ResponseMetadata, ResponseStatus};
use crate::tool::{ToolRegistry, ToolRequest, ToolResponse};
use parking_lot::RwLock;
use std::sync::Arc;

#[derive(Clone)]
pub struct ToolDispatcher {
    registry: ToolRegistry,
    history: Arc<RwLock<Option<super::history::ExecutionHistory>>>,
}

impl ToolDispatcher {
    pub fn new(registry: ToolRegistry) -> Self {
        Self {
            registry,
            history: Arc::new(RwLock::new(None)),
        }
    }

    pub fn with_history(self, history: super::history::ExecutionHistory) -> Self {
        *self.history.write() = Some(history);
        Self {
            registry: self.registry,
            history: Arc::clone(&self.history),
        }
    }

    pub fn history(&self) -> Option<super::history::ExecutionHistory> {
        self.history.read().clone()
    }

    pub async fn dispatch(&self, request: ToolRequest) -> Result<ToolResponse, EggsecError> {
        if request.is_cancelled() {
            return Err(EggsecError::Cancelled);
        }

        let tool = self
            .registry
            .get(&request.tool)
            .ok_or_else(|| EggsecError::Config(format!("Tool '{}' not found", request.tool)))?;

        tool.validate(&request)?;

        let started_at = chrono::Utc::now();
        let result = tool.execute(request.clone()).await;
        let completed_at = chrono::Utc::now();

        let response = match &result {
            Ok(resp) => resp.clone(),
            Err(_) => ToolResponse {
                request_id: request.id.clone(),
                tool_id: request.tool.clone(),
                status: ResponseStatus::Failed,
                results: serde_json::json!({}),
                metadata: ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds().max(0) as u64,
                    targets_scanned: 0,
                    findings_count: 0,
                },
                errors: vec![],
                findings: vec![],
            },
        };

        if let Some(ref history) = *self.history.read() {
            let capability = request
                .params
                .get("_capability")
                .and_then(|v| v.as_str())
                .map(String::from);
            history.record(&request, &response, capability);
        }

        result
    }

    pub fn registry(&self) -> &ToolRegistry {
        &self.registry
    }
}

impl Default for ToolDispatcher {
    fn default() -> Self {
        Self::new(ToolRegistry::new())
    }
}

/// Wrapper around [`ToolDispatcher`] that requires an [`ApprovedOperation`]
/// token before dispatching. This enforces type-level access control so
/// strict programmatic surfaces cannot accidentally bypass policy.
#[derive(Clone)]
pub struct EnforcedDispatcher {
    inner: ToolDispatcher,
}

impl EnforcedDispatcher {
    pub fn new(inner: ToolDispatcher) -> Self {
        Self { inner }
    }

    /// Dispatch a tool request, verifying it matches the approved operation.
    ///
    /// Checks that:
    /// - The tool name in the request matches the approved descriptor's operation.
    /// - If the descriptor has a target, it matches the request's target parameter.
    ///
    /// Fails closed on any mismatch.
    pub async fn dispatch_checked(
        &self,
        approved: &ApprovedOperation,
        request: ToolRequest,
    ) -> Result<ToolResponse, EggsecError> {
        let descriptor = approved.descriptor();

        if request.tool != descriptor.operation {
            return Err(EggsecError::Config(format!(
                "dispatch mismatch: request tool '{}' does not match approved operation '{}'",
                request.tool, descriptor.operation
            )));
        }

        if let Some(ref expected_target) = descriptor.target {
            let expected_str = expected_target.as_str();
            let matches = request.target.value == expected_str
                || request
                    .params
                    .get("target")
                    .and_then(|v| v.as_str())
                    .map(|v| v == expected_str)
                    .unwrap_or(false);
            if !matches {
                return Err(EggsecError::Config(format!(
                    "dispatch mismatch: request target '{}' does not match approved target '{}'",
                    request.target.value, expected_target
                )));
            }
        }

        self.inner.dispatch(request).await
    }

    /// Access the underlying dispatcher (for cases where the caller has
    /// already obtained an approval token through another path).
    pub fn inner(&self) -> &ToolDispatcher {
        &self.inner
    }

    pub fn with_history(self, history: super::history::ExecutionHistory) -> Self {
        Self {
            inner: self.inner.with_history(history),
        }
    }

    pub fn history(&self) -> Option<super::history::ExecutionHistory> {
        self.inner.history()
    }

    pub fn registry(&self) -> &ToolRegistry {
        self.inner.registry()
    }
}
