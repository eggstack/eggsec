use chrono::Utc;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{Duration, Interval};

#[cfg(feature = "rest-api")]
use crate::config::Scope;

use crate::tool::{
    CancellationToken, ExecutionHistory, RateLimitConfig, RateLimiter, RequestOptions, SessionManager, Target, ToolDispatcher, ToolInfo, 
    ToolRegistry, ToolRequest, ToolResponse,
};

#[cfg(feature = "ai-integration")]
use crate::ai::AiClient;

use super::auth::{validate_auth, validate_auth_params};
use super::prompts::get_builtin_prompts;
use super::types::{CapabilitySummary, McpError, McpRequest, McpResource, McpResponse, McpTool};
use super::streaming::StreamEvent;

#[derive(Clone)]
pub struct McpServer {
    pub(crate) registry: ToolRegistry,
    dispatcher: ToolDispatcher,
    api_key: Option<String>,
    rate_limiter: RateLimiter,
    session_manager: Option<SessionManager>,
    pending_cancellations: Arc<RwLock<HashMap<String, CancellationToken>>>,
    completed_results: Arc<RwLock<HashMap<String, ToolResponse>>>,
    stream_events: Arc<tokio::sync::broadcast::Sender<StreamEvent>>,
    #[cfg(feature = "ai-integration")]
    ai_client: Option<AiClient>,
    #[cfg(feature = "rest-api")]
    scope: Option<Scope>,
}

impl McpServer {
    pub fn new(registry: ToolRegistry, api_key: Option<String>) -> Self {
        Self::with_scope(registry, api_key, None)
    }

    pub fn with_scope(registry: ToolRegistry, api_key: Option<String>, scope: Option<Scope>) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        let (stream_events, _) = tokio::sync::broadcast::channel(1000);
        
