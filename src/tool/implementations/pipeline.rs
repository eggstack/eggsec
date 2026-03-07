use async_trait::async_trait;
use chrono::Utc;

use crate::error::SlapperError;
use crate::tool::traits::{SecurityTool, ToolCategory, ToolCapability, ParameterDef, ParameterType};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

#[derive(Clone)]
pub struct PipelineTool;

impl PipelineTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for PipelineTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for PipelineTool {
    fn id(&self) -> &'static str {
        "scan"
    }

    fn name(&self) -> &'static str {
        "Security Assessment Pipeline"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Pipeline
    }

    fn description(&self) -> &'static str {
        "Execute chained security assessments with multiple stages including port scanning, fingerprinting, endpoint discovery, fuzzing, and load testing."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let target = &request.target.value;
        
        let params = &request.params;
        
        let profile = params
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("quick")
            .to_string();

        let profile_enum = match profile.as_str() {
            "quick" => crate::cli::ScanProfile::Quick,
            "endpoint" => crate::cli::ScanProfile::Endpoint,
            "web" => crate::cli::ScanProfile::Web,
            "waf" => crate::cli::ScanProfile::Waf,
            "full" => crate::cli::ScanProfile::Full,
            "api" => crate::cli::ScanProfile::Api,
            "recon" => crate::cli::ScanProfile::Recon,
            "stealth" => crate::cli::ScanProfile::Stealth,
            "deep" => crate::cli::ScanProfile::Deep,
            "vuln" => crate::cli::ScanProfile::Vuln,
            "auth" => crate::cli::ScanProfile::Auth,
            _ => crate::cli::ScanProfile::Quick,
        };

        let args = crate::cli::ScanArgs {
            target: target.clone(),
            profile: profile_enum,
            stages: None,
            concurrency: 10,
            json: true,
            output: None,
            format: None,
            web_types: None,
            common: crate::cli::CommonHttpArgs::default(),
            source_ip: None,
            spoof_range: None,
            decoy: None,
            decoy_range: None,
            decoy_count: None,
            decoy_mode: None,
            include_me: false,
            random_source_port: false,
            fragment: false,
            scan_type: None,
            packet_trace: None,
            max_rate: None,
            ttl: None,
            source_port: None,
            verbose: false,
        };

        let config = crate::config::load_config(None::<&str>).unwrap_or_default();
        let result = crate::pipeline::run_cli(args, &config).await;

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        match result {
            Ok(_) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: "scan".to_string(),
                status: crate::tool::ResponseStatus::Success,
                results: serde_json::json!({ "target": target, "profile": profile }),
                metadata: crate::tool::ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms,
                    targets_scanned: 1,
                    findings_count: 0,
                },
                errors: vec![],
                findings: vec![],
            }),
            Err(e) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: "scan".to_string(),
                status: crate::tool::ResponseStatus::Failed,
                results: serde_json::json!({}),
                metadata: crate::tool::ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms,
                    targets_scanned: 0,
                    findings_count: 0,
                },
                errors: vec![crate::tool::ToolError::new("EXECUTION_ERROR", e.to_string())],
                findings: vec![],
            }),
        }
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability {
                name: "quick".to_string(),
                description: "Fast port scan and service fingerprinting".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain or IP".to_string(),
                }],
            },
            ToolCapability {
                name: "endpoint".to_string(),
                description: "Quick + directory/endpoint discovery".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain or IP".to_string(),
                }],
            },
            ToolCapability {
                name: "web".to_string(),
                description: "Endpoint + web vulnerability fuzzing".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain or IP".to_string(),
                }],
            },
            ToolCapability {
                name: "full".to_string(),
                description: "All stages including load testing".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain or IP".to_string(),
                }],
            },
            ToolCapability {
                name: "api".to_string(),
                description: "GraphQL, JWT, OAuth focused".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain or IP".to_string(),
                }],
            },
            ToolCapability {
                name: "recon".to_string(),
                description: "Intelligence-led with tech detection and CVE mapping".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain or IP".to_string(),
                }],
            },
        ]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
