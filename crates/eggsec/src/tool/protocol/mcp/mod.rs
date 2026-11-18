mod auth;
pub mod bridge;
pub mod coding_agent_output;
mod constraints;
mod handlers;
pub mod policy;
pub mod profile;
#[cfg(feature = "rest-api")]
pub mod prompts;
pub mod routes;

mod streaming;
mod types;

pub use auth::{validate_auth, validate_auth_internal, validate_auth_params};
pub use bridge::McpEngineBridge;
pub use constraints::McpConstraintContext;
pub use handlers::McpServer;
pub use policy::{
    classify_tool_risk, McpProfilePolicy, PolicyViolation, TargetPolicy, ToolSelector,
};
pub use profile::McpProfile;
pub use routes::{
    create_mcp_router, create_mcp_router_with_allowed_tools, run_stdio,
    run_stdio_with_allowed_tools,
};
pub use streaming::StreamEvent;
pub use types::{
    CapabilitySummary, McpError, McpNotification, McpRequest, McpResource, McpResponse, McpRoot,
    McpTool,
};

#[cfg(test)]
mod tests {
    use crate::tool::protocol::mcp::McpRequest;
    use crate::tool::{
        create_default_registry, protocol::mcp::McpServer, ChainPlanner, OpenApiGenerator,
        PlanRequest,
    };

    struct ContextAwareLoadTestTool;

    #[async_trait::async_trait]
    impl crate::tool::traits::SecurityTool for ContextAwareLoadTestTool {
        fn id(&self) -> &'static str {
            "load"
        }

