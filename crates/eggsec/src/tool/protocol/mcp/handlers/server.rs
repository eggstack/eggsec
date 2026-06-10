use chrono::Utc;
use rustc_hash::FxHashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Interval};

#[cfg(feature = "rest-api")]
use crate::config::Scope;

use crate::tool::{
    CancellationToken, ExecutionHistory, RateLimitConfig, RateLimiter, RequestOptions,
    SessionManager, Target, ToolDispatcher, ToolRegistry, ToolRequest, ToolResponse,
};

#[cfg(feature = "ai-integration")]
use crate::ai::AiClient;

use crate::tool::protocol::mcp::auth::{validate_auth, validate_auth_params};
use crate::tool::protocol::mcp::handlers::helpers::{
    build_capabilities_summary, build_input_schema,
};
use crate::tool::protocol::mcp::policy::{policy_decision_for_mcp_call, McpProfilePolicy};
use crate::tool::protocol::mcp::profile::McpProfile;
use crate::tool::protocol::mcp::prompts::get_builtin_prompts_for_profile;
use crate::tool::protocol::mcp::streaming::StreamEvent;
use crate::tool::protocol::mcp::types::{
    McpError, McpRequest, McpResource, McpResponse, McpRoot, McpTool,
};

#[derive(Clone)]
pub struct McpServer {
    pub(crate) registry: ToolRegistry,
    dispatcher: ToolDispatcher,
    api_key: Option<String>,
    rate_limiter: RateLimiter,
    session_manager: Option<SessionManager>,
    pending_cancellations: Arc<RwLock<FxHashMap<String, CancellationToken>>>,
    completed_results: Arc<RwLock<FxHashMap<String, ToolResponse>>>,
    stream_events: Arc<tokio::sync::broadcast::Sender<StreamEvent>>,
    #[cfg(feature = "ai-integration")]
    ai_client: Option<AiClient>,
    #[cfg(feature = "rest-api")]
    scope: Option<Scope>,
    shutdown_requested: Arc<std::sync::atomic::AtomicBool>,
    pub(crate) profile: McpProfile,
    pub(crate) policy: McpProfilePolicy,
    pub(crate) execution_policy: crate::config::ExecutionPolicy,
}

impl McpServer {
    pub fn new(registry: ToolRegistry, api_key: Option<String>) -> Self {
        Self::with_scope_and_profile(registry, api_key, None, McpProfile::default())
    }

    pub fn with_scope(
        registry: ToolRegistry,
        api_key: Option<String>,
        scope: Option<Scope>,
    ) -> Self {
        Self::with_scope_and_profile(registry, api_key, scope, McpProfile::default())
    }

    pub fn with_scope_and_profile(
        registry: ToolRegistry,
        api_key: Option<String>,
        scope: Option<Scope>,
        profile: McpProfile,
    ) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        let (stream_events, _) = tokio::sync::broadcast::channel(1000);

        let pending_cancellations = Arc::new(RwLock::new(FxHashMap::default()));
        let completed_results = Arc::new(RwLock::new(FxHashMap::default()));
        let policy = McpProfilePolicy::for_profile(profile);

        let server = Self {
            registry,
            dispatcher,
            api_key,
            rate_limiter: RateLimiter::new(RateLimitConfig::default()),
            session_manager: None,
            pending_cancellations,
            completed_results,
            stream_events: Arc::new(stream_events),
            #[cfg(feature = "ai-integration")]
            ai_client: None,
            #[cfg(feature = "rest-api")]
            scope,
            shutdown_requested: Arc::new(std::sync::atomic::AtomicBool::new(false)),
            profile,
            policy,
            execution_policy: crate::config::ExecutionPolicy::default(),
        };

        server.start_hashmap_reaper(60);

