use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::{sse::Event as SseEvent, IntoResponse, Sse},
    routing::{get, post},
    Router,
};
use async_stream::stream;
use chrono::Utc;
use futures::Stream;
use serde::{Deserialize, Serialize};
use std::{sync::Arc, time::Duration};
use subtle::ConstantTimeEq;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::sync::broadcast;

use crate::tool::{
    CancellationToken, ExecutionHistory, RateLimitConfig, RateLimiter, OpenApiGenerator, ChainPlanner,
    ExecutionPlan, PlanRequest, RequestOptions, SessionManager, Target, ToolDispatcher, ToolInfo, 
    ToolRegistry, ToolRequest, ToolResponse,
};
use std::collections::HashMap;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct McpServer {
    registry: ToolRegistry,
    dispatcher: ToolDispatcher,
    api_key: Option<String>,
    rate_limiter: RateLimiter,
    session_manager: Option<SessionManager>,
    pending_cancellations: Arc<Mutex<HashMap<String, CancellationToken>>>,
    completed_results: Arc<Mutex<HashMap<String, ToolResponse>>>,
    stream_events: Arc<broadcast::Sender<StreamEvent>>,
}

impl McpServer {
    pub fn new(registry: ToolRegistry, api_key: Option<String>) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        let (stream_events, _) = broadcast::channel(1000);
        
