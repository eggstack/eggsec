use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use axum::{
    routing::post,
    Router,
    extract::Json,
    response::IntoResponse,
    http::StatusCode,
    extract::State,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use subtle::ConstantTimeEq;

use crate::tool::{ToolRegistry, ToolDispatcher, ToolRequest, Target, RequestOptions};

#[derive(Clone)]
pub struct McpServer {
    registry: ToolRegistry,
    dispatcher: ToolDispatcher,
    api_key: Option<String>,
}

impl McpServer {
    pub fn new(registry: ToolRegistry, api_key: Option<String>) -> Self {
        let dispatcher = ToolDispatcher::new(registry.clone());
        Self { registry, dispatcher, api_key }
    }

    fn validate_auth(&self, headers: &axum::http::HeaderMap) -> Result<(), McpError> {
        if let Some(ref key) = self.api_key {
            let auth = headers.get("authorization")
                .or_else(|| headers.get("x-api-key"))
                .and_then(|v| v.to_str().ok());
            
            match auth {
                Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
                _ => Err(McpError::unauthorized()),
            }
        } else {
            Ok(())
        }
    }

    pub fn validate_auth_params(&self, params: &Option<serde_json::Value>) -> Result<(), McpError> {
        if let Some(ref key) = self.api_key {
            let auth = params
                .as_ref()
                .and_then(|p| p.get("api_key"))
                .and_then(|v| v.as_str());
            
            match auth {
                Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
                _ => Err(McpError::unauthorized()),
            }
        } else {
            Ok(())
        }
    }

    pub async fn handle_request(&self, req: McpRequest) -> McpResponse {
        match req.method.as_str() {
            "initialize" => self.handle_initialize(req).await,
            "tools/list" => self.handle_tools_list(req).await,
            "tools/call" => self.handle_tools_call(req).await,
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
                "tools": {}
            },
            "serverInfo": {
                "name": "slapper-tool-api",
                "version": "0.1.0"
            }
        });
        
        req.success_response(result)
    }

    async fn handle_tools_list(&self, req: McpRequest) -> McpResponse {
        let tools = self.registry.list();
        
        let mcp_tools: Vec<McpTool> = tools.into_iter().map(|info| {
            let input_schema = build_input_schema(&info.capabilities);
            
            McpTool {
                name: info.id,
                description: info.description,
                input_schema,
            }
        }).collect();

        let result = serde_json::json!({
            "tools": mcp_tools
        });

        req.success_response(result)
    }

    async fn handle_tools_call(&self, req: McpRequest) -> McpResponse {
        let params = match &req.params {
            Some(p) => p,
            None => return req.error_response(McpError::invalid_params("Missing params")),
        };

        let tool_name = match params.get("name").and_then(|v| v.as_str()) {
            Some(name) => name,
            None => return req.error_response(McpError::invalid_params("Missing tool name")),
        };

        let arguments = params.get("arguments")
            .cloned()
            .unwrap_or(serde_json::Value::Object(serde_json::Map::new()));

        let target_value = arguments.get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let target_type = arguments.get("target_type")
            .and_then(|v| v.as_str())
            .unwrap_or("url");

        let target = match target_type {
            "domain" => Target::domain(target_value),
            "ip" => Target::ip(target_value),
            "cidr" => Target::cidr(target_value),
            _ => Target::url(target_value),
        };

        let options = RequestOptions {
            timeout_ms: arguments.get("timeout_ms").and_then(|v| v.as_u64()),
            concurrency: arguments.get("concurrency").and_then(|v| v.as_u64()).map(|v| v as usize),
            ..Default::default()
        };

        let request = ToolRequest::new(tool_name.to_string(), target)
            .with_params(arguments)
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

    async fn handle_resources_list(&self, req: McpRequest) -> McpResponse {
        let resources = vec![
            McpResource {
                uri: "slapper://tools".to_string(),
                name: "Available Tools".to_string(),
                description: "List of all available security tools".to_string(),
                mime_type: Some("application/json".to_string()),
            }
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

        if uri == "slapper://tools" {
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

        req.error_response(McpError::invalid_params("Unknown resource uri"))
    }

    async fn handle_ping(&self, req: McpRequest) -> McpResponse {
        req.success_response(serde_json::json!({}))
    }
}

fn build_input_schema(capabilities: &[crate::tool::ToolCapability]) -> serde_json::Value {
    let mut properties = serde_json::Map::new();
    
    properties.insert("target".to_string(), serde_json::json!({
        "type": "string",
        "description": "Target URL, domain, or IP address"
    }));

    for cap in capabilities {
        for param in &cap.parameters {
            properties.insert(param.name.clone(), serde_json::json!({
                "type": param.param_type.to_string(),
                "description": param.description,
            }));
        }
    }

    serde_json::json!({
        "type": "object",
        "properties": properties,
        "required": ["target"]
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpTool {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpResource {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[cfg(feature = "mcp-server")]
pub async fn create_mcp_router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let server = Arc::new(McpServer::new(registry, api_key));

    Router::new()
        .route("/mcp", post(handle_mcp))
        .route("/json-rpc", post(handle_mcp))
        .with_state(server)
}

#[cfg(feature = "mcp-server")]
async fn handle_mcp(
    State(server): State<Arc<McpServer>>,
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
                error: Some(McpError::invalid_request(&format!("Batch size exceeds limit of {}", MAX_BATCH_SIZE))),
            }])
        );
    }
    
    let mut responses = Vec::new();
    
    for req in requests {
        if req.method != "initialize" {
            if let Err(e) = server.validate_auth(&headers) {
                responses.push(req.error_response(e));
                continue;
            }
        }
        
        let response = server.handle_request(req).await;
        responses.push(response);
    }

    (StatusCode::OK, Json(responses))
}

#[cfg(feature = "mcp-server")]
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