        let pending_cancellations = Arc::new(RwLock::new(HashMap::new()));
        let completed_results = Arc::new(RwLock::new(HashMap::new()));
        
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
        };
        
        server.start_hashmap_reaper(60);
        
        server
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
                    pending.retain(|_, token| {
                        !token.is_cancelled()
                    });
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
            "prompts/list" => self.handle_prompts_list(req).await,
            "prompts/read" => self.handle_prompts_read(req).await,
            "ping" => self.handle_ping(req).await,
            _ => req.not_found_method(),
        }
    }

    async fn handle_initialize(&self, req: McpRequest) -> McpResponse {
        let result = serde_json::json!({
            "protocolVersion": "2024-11-05",
            "capabilities": {
                "tools": {
                    "listChanged": true
                },
                "streaming": true,
                "sessions": true
            },
            "serverInfo": {
                "name": "slapper-tool-api",
                "version": "0.1.0",
                "description": "High-performance security testing toolkit for AI agents"
            }
        });

        req.success_response(result)
    }

    async fn handle_tools_list(&self, req: McpRequest) -> McpResponse {
        let tools = self.registry.list();

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
        let tools = self.registry.list();
        let total_tools = tools.len();
        
        let mut categorized: HashMap<String, Vec<McpTool>> = HashMap::new();
        
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

        let client_id = params.get("api_key").and_then(|v| v.as_str()).unwrap_or("anonymous");
        
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

        let (tool_id, capability) = match self.resolve_tool_id(name) {
            Some(result) => result,
            None => {
                return req.error_response(McpError::invalid_params(&format!(
                    "Unknown tool or capability: {}",
                    name
                )))
            }
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
                let result = serde_json::json!({
                    "content": [
                        {
                            "type": "text",
                            "text": serde_json::to_string_pretty(&response).unwrap_or_default()
                        }
                    ],
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
        let resources = vec![
            McpResource {
                uri: "slapper://tools".to_string(),
                name: "Available Tools".to_string(),
                description: "List of all available security tools".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpResource {
                uri: "slapper://manifest".to_string(),
                name: "Tool Manifest".to_string(),
                description: "Complete manifest of all tools, capabilities, and attack surfaces".to_string(),
                mime_type: Some("application/json".to_string()),
            },
            McpResource {
                uri: "slapper://vulnerabilities".to_string(),
                name: "Vulnerability Types".to_string(),
                description: "List of detectable vulnerability types with CWE mappings".to_string(),
                mime_type: Some("application/json".to_string()),
            },
        ];

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
            "slapper://tools" => {
                let tools = self.registry.list();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": "slapper://tools",
                            "mimeType": "application/json",
                            "text": serde_json::to_string(&tools).unwrap_or_default()
                        }
                    ]
                });
                req.success_response(result)
            }
            "slapper://manifest" => {
                let manifest = self.build_manifest();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": "slapper://manifest",
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&manifest).unwrap_or_default()
                        }
                    ]
                });
                req.success_response(result)
            }
            "slapper://vulnerabilities" => {
                let vulns = self.build_vulnerability_catalog();
                let result = serde_json::json!({
                    "contents": [
                        {
                            "uri": "slapper://vulnerabilities",
                            "mimeType": "application/json",
                            "text": serde_json::to_string_pretty(&vulns).unwrap_or_default()
                        }
                    ]
                });
                req.success_response(result)
            }
            _ => req.error_response(McpError::invalid_params("Unknown resource uri")),
        }
    }

    async fn handle_prompts_list(&self, req: McpRequest) -> McpResponse {
        let prompts = get_builtin_prompts();
        
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
        
        let prompts = get_builtin_prompts();
        
        if let Some(prompt) = prompts.into_iter().find(|p| p.name == name) {
            let result = serde_json::json!({
                "prompt": prompt
            });
            req.success_response(result)
        } else {
            req.error_response(McpError::invalid_params(&format!("Unknown prompt: {}", name)))
        }
    }

    fn build_manifest(&self) -> serde_json::Value {
        let tools = self.registry.list();
        
        let mut attack_surfaces: HashMap<String, Vec<String>> = HashMap::new();
        
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
            "version": "0.1.0",
            "server": "slapper-tool-api",
            "tools_count": tools.len(),
            "attack_surfaces": attack_surfaces,
            "generated_at": Utc::now().to_rfc3339()
        })
    }

    fn build_vulnerability_catalog(&self) -> serde_json::Value {
        use crate::output::AgentSeverity;

        #[derive(Default)]
        struct VulnEntry {
            name: String,
            max_severity: AgentSeverity,
            tools: Vec<String>,
            attack_surfaces: Vec<String>,
        }

        let mut vuln_map: HashMap<String, VulnEntry> = HashMap::new();

        for tool_info in self.registry.list() {
            for cap in &tool_info.capabilities {
                let type_key = cap.name.to_lowercase().replace(' ', "_");

                let entry = vuln_map.entry(type_key.clone()).or_default();
                if entry.name.is_empty() {
                    entry.name = cap.name.clone();
                }

                for severity in &cap.severity_potential {
                    let current_int = match entry.max_severity {
                        AgentSeverity::Critical => 4,
                        AgentSeverity::High => 3,
                        AgentSeverity::Medium => 2,
                        AgentSeverity::Low => 1,
                        AgentSeverity::Info => 0,
                    };
                    let new_int = match severity {
                        AgentSeverity::Critical => 4,
                        AgentSeverity::High => 3,
                        AgentSeverity::Medium => 2,
                        AgentSeverity::Low => 1,
                        AgentSeverity::Info => 0,
                    };
                    if new_int > current_int {
                        entry.max_severity = *severity;
                    }
                }

                if !entry.tools.contains(&tool_info.name) {
                    entry.tools.push(tool_info.name.clone());
                }

                for surface in &cap.attack_surface {
                    let surface_name = surface.display_name();
                    if !entry.attack_surfaces.contains(&surface_name.to_string()) {
                        entry.attack_surfaces.push(surface_name.to_string());
                    }
                }
            }
        }

        let severity_str = |s: AgentSeverity| match s {
            AgentSeverity::Critical => "critical",
            AgentSeverity::High => "high",
            AgentSeverity::Medium => "medium",
            AgentSeverity::Low => "low",
            AgentSeverity::Info => "info",
        };

        let vulnerabilities: Vec<_> = vuln_map
            .into_iter()
            .map(|(vtype, entry)| {
                serde_json::json!({
                    "type": vtype,
                    "name": entry.name,
                    "severity": severity_str(entry.max_severity),
                    "tools": entry.tools,
                    "attack_surfaces": entry.attack_surfaces,
                })
            })
            .collect();

        serde_json::json!({
            "vulnerabilities": vulnerabilities,
            "generated_at": Utc::now().to_rfc3339()
        })
    }

    async fn handle_ping(&self, req: McpRequest) -> McpResponse {
        req.success_response(serde_json::json!({
            "status": "ok",
            "timestamp": Utc::now().to_rfc3339(),
            "version": "0.1.0"
        }))
    }

    async fn handle_tools_call_stream(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let client_id = params.get("api_key").and_then(|v| v.as_str()).unwrap_or("anonymous");
        
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

        let (tool_id, capability) = match self.resolve_tool_id(name) {
            Some(result) => result,
            None => {
                return req.error_response(McpError::invalid_params(&format!(
                    "Unknown tool or capability: {}",
                    name
                )))
            }
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

        let cancellation_token = CancellationToken::new();
        let cancellation_token_clone = cancellation_token.clone();
        let cancellation_handle = cancellation_token.wrap();
        let request_id = uuid::Uuid::new_v4().to_string();
        let request_id_for_response = request_id.clone();

        self.pending_cancellations
            .write()
            .await
            .insert(request_id.clone(), cancellation_token_clone);

        let mut request = ToolRequest::new(tool_id.clone(), target);
        request = request
            .with_params(request_args.clone())
            .with_options(options.clone());
        request =
            request.with_cancellation(cancellation_handle.with_request_id(request_id.clone()));

        let dispatcher = self.dispatcher.clone();
        let completed_results = Arc::clone(&self.completed_results);
        let pending_cancellations = Arc::clone(&self.pending_cancellations);
        let stream_events = Arc::clone(&self.stream_events);
        let request_id_for_result = request_id.clone();

        tokio::spawn(async move {
            let start_time = Utc::now();
            
            let _ = stream_events.send(StreamEvent {
                event_type: "started".to_string(),
                request_id: request_id_for_result.clone(),
                data: serde_json::json!({
                    "message": "Tool execution started",
                    "started_at": start_time.to_rfc3339()
                }),
            });

            let result = dispatcher.dispatch(request).await;
            pending_cancellations
                .write()
                .await
                .remove(&request_id_for_result);
            
            match result {
                Ok(response) => {
                    let _ = stream_events.send(StreamEvent {
                        event_type: "completed".to_string(),
                        request_id: request_id_for_result.clone(),
                        data: serde_json::json!({
                            "message": "Tool execution completed",
                            "completed_at": Utc::now().to_rfc3339(),
                            "status": format!("{:?}", response.status)
                        }),
                    });
                    completed_results
                        .write()
                        .await
                        .insert(request_id_for_result, response);
                }
                Err(e) => {
                    let _ = stream_events.send(StreamEvent {
                        event_type: "error".to_string(),
                        request_id: request_id_for_result.clone(),
                        data: serde_json::json!({
                            "message": "Tool execution failed",
                            "error": e.to_string()
                        }),
                    });
                    let error_response = ToolResponse {
                        request_id: request_id_for_result.clone(),
                        tool_id,
                        status: crate::tool::ResponseStatus::Failed,
                        results: serde_json::json!({}),
                        metadata: crate::tool::ResponseMetadata {
                            started_at: start_time,
                            completed_at: Utc::now(),
                            duration_ms: 0,
                            targets_scanned: 0,
                            findings_count: 0,
                        },
                        errors: vec![crate::tool::ToolError::new(
                            "EXECUTION_ERROR",
                            e.to_string(),
                        )],
                        findings: vec![],
                    };
                    completed_results
                        .write()
                        .await
                        .insert(request_id_for_result, error_response);
                }
            }
        });

        let result = serde_json::json!({
            "request_id": request_id_for_response,
            "status": "started",
            "stream_url": format!("/mcp/stream/{}", request_id_for_response)
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

        if let Some(cancellation) = self.pending_cancellations.write().await.remove(request_id) {
            cancellation.cancel();
            let result = serde_json::json!({
                "cancelled": true,
                "request_id": request_id
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
                    
                    let _ = manager.update_session(&session).await;
                    
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
                let offset = p.get("offset")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0) as usize;
                let limit = p.get("limit")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(50) as usize;
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

fn build_capabilities_summary(info: &ToolInfo) -> Vec<CapabilitySummary> {
    info.capabilities
        .iter()
        .map(|cap| CapabilitySummary {
            name: cap.name.clone(),
            description: cap.description.clone(),
            attack_surface: cap
                .attack_surface
                .iter()
                .map(|s| format!("{:?}", s).to_lowercase())
                .collect(),
            severity_potential: cap
                .severity_potential
                .iter()
                .map(|s| format!("{}", s))
                .collect(),
        })
        .collect()
}

fn build_input_schema(info: &ToolInfo) -> serde_json::Value {
    let mut properties = serde_json::Map::new();

    properties.insert(
        "target".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "Target URL, domain, or IP address"
        }),
    );

    properties.insert(
        "target_type".to_string(),
        serde_json::json!({
            "type": "string",
            "description": "Type of target: url, domain, ip, or cidr",
            "enum": ["url", "domain", "ip", "cidr"],
            "default": "url"
        }),
    );

    for cap in &info.capabilities {
        for param in &cap.parameters {
            let mut param_schema = serde_json::json!({
                "type": param.param_type.to_string(),
                "description": param.description,
            });

            if let Some(ref default) = param.default {
                param_schema["default"] = default.clone();
            }

            properties.insert(param.name.clone(), param_schema);
        }
    }

    let required: Vec<String> = vec!["target".to_string()];

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": required
    })
}
