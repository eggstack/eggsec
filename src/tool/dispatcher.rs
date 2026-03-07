use crate::error::SlapperError;
use crate::tool::{request::ToolRequest, response::ToolResponse, ToolRegistry};

#[derive(Clone)]
pub struct ToolDispatcher {
    registry: ToolRegistry,
}

impl ToolDispatcher {
    pub fn new(registry: ToolRegistry) -> Self {
        Self { registry }
    }

    pub async fn dispatch(&self, request: ToolRequest) -> Result<ToolResponse, SlapperError> {
        let tool = self
            .registry
            .get(&request.tool)
            .ok_or_else(|| SlapperError::Config(format!("Tool '{}' not found", request.tool)))?;

        tool.validate(&request)?;
        tool.execute(request).await
    }

    pub fn dispatch_blocking(&self, request: ToolRequest) -> Result<ToolResponse, SlapperError> {
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