        server
    }

    pub fn with_profile(mut self, profile: McpProfile) -> Self {
        self.profile = profile;
        self.policy = McpProfilePolicy::for_profile(self.profile);
        self
    }

    pub fn with_execution_policy(mut self, policy: crate::config::ExecutionPolicy) -> Self {
        self.execution_policy = policy;
        self
    }

    pub fn request_shutdown(&self) {
        self.shutdown_requested
            .store(true, std::sync::atomic::Ordering::SeqCst);
    }

    pub fn is_shutdown_requested(&self) -> bool {
        self.shutdown_requested
            .load(std::sync::atomic::Ordering::SeqCst)
    }

    pub fn with_session_manager(mut self, session_manager: SessionManager) -> Self {
        self.session_manager = Some(session_manager);
        self
    }

    pub fn with_rate_limiter(mut self, rate_limiter: RateLimiter) -> Self {
        self.rate_limiter = rate_limiter;
        self
    }

    pub fn with_history(self, history: ExecutionHistory) -> Self {
        let dispatcher = self.dispatcher.with_history(history);
        Self {
            registry: self.registry,
            dispatcher,
            api_key: self.api_key,
            rate_limiter: self.rate_limiter,
            session_manager: self.session_manager,
            pending_cancellations: self.pending_cancellations,
            completed_results: self.completed_results,
            stream_events: self.stream_events,
            #[cfg(feature = "ai-integration")]
            ai_client: self.ai_client,
            #[cfg(feature = "rest-api")]
            scope: self.scope,
            shutdown_requested: self.shutdown_requested,
            profile: self.profile,
            policy: self.policy,
            execution_policy: self.execution_policy,
        }
    }

    #[cfg(feature = "ai-integration")]
    pub fn with_ai_client(mut self, client: AiClient) -> Self {
        self.ai_client = Some(client);
        self
    }

    #[cfg(feature = "ai-integration")]
    pub fn ai_client(&self) -> Option<&AiClient> {
        self.ai_client.as_ref()
    }

    pub fn validate_auth(&self, headers: &axum::http::HeaderMap) -> Result<(), McpError> {
        validate_auth(&self.api_key, headers)
    }

    pub fn validate_auth_params(&self, params: &Option<serde_json::Value>) -> Result<(), McpError> {
        validate_auth_params(&self.api_key, params)
    }

    /// Starts a background task that periodically cleans up expired hashmap entries.
    ///
    /// This is a fire-and-forget cleanup routine with no shutdown mechanism because it's
    /// designed for long-running servers. The reaper runs indefinitely and cleans:
    /// - Pending cancellations older than their timeout
    /// - Completed results older than 5 minutes (ENTRY_TTL_SECS)
    ///
    /// Memory is bounded because entries are removed, not accumulated.
    /// This is an intentional design choice - the server is expected to run
    /// until shutdown, at which point the process exits and OS reclaims memory.
    pub fn start_hashmap_reaper(&self, interval_secs: u64) {
        let pending_cancellations = Arc::clone(&self.pending_cancellations);
        let completed_results = Arc::clone(&self.completed_results);

        tokio::spawn(async move {
            let mut interval: Interval = tokio::time::interval(Duration::from_secs(interval_secs));

            loop {
                interval.tick().await;

                const ENTRY_TTL_SECS: i64 = 300;

                {
                    let mut pending = pending_cancellations.write().await;
                    pending.retain(|_, token| !token.is_cancelled());
                }

                let mut to_remove: Vec<String> = Vec::new();
                {
                    let mut results = completed_results.write().await;
                    let now = Utc::now();
                    for (id, response) in results.iter() {
                        let age = now.signed_duration_since(response.metadata.completed_at);
                        if age.num_seconds() > ENTRY_TTL_SECS {
                            to_remove.push(id.clone());
                        }
                    }
                    for id in to_remove {
                        results.remove(&id);
                    }
                }
            }
        });
    }

    pub async fn handle_request(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req).await,
            "tools/list" => self.handle_tools_list(req).await,
            "tools/list-by-category" => self.handle_tools_list_by_category(req).await,
            "tools/call" => self.handle_tools_call(req).await,
            "tools/cancel" => self.handle_tools_cancel(req).await,
            "tools/call-stream" => self.handle_tools_call_stream(req).await,
            "tools/result" => self.handle_tools_result(req).await,
            "tools/history" => self.handle_tools_history(req).await,
            "session/create" => self.handle_session_create(req).await,
            "session/get" => self.handle_session_get(req).await,
            "session/list" => self.handle_session_list(req).await,
            "rate-limit/status" => self.handle_rate_limit_status(req).await,
            "resources/list" => self.handle_resources_list(req).await,
            "resources/read" => self.handle_resources_read(req).await,
            "roots/list" => self.handle_roots_list(req).await,
            "prompts/list" => self.handle_prompts_list(req).await,
            "prompts/read" => self.handle_prompts_read(req).await,
            "ping" => self.handle_ping(req).await,
            "shutdown" => self.handle_shutdown(req).await,
            _ => req.not_found_method(),
        }
    }

    async fn handle_roots_list(&self, req: McpRequest) -> McpResponse {
        let roots = vec![
            McpRoot {
                uri: "eggsec://config".to_string(),
                name: "Configuration".to_string(),
                description: "Eggsec configuration directory".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpRoot {
                uri: "eggsec://payloads".to_string(),
                name: "Payloads".to_string(),
                description: "Security testing payloads directory".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpRoot {
                uri: "eggsec://templates".to_string(),
                name: "Templates".to_string(),
                description: "Security testing templates".to_string(),
                mime_type: Some("application/yaml".to_string()),
            },
        ];

        let result = serde_json::json!({
            "roots": roots,
            "count": roots.len()
        });

        req.success_response(result)
    }

    async fn handle_shutdown(&self, req: McpRequest) -> McpResponse {
        self.request_shutdown();

        let result = serde_json::json!({
            "success": true,
            "message": "Server shutdown requested"
        });

        req.success_response(result)
    }

    async fn handle_initialize(&self, req: McpRequest) -> McpResponse {
        let safety = &self.policy.to_initialize_metadata()["safety"];

        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": { "listChanged": true },
                "sessions": self.policy.allow_sessions,
                "roots": { "listChanged": true },
                "streaming": self.policy.allow_streaming
            },
            "serverInfo": {
                "name": self.profile.server_name(),
                "version": "0.1.0",
                "description": self.profile.server_description()
            },
            "profile": self.profile.as_str(),
            "safety": safety
        });

        req.success_response(result)
    }

    async fn handle_tools_list(&self, req: McpRequest) -> McpResponse {
        let all_tools = self.registry.list();
        let tools = self.policy.filter_tools(all_tools);

        let mcp_tools: Vec<McpTool> = tools
            .into_iter()
            .map(|info| {
                let input_schema = build_input_schema(&info);
                let capabilities = build_capabilities_summary(&info);

                McpTool {
                    name: info.id,
                    description: info.description,
                    input_schema,
                    capabilities: Some(capabilities),
                }
            })
            .collect();

        let result = serde_json::json!({
            "tools": mcp_tools,
            "count": mcp_tools.len()
        });

        req.success_response(result)
    }

    async fn handle_tools_list_by_category(&self, req: McpRequest) -> McpResponse {
        let all_tools = self.registry.list();
        let tools = self.policy.filter_tools(all_tools);
        let total_tools = tools.len();

        let mut categorized: FxHashMap<String, Vec<McpTool>> = FxHashMap::default();

        for info in tools {
            let input_schema = build_input_schema(&info);
            let capabilities = build_capabilities_summary(&info);

            let mcp_tool = McpTool {
                name: info.id,
                description: info.description,
                input_schema,
                capabilities: Some(capabilities),
            };

            let category = format!("{:?}", info.category).to_lowercase();
            categorized.entry(category).or_default().push(mcp_tool);
        }

        let result = serde_json::json!({
            "categories": categorized,
            "total_tools": total_tools
        });

        req.success_response(result)
    }

    async fn handle_tools_call(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let client_id = params
            .get("api_key")
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous");

        if let Err(e) = self.rate_limiter.check_rate_limit(client_id) {
            return req.error_response(McpError::rate_limited(&e.to_string()));
        }

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return req.error_response(McpError::invalid_params("Missing tool name")),
        };

        let arguments = params
            .get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let target_value = arguments
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        // Policy enforcement: validate tool call before resolving
        let (tool_id, capability) = match self.resolve_tool_id(name) {
            Some(result) => result,
            None => {
                return req.error_response(McpError::invalid_params(&format!(
                    "Unknown tool or capability: {}",
                    name
                )))
            }
        };

        if let Err(violation) =
            self.policy
                .validate_tool_call(&tool_id, capability.as_deref(), &arguments)
        {
            let execution_policy = self.execution_policy.clone();
            let scope_ref = self.scope.as_ref();
            let decision = policy_decision_for_mcp_call(
                &self.policy,
                &tool_id,
                &arguments,
                &execution_policy,
                scope_ref,
            );
            return req.error_response(McpError {
                code: violation.to_mcp_error_code(),
                message: violation.to_string(),
                data: Some(serde_json::to_value(&decision).unwrap_or_default()),
            });
        }

        // Target policy enforcement
        if !target_value.is_empty() {
            if let Err(violation) = self.policy.validate_target(target_value) {
                let execution_policy = self.execution_policy.clone();
                let scope_ref = self.scope.as_ref();
                let decision = policy_decision_for_mcp_call(
                    &self.policy,
                    &tool_id,
                    &arguments,
                    &execution_policy,
                    scope_ref,
                );
                return req.error_response(McpError {
                    code: violation.to_mcp_error_code(),
                    message: violation.to_string(),
                    data: Some(serde_json::to_value(&decision).unwrap_or_default()),
                });
            }
        }

        #[cfg(feature = "rest-api")]
        {
            if let Some(ref scope) = self.scope {
                match scope.is_target_allowed(target_value) {
                    Ok(false) | Err(_) => {
                        return req.error_response(McpError::invalid_params(&format!(
                            "Scope violation: {} not allowed",
                            target_value
                        )));
                    }
                    Ok(true) => {}
                }
            }
        }

        let target_type = arguments
            .get("target_type")
            .and_then(|v| v.as_str())
            .unwrap_or("url");

        let target = match target_type {
            "domain" => Target::domain(target_value),
            "ip" => Target::ip(target_value),
            "cidr" => Target::cidr(target_value),
            _ => Target::url(target_value),
        };

        let mut request_args = arguments.clone();
        if let Some(cap) = &capability {
            request_args["_capability"] = serde_json::json!(cap);
        }

        let options = RequestOptions {
            timeout_ms: arguments.get("timeout_ms").and_then(|v| v.as_u64()),
            concurrency: arguments
                .get("concurrency")
                .and_then(|v| v.as_u64())
                .map(|v| v as usize),
            ..Default::default()
        };

        let request = ToolRequest::new(tool_id, target)
            .with_params(request_args)
            .with_options(options);

        match self.dispatcher.dispatch(request).await {
            Ok(response) => {
                let content = if self.profile.is_coding_agent() {
                    let output = self.build_coding_agent_output(&target_value, &response);
                    vec![serde_json::json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&output).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize coding-agent output");
                        }).unwrap_or_default()
                    })]
                } else {
                    vec![serde_json::json!({
                        "type": "text",
                        "text": serde_json::to_string_pretty(&response).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize tool response");
                        }).unwrap_or_default()
                    })]
                };

                let result = serde_json::json!({
                    "content": content,
                    "isError": !response.is_success()
                });
                req.success_response(result)
            }
            Err(e) => {
                let error = McpError::internal(&e.to_string());
                req.error_response(error)
            }
        }
    }

    fn resolve_tool_id(&self, name: &str) -> Option<(String, Option<String>)> {
        if self.registry.get(name).is_some() {
            return Some((name.to_string(), None));
        }

        for tool_info in self.registry.list() {
            if tool_info.capabilities.iter().any(|c| c.name == name) {
                return Some((tool_info.id, Some(name.to_string())));
            }
        }

        None
    }

    async fn handle_resources_list(&self, req: McpRequest) -> McpResponse {
        let mut resources = vec![
            McpResource {
                uri: "eggsec://tools".to_string(),
                name: "Available Tools".to_string(),
                description: "List of all available security tools".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpResource {
                uri: "eggsec://manifest".to_string(),
                name: "Tool Manifest".to_string(),
                description: "Complete manifest of all tools, capabilities, and attack surfaces"
                    .to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpResource {
                uri: "eggsec://vulnerabilities".to_string(),
                name: "Vulnerability Types".to_string(),
                description: "List of detectable vulnerability types with CWE mappings".to_string(),
                mime_type: Some("application/json".to_string()),
            },
        ];

        match self.policy.profile {
            crate::tool::protocol::mcp::profile::McpProfile::OpsAgent => {
                resources.push(McpResource {
                    uri: "eggsec://ops-agent/safety-policy".to_string(),
                    name: "Ops Agent Safety Policy".to_string(),
                    description:
                        "Safety policy, target rules, and enforcement settings for ops-agent profile"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
                resources.push(McpResource {
                    uri: "eggsec://ops-agent/task-schema".to_string(),
                    name: "Ops Agent Task Schema".to_string(),
                    description:
                        "Schema for task assignment and result submission in the ops-agent agent runtime"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
                resources.push(McpResource {
                    uri: "eggsec://ops-agent/event-schema".to_string(),
                    name: "Ops Agent Event Schema".to_string(),
                    description:
                        "Schema for streaming events emitted during ops-agent tool execution"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
            }
            crate::tool::protocol::mcp::profile::McpProfile::CodingAgent => {
                resources.push(McpResource {
                    uri: "eggsec://coding-agent/manifest".to_string(),
                    name: "Coding Agent Manifest".to_string(),
                    description:
                        "Available coding-agent validation tools, safe defaults, and recommended workflow"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
                resources.push(McpResource {
                    uri: "eggsec://coding-agent/safety-policy".to_string(),
                    name: "Coding Agent Safety Policy".to_string(),
                    description:
                        "Safety defaults, hard caps, and allowed targets for coding-agent profile"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
                resources.push(McpResource {
                    uri: "eggsec://coding-agent/finding-schema".to_string(),
                    name: "Coding Agent Finding Schema".to_string(),
                    description:
                        "Schema for structured findings produced by coding-agent validation tools"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
                resources.push(McpResource {
                    uri: "eggsec://coding-agent/workflow".to_string(),
                    name: "Coding Agent Workflow".to_string(),
                    description:
                        "Recommended validation workflow, step ordering, and summarization strategy"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
                resources.push(McpResource {
                    uri: "eggsec://coding-agent/tool-contracts".to_string(),
                    name: "Coding Agent Tool Contracts".to_string(),
                    description:
                        "Per-tool contracts including input/output schemas, latency, determinism, and network flags"
                            .to_string(),
                    mime_type: Some("application/json".to_string()),
                });
            }
        }

        let result = serde_json::json!({
            "resources": resources
        });

        req.success_response(result)
    }

    async fn handle_resources_read(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let uri = match params.get("uri").and_then(|v| v.as_str()) {
            Some(uri) => uri,
            None => return req.error_response(McpError::invalid_params("Missing uri")),
        };

        match uri {
            "eggsec://tools" => {
                let tools = self.registry.list();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": "eggsec://tools",
                            "mimeType": "application/json",
                            "text": serde_json::to_string(&tools).inspect_err(|e| {
                                tracing::warn!(error = %e, "Failed to serialize tools list");
                            }).unwrap_or_default()
                        }
                    ]
                });
                req.success_response(result)
            }
            "eggsec://manifest" => {
                let manifest = self.build_manifest();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": "eggsec://manifest",
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&manifest).inspect_err(|e| {
                                tracing::warn!(error = %e, "Failed to serialize manifest");
                            }).unwrap_or_default()
                        }
                    ]
                });
                req.success_response(result)
            }
            "eggsec://vulnerabilities" => {
                let vulns = self.build_vulnerability_catalog();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": "eggsec://vulnerabilities",
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&vulns).inspect_err(|e| {
                                tracing::warn!(error = %e, "Failed to serialize vulnerabilities catalog");
                            }).unwrap_or_default()
                        }
                    ]
                });
                req.success_response(result)
            }
            "eggsec://ops-agent/safety-policy" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::OpsAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Ops-agent resources are only available in ops-agent profile",
                    ));
                }
                let policy = self.build_ops_agent_safety_policy();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://ops-agent/safety-policy",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&policy).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize ops-agent safety policy");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://ops-agent/task-schema" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::OpsAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Ops-agent resources are only available in ops-agent profile",
                    ));
                }
                let schema = self.build_ops_agent_task_schema();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://ops-agent/task-schema",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&schema).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize ops-agent task schema");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://ops-agent/event-schema" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::OpsAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Ops-agent resources are only available in ops-agent profile",
                    ));
                }
                let schema = self.build_ops_agent_event_schema();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://ops-agent/event-schema",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&schema).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize ops-agent event schema");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://coding-agent/manifest" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::CodingAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Coding-agent resources are only available in coding-agent profile",
                    ));
                }
                let manifest = self.build_coding_agent_manifest();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://coding-agent/manifest",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&manifest).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize coding-agent manifest");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://coding-agent/safety-policy" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::CodingAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Coding-agent resources are only available in coding-agent profile",
                    ));
                }
                let policy = self.build_coding_agent_safety_policy();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://coding-agent/safety-policy",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&policy).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize coding-agent safety policy");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://coding-agent/finding-schema" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::CodingAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Coding-agent resources are only available in coding-agent profile",
                    ));
                }
                let schema = self.build_coding_agent_finding_schema();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://coding-agent/finding-schema",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&schema).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize coding-agent finding schema");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://coding-agent/workflow" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::CodingAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Coding-agent resources are only available in coding-agent profile",
                    ));
                }
                let workflow = self.build_coding_agent_workflow();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://coding-agent/workflow",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&workflow).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize coding-agent workflow");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            "eggsec://coding-agent/tool-contracts" => {
                if !matches!(
                    self.policy.profile,
                    crate::tool::protocol::mcp::profile::McpProfile::CodingAgent
                ) {
                    return req.error_response(McpError::invalid_params(
                        "Coding-agent resources are only available in coding-agent profile",
                    ));
                }
                let contracts = self.build_coding_agent_tool_contracts();
                let result = serde_json::json!({
                    "contents": [{
                        "uri": "eggsec://coding-agent/tool-contracts",
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&contracts).inspect_err(|e| {
                            tracing::warn!(error = %e, "Failed to serialize coding-agent tool contracts");
                        }).unwrap_or_default()
                    }]
                });
                req.success_response(result)
            }
            _ => req.error_response(McpError::invalid_params("Unknown resource uri")),
        }
    }

    async fn handle_prompts_list(&self, req: McpRequest) -> McpResponse {
        let prompts = get_builtin_prompts_for_profile(&self.profile);

        let result = serde_json::json!({
            "prompts": prompts,
            "count": prompts.len()
        });

        req.success_response(result)
    }

    async fn handle_prompts_read(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return req.error_response(McpError::invalid_params("Missing prompt name")),
        };

        let prompts = get_builtin_prompts_for_profile(&self.profile);

        if let Some(prompt) = prompts.into_iter().find(|p| p.name == name) {
            let result = serde_json::json!({
                "prompt": prompt
            });
            req.success_response(result)
        } else {
            req.error_response(McpError::invalid_params(&format!(
                "Unknown prompt: {}",
                name
            )))
        }
    }

    fn build_manifest(&self) -> serde_json::Value {
        let tools = self.registry.list();

        let mut attack_surfaces: FxHashMap<String, Vec<String>> = FxHashMap::default();

        for tool in &tools {
            for cap in &tool.capabilities {
                for surface in &cap.attack_surface {
                    let surface_name = format!("{:?}", surface).to_lowercase();
                    attack_surfaces
                        .entry(surface_name)
                        .or_default()
                        .push(tool.id.clone());
                }
            }
        }

        serde_json::json!({
            "tools": tools,
            "attack_surfaces": attack_surfaces
        })
    }

    fn build_vulnerability_catalog(&self) -> serde_json::Value {
        serde_json::json!({
            "vulnerabilities": [
                {"name": "SQL Injection", "cwe": "CWE-89"},
                {"name": "Cross-Site Scripting", "cwe": "CWE-79"},
                {"name": "Command Injection", "cwe": "CWE-78"},
                {"name": "Path Traversal", "cwe": "CWE-22"},
                {"name": "XML External Entity", "cwe": "CWE-611"},
                {"name": "Insecure Deserialization", "cwe": "CWE-502"},
                {"name": "Broken Authentication", "cwe": "CWE-287"},
                {"name": "Security Misconfiguration", "cwe": "CWE-16"}
            ]
        })
    }

    fn build_coding_agent_manifest(&self) -> serde_json::Value {
        serde_json::json!({
            "profile": "coding-agent",
            "description": "Bounded live security validation tools for coding agents",
            "tools": [
                {
                    "name": "prepare_validation_target",
                    "description": "Validate and normalize a local or explicitly scoped live application target before running coding-agent security checks"
                },
                {
                    "name": "validate_live_web_app",
                    "description": "Run bounded, low-volume live validation of a running web application for headers, cookies, CORS, unsafe methods, error leakage, and lightweight input handling issues"
                },
                {
                    "name": "validate_auth_boundaries",
                    "description": "Validate authentication and authorization boundaries across supplied routes and test sessions"
                },
                {
                    "name": "validate_api_surface",
                    "description": "Validate an explicit API surface or supplied API schema for method handling, content-type handling, malformed input behavior, and error leakage"
                },
                {
                    "name": "validate_file_surface",
                    "description": "Validate explicitly supplied upload, download, import, or export endpoints for safe path handling, content-type behavior, and size limits"
                },
                {
                    "name": "run_targeted_probe",
                    "description": "Run one narrow hypothesis-driven validation probe against one explicit endpoint and parameter"
                },
                {
                    "name": "retest_finding",
                    "description": "Rerun the minimal validation recipe for a previous coding-agent finding to determine whether it is fixed, still present, inconclusive, or changed"
                }
            ],
            "safe_defaults": {
                "allowed_hosts": ["localhost", "127.0.0.1", "::1"],
                "allowed_schemes": ["http", "https"],
                "allow_external_network": false,
                "max_requests": 100,
                "max_duration_ms": 60000,
                "max_concurrency": 5,
                "max_depth": 1
            },
            "hard_caps": {
                "max_requests": 500,
                "max_duration_ms": 300000,
                "max_concurrency": 10
            },
            "workflow": [
                "1. Call prepare_validation_target to establish safe target context",
                "2. Call appropriate validation tool based on change type",
                "3. Review structured findings with evidence",
                "4. Patch source code based on findings",
                "5. Call retest_finding to verify fix"
            ]
        })
    }

    fn build_coding_agent_safety_policy(&self) -> serde_json::Value {
        serde_json::json!({
            "profile": "coding-agent",
            "default_allowlist": ["localhost", "127.0.0.1", "[::1]"],
            "default_deny": [
                "broad CIDRs",
                "arbitrary external domains",
                "production-looking hosts unless explicitly allowed",
                "stress testing",
                "WAF bypass tooling",
                "proxy/Tor pools",
                "cluster mode",
                "high request volume",
                "SYN/UDP/ICMP/TCP flood tools",
                "destructive payloads",
                "credential brute force",
                "key brute force",
                "arbitrary crawl depth"
            ],
            "budgets": {
                "default_max_requests": 100,
                "hard_max_requests": 500,
                "default_max_duration_ms": 60000,
                "hard_max_duration_ms": 300000,
                "default_max_concurrency": 5,
                "hard_max_concurrency": 10
            }
        })
    }

    fn build_coding_agent_finding_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "required": ["id", "title", "category", "severity", "confidence", "observed_behavior", "patch_relevance"],
            "properties": {
                "id": { "type": "string", "description": "Stable finding identifier" },
                "title": { "type": "string", "description": "Short finding title" },
                "category": { "type": "string", "description": "Finding category (e.g., auth_boundary, header_missing, error_leakage)" },
                "severity": { "type": "string", "enum": ["critical", "high", "medium", "low", "info"] },
                "confidence": { "type": "string", "enum": ["confirmed", "high", "medium", "low", "inconclusive"] },
                "endpoint": {
                    "type": "object",
                    "properties": {
                        "method": { "type": "string" },
                        "path": { "type": "string" }
                    }
                },
                "parameter": { "type": "string" },
                "observed_behavior": { "type": "string" },
                "expected_behavior": { "type": "string" },
                "evidence": {
                    "type": "array",
                    "items": {
                        "type": "object",
                        "properties": {
                            "type": { "type": "string" },
                            "content": { "type": "string" }
                        }
                    }
                },
                "retest_recipe": { "type": "object" },
                "patch_relevance": { "type": "string", "enum": ["blocks_merge", "should_fix", "review_manually", "informational"] }
            }
        })
    }

    fn build_coding_agent_output(
        &self,
        target: &str,
        response: &ToolResponse,
    ) -> serde_json::Value {
        use crate::tool::protocol::mcp::coding_agent_output::CodingAgentFindingReport;

        let findings: Vec<_> = response
            .findings
            .iter()
            .map(CodingAgentFindingReport::from_finding)
            .collect();

        let summary = CodingAgentFindingReport::build_summary(&findings);

        let status = if response.is_success() {
            "completed"
        } else {
            "failed"
        };

        let report = CodingAgentFindingReport {
            schema_version: "1.0".to_string(),
            target: target.to_string(),
            profile: "coding-agent".to_string(),
            run_id: response.request_id.clone(),
            status: status.to_string(),
            findings,
            summary,
        };

        serde_json::to_value(&report).unwrap_or_else(|e| {
            tracing::error!(error = %e, "Failed to serialize CodingAgentFindingReport");
            serde_json::json!({"error": "serialization_failed"})
        })
    }

    fn build_ops_agent_safety_policy(&self) -> serde_json::Value {
        let allowed_tools = match &self.policy.allowed_tool_ids {
            crate::tool::protocol::mcp::policy::ToolSelector::All => {
                serde_json::json!("all")
            }
            crate::tool::protocol::mcp::policy::ToolSelector::None => {
                serde_json::json!("none")
            }
            crate::tool::protocol::mcp::policy::ToolSelector::Exact(ids) => {
                serde_json::json!(ids)
            }
            crate::tool::protocol::mcp::policy::ToolSelector::Category(cats) => {
                serde_json::json!({"categories": cats})
            }
            crate::tool::protocol::mcp::policy::ToolSelector::Capability(caps) => {
                serde_json::json!({"capabilities": caps})
            }
        };

        let denied_tools = match &self.policy.denied_tool_ids {
            crate::tool::protocol::mcp::policy::ToolSelector::None => {
                serde_json::json!("none")
            }
            crate::tool::protocol::mcp::policy::ToolSelector::All => {
                serde_json::json!("all")
            }
            crate::tool::protocol::mcp::policy::ToolSelector::Exact(ids) => {
                serde_json::json!(ids)
            }
            crate::tool::protocol::mcp::policy::ToolSelector::Category(cats) => {
                serde_json::json!({"categories": cats})
            }
            crate::tool::protocol::mcp::policy::ToolSelector::Capability(caps) => {
                serde_json::json!({"capabilities": caps})
            }
        };

        let target_policy_desc = match self.policy.default_target_policy {
            crate::tool::protocol::mcp::policy::TargetPolicy::AnyWithScopeEngine => {
                "Any target subject to scope engine"
            }
            crate::tool::protocol::mcp::policy::TargetPolicy::ExplicitScopeOnly => {
                "Only targets with explicit scope configuration"
            }
            crate::tool::protocol::mcp::policy::TargetPolicy::LocalhostAndPrivateCidrsOnly => {
                "Only loopback and private network targets"
            }
            crate::tool::protocol::mcp::policy::TargetPolicy::ScopeOrLocalDevOnly => {
                "Loopback, private CIDRs, or explicit scope"
            }
        };

        serde_json::json!({
            "profile": "ops-agent",
            "target_policy": target_policy_desc,
            "allowed_tools": allowed_tools,
            "denied_tools": denied_tools,
            "enforcement": {
                "max_concurrency": self.policy.max_concurrency,
                "max_timeout_ms": self.policy.max_timeout_ms,
                "max_batch_size": self.policy.max_batch_size,
                "allow_streaming": self.policy.allow_streaming,
                "allow_sessions": self.policy.allow_sessions,
                "allow_plan_endpoint": self.policy.allow_plan_endpoint,
            },
            "capabilities": {
                "external_network": self.policy.allow_external_network,
                "stress_testing": self.policy.allow_stress_testing,
                "packet_features": self.policy.allow_packet_features,
                "broad_recon": self.policy.allow_broad_recon,
            },
            "denied_argument_keys": self.policy.denied_argument_keys,
        })
    }

    fn build_ops_agent_task_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "OpsAgentTask",
            "description": "Schema for task assignment and result submission in the ops-agent runtime",
            "type": "object",
            "required": ["task_id", "tool_name", "target"],
            "properties": {
                "task_id": {
                    "type": "string",
                    "description": "Unique task identifier"
                },
                "tool_name": {
                    "type": "string",
                    "description": "Name of the tool to execute"
                },
                "target": {
                    "type": "string",
                    "description": "Target URL, IP, or domain for the task"
                },
                "arguments": {
                    "type": "object",
                    "description": "Tool-specific arguments",
                    "additionalProperties": true
                },
                "timeout_ms": {
                    "type": "integer",
                    "description": "Maximum execution time in milliseconds",
                    "minimum": 1000,
                    "maximum": self.policy.max_timeout_ms
                },
                "callback_url": {
                    "type": "string",
                    "description": "URL to POST results to when task completes"
                },
                "priority": {
                    "type": "string",
                    "enum": ["low", "normal", "high", "critical"],
                    "default": "normal"
                },
                "metadata": {
                    "type": "object",
                    "description": "Arbitrary metadata for the task",
                    "additionalProperties": true
                }
            }
        })
    }

    fn build_ops_agent_event_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "$schema": "https://json-schema.org/draft/2020-12/schema",
            "title": "OpsAgentEvent",
            "description": "Schema for streaming events emitted during tool execution",
            "type": "object",
            "required": ["event_type", "task_id", "timestamp"],
            "properties": {
                "event_type": {
                    "type": "string",
                    "enum": [
                        "task_started",
                        "progress",
                        "tool_output",
                        "finding",
                        "error",
                        "task_completed",
                        "task_cancelled"
                    ]
                },
                "task_id": {
                    "type": "string",
                    "description": "Task identifier this event belongs to"
                },
                "timestamp": {
                    "type": "string",
                    "format": "date-time",
                    "description": "ISO 8601 timestamp of the event"
                },
                "progress": {
                    "type": "object",
                    "properties": {
                        "percent": { "type": "number", "minimum": 0, "maximum": 100 },
                        "stage": { "type": "string" },
                        "message": { "type": "string" }
                    }
                },
                "data": {
                    "type": "object",
                    "description": "Event-specific payload",
                    "additionalProperties": true
                },
                "error": {
                    "type": "object",
                    "properties": {
                        "code": { "type": "integer" },
                        "message": { "type": "string" },
                        "recoverable": { "type": "boolean" }
                    }
                }
            }
        })
    }

    fn build_coding_agent_workflow(&self) -> serde_json::Value {
        let tool_names: Vec<String> = self
            .policy
            .filter_tools(self.registry.list())
            .into_iter()
            .map(|t| t.id)
            .collect();

        serde_json::json!({
            "profile": "coding-agent",
            "description": "Recommended validation workflow for coding agents",
            "steps": [
                {
                    "order": 1,
                    "name": "establish_target",
                    "tool": "scan",
                    "purpose": "Fingerprint the target and confirm it is within allowed scope",
                    "required": true,
                    "summarization": "Extract technologies, framework, and target status"
                },
                {
                    "order": 2,
                    "name": "validate_headers_and_cookies",
                    "tool": "scan",
                    "purpose": "Check security headers, cookie flags, and CORS configuration",
                    "required": true,
                    "summarization": "List missing headers and insecure cookie attributes"
                },
                {
                    "order": 3,
                    "name": "validate_endpoints",
                    "tool": "endpoints",
                    "purpose": "Discover and validate API endpoints for method handling and error behavior",
                    "required": false,
                    "summarization": "Summarize endpoint count, methods, and error patterns"
                },
                {
                    "order": 4,
                    "name": "check_waf",
                    "tool": "waf-detect",
                    "purpose": "Detect WAF presence that may affect further testing",
                    "required": false,
                    "summarization": "Report WAF vendor if detected, else 'none'"
                },
                {
                    "order": 5,
                    "name": "retest_findings",
                    "tool": "scan",
                    "purpose": "Re-run targeted checks after code changes to verify fixes",
                    "required": false,
                    "summarization": "Compare before/after for each finding"
                }
            ],
            "allowed_tools": tool_names,
            "constraints": {
                "max_requests": self.policy.max_batch_size * 10,
                "max_concurrency": self.policy.max_concurrency,
                "max_duration_ms": self.policy.max_timeout_ms,
                "allow_external_network": self.policy.allow_external_network,
            },
            "summarization_strategy": {
                "max_output_tokens": 4096,
                "include_evidence": true,
                "include_reproduction_steps": true,
                "redact_sensitive_data": true,
                "format": "structured_json"
            }
        })
    }

    fn build_coding_agent_tool_contracts(&self) -> serde_json::Value {
        let tools = self.policy.filter_tools(self.registry.list());

        let contracts: Vec<serde_json::Value> = tools
            .iter()
            .map(|tool| {
                let input_schema = build_input_schema(tool);
                let category = format!("{:?}", tool.category).to_lowercase();

                let (
                    latency_class,
                    deterministic,
                    makes_network_requests,
                    requires_running_service,
                    use_context,
                ) = match tool.id.as_str() {
                    "scan" => ("seconds", true, true, true, "security_review"),
                    "scan-ports" => ("seconds", true, true, true, "security_review"),
                    "fingerprint" => ("seconds", true, true, true, "security_review"),
                    "endpoints" => ("seconds", true, true, true, "security_review"),
                    "waf-detect" => ("seconds", true, true, true, "security_review"),
                    "search" => ("milliseconds", true, false, false, "coding_flow"),
                    _ => ("seconds", true, true, true, "security_review"),
                };

                serde_json::json!({
                    "tool_id": tool.id,
                    "name": tool.name,
                    "category": category,
                    "description": tool.description,
                    "input_schema": input_schema,
                    "latency_class": latency_class,
                    "deterministic": deterministic,
                    "makes_network_requests": makes_network_requests,
                    "requires_running_service": requires_running_service,
                    "use_context": use_context,
                    "capabilities": tool.capabilities.iter().map(|c| serde_json::json!({
                        "name": c.name,
                        "description": c.description,
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();

        serde_json::json!({
            "profile": "coding-agent",
            "description": "Per-tool contracts for coding-agent profile",
            "contracts": contracts,
        })
    }

    async fn handle_ping(&self, req: McpRequest) -> McpResponse {
        let result = serde_json::json!({
            "status": "ok",
            "timestamp": Utc::now().to_rfc3339()
        });

        req.success_response(result)
    }

    pub async fn handle_tools_call_stream(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let request_id = match params.get("request_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return req.error_response(McpError::invalid_params("Missing request_id")),
        };

        if self
            .pending_cancellations
            .write()
            .await
            .contains_key(request_id)
        {
            let result = serde_json::json!({
                "cancelled": false,
                "request_id": request_id,
                "message": "Request already in progress"
            });
            return req.success_response(result);
        }

        let token = CancellationToken::new();
        self.pending_cancellations
            .write()
            .await
            .insert(request_id.to_string(), token);

        let result = serde_json::json!({
            "request_id": request_id,
            "status": "streaming"
        });

        req.success_response(result)
    }

    async fn handle_tools_cancel(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let request_id = match params.get("request_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return req.error_response(McpError::invalid_params("Missing request_id")),
        };

        if let Some(token) = self.pending_cancellations.write().await.remove(request_id) {
            token.cancel();
            let result = serde_json::json!({
                "cancelled": true,
                "request_id": request_id,
                "message": "Request cancelled successfully"
            });
            req.success_response(result)
        } else {
            let result = serde_json::json!({
                "cancelled": false,
                "request_id": request_id,
                "message": "Request not found or already completed"
            });
            req.success_response(result)
        }
    }

    async fn handle_tools_result(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let request_id = match params.get("request_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return req.error_response(McpError::invalid_params("Missing request_id")),
        };

        if let Some(response) = self.completed_results.write().await.remove(request_id) {
            let result = serde_json::json!({
                "request_id": request_id,
                "status": "completed",
                "response": response
            });
            req.success_response(result)
        } else if self
            .pending_cancellations
            .write()
            .await
            .contains_key(request_id)
        {
            let result = serde_json::json!({
                "request_id": request_id,
                "status": "running"
            });
            req.success_response(result)
        } else {
            let result = serde_json::json!({
                "request_id": request_id,
                "status": "not_found",
                "message": "Request not found or result already retrieved"
            });
            req.success_response(result)
        }
    }

    async fn handle_tools_history(&self, req: McpRequest) -> McpResponse {
        let params = req.params.as_ref();

        let limit = params
            .and_then(|p| p.get("limit"))
            .and_then(|v| v.as_u64())
            .unwrap_or(100) as usize;

        let history = self.dispatcher.history();

        let entries = history.map(|h| h.get_recent(limit)).unwrap_or_default();

        let result = serde_json::json!({
            "entries": entries,
            "count": entries.len()
        });

        req.success_response(result)
    }

    async fn handle_session_create(&self, req: McpRequest) -> McpResponse {
        let params = req.params.as_ref();
        let target = params
            .and_then(|p| p.get("target"))
            .and_then(|v| v.as_str())
            .map(String::from);

        if let Some(ref manager) = self.session_manager {
            match manager.create_session().await {
                Ok(mut session) => {
                    if let Some(t) = target {
                        session.context.target = Some(t);
                    }

                    if let Err(e) = manager.update_session(&session).await {
                        tracing::debug!("Failed to update session: {}", e);
                    }

                    let result = serde_json::json!({
                        "session_id": session.session_id,
                        "created_at": session.created_at.to_rfc3339(),
                        "target": session.context.target,
                        "status": format!("{:?}", session.status).to_lowercase(),
                        "scopes": session.context.stages_completed.len(),
                        "findings_count": session.findings.len()
                    });

                    return req.success_response(result);
                }
                Err(e) => {
                    return req.error_response(McpError::internal(&e.to_string()));
                }
            }
        }

        let session_id = uuid::Uuid::new_v4().to_string();

        let session = serde_json::json!({
            "session_id": session_id,
            "created_at": Utc::now().to_rfc3339(),
            "target": target,
            "status": "active",
            "message": "Session created (no persistence configured)"
        });

        req.success_response(session)
    }

    async fn handle_session_get(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let session_id = match params.get("session_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return req.error_response(McpError::invalid_params("Missing session_id")),
        };

        if let Some(ref manager) = self.session_manager {
            match manager.get_session(session_id).await {
                Some(session) => {
                    let result = serde_json::json!({
                        "session_id": session.session_id,
                        "created_at": session.created_at.to_rfc3339(),
                        "updated_at": session.updated_at.to_rfc3339(),
                        "target": session.context.target,
                        "target_type": session.context.target_type,
                        "scan_type": session.context.scan_type,
                        "status": format!("{:?}", session.status).to_lowercase(),
                        "stages_completed": session.context.stages_completed,
                        "discovered_endpoints": session.context.discovered_endpoints,
                        "discovered_technologies": session.context.discovered_technologies,
                        "open_ports": session.context.open_ports,
                        "authenticated": session.context.authenticated,
                        "waf_detected": session.context.waf_detected,
                        "findings_count": session.findings.len(),
                        "findings_summary": session.severity_summary(),
                        "last_activity": session.context.last_activity.map(|t| t.to_rfc3339())
                    });
                    return req.success_response(result);
                }
                None => {
                    let result = serde_json::json!({
                        "session_id": session_id,
                        "status": "not_found",
                        "message": "Session not found or expired"
                    });
                    return req.success_response(result);
                }
            }
        }

        let session = serde_json::json!({
            "session_id": session_id,
            "status": "unavailable",
            "message": "Session manager not configured"
        });

        req.success_response(session)
    }

    async fn handle_session_list(&self, req: McpRequest) -> McpResponse {
        let (offset, limit) = match &req.params {
            Some(p) => {
                let offset = p.get("offset").and_then(|v| v.as_u64()).unwrap_or(0) as usize;
                let limit = p.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
                (offset, limit.clamp(1, 100))
            }
            None => (0, 50),
        };

        if let Some(ref manager) = self.session_manager {
            let all_sessions = manager.list_sessions().await;
            let total = all_sessions.len();

            let paginated_sessions: Vec<serde_json::Value> = all_sessions
                .iter()
                .skip(offset)
                .take(limit)
                .map(|s| {
                    serde_json::json!({
                        "session_id": s.session_id,
                        "created_at": s.created_at.to_rfc3339(),
                        "updated_at": s.updated_at.to_rfc3339(),
                        "status": format!("{:?}", s.status).to_lowercase(),
                        "target": s.context.target,
                        "findings_count": s.findings.len()
                    })
                })
                .collect();

            let result = serde_json::json!({
                "sessions": paginated_sessions,
                "total": total,
                "offset": offset,
                "limit": limit
            });

            return req.success_response(result);
        }

        let result = serde_json::json!({
            "sessions": [],
            "total": 0,
            "offset": offset,
            "limit": limit
        });

        req.success_response(result)
    }

    async fn handle_rate_limit_status(&self, req: McpRequest) -> McpResponse {
        let params = req.params.as_ref();
        let client_id = params
            .and_then(|p| p.get("api_key"))
            .and_then(|v| v.as_str())
            .unwrap_or("anonymous");

        let status = self.rate_limiter.get_status(client_id);

        let result = serde_json::json!({
            "client_id": client_id,
            "tokens_available": status.tokens_available,
            "requests_this_minute": status.requests_this_minute,
            "requests_per_minute": status.requests_per_minute,
            "concurrent_available": status.concurrent_available,
            "concurrent_limit": status.concurrent_limit
        });

        req.success_response(result)
    }

    pub fn subscribe_to_stream(&self) -> tokio::sync::broadcast::Receiver<StreamEvent> {
        self.stream_events.subscribe()
    }
}
