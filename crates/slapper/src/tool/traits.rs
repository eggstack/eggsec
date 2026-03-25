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

pub use crate::output::agent::AttackSurface;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCapability {
    pub name: String,
    pub description: String,
    pub parameters: Vec<ParameterDef>,
    pub examples: Vec<CapabilityExample>,
    pub attack_surface: Vec<AttackSurface>,
    pub severity_potential: Vec<crate::output::AgentSeverity>,
    pub prerequisites: Vec<String>,
    pub estimated_duration_ms: u32,
}

impl Default for ToolCapability {
    fn default() -> Self {
        Self {
            name: String::new(),
            description: String::new(),
            parameters: Vec::new(),
            examples: Vec::new(),
            attack_surface: Vec::new(),
            severity_potential: Vec::new(),
            prerequisites: Vec::new(),
            estimated_duration_ms: 30000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CapabilityExample {
    pub description: String,
    pub params: serde_json::Value,
}

impl Default for CapabilityExample {
    fn default() -> Self {
        Self {
            description: String::new(),
            params: serde_json::Value::Object(serde_json::Map::new()),
        }
    }
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

pub fn validate_parameters(
    params: &serde_json::Value,
    param_defs: &[ParameterDef],
) -> ToolResult<()> {
    let obj = match params {
        serde_json::Value::Object(m) => m,
        _ => {
            return Err(SlapperError::Validation(
                "Parameters must be an object".to_string(),
            ))
        }
    };

    for def in param_defs {
        if def.required && !obj.contains_key(&def.name) {
            return Err(SlapperError::Validation(format!(
                "Required parameter '{}' is missing",
                def.name
            )));
        }

        if let Some(value) = obj.get(&def.name) {
            match &def.param_type {
                ParameterType::Integer => {
                    if !value.is_i64() && !value.is_u64() {
                        return Err(SlapperError::Validation(format!(
                            "Parameter '{}' must be an integer",
                            def.name
                        )));
                    }
                }
                ParameterType::Boolean => {
                    if !value.is_boolean() {
                        return Err(SlapperError::Validation(format!(
                            "Parameter '{}' must be a boolean",
                            def.name
                        )));
                    }
                }
                ParameterType::Float => {
                    if !value.is_number() {
                        return Err(SlapperError::Validation(format!(
                            "Parameter '{}' must be a number",
                            def.name
                        )));
                    }
                }
                ParameterType::Array => {
                    if !value.is_array() {
                        return Err(SlapperError::Validation(format!(
                            "Parameter '{}' must be an array",
                            def.name
                        )));
                    }
                }
                ParameterType::Object => {
                    if !value.is_object() {
                        return Err(SlapperError::Validation(format!(
                            "Parameter '{}' must be an object",
                            def.name
                        )));
                    }
                }
                ParameterType::Url => {
                    if let Some(s) = value.as_str() {
                        if !s.starts_with("http://") && !s.starts_with("https://") {
                            return Err(SlapperError::Validation(format!(
                                "Parameter '{}' must be a valid URL (starting with http:// or https://)",
                                def.name
                            )));
                        }
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
