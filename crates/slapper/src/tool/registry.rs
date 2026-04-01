use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::SlapperError;
use crate::tool::traits::{SecurityTool, ToolCapability, ToolCategory};
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
                let protos: Vec<String> = tool
                    .supported_protocols()
                    .iter()
                    .map(|s| s.to_string())
                    .collect();
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
        let mut categories: Vec<ToolCategory> =
            self.tools.read().values().map(|t| t.category()).collect();
        categories.sort_by(|a, b| a.cmp(b));
        categories.dedup();
        categories
    }

    pub fn find_by_capability(&self, capability_name: &str) -> Vec<ToolInfo> {
        self.list()
            .into_iter()
            .filter(|t| t.capabilities.iter().any(|c| c.name == capability_name))
            .collect()
    }

    pub fn find_by_keyword(&self, keyword: &str) -> Vec<ToolInfo> {
        let keyword_lower = keyword.to_lowercase();
        self.list()
            .into_iter()
            .filter(|t| {
                t.name.to_lowercase().contains(&keyword_lower)
                    || t.description.to_lowercase().contains(&keyword_lower)
                    || t.capabilities.iter().any(|c| {
                        c.name.to_lowercase().contains(&keyword_lower)
                            || c.description.to_lowercase().contains(&keyword_lower)
                    })
            })
            .collect()
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

impl ToolInfo {
    pub fn find_capability(&self, name: &str) -> Option<&ToolCapability> {
        self.capabilities.iter().find(|c| c.name == name)
    }
}

impl Default for ToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}
