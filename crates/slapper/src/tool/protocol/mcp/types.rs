use serde::{Deserialize, Serialize};

use crate::utils::error::sanitize_error_message;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
            message: sanitize_error_message(msg),
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
            message: format!("Rate limit exceeded: {}", crate::utils::error::sanitize_error_message(msg)),
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilitySummary {
    pub name: String,
    pub description: String,
    pub attack_surface: Vec<String>,
    pub severity_potential: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpRoot {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct McpNotification {
    pub method: String,
    pub params: Option<serde_json::Value>,
}

impl McpNotification {
    pub fn shutdown() -> Self {
        Self {
            method: "shutdown".to_string(),
            params: None,
        }
    }

    pub fn roots_changed() -> Self {
        Self {
            method: "roots/list_changed".to_string(),
            params: None,
        }
    }

    pub fn to_jsonrpc_notification(&self) -> serde_json::Value {
        serde_json::json!({
            "jsonrpc": "2.0",
            "method": self.method,
            "params": self.params
        })
    }
}
