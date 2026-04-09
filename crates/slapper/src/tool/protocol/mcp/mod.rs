mod auth;
mod handlers;
#[cfg(feature = "rest-api")]
pub mod prompts;
pub mod routes;
#[cfg(feature = "ai-integration")]
pub mod sampling;
mod streaming;
mod types;

pub use handlers::McpServer;
pub use routes::{create_mcp_router, run_stdio};
pub use streaming::StreamEvent;
pub use types::{CapabilitySummary, McpError, McpRequest, McpResource, McpResponse, McpTool};

#[cfg(test)]
mod tests {
    use crate::tool::protocol::mcp::{McpRequest, McpResponse};
    use crate::tool::{
        ChainPlanner, create_default_registry, OpenApiGenerator, PlanRequest, 
        protocol::mcp::McpServer,
    };

    fn create_test_server() -> McpServer {
        let registry = create_default_registry();
        McpServer::new(registry, Some("test-api-key".to_string()))
    }

    #[tokio::test]
    async fn test_initialize() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        
        let result = response.result.unwrap();
        assert!(result.get("serverInfo").is_some());
        assert!(result.get("capabilities").is_some());
    }

    #[tokio::test]
    async fn test_tools_list() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key"
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        assert!(response.result.is_some());
        
        let result = response.result.unwrap();
        assert!(result.get("tools").is_some());
        assert!(result.get("count").is_some());
    }

    #[tokio::test]
    async fn test_tools_list_by_category() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list-by-category".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key"
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("categories").is_some());
        assert!(result.get("total_tools").is_some());
    }

    #[tokio::test]
    async fn test_ping() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "ping".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert_eq!(result.get("status").unwrap(), "ok");
    }

    #[tokio::test]
    async fn test_session_create() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "session/create".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "target": "https://example.com"
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("session_id").is_some());
        assert!(result.get("status").is_some());
    }

    #[tokio::test]
    async fn test_rate_limit_status() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "rate-limit/status".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key"
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("requests_per_minute").is_some());
        assert!(result.get("concurrent_limit").is_some());
    }

    #[tokio::test]
    async fn test_resources_list() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "resources/list".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("resources").is_some());
    }

    #[tokio::test]
    async fn test_resources_read_manifest() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "resources/read".to_string(),
            params: Some(serde_json::json!({
                "uri": "slapper://manifest"
            })),
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_none());
        
        let result = response.result.unwrap();
        assert!(result.get("contents").is_some());
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let server = create_test_server();
        
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "unknown/method".to_string(),
            params: None,
        };
        
        let response = server.handle_request(request).await;
        
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32601);
    }

    #[tokio::test]
    async fn test_authorization() {
        let server = create_test_server();
        
        assert!(server.validate_auth_params(&Some(serde_json::json!({
            "api_key": "wrong-key"
        }))).is_err());
    }

    #[tokio::test]
    async fn test_auth_with_correct_key() {
        let server = create_test_server();
        
        assert!(server.validate_auth_params(&Some(serde_json::json!({
            "api_key": "test-api-key"
        }))).is_ok());
    }

    #[tokio::test]
    async fn test_planner_integration() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);
        
        let request = PlanRequest {
            goal: "full_assessment".to_string(),
            target: "https://example.com".to_string(),
            ..Default::default()
        };
        
        let plan = planner.plan(&request);
        assert!(!plan.stages.is_empty());
        assert!(plan.total_tools() > 0);
        
        let validation = planner.validate_plan(&plan);
        assert!(validation.valid);
    }

    #[tokio::test]
    async fn test_planner_recon_only() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);
        
        let request = PlanRequest {
            goal: "recon".to_string(),
            target: "https://example.com".to_string(),
            ..Default::default()
        };
        
        let plan = planner.plan(&request);
        assert!(!plan.stages.is_empty());
        
        let validation = planner.validate_plan(&plan);
        assert!(validation.valid);
    }

    #[tokio::test]
    async fn test_planner_vuln_scan() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);
        
        let request = PlanRequest {
            goal: "vuln_scan".to_string(),
            target: "https://api.example.com".to_string(),
            ..Default::default()
        };
        
        let plan = planner.plan(&request);
        assert!(!plan.stages.is_empty());
        
        let stage_names: Vec<&str> = plan.stages.iter().map(|s| s.name.as_str()).collect();
        assert!(stage_names.contains(&"reconnaissance"));
        assert!(stage_names.contains(&"vulnerability_scanning"));
    }

    #[tokio::test]
    async fn test_planner_api_security() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);
        
        let request = PlanRequest {
            goal: "api".to_string(),
            target: "https://api.example.com".to_string(),
            ..Default::default()
        };
        
        let plan = planner.plan(&request);
        assert!(!plan.stages.is_empty());
        
        let stage_names: Vec<&str> = plan.stages.iter().map(|s| s.name.as_str()).collect();
        assert!(stage_names.contains(&"api_security"));
    }

    #[tokio::test]
    async fn test_planner_quick_scan() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);
        
        let request = PlanRequest {
            goal: "quick".to_string(),
            target: "https://example.com".to_string(),
            ..Default::default()
        };
        
        let plan = planner.plan(&request);
        assert!(!plan.stages.is_empty());
        
        let validation = planner.validate_plan(&plan);
        assert!(validation.valid);
    }

    #[tokio::test]
    async fn test_openapi_generation() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
        let spec = generator.generate(&registry);
        
        assert_eq!(spec.openapi, "3.1.0");
        assert!(!spec.paths.is_empty());
        assert!(spec.paths.contains_key("/health"));
    }

    #[tokio::test]
    async fn test_openapi_has_required_paths() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
        let spec = generator.generate(&registry);
        
        assert!(spec.paths.contains_key("/mcp"));
        assert!(spec.paths.contains_key("/health"));
        assert!(!spec.components.schemas.is_empty());
    }

    #[tokio::test]
    async fn test_openapi_json_output() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
        let spec = generator.generate(&registry);
        
        let json = spec.to_json();
        assert!(json.contains("openapi"));
        assert!(json.contains("Slapper"));
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
    }

    #[tokio::test]
    async fn test_openapi_yaml_output() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
        let spec = generator.generate(&registry);
        
        let yaml = spec.to_yaml();
        assert!(yaml.contains("openapi:"));
        assert!(yaml.contains("Slapper"));
    }

    #[tokio::test]
    async fn test_tool_suggestions() {
        use crate::tool::create_default_registry;
        
        let registry = create_default_registry();
        let planner = ChainPlanner::new(registry);
        
        let web_tools = planner.suggest_tools_for_attack_surface(crate::tool::AttackSurface::Web);
        assert!(!web_tools.is_empty());
        
        let api_tools = planner.suggest_tools_for_attack_surface(crate::tool::AttackSurface::Api);
        assert!(!api_tools.is_empty());
        
        let network_tools = planner.suggest_tools_for_attack_surface(crate::tool::AttackSurface::Network);
        assert!(!network_tools.is_empty());
    }
}