        fn name(&self) -> &'static str {
            "Context-aware test load tool"
        }

        fn category(&self) -> crate::tool::ToolCategory {
            crate::tool::ToolCategory::LoadTest
        }

        fn description(&self) -> &'static str {
            "Test double that requires the approved scope context."
        }

        async fn execute(
            &self,
            _request: crate::tool::ToolRequest,
        ) -> crate::tool::ToolResult<crate::tool::ToolResponse> {
            Err(crate::error::EggsecError::Validation(
                "scope context was not supplied".to_string(),
            ))
        }

        async fn execute_with_context(
            &self,
            request: crate::tool::ToolRequest,
            context: &crate::tool::ToolExecutionContext,
        ) -> crate::tool::ToolResult<crate::tool::ToolResponse> {
            Ok(crate::tool::ToolResponse::success(
                request.id,
                self.id(),
                serde_json::json!({
                    "scope_rule_count": context.scope.allowed_targets.len()
                }),
            ))
        }
    }

    fn create_test_server() -> McpServer {
        let registry = create_default_registry();
        let selected = registry.list().into_iter().map(|tool| tool.id).collect();
        McpServer::new(registry, Some("test-api-key".to_string())).with_allowed_tool_ids(selected)
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
    async fn test_operator_selected_tool_set_is_fail_closed_at_discovery_and_call_time() {
        let registry = create_default_registry();
        let empty_server = McpServer::new(registry.clone(), Some("test-api-key".to_string()));
        let list_request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".to_string(),
            params: Some(serde_json::json!({"api_key": "test-api-key"})),
        };
        let empty_response = empty_server.handle_request(list_request.clone()).await;
        let empty_result = empty_response.result.unwrap();
        assert_eq!(empty_result["count"], 0);

        let selected_server = McpServer::new(registry, Some("test-api-key".to_string()))
            .with_allowed_tool_ids(vec!["scan-ports".to_string()]);
        let selected_response = selected_server.handle_request(list_request.clone()).await;
        let selected_result = selected_response.result.unwrap();
        let names: Vec<&str> = selected_result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert!(names.contains(&"scan-ports"));
        assert!(!names.contains(&"fingerprint"));

        let alias_server =
            McpServer::new(create_default_registry(), Some("test-api-key".to_string()))
                .with_allowed_tool_ids(vec!["scan".to_string(), "load".to_string()]);
        let alias_response = alias_server.handle_request(list_request).await;
        let alias_result = alias_response.result.expect("alias listing should succeed");
        let alias_names: Vec<&str> = alias_result["tools"]
            .as_array()
            .expect("tools should be an array")
            .iter()
            .filter_map(|tool| tool["name"].as_str())
            .collect();
        assert_eq!(alias_names.len(), 2);
        assert!(alias_names.contains(&"scan"));
        assert!(alias_names.contains(&"load"));
        assert!(!alias_names.contains(&"eggsec_preflight"));

        let preflight_call = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(3)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "eggsec_preflight"
            })),
        };
        let rejected_preflight = alias_server.handle_request(preflight_call).await;
        let preflight_error = rejected_preflight
            .error
            .expect("unregistered synthetic MCP tools must be rejected");
        assert!(preflight_error
            .message
            .contains("Unknown tool or capability"));

        let unselected_call = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(2)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "fingerprint",
                "arguments": {"target": "https://example.com"}
            })),
        };
        let denied = selected_server.handle_request(unselected_call).await;
        assert_eq!(denied.error.unwrap().code, -32020);
    }

    #[tokio::test]
    async fn scope_sensitive_load_dispatch_receives_approved_scope_snapshot() {
        use crate::config::{
            Capability, EnforcementContext, ExecutionPolicy, LoadedScope, Scope, ScopeRule,
            ScopeSource,
        };
        use crate::tool::protocol::mcp::McpProfile;
        use crate::tool::ToolRegistry;

        let registry = ToolRegistry::new();
        registry
            .register(ContextAwareLoadTestTool)
            .expect("test tool should register");

        let mut scope = Scope::default();
        scope
            .allowed_targets
            .push(ScopeRule::new("127.0.0.1".to_string()));
        let loaded_scope = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);

        let mut execution_policy = ExecutionPolicy::default();
        execution_policy.allow_load_testing = true;
        execution_policy
            .allowed_capabilities
            .push(Capability::LoadTest);
        let enforcement = EnforcementContext::mcp_strict(execution_policy, loaded_scope);
        let server = McpServer::with_enforcement(
            registry,
            Some("test-api-key".to_string()),
            McpProfile::OpsAgent,
            enforcement,
        )
        .with_allowed_tool_ids(vec!["load".to_string()]);

        let response = server
            .handle_request(McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(serde_json::json!(1)),
                method: "tools/call".to_string(),
                params: Some(serde_json::json!({
                    "api_key": "test-api-key",
                    "name": "load",
                    "arguments": {"target": "http://127.0.0.1"}
                })),
            })
            .await;

        assert!(
            response.error.is_none(),
            "MCP call should execute: {:?}",
            response.error
        );
        let result = response.result.expect("successful MCP result");
        assert_eq!(result["isError"], false);
        let text = result["content"][0]["text"]
            .as_str()
            .expect("tool response text");
        assert!(text.contains("\"scope_rule_count\": 1"), "{text}");
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
                "uri": "eggsec://manifest"
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
    async fn test_roots_list() {
        let server = create_test_server();

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "roots/list".to_string(),
            params: None,
        };

        let response = server.handle_request(request).await;

        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert!(result.get("roots").is_some());
        assert!(result.get("count").is_some());
    }

    #[tokio::test]
    async fn test_shutdown() {
        let server = create_test_server();

        assert!(!server.is_shutdown_requested());

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "shutdown".to_string(),
            params: None,
        };

        let response = server.handle_request(request).await;

        assert!(response.error.is_none());
        assert!(response.result.is_some());

        let result = response.result.unwrap();
        assert_eq!(result.get("success").unwrap(), &serde_json::json!(true));

        assert!(server.is_shutdown_requested());
    }

    #[tokio::test]
    async fn test_authorization() {
        let server = create_test_server();

        assert!(server
            .validate_auth_params(&Some(serde_json::json!({
                "api_key": "wrong-key"
            })))
            .is_err());
    }

    #[tokio::test]
    async fn test_auth_with_correct_key() {
        let server = create_test_server();

        assert!(server
            .validate_auth_params(&Some(serde_json::json!({
                "api_key": "test-api-key"
            })))
            .is_ok());
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
        assert!(json.contains("Eggsec"));
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
        assert!(yaml.contains("Eggsec"));
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

        let network_tools =
            planner.suggest_tools_for_attack_surface(crate::tool::AttackSurface::Network);
        assert!(!network_tools.is_empty());
    }

    // Phase 12: Coding-agent profile tests

    fn create_coding_agent_server() -> McpServer {
        let selected = crate::tool::registration::mcp_tool_registrations("coding-agent")
            .into_iter()
            .map(|registration| registration.tool_id.to_string())
            .collect();
        create_coding_agent_server_with_selection(selected)
    }

    fn create_coding_agent_server_with_selection(selected: Vec<String>) -> McpServer {
        let registry = create_default_registry();
        let enforcement = crate::config::EnforcementContext::mcp_strict(
            crate::config::ExecutionPolicy::default(),
            crate::config::LoadedScope::default_empty(),
        );
        McpServer::with_enforcement(
            registry,
            Some("test-api-key".to_string()),
            super::profile::McpProfile::CodingAgent,
            enforcement,
        )
        .with_allowed_tool_ids(selected)
    }

    #[tokio::test]
    async fn coding_agent_resources_reflect_operator_selection_and_loaded_scope() {
        async fn read_resource(server: &McpServer, uri: &str) -> serde_json::Value {
            let request = McpRequest {
                jsonrpc: "2.0".to_string(),
                id: Some(serde_json::json!(1)),
                method: "resources/read".to_string(),
                params: Some(serde_json::json!({"api_key": "test-api-key", "uri": uri})),
            };
            let response = server.handle_request(request).await;
            assert!(response.error.is_none(), "resource read should succeed");
            let result = response.result.expect("resource result should be present");
            let text = result["contents"][0]["text"]
                .as_str()
                .expect("resource text should be a string");
            serde_json::from_str(text).expect("resource text should contain JSON")
        }

        let server = create_coding_agent_server_with_selection(vec!["recon".to_string()]);
        let manifest = read_resource(&server, "eggsec://coding-agent/manifest").await;
        assert_eq!(
            manifest["tool_selection"]["tool_ids"],
            serde_json::json!(["recon"])
        );
        assert_eq!(manifest["tools"].as_array().unwrap().len(), 1);
        assert_eq!(manifest["tools"][0]["tool_id"], "recon");

        let safety = read_resource(&server, "eggsec://coding-agent/safety-policy").await;
        assert_eq!(safety["target_authority"]["requires_explicit_scope"], true);
        assert_eq!(
            safety["tool_selection"]["tool_ids"],
            serde_json::json!(["recon"])
        );
        assert!(safety.get("default_allowlist").is_none());

        let workflow = read_resource(&server, "eggsec://coding-agent/workflow").await;
        assert_eq!(workflow["allowed_tools"], serde_json::json!(["recon"]));
        assert_eq!(
            workflow["steps"][1]["tool_options"],
            serde_json::json!(["recon"])
        );

        let contracts = read_resource(&server, "eggsec://coding-agent/tool-contracts").await;
        assert_eq!(contracts["contracts"].as_array().unwrap().len(), 1);
        assert_eq!(contracts["contracts"][0]["tool_id"], "recon");
    }

    #[tokio::test]
    async fn test_coding_agent_tools_list_returns_only_allowed() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list".to_string(),
            params: Some(serde_json::json!({"api_key": "test-api-key"})),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let tools = result.get("tools").unwrap().as_array().unwrap();
        let count = result.get("count").unwrap().as_u64().unwrap();

        assert!(count > 0, "operator-selected MCP tools should be listed");
        let names: Vec<&str> = tools
            .iter()
            .filter_map(|t| t.get("name").and_then(|n| n.as_str()))
            .collect();
        let metadata_ids: Vec<String> =
            crate::tool::registration::mcp_tool_registrations("coding-agent")
                .into_iter()
                .map(|registration| registration.tool_id.to_string())
                .collect();
        for name in &names {
            assert!(metadata_ids.iter().any(|id| id.as_str() == *name));
        }
    }

    #[tokio::test]
    async fn test_coding_agent_tools_list_by_category_returns_only_allowed() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/list-by-category".to_string(),
            params: Some(serde_json::json!({"api_key": "test-api-key"})),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let categories = result.get("categories").unwrap().as_object().unwrap();

        // Should not have stress or load testing categories
        assert!(
            !categories.contains_key("stress"),
            "coding-agent should not expose stress testing"
        );
        assert!(
            !categories.contains_key("loadtest"),
            "coding-agent should not expose load testing"
        );
    }

    #[tokio::test]
    async fn test_coding_agent_tool_call_denied_tool_returns_policy_error() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "waf-stress",
                "arguments": {"target": "http://localhost:8080"}
            })),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32020); // ToolDenied error code
    }

    #[tokio::test]
    async fn test_coding_agent_tool_call_denied_argument_returns_policy_error() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "scan-ports",
                "arguments": {"target": "http://localhost:8080", "stealth": true}
            })),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32021); // ArgumentDenied error code
    }

    #[tokio::test]
    async fn test_coding_agent_tool_call_excessive_concurrency_returns_error() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "scan-ports",
                "arguments": {"target": "http://localhost:8080", "concurrency": 100}
            })),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32022); // ConcurrencyExceeded error code
    }

    #[tokio::test]
    async fn test_coding_agent_tool_call_public_target_returns_error() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "search",
                "arguments": {"target": "https://example.com"}
            })),
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_some());
        let error = response.error.unwrap();
        assert_eq!(error.code, -32025); // Strict enforcement / LoadedScope denial
    }

    #[tokio::test]
    async fn test_coding_agent_initialize_returns_correct_profile_metadata() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        let result = response.result.unwrap();

        assert_eq!(result.get("profile").unwrap(), "coding-agent");
        assert_eq!(
            result.get("serverInfo").unwrap().get("name").unwrap(),
            "eggsec-coding-agent-mcp"
        );
        let safety = result.get("safety").unwrap();
        assert_eq!(safety.get("max_concurrency").unwrap(), 5);
        assert_eq!(safety.get("max_timeout_ms").unwrap(), 60000);
        assert_eq!(safety.get("default_external_network").unwrap(), true);
        assert_eq!(safety.get("stress_testing_available").unwrap(), false);
    }

    #[tokio::test]
    async fn test_ops_agent_initialize_returns_correct_profile_metadata() {
        let server = create_test_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "initialize".to_string(),
            params: None,
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        let result = response.result.unwrap();

        assert_eq!(result.get("profile").unwrap(), "ops-agent");
        assert_eq!(
            result.get("serverInfo").unwrap().get("name").unwrap(),
            "eggsec-tool-api"
        );
        let safety = result.get("safety").unwrap();
        assert_eq!(safety.get("max_concurrency").unwrap(), 50);
        assert_eq!(safety.get("default_external_network").unwrap(), true);
        assert_eq!(safety.get("stress_testing_available").unwrap(), true);
    }

    #[tokio::test]
    async fn test_coding_agent_resources_list_returns_coding_resources() {
        let server = create_coding_agent_server();
        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "resources/list".to_string(),
            params: None,
        };

        let response = server.handle_request(request).await;
        assert!(response.error.is_none());
        let result = response.result.unwrap();
        let resources = result.get("resources").unwrap().as_array().unwrap();
        let uris: Vec<&str> = resources
            .iter()
            .filter_map(|r| r.get("uri").and_then(|u| u.as_str()))
            .collect();

        assert!(
            uris.iter().any(|u| u.starts_with("eggsec://coding-agent/")),
            "should have coding-agent resources"
        );
        assert!(
            !uris.iter().any(|u| u.starts_with("eggsec://ops-agent/")),
            "should not have ops-agent resources"
        );
    }

    #[tokio::test]
    async fn test_ops_agent_resources_list_returns_ops_resources() {
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
        let resources = result.get("resources").unwrap().as_array().unwrap();
        let uris: Vec<&str> = resources
            .iter()
            .filter_map(|r| r.get("uri").and_then(|u| u.as_str()))
            .collect();

        assert!(
            uris.iter().any(|u| u.starts_with("eggsec://ops-agent/")),
            "should have ops-agent resources"
        );
        assert!(
            !uris.iter().any(|u| u.starts_with("eggsec://coding-agent/")),
            "should not have coding-agent resources"
        );
    }

    #[tokio::test]
    async fn test_profile_serde_roundtrip() {
        use super::profile::McpProfile;

        let ops = McpProfile::OpsAgent;
        let coding = McpProfile::CodingAgent;

        let ops_json = serde_json::to_string(&ops).unwrap();
        let coding_json = serde_json::to_string(&coding).unwrap();

        let ops_de: McpProfile = serde_json::from_str(&ops_json).unwrap();
        let coding_de: McpProfile = serde_json::from_str(&coding_json).unwrap();

        assert_eq!(ops_de, McpProfile::OpsAgent);
        assert_eq!(coding_de, McpProfile::CodingAgent);
    }

    #[tokio::test]
    async fn test_transport_single_jsonrpc_object_deserializes() {
        // McpIncoming is private in routes.rs, so test that McpRequest deserializes correctly
        let single = serde_json::json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "ping"
        });
        let req: McpRequest = serde_json::from_value(single).unwrap();
        assert_eq!(req.method, "ping");
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.id, Some(serde_json::json!(1)));
    }

    #[tokio::test]
    async fn test_transport_batch_array_deserializes() {
        // Test that a batch of McpRequest deserializes correctly
        let batch = serde_json::json!([
            {"jsonrpc": "2.0", "id": 1, "method": "ping"},
            {"jsonrpc": "2.0", "id": 2, "method": "initialize"}
        ]);
        let requests: Vec<McpRequest> = serde_json::from_value(batch).unwrap();
        assert_eq!(requests.len(), 2);
        assert_eq!(requests[0].method, "ping");
        assert_eq!(requests[1].method, "initialize");
    }

    // Pass 6: dispatch-prevention regression tests for MCP (per final-enforcement-cleanup-plan)
    // These prove that `EnforcementContext::evaluate()` is the pre-dispatch gate for production `with_enforcement` paths.

    use crate::config::{EnforcementContext, ExecutionPolicy, LoadedScope};
    use crate::tool::protocol::mcp::profile::McpProfile;
    use crate::tool::traits::{SecurityTool, ToolCategory};
    use crate::tool::{ToolRegistry, ToolRequest, ToolResponse, ToolResult};
    use async_trait::async_trait;

    struct DispatchRecordingTool {
        called: std::sync::Arc<std::sync::atomic::AtomicBool>,
    }

    #[async_trait]
    impl SecurityTool for DispatchRecordingTool {
        fn id(&self) -> &'static str {
            // Both test functions register this tool in an empty registry,
            // so this ID is the only registered tool. "recon" has metadata
            // with no required_features and only baseline capabilities.
            "recon"
        }
        fn name(&self) -> &'static str {
            "Dispatch Recording Tool"
        }
        fn category(&self) -> ToolCategory {
            ToolCategory::Recon
        }
        fn description(&self) -> &'static str {
            "Test double that records if execute was reached"
        }

        async fn execute(&self, _request: ToolRequest) -> ToolResult<ToolResponse> {
            self.called.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(ToolResponse::success(
                "rec-1",
                self.id(),
                serde_json::json!({"reached": true}),
            ))
        }
    }

    #[tokio::test]
    async fn test_mcp_enforcement_with_default_empty_denies_networked_call_and_prevents_dispatch() {
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let flag = Arc::new(AtomicBool::new(false));
        let mut registry = ToolRegistry::new();
        registry
            .register(DispatchRecordingTool {
                called: flag.clone(),
            })
            .unwrap();

        let server = McpServer::with_enforcement(
            registry,
            Some("test-api-key".to_string()),
            McpProfile::OpsAgent,
            EnforcementContext::mcp_strict(
                ExecutionPolicy::default(),
                LoadedScope::default_empty(),
            ),
        )
        .with_allowed_tool_ids(vec!["recon".to_string()]);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "recon",
                "arguments": {"target": "https://example.com"}
            })),
        };

        let response = server.handle_request(request).await;
        assert!(
            response.error.is_some(),
            "expected enforcement denial error"
        );
        let error = response.error.unwrap();
        // Enforcement denials in handle_tools_call use code -32025
        assert_eq!(error.code, -32025);
        assert!(
            !flag.load(Ordering::SeqCst),
            "dispatch must not have been reached on denial"
        );
    }

    #[tokio::test]
    async fn test_mcp_enforcement_with_explicit_scope_allows_and_reaches_dispatch() {
        use crate::config::{Scope, ScopeRule, ScopeSource};
        use std::sync::atomic::{AtomicBool, Ordering};
        use std::sync::Arc;

        let flag = Arc::new(AtomicBool::new(false));
        let mut registry = ToolRegistry::new();
        registry
            .register(DispatchRecordingTool {
                called: flag.clone(),
            })
            .unwrap();

        let mut scope = Scope::default();
        scope
            .allowed_targets
            .push(ScopeRule::new("example.com".to_string()));
        let loaded = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);

        let policy = ExecutionPolicy::default();

        let server = McpServer::with_enforcement(
            registry,
            Some("test-api-key".to_string()),
            McpProfile::OpsAgent,
            EnforcementContext::mcp_strict(policy, loaded),
        )
        .with_allowed_tool_ids(vec!["recon".to_string()]);

        let request = McpRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(serde_json::json!(1)),
            method: "tools/call".to_string(),
            params: Some(serde_json::json!({
                "api_key": "test-api-key",
                "name": "recon",
                "arguments": {"target": "https://example.com"}
            })),
        };

        let response = server.handle_request(request).await;
        assert!(
            response.error.is_none(),
            "expected success with explicit in-scope manifest"
        );
        assert!(
            flag.load(Ordering::SeqCst),
            "dispatch should have been reached for allowed call"
        );
    }

    #[test]
    fn test_operation_descriptor_for_mcp_call_is_single_source_for_pre_dispatch() {
        // Exercises the helper that is now the only production construction path in handle_tools_call.
        use crate::tool::protocol::mcp::policy::operation_descriptor_for_mcp_call;
        use crate::tool::protocol::mcp::policy::McpProfilePolicy;

        let policy = McpProfilePolicy::for_profile(McpProfile::OpsAgent);
        let args = serde_json::json!({"target": "https://example.com"});
        let desc = operation_descriptor_for_mcp_call(&policy, "scan", None, &args).unwrap();

        // Metadata lookup resolves the runtime "scan" alias to the pipeline operation.
        assert_eq!(desc.operation, "pipeline");
        assert!(desc.target.is_some());
        assert!(desc.requires_explicit_scope); // Ops and Coding both set true
                                               // required_capabilities populated by the helper (exact contents depend on registry mapping; non-emptiness for scan is typical)
    }

    #[test]
    fn production_mcp_handler_uses_operation_descriptor_helper_not_inline_construction() {
        // Guard that the deduplication (Pass 1) holds: the enforcement block calls the helper.
        let src = include_str!("handlers/server.rs");
        assert!(
            src.contains("operation_descriptor_for_mcp_call("),
            "production handler must use the shared descriptor helper"
        );
        // The previous inline construction used a local `let risk = classify_tool_risk` + manual OperationDescriptor in the enforcement block.
        // After dedup that pattern is removed from the shared enforcement evaluation site.
        // We assert the old classify+manual combo no longer appears in that context by checking for the removed marker.
        // (classify_tool_risk may still exist in policy.rs; we look for the specific combination that was the hand-build.)
        let has_old_inline_hand_build = src.contains("let risk = classify_tool_risk(&tool_id)")
            && src.contains("OperationDescriptor {");
        assert!(
            !has_old_inline_hand_build,
            "handler must not hand-build OperationDescriptor in the shared enforcement block"
        );
    }
}
