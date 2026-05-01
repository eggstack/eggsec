use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;

use crate::error::SlapperError;
use crate::tool::traits::{SecurityTool, ToolCapability, ToolCategory};

/// Registry for managing security tools.
///
/// The `ToolRegistry` provides centralized tool management, allowing tools
/// to be registered, unregistered, and queried. It is the primary interface
/// for the tool abstraction layer.
///
/// # Example
///
/// ```rust
/// use slapper::tool::registry::ToolRegistry;
///
/// let registry = ToolRegistry::new();
/// // Register tools...
/// let tools = registry.list();
/// ```
pub struct ToolRegistry {
    tools: Arc<RwLock<HashMap<String, Arc<dyn SecurityTool>>>>,
}

impl ToolRegistry {
    /// Creates a new empty tool registry.
    pub fn new() -> Self {
        Self {
            tools: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Registers a tool with the registry.
    ///
    /// # Arguments
    ///
    /// * `tool` - The tool to register (must implement `SecurityTool`)
    ///
    /// # Errors
    ///
    /// Returns `Err(SlapperError::Config)` if a tool with the same ID
    /// is already registered.
    ///
    /// # Example
    ///
    /// ```rust
    /// use slapper::tool::registry::ToolRegistry;
    ///
    /// let registry = ToolRegistry::new();
    /// // registry.register(my_tool)?;  // Returns Result
    /// ```
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

    /// Unregisters a tool from the registry.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the tool to unregister
    ///
    /// # Returns
    ///
    /// Returns the unregistered tool if it existed, or `None` if no
    /// tool with that ID was found.
    pub fn unregister(&self, id: &str) -> Option<Arc<dyn SecurityTool>> {
        self.tools.write().remove(id)
    }

    /// Gets a tool by its ID.
    ///
    /// # Arguments
    ///
    /// * `id` - The ID of the tool to retrieve
    ///
    /// # Returns
    ///
    /// Returns the tool if found, or `None` if no tool with that ID exists.
    pub fn get(&self, id: &str) -> Option<Arc<dyn SecurityTool>> {
        self.tools.read().get(id).cloned()
    }

    /// Lists all registered tools.
    ///
    /// # Returns
    ///
    /// Returns a vector of `ToolInfo` structs containing tool metadata.
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

    /// Lists all tools in a specific category.
    ///
    /// # Arguments
    ///
    /// * `category` - The category to filter by
    ///
    /// # Returns
    ///
    /// Returns a vector of `ToolInfo` structs for tools in the category.
    pub fn list_by_category(&self, category: ToolCategory) -> Vec<ToolInfo> {
        self.list()
            .into_iter()
            .filter(|t| t.category == category)
            .collect()
    }

    /// Returns all unique tool categories in the registry.
    ///
    /// # Returns
    ///
    /// Returns a vector of `ToolCategory` values.
    pub fn categories(&self) -> Vec<ToolCategory> {
        let mut categories: Vec<ToolCategory> =
            self.tools.read().values().map(|t| t.category()).collect();
        categories.sort();
        categories.dedup();
        categories
    }

    /// Finds tools that provide a specific capability.
    ///
    /// # Arguments
    ///
    /// * `capability_name` - The name of the capability to search for
    ///
    /// # Returns
    ///
    /// Returns a vector of `ToolInfo` structs for tools that have
    /// the specified capability.
    pub fn find_by_capability(&self, capability_name: &str) -> Vec<ToolInfo> {
        self.list()
            .into_iter()
            .filter(|t| t.capabilities.iter().any(|c| c.name == capability_name))
            .collect()
    }

    /// Finds tools matching a keyword in name, description, or capabilities.
    ///
    /// # Arguments
    ///
    /// * `keyword` - The keyword to search for (case-insensitive)
    ///
    /// # Returns
    ///
    /// Returns a vector of `ToolInfo` structs for tools that match
    /// the keyword in name, description, or any capability field.
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
