use async_trait::async_trait;
use chrono::Utc;

use crate::error::SlapperError;
use crate::tool::traits::{SecurityTool, ToolCategory, ToolCapability, ParameterDef, ParameterType};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

#[derive(Clone)]
pub struct WafTool {
    mode: WafMode,
}

#[derive(Clone, Copy)]
pub enum WafMode {
    Detect,
    Bypass,
    Stress,
}

impl WafTool {
    pub fn new() -> Self {
        Self { mode: WafMode::Detect }
    }

    pub fn detect() -> Self {
        Self { mode: WafMode::Detect }
    }

    pub fn bypass() -> Self {
        Self { mode: WafMode::Bypass }
    }

    pub fn stress() -> Self {
        Self { mode: WafMode::Stress }
    }
}

impl Default for WafTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for WafTool {
    fn id(&self) -> &'static str {
        match self.mode {
            WafMode::Detect => "waf-detect",
            WafMode::Bypass => "waf-bypass",
            WafMode::Stress => "waf-stress",
        }
    }

    fn name(&self) -> &'static str {
        match self.mode {
            WafMode::Detect => "WAF Detector",
            WafMode::Bypass => "WAF Bypasser",
            WafMode::Stress => "WAF Stress Tester",
        }
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Waf
    }

    fn description(&self) -> &'static str {
        match self.mode {
            WafMode::Detect => "Detect Web Application Firewalls protecting the target",
            WafMode::Bypass => "Attempt to bypass WAF protections using various techniques",
            WafMode::Stress => "Comprehensive WAF stress testing with multiple attack vectors",
        }
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let target = &request.target.value;
        
        let concurrency = request.options.concurrency.unwrap_or(10);
        let timeout = request.options.timeout_ms.unwrap_or(30000);

        let result = match self.mode {
            WafMode::Detect => {
                let args = crate::cli::WafArgs {
                    url: target.clone(),
                    detect_only: true,
                    bypass: false,
                    header_bypass: false,
                    smuggling: false,
                    evasion: false,
                    profile: "auto".to_string(),
                    test_type: None,
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    verbose: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                let config = crate::config::load_config(None::<&str>).unwrap_or_default();
                crate::waf::run_cli(args, &config).await
            }
            WafMode::Bypass => {
                let args = crate::cli::WafArgs {
                    url: target.clone(),
                    detect_only: false,
                    bypass: true,
                    header_bypass: true,
                    smuggling: true,
                    evasion: true,
                    profile: "auto".to_string(),
                    test_type: None,
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    verbose: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                let config = crate::config::load_config(None::<&str>).unwrap_or_default();
                crate::waf::run_cli(args, &config).await
            }
            WafMode::Stress => {
                let args = crate::cli::WafStressArgs {
                    url: target.clone(),
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    verbose: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                let config = crate::config::load_config(None::<&str>).unwrap_or_default();
                crate::fuzzer::run_waf_stress(args, &config).await
            }
        };

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        match result {
            Ok(_) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: self.id().to_string(),
                status: crate::tool::ResponseStatus::Success,
                results: serde_json::json!({ "target": target }),
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
                tool_id: self.id().to_string(),
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
        vec![ToolCapability {
            name: match self.mode {
                WafMode::Detect => "detect".to_string(),
                WafMode::Bypass => "bypass".to_string(),
                WafMode::Stress => "stress".to_string(),
            },
            description: self.description().to_string(),
            parameters: vec![ParameterDef {
                name: "target".to_string(),
                param_type: ParameterType::Url,
                required: true,
                default: None,
                description: "Target URL".to_string(),
            }],
        }]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
