use crate::error::SlapperError;
use crate::tool::{request::ToolRequest, response::ToolResponse, ToolRegistry};
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

    pub async fn dispatch(&self, request: ToolRequest) -> Result<ToolResponse, SlapperError> {
        if request.is_cancelled() {
            return Err(SlapperError::Cancelled);
        }

        let tool = self
            .registry
            .get(&request.tool)
            .ok_or_else(|| SlapperError::Config(format!("Tool '{}' not found", request.tool)))?;

        tool.validate(&request)?;

        let started_at = chrono::Utc::now();
        let result = tool.execute(request.clone()).await;
        let completed_at = chrono::Utc::now();

        let response = match &result {
            Ok(resp) => resp.clone(),
            Err(_) => ToolResponse {
                request_id: request.id.clone(),
                tool_id: request.tool.clone(),
                status: crate::tool::ResponseStatus::Failed,
                results: serde_json::json!({}),
                metadata: crate::tool::ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms: (completed_at - started_at).num_milliseconds() as u64,
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

    pub fn dispatch_blocking(&self, request: ToolRequest) -> Result<ToolResponse, SlapperError> {
        if request.is_cancelled() {
            return Err(SlapperError::Cancelled);
        }

        let tool = self
            .registry
            .get(&request.tool)
            .ok_or_else(|| SlapperError::Config(format!("Tool '{}' not found", request.tool)))?;

        tool.validate(&request)?;

        let rt = tokio::runtime::Handle::current();
        rt.block_on(tool.execute(request))
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