        Self {
            registry,
            dispatcher,
            api_key,
            rate_limiter: RateLimiter::new(RateLimitConfig::default()),
            session_manager: None,
            pending_cancellations: Arc::new(Mutex::new(HashMap::new())),
            completed_results: Arc::new(Mutex::new(HashMap::new())),
            stream_events: Arc::new(stream_events),
        }
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
        }
    }

    fn validate_auth_internal(&self, key_input: Option<&str>) -> Result<(), McpError> {
        if let Some(ref key) = self.api_key {
            match key_input {
                Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
                _ => Err(McpError::unauthorized()),
            }
        } else {
            Ok(())
        }
    }

    pub fn validate_auth(&self, headers: &axum::http::HeaderMap) -> Result<(), McpError> {
        let key = headers
            .get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok());
        self.validate_auth_internal(key)
    }

    pub fn validate_auth_params(&self, params: &Option<serde_json::Value>) -> Result<(), McpError> {
        let key = params
            .as_ref()
            .and_then(|p| p.get("api_key"))
            .and_then(|v| v.as_str());
        self.validate_auth_internal(key)
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
            categorized.entry(category).or_insert_with(Vec::new).push(mcp_tool);
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
                return req.success_response(result);
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
                return req.success_response(result);
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
                return req.success_response(result);
            }
            _ => req.error_response(McpError::invalid_params("Unknown resource uri")),
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
                        .or_insert_with(Vec::new)
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
        serde_json::json!({
            "vulnerabilities": [
                {"type": "sqli", "name": "SQL Injection", "cwe": ["CWE-89"], "severity": "critical"},
                {"type": "xss", "name": "Cross-Site Scripting", "cwe": ["CWE-79"], "severity": "high"},
                {"type": "ssrf", "name": "Server-Side Request Forgery", "cwe": ["CWE-918"], "severity": "high"},
                {"type": "path_traversal", "name": "Path Traversal", "cwe": ["CWE-22"], "severity": "high"},
                {"type": "cmd_injection", "name": "Command Injection", "cwe": ["CWE-78"], "severity": "critical"},
                {"type": "idor", "name": "Insecure Direct Object Reference", "cwe": ["CWE-639"], "severity": "medium"},
                {"type": "ssti", "name": "Server-Side Template Injection", "cwe": ["CWE-1336"], "severity": "critical"},
                {"type": "xxe", "name": "XML External Entity", "cwe": ["CWE-611"], "severity": "high"},
                {"type": "jwt", "name": "JWT Vulnerabilities", "cwe": ["CWE-345", "CWE-347"], "severity": "high"},
                {"type": "oauth", "name": "OAuth/OIDC Vulnerabilities", "cwe": ["CWE-287"], "severity": "high"},
                {"type": "graphql", "name": "GraphQL Security Issues", "cwe": ["CWE-20"], "severity": "medium"},
                {"type": "redirect", "name": "Open Redirect", "cwe": ["CWE-601"], "severity": "medium"},
                {"type": "deser", "name": "Insecure Deserialization", "cwe": ["CWE-502"], "severity": "critical"},
                {"type": "ldap", "name": "LDAP Injection", "cwe": ["CWE-90"], "severity": "high"},
                {"type": "host", "name": "Host Header Injection", "cwe": ["CWE-74"], "severity": "medium"},
                {"type": "cache", "name": "Cache Poisoning", "cwe": ["CWE-444"], "severity": "medium"},
                {"type": "headers", "name": "HTTP Header Injection", "cwe": ["CWE-113"], "severity": "medium"},
                {"type": "redos", "name": "Regular Expression DoS", "cwe": ["CWE-1333"], "severity": "medium"},
            ]
        })
    }

    async fn handle_ping(&self, req: McpRequest) -> McpResponse {
        req.success_response(serde_json::json!({
            "status": "ok",
            "timestamp": Utc::now().to_rfc3339(),
            "version": "0.1.0"
        }))
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

        if let Some(cancellation) = self.pending_cancellations.lock().await.remove(request_id) {
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
            .lock()
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
                .lock()
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
                        .lock()
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
                        tool_id: tool_id,
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
                        .lock()
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

    async fn handle_tools_result(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let request_id = match params.get("request_id").and_then(|v| v.as_str()) {
            Some(id) => id,
            None => return req.error_response(McpError::invalid_params("Missing request_id")),
        };

        if let Some(response) = self.completed_results.lock().await.remove(request_id) {
            let result = serde_json::json!({
                "request_id": request_id,
                "status": "completed",
                "response": response
            });
            req.success_response(result)
        } else if self
            .pending_cancellations
            .lock()
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
        if let Some(ref manager) = self.session_manager {
            let sessions = manager.list_sessions().await;
            let session_list: Vec<serde_json::Value> = sessions
                .iter()
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
                "sessions": session_list,
                "count": session_list.len()
            });
            
            return req.success_response(result);
        }

        let result = serde_json::json!({
            "sessions": [],
            "count": 0,
            "message": "Session manager not configured"
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

    pub fn subscribe_to_stream(&self) -> broadcast::Receiver<StreamEvent> {
        self.stream_events.subscribe()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "event")]
    pub event_type: String,
    pub request_id: String,
    pub data: serde_json::Value,
}

impl StreamEvent {
    pub fn to_sse_data(&self) -> String {
        format!(
            "event: {}\ndata: {}\n\n",
            self.event_type,
            serde_json::to_string(&self.data).unwrap_or_default()
        )
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySummary {
    pub name: String,
    pub description: String,
    pub attack_surface: Vec<String>,
    pub severity_potential: Vec<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpRequest {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl McpRequest {
    pub fn success_response(&self, result: serde_json::Value) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id: self.id.clone(),
            result: Some(result),
            error: None,
        }
    }

    pub fn error_response(&self, error: McpError) -> McpResponse {
        McpResponse {
            jsonrpc: "2.0".to_string(),
            id: self.id.clone(),
            result: None,
            error: Some(error),
        }
    }

    pub fn not_found_method(&self) -> McpResponse {
        self.error_response(McpError::method_not_found(&self.method))
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpResponse {
    #[serde(rename = "jsonrpc")]
    pub jsonrpc: String,
    pub id: Option<serde_json::Value>,
    pub result: Option<serde_json::Value>,
    pub error: Option<McpError>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

impl McpError {
    pub fn parse_error(msg: &str) -> Self {
        Self {
            code: -32700,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn invalid_request(msg: &str) -> Self {
        Self {
            code: -32600,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    pub fn invalid_params(msg: &str) -> Self {
        Self {
            code: -32602,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn internal(msg: &str) -> Self {
        Self {
            code: -32603,
            message: msg.to_string(),
            data: None,
        }
    }

    pub fn unauthorized() -> Self {
        Self {
            code: -32001,
            message: "Unauthorized".to_string(),
            data: None,
        }
    }

    pub fn rate_limited(msg: &str) -> Self {
        Self {
            code: -32002,
            message: format!("Rate limit exceeded: {}", msg),
            data: None,
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<Vec<CapabilitySummary>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

struct AppState {
    mcp_server: Arc<McpServer>,
    planner: ChainPlanner,
    openapi_generator: OpenApiGenerator,
}

async fn handle_openapi_json(
    State(state): State<Arc<AppState>>,
) -> axum::Json<serde_json::Value> {
    let spec = state.openapi_generator.generate(&state.mcp_server.registry);
    axum::Json(serde_json::from_str(&spec.to_json()).unwrap_or_default())
}

async fn handle_openapi_yaml(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    let spec = state.openapi_generator.generate(&state.mcp_server.registry);
    (
        [("Content-Type", "application/x-yaml")],
        spec.to_yaml(),
    )
}

async fn handle_create_plan(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PlanRequest>,
) -> axum::Json<ExecutionPlan> {
    let plan = state.planner.plan(&request);
    let validation = state.planner.validate_plan(&plan);
    
    if !validation.valid {
        tracing::warn!("Plan validation failed: {:?}", validation.errors);
    }
    
    if !validation.warnings.is_empty() {
        tracing::info!("Plan warnings: {:?}", validation.warnings);
    }
    
    axum::Json(plan)
}

pub async fn create_mcp_router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let server = Arc::new(McpServer::new(registry.clone(), api_key));
    let planner = ChainPlanner::new(registry.clone());
    let openapi_generator = OpenApiGenerator::new("http://localhost:8080", "0.1.0");
    
    let app_state = Arc::new(AppState {
        mcp_server: server,
        planner,
        openapi_generator,
    });

    Router::new()
        .route("/mcp", post(handle_mcp))
        .route("/json-rpc", post(handle_mcp))
        .route("/mcp/stream/:request_id", get(handle_sse_stream))
        .route("/health", get(handle_health))
        .route("/openapi.json", get(handle_openapi_json))
        .route("/openapi.yaml", get(handle_openapi_yaml))
        .route("/plan", post(handle_create_plan))
        .with_state(app_state)
}

async fn handle_health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({
        "status": "healthy",
        "service": "slapper-mcp",
        "version": "0.1.0"
    }))
}

struct SseStreamState {
    receiver: broadcast::Receiver<StreamEvent>,
    request_id: String,
}

async fn handle_sse_stream(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(request_id): axum::extract::Path<String>,
) -> Sse<impl Stream<Item = Result<SseEvent, axum::Error>>> {
    let receiver = state.mcp_server.subscribe_to_stream();
    
    let state = Arc::new(tokio::sync::Mutex::new(SseStreamState {
        receiver,
        request_id: request_id.clone(),
    }));
    
    let stream = stream! {
        let mut tick_interval = tokio::time::interval(Duration::from_secs(30));
        
        loop {
            let event = {
                let mut s = state.lock().await;
                s.receiver.recv().await
            };
            
            match event {
                Ok(event) => {
                    let current_request_id = {
                        let s = state.lock().await;
                        s.request_id.clone()
                    };
                    if event.request_id == current_request_id || event.request_id == "*" {
                        yield Ok::<_, axum::Error>(SseEvent::default()
                            .event(&event.event_type)
                            .data(serde_json::to_string(&event.data).unwrap_or_default()));
                    }
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    yield Ok::<_, axum::Error>(SseEvent::default()
                        .event("lagged")
                        .data(format!("{{\"lagged_events\": {}}}", n)));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }

            tokio::select! {
                _ = tick_interval.tick() => {
                    yield Ok::<_, axum::Error>(SseEvent::default()
                        .event("heartbeat")
                        .data("{\"timestamp\": \"alive\"}"));
                }
                _ = tokio::time::sleep(Duration::from_millis(100)) => {
                    // Small delay to prevent busy loop
                }
            }
        }
    };

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new()
            .interval(Duration::from_secs(15)))
}

async fn handle_mcp(
    State(state): State<Arc<AppState>>,
    headers: axum::http::HeaderMap,
    Json(requests): Json<Vec<McpRequest>>,
) -> impl IntoResponse {
    const MAX_BATCH_SIZE: usize = 100;

    if requests.len() > MAX_BATCH_SIZE {
        return (
            StatusCode::BAD_REQUEST,
            Json(vec![McpResponse {
                jsonrpc: "2.0".to_string(),
                id: None,
                result: None,
                error: Some(McpError::invalid_request(&format!(
                    "Batch size exceeds limit of {}",
                    MAX_BATCH_SIZE
                ))),
            }]),
        );
    }

    let mut responses = Vec::new();

    for req in requests {
        if req.method != "initialize" {
            if let Err(e) = state.mcp_server.validate_auth(&headers) {
                responses.push(req.error_response(e));
                continue;
            }
        }

        let response = state.mcp_server.handle_request(req).await;
        responses.push(response);
    }

    (StatusCode::OK, Json(responses))
}

pub async fn run_stdio(registry: ToolRegistry, api_key: Option<String>) {
    let server = Arc::new(McpServer::new(registry, api_key));

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let mut reader: tokio::io::Lines<BufReader<tokio::io::Stdin>> = BufReader::new(stdin).lines();
    let mut writer = BufWriter::new(stdout);

    tracing::info!("MCP stdio server started, waiting for requests...");

    while let Ok(Some(line)) = reader.next_line().await {
        if line.trim().is_empty() {
            continue;
        }

        let requests: Result<Vec<McpRequest>, _> = serde_json::from_str(&line);

        match requests {
            Ok(reqs) => {
                if reqs.len() > 100 {
                    let error = McpError::invalid_request("Batch size exceeds limit of 100");
                    let response = McpResponse {
                        jsonrpc: "2.0".to_string(),
                        id: None,
                        result: None,
                        error: Some(error),
                    };
                    if let Ok(response_json) = serde_json::to_string(&response) {
                        let _ = writer.write_all(response_json.as_bytes()).await;
                        let _ = writer.write_all(b"\n").await;
                        let _ = writer.flush().await;
                    }
                    continue;
                }

                let mut responses = Vec::new();

                for req in reqs {
                    if req.method != "initialize" {
                        if let Err(e) = server.validate_auth_params(&req.params) {
                            responses.push(req.error_response(e));
                            continue;
                        }
                    }

                    let response = server.handle_request(req).await;
                    responses.push(response);
                }

                if let Ok(response_json) = serde_json::to_string(&responses) {
                    let _ = writer.write_all(response_json.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    let _ = writer.flush().await;
                }
            }
            Err(e) => {
                let error = McpError::parse_error(&format!("Invalid JSON: {}", e));
                let response = McpResponse {
                    jsonrpc: "2.0".to_string(),
                    id: None,
                    result: None,
                    error: Some(error),
                };
                if let Ok(response_json) = serde_json::to_string(&response) {
                    let _ = writer.write_all(response_json.as_bytes()).await;
                    let _ = writer.write_all(b"\n").await;
                    let _ = writer.flush().await;
                }
            }
        }
    }
}

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
