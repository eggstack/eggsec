#![cfg(feature = "rest-api")]

use eggsec::tool::registry::ToolRegistry;
use eggsec::tool::traits::{SecurityTool, ToolCapability, ToolCategory};
struct MockTool {
    id: &'static str,
    name: &'static str,
    description: &'static str,
    category: ToolCategory,
    capabilities: Vec<ToolCapability>,
}

impl MockTool {
    fn new(id: &'static str, name: &'static str, category: ToolCategory) -> Self {
        Self {
            id,
            name,
            description: "Mock tool",
            category,
            capabilities: vec![],
        }
    }
}

#[async_trait::async_trait]
impl SecurityTool for MockTool {
    fn id(&self) -> &'static str {
        self.id
    }

    fn name(&self) -> &'static str {
        self.name
    }

    fn description(&self) -> &'static str {
        self.description
    }

    fn category(&self) -> ToolCategory {
        self.category
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        self.capabilities.clone()
    }

    fn supported_protocols(&self) -> Vec<&'static str> {
        vec!["http", "https"]
    }

    async fn execute(
        &self,
        _request: eggsec::tool::ToolRequest,
    ) -> eggsec::tool::ToolResult<eggsec::tool::ToolResponse> {
        unreachable!("mock registry tools are not executed")
    }
}

#[test]
fn test_registry_new() {
    let registry = ToolRegistry::new();
    let tools = registry.list();
    assert!(tools.is_empty());
}

#[test]
fn test_registry_register() {
    let registry = ToolRegistry::new();
    let result = registry.register(MockTool::new(
        "test",
        "Test Tool",
        ToolCategory::Recon,
    ));
    assert!(result.is_ok());
}

#[test]
fn test_registry_register_duplicate() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new(
            "test",
            "Test Tool",
            ToolCategory::Recon,
        ))
        .unwrap();
    let result = registry.register(MockTool::new("test", "Test Tool 2", ToolCategory::Scanning));
    assert!(result.is_err());
}

#[test]
fn test_registry_unregister() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new(
            "test",
            "Test Tool",
            ToolCategory::Recon,
        ))
        .unwrap();
    let removed = registry.unregister("test");
    assert!(removed.is_some());
    assert_eq!(removed.unwrap().id(), "test");
}

#[test]
fn test_registry_unregister_not_found() {
    let registry = ToolRegistry::new();
    let removed = registry.unregister("nonexistent");
    assert!(removed.is_none());
}

#[test]
fn test_registry_get() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new(
            "test",
            "Test Tool",
            ToolCategory::Recon,
        ))
        .unwrap();
    let tool = registry.get("test");
    assert!(tool.is_some());
    assert_eq!(tool.unwrap().id(), "test");
}

#[test]
fn test_registry_get_not_found() {
    let registry = ToolRegistry::new();
    let tool = registry.get("nonexistent");
    assert!(tool.is_none());
}

#[test]
fn test_registry_list() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new(
            "test1",
            "Test Tool 1",
            ToolCategory::Recon,
        ))
        .unwrap();
    registry
        .register(MockTool::new("test2", "Test Tool 2", ToolCategory::Scanning))
        .unwrap();

    let tools = registry.list();
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_registry_list_by_category() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new(
            "recon",
            "Recon",
            ToolCategory::Recon,
        ))
        .unwrap();
    registry
        .register(MockTool::new("scan", "Scanner", ToolCategory::Scanning))
        .unwrap();

    let recon_tools = registry.list_by_category(ToolCategory::Recon);
    assert_eq!(recon_tools.len(), 1);
    assert_eq!(recon_tools[0].id, "recon");
}

#[test]
fn test_registry_categories() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new(
            "recon",
            "Recon",
            ToolCategory::Recon,
        ))
        .unwrap();
    registry
        .register(MockTool::new("scan", "Scanner", ToolCategory::Scanning))
        .unwrap();
    registry
        .register(MockTool::new("fuzz", "Fuzzer", ToolCategory::Fuzzing))
        .unwrap();

    let categories = registry.categories();
    assert!(categories.contains(&ToolCategory::Recon));
    assert!(categories.contains(&ToolCategory::Scanning));
    assert!(categories.contains(&ToolCategory::Fuzzing));
}

#[test]
fn test_registry_clone() {
    let registry = ToolRegistry::new();
    registry
        .register(MockTool::new("test", "Test", ToolCategory::Recon))
        .unwrap();

    let cloned = registry.clone();
    assert_eq!(cloned.list().len(), 1);
}

#[tokio::test]
async fn test_registry_concurrent_access() {
    use std::sync::Arc;
    use tokio::task;

    let registry = Arc::new(ToolRegistry::new());

    const IDS: [&str; 10] = [
        "tool0", "tool1", "tool2", "tool3", "tool4", "tool5", "tool6", "tool7", "tool8",
        "tool9",
    ];
    const NAMES: [&str; 10] = [
        "Tool 0", "Tool 1", "Tool 2", "Tool 3", "Tool 4", "Tool 5", "Tool 6", "Tool 7",
        "Tool 8", "Tool 9",
    ];

    let mut handles = Vec::new();
    for (id, name) in IDS.into_iter().zip(NAMES) {
        let reg = registry.clone();
        let handle = task::spawn(async move {
            reg.register(MockTool::new(id, name, ToolCategory::Recon))
        });
        handles.push(handle);
    }

    for handle in handles {
        handle.await.unwrap().unwrap();
    }

    assert_eq!(registry.list().len(), 10);
}
