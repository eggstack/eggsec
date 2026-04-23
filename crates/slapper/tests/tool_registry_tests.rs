#![cfg(feature = "rest-api")]

use slapper::tool::registry::ToolRegistry;
use slapper::tool::traits::{SecurityTool, ToolCapability, ToolCategory};
use std::sync::Arc;
use std::time::Duration;

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
            description: format!("Mock {} tool", name),
            category,
            capabilities: vec![],
        }
    }
}

impl SecurityTool for MockTool {
    fn id(&self) -> &str {
        self.id
    }

    fn name(&self) -> &str {
        self.name
    }

    fn description(&self) -> &str {
        self.description.as_str()
    }

    fn category(&self) -> ToolCategory {
        self.category
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        self.capabilities.clone()
    }

    fn supported_protocols(&self) -> Vec<String> {
        vec!["http".to_string(), "https".to_string()]
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
    let result = registry.register(MockTool::new("test", "Test Tool", ToolCategory::Reconnaissance));
    assert!(result.is_ok());
}

#[test]
fn test_registry_register_duplicate() {
    let registry = ToolRegistry::new();
    registry.register(MockTool::new("test", "Test Tool", ToolCategory::Reconnaissance)).unwrap();
    let result = registry.register(MockTool::new("test", "Test Tool 2", ToolCategory::Scanner));
    assert!(result.is_err());
}

#[test]
fn test_registry_unregister() {
    let registry = ToolRegistry::new();
    registry.register(MockTool::new("test", "Test Tool", ToolCategory::Reconnaissance)).unwrap();
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
    registry.register(MockTool::new("test", "Test Tool", ToolCategory::Reconnaissance)).unwrap();
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
    registry.register(MockTool::new("test1", "Test Tool 1", ToolCategory::Reconnaissance)).unwrap();
    registry.register(MockTool::new("test2", "Test Tool 2", ToolCategory::Scanner)).unwrap();
    
    let tools = registry.list();
    assert_eq!(tools.len(), 2);
}

#[test]
fn test_registry_list_by_category() {
    let registry = ToolRegistry::new();
    registry.register(MockTool::new("recon", "Recon", ToolCategory::Reconnaissance)).unwrap();
    registry.register(MockTool::new("scan", "Scanner", ToolCategory::Scanner)).unwrap();
    
    let recon_tools = registry.list_by_category(ToolCategory::Reconnaissance);
    assert_eq!(recon_tools.len(), 1);
    assert_eq!(recon_tools[0].id, "recon");
}

#[test]
fn test_registry_categories() {
    let registry = ToolRegistry::new();
    registry.register(MockTool::new("recon", "Recon", ToolCategory::Reconnaissance)).unwrap();
    registry.register(MockTool::new("scan", "Scanner", ToolCategory::Scanner)).unwrap();
    registry.register(MockTool::new("fuzz", "Fuzzer", ToolCategory::Fuzzer)).unwrap();
    
    let categories = registry.categories();
    assert!(categories.contains(&ToolCategory::Reconnaissance));
    assert!(categories.contains(&ToolCategory::Scanner));
    assert!(categories.contains(&ToolCategory::Fuzzer));
}

#[test]
fn test_registry_clone() {
    let registry = ToolRegistry::new();
    registry.register(MockTool::new("test", "Test", ToolCategory::Reconnaissance)).unwrap();
    
    let cloned = registry.clone();
    assert_eq!(cloned.list().len(), 1);
}

#[tokio::test]
async fn test_registry_concurrent_access() {
    use std::sync::Arc;
    use tokio::task;

    let registry = Arc::new(ToolRegistry::new());
    
    let mut handles = Vec::new();
    for i in 0..10 {
        let reg = registry.clone();
        let handle = task::spawn(async move {
            reg.register(MockTool::new(
                format!("tool{}", i), 
                format!("Tool {}", i), 
                ToolCategory::Reconnaissance
            ))
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap().unwrap();
    }
    
    assert_eq!(registry.list().len(), 10);
}