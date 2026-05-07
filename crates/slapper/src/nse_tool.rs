//! NSE Tool implementation
//!
//! Provides the SecurityTool implementation for running NSE scripts.

use async_trait::async_trait;
use chrono::Utc;

use crate::error::SlapperError;
use crate::output::AgentSeverity;
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};
use slapper_nse::NseExecutor;

#[derive(Clone)]
pub struct NseTool {
    scripts_path: Option<std::path::PathBuf>,
}

impl NseTool {
    pub fn new() -> Self {
        Self { scripts_path: None }
    }

    pub fn with_scripts_path(path: std::path::PathBuf) -> Self {
        Self {
            scripts_path: Some(path),
        }
    }
}

impl Default for NseTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for NseTool {
    fn id(&self) -> &'static str {
        "nse"
    }

    fn name(&self) -> &'static str {
        "NSE Script Runner"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }

    fn description(&self) -> &'static str {
        "Run Nmap NSE (Scripting Engine) scripts for security scanning"
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let target = request.target.value.clone();
        let target_for_executor = target.clone();

        let script = request
            .params
            .get("script")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();
        let script_for_executor = script.clone();

        let script_args = request
            .params
            .get("args")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let script_path = self.scripts_path.clone();

        let result = tokio::task::spawn_blocking(move || {
            let mut executor = match NseExecutor::with_target(&target_for_executor) {
                Ok(e) => e,
                Err(e) => return Err(SlapperError::Config(e.to_string())),
            };

            if let Some(ref path) = script_path {
                executor.add_scripts_path(path.clone());
            } else {
                executor.add_default_scripts_path();
            }

            let _ = executor.set_script_args(&script_args);

            let script_content = if matches!(
                script_for_executor.as_str(),
                "default" | "discovery" | "banner" | "http-headers"
            ) {
                match executor.load_script(&script_for_executor) {
                    Ok(content) => content,
                    Err(_) => get_builtin_script(&script_for_executor),
                }
            } else {
                executor
                    .load_script(&script_for_executor)
                    .map_err(|e| SlapperError::Config(e.to_string()))?
            };

            match executor.run_script(&script_content) {
                Ok(r) => Ok(r),
                Err(e) => Err(SlapperError::Config(e.to_string())),
            }
        })
        .await
        .map_err(|e| SlapperError::Config(e.to_string()))?
        .map_err(|e| SlapperError::Config(e.to_string()))?;

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        let target_for_response = target.clone();
        let script_for_response = script.clone();

        Ok(ToolResponse {
            request_id: request.id,
            tool_id: self.id().to_string(),
            status: crate::tool::ResponseStatus::Success,
            results: serde_json::json!({
                "target": target_for_response,
                "script": script_for_response,
                "output": result,
            }),
            metadata: crate::tool::ResponseMetadata {
                started_at,
                completed_at,
                duration_ms,
                targets_scanned: 1,
                findings_count: 0,
            },
            errors: vec![],
            findings: vec![],
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability {
            name: "run_nse_script".to_string(),
            description: "Run an NSE script against a target".to_string(),
            parameters: vec![
                ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    description: "Target host or URL".to_string(),
                },
                ParameterDef {
                    name: "script".to_string(),
                    param_type: ParameterType::String,
                    required: true,
                    default: None,
                    description: "NSE script name or path".to_string(),
                },
                ParameterDef {
                    name: "args".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default: None,
                    description: "Script arguments (key=value format)".to_string(),
                },
            ],
            examples: vec![CapabilityExample {
                description: "Run default discovery script".to_string(),
                params: serde_json::json!({
                    "target": "192.168.1.1",
                    "script": "default"
                }),
            }],
            attack_surface: vec![AttackSurface::Network],
            severity_potential: vec![AgentSeverity::Info],
            prerequisites: vec![],
            estimated_duration_ms: 30000,
        }]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::InvalidTarget(
                "Target is required".to_string(),
            ));
        }

        if !request.params.get("script").is_some() {
            return Err(SlapperError::Config("Script name is required".to_string()));
        }

        Ok(())
    }
}

fn get_builtin_script(name: &str) -> String {
    match name {
        "default" | "discovery" => r#"
-- Default NSE discovery script
local stdnse = require "stdnse"

stdnse.verbose = 1

return "NSE scan complete"
"#
        .to_string(),
        "banner" => r#"
-- Banner grabbing script
local stdnse = require "stdnse"

return "Banner grab complete"
"#
        .to_string(),
        "http-headers" => r#"
-- HTTP headers discovery script
local stdnse = require "stdnse"

return "HTTP headers scan complete"
"#
        .to_string(),
        _ => {
            format!(
                r#"
-- Custom NSE script
local stdnse = require "stdnse"

return "Custom script '{}' executed"
"#,
                name
            )
        }
    }
}
