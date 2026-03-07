use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;

use crate::error::SlapperError;
use crate::tool::traits::{SecurityTool, ToolCategory, ToolCapability};
use crate::tool::{ToolRequest, ToolResponse};

pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn SecurityTool>>>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub fn register(&self, tool: impl SecurityTool + 'static) -> Result<(), SlapperError> {
        let id = tool.id().to_string();
        let mut tools = self.tools.write();
        
        if tools.contains_key(&id) {
            return Err(SlapperError::Config(format!(
                "Tool with id '{}' already registered",
                id
            )));
        }
        
        tools.insert(id, Arc::new(tool));
        Ok(())
    }

    pub fn unregister(&self, id: &str) -> Option<Arc<dyn SecurityTool>> {
        self.tools.write().remove(id)
    }

    pub fn get(&self, id: &str) -> Option<Arc<dyn SecurityTool>> {
        self.tools.read().get(id).cloned()
    }

    pub fn list(&self) -> Vec<ToolInfo> {
        self.tools
            .read()
            .iter()
            .map(|(id, tool)| {
                let caps = tool.capabilities();
                let protos: Vec<String> = tool.supported_protocols().iter().map(|s| s.to_string()).collect();
                ToolInfo {
                    id: id.clone(),
                    name: tool.name().to_string(),
                    category: tool.category(),
                    description: tool.description().to_string(),
                    capabilities: caps,
                    protocols: protos,
                }
            })
            .collect()
    }

    pub fn list_by_category(&self, category: ToolCategory) -> Vec<ToolInfo> {
        self.list()
            .into_iter()
            .filter(|t| t.category == category)
            .collect()
    }

    pub fn categories(&self) -> Vec<ToolCategory> {
        let mut categories: Vec<ToolCategory> = self
            .tools
            .read()
            .values()
            .map(|t| t.category())
            .collect();
        categories.sort_by(|a, b| a.cmp(b));
        categories.dedup();
        categories
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for ToolRegistry {
    fn clone(&self) -> Self {
        Self {
            tools: Arc::clone(&self.tools),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolInfo {
    pub id: String,
    pub name: String,
    pub category: ToolCategory,
    pub description: String,
    pub capabilities: Vec<ToolCapability>,
    pub protocols: Vec<String>,
}

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
