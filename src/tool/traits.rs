use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::error::SlapperError;
use crate::tool::{request::ToolRequest, response::ToolResponse};

pub type ToolResult<T> = Result<T, SlapperError>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Ord, PartialOrd)]
pub enum ToolCategory {
    Recon,
    Scanning,
    Fuzzing,
    Waf,
    LoadTest,
    Stress,
    Pipeline,
}

impl std::fmt::Display for ToolCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolCategory::Recon => write!(f, "Reconnaissance"),
            ToolCategory::Scanning => write!(f, "Scanning"),
            ToolCategory::Fuzzing => write!(f, "Fuzzing"),
            ToolCategory::Waf => write!(f, "WAF"),
            ToolCategory::LoadTest => write!(f, "Load Testing"),
            ToolCategory::Stress => write!(f, "Stress Testing"),
            ToolCategory::Pipeline => write!(f, "Pipeline"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterDef>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterDef {
    pub name: String,
    pub param_type: ParameterType,
    pub required: bool,
    pub default: Option<serde_json::Value>,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ParameterType {
    String,
    Integer,
    Boolean,
    Float,
    Array,
    Object,
    Url,
    Ip,
    Domain,
}

impl std::fmt::Display for ParameterType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParameterType::String => write!(f, "string"),
            ParameterType::Integer => write!(f, "integer"),
            ParameterType::Boolean => write!(f, "boolean"),
            ParameterType::Float => write!(f, "float"),
            ParameterType::Array => write!(f, "array"),
            ParameterType::Object => write!(f, "object"),
            ParameterType::Url => write!(f, "url"),
            ParameterType::Ip => write!(f, "ip"),
            ParameterType::Domain => write!(f, "domain"),
        }
    }
}

#[async_trait]
pub trait SecurityTool: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn category(&self) -> ToolCategory;
    fn description(&self) -> &'static str;

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse>;

    fn validate(&self, _request: &ToolRequest) -> ToolResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![]
    }

    fn supported_protocols(&self) -> Vec<&'static str> {
        vec!["http", "https"]
    }
}

#[async_trait]
pub trait SyncSecurityTool: Send + Sync {
    fn id(&self) -> &'static str;
    fn name(&self) -> &'static str;
    fn category(&self) -> ToolCategory;
    fn description(&self) -> &'static str;

    fn execute_blocking(&self, request: ToolRequest) -> ToolResult<ToolResponse>;

    fn validate(&self, _request: &ToolRequest) -> ToolResult<()> {
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![]
    }
}
