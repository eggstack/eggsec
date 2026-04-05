use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsesRequest {
    pub model: String,
    pub input: Input,
    #[serde(default)]
    pub instructions: Option<String>,
    #[serde(default)]
    pub include: Option<Vec<String>>,
    #[serde(default)]
    pub tools: Option<Vec<FunctionTool>>,
    #[serde(default)]
    pub tool_choice: Option<ToolChoice>,
    #[serde(default)]
    pub stream: Option<bool>,
    #[serde(default)]
    pub temperature: Option<f64>,
    #[serde(default)]
    pub max_output_tokens: Option<u32>,
    #[serde(default)]
    pub previous_response_id: Option<String>,
    #[serde(default)]
    pub metadata: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Input {
    Text(String),
    Items(Vec<InputItem>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputItem {
    #[serde(rename = "type")]
    pub item_type: String,
    pub content: Option<String>,
    pub name: Option<String>,
    pub call_id: Option<String>,
    pub output: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionTool {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ToolChoice {
    Auto,
    Required,
    None,
    Function { name: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponsesResponse {
    pub id: String,
    pub object: String,
    pub created_at: u64,
    pub status: String,
    pub output: Vec<OutputItem>,
    pub model: String,
    pub incomplete_details: Option<IncompleteDetails>,
    pub metadata: Option<HashMap<String, String>>,
    pub usage: Option<Usage>,
    pub error: Option<ErrorResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum OutputItem {
    Message {
        #[serde(rename = "type")]
        item_type: String,
        id: String,
        role: String,
        content: Vec<MessageContent>,
        status: Option<String>,
    },
    FunctionCall {
        #[serde(rename = "type")]
        item_type: String,
        id: String,
        name: String,
        call_id: String,
        arguments: String,
        status: Option<String>,
    },
    Finding {
        #[serde(rename = "type")]
        item_type: String,
        id: String,
        finding: AiFinding,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContent {
    Text {
        #[serde(rename = "type")]
        content_type: String,
        text: String,
    },
    Finding {
        #[serde(rename = "type")]
        content_type: String,
        finding: AiFinding,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFinding {
    pub severity: String,
    pub title: String,
    pub description: String,
    pub location: Option<String>,
    pub evidence: Vec<AiEvidence>,
    pub remediation: Option<AiRemediation>,
    pub cwe_id: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEvidence {
    pub description: String,
    pub raw_data: Option<String>,
    pub source: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRemediation {
    pub summary: String,
    pub steps: Vec<String>,
    pub priority: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IncompleteDetails {
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Usage {
    pub input_tokens: u32,
    pub output_tokens: u32,
    pub total_tokens: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub data: serde_json::Value,
}
