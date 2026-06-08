use async_trait::async_trait;
use chrono::Utc;

use crate::error::SlapperError;
use crate::output::AgentSeverity;
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
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
        Self {
            mode: WafMode::Detect,
        }
    }

    pub fn detect() -> Self {
        Self {
            mode: WafMode::Detect,
        }
    }

    pub fn bypass() -> Self {
        Self {
            mode: WafMode::Bypass,
        }
    }

    pub fn stress() -> Self {
        Self {
            mode: WafMode::Stress,
        }
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
        let timeout = request
            .options
            .timeout_ms
            .unwrap_or(crate::constants::DEFAULT_TOOL_TIMEOUT_MS);

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
                    quiet: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                crate::waf::run_cli(args).await
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
                    quiet: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                crate::waf::run_cli(args).await
            }
            WafMode::Stress => {
                let args = crate::cli::WafStressArgs {
                    url: target.clone(),
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    verbose: false,
                    quiet: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                crate::fuzzer::run_waf_stress(args).await
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
                errors: vec![crate::tool::ToolError::new(
                    "EXECUTION_ERROR",
                    e.to_string(),
                )],
                findings: vec![],
            }),
        }
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        let cap_name = match self.mode {
            WafMode::Detect => "detect",
            WafMode::Bypass => "bypass",
            WafMode::Stress => "stress",
        };

        let description = match self.mode {
            WafMode::Detect => "Detect WAF and its type",
            WafMode::Bypass => "Test WAF bypass techniques",
            WafMode::Stress => "Stress test WAF defenses",
        };

        let (attack_surface, severity_potential, estimated_duration) = match self.mode {
            WafMode::Detect => (
                vec![AttackSurface::Web, AttackSurface::Cdn],
                vec![AgentSeverity::Info],
                30000u32,
            ),
            WafMode::Bypass => (
                vec![AttackSurface::Web],
                vec![AgentSeverity::Medium, AgentSeverity::Low],
                120000,
            ),
            WafMode::Stress => (vec![AttackSurface::Web], vec![AgentSeverity::Info], 300000),
        };

        vec![ToolCapability {
            name: cap_name.to_string(),
            description: description.to_string(),
            parameters: vec![ParameterDef {
                name: "target".to_string(),
                param_type: ParameterType::Url,
                required: true,
                default: None,
                description: "Target URL".to_string(),
            }],
            examples: vec![CapabilityExample {
                description: format!("{} on target", description),
                params: serde_json::json!({
                    "target": "https://example.com"
                }),
            }],
            attack_surface,
            severity_potential,
            prerequisites: vec![],
            estimated_duration_ms: estimated_duration,
        }]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
