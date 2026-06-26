use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::error::EggsecError;
use crate::output::AgentSeverity;
use crate::pipeline::stage::profile_from_str;
use crate::tool::response::Finding;
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
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

        let findings: Arc<parking_lot::Mutex<Vec<Finding>>> =
            Arc::new(parking_lot::Mutex::new(Vec::new()));
        let findings_clone = findings.clone();

        let profile = params
            .get("profile")
            .and_then(|v| v.as_str())
            .unwrap_or("quick")
            .to_string();

        let profile_enum = profile_from_str(&profile).unwrap_or(crate::cli::ScanProfile::Quick);

        let args = crate::cli::ScanArgs {
            target: target.clone(),
            profile: profile_enum,
            stages: None,
            concurrency: None,
            concurrent_stages: false,
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

        let config = crate::config::load_config(None::<&str>)
            .inspect_err(|e| {
                tracing::warn!(error = %e, "Failed to load config for pipeline, using defaults");
            })
            .unwrap_or_default();

        let stage_count = crate::pipeline::stage::Stage::from_profile(profile_enum).len();
        let timeout_secs = (stage_count as u64 * 120).clamp(60, 600);

        tokio::time::timeout(
            std::time::Duration::from_secs(timeout_secs),
            crate::pipeline::run_cli_with_callback(args, &config, move |f| {
                let mut findings = findings_clone.lock();
                findings.push(f);
            }),
        )
        .await
        .map_err(|_| crate::error::EggsecError::Timeout {
            timeout_ms: timeout_secs * 1000,
            operation: format!(
                "Pipeline timed out after {}s (profile: {}, {} stages)",
                timeout_secs, profile, stage_count
            ),
        })?
        .map_err(|e| crate::error::EggsecError::Runtime(format!("Pipeline failed: {}", e)))?;

        let findings = findings.lock().clone();
        let findings_count = findings.len();

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds().max(0) as u64;

        Ok(ToolResponse {
            request_id: request.id,
            tool_id: "scan".to_string(),
            status: crate::tool::ResponseStatus::Success,
            results: serde_json::json!({ "target": target, "profile": profile }),
            metadata: crate::tool::ResponseMetadata {
                started_at,
                completed_at,
                duration_ms,
                targets_scanned: 1,
                findings_count,
            },
            errors: vec![],
            findings,
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability {
                name: "passive-fingerprint".to_string(),
                description: "Passive fingerprinting capability required by policy".to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "active-probe".to_string(),
                description: "Active probing capability required by policy".to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "crawl".to_string(),
                description: "Web crawling capability required by policy".to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "http-fuzz-low-impact".to_string(),
                description: "Low-impact HTTP fuzzing capability required by policy".to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
            ToolCapability {
                name: "load-test".to_string(),
                description: "Load testing capability required by policy".to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![],
                prerequisites: vec![],
                estimated_duration_ms: 0,
            },
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
                examples: vec![CapabilityExample {
                    description: "Quick scan of example.com".to_string(),
                    params: serde_json::json!({"target": "example.com"}),
                }],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 60000,
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
                examples: vec![CapabilityExample {
                    description: "Endpoint discovery on example.com".to_string(),
                    params: serde_json::json!({"target": "example.com"}),
                }],
                attack_surface: vec![AttackSurface::Network, AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Low, AgentSeverity::Info],
                prerequisites: vec!["quick".to_string()],
                estimated_duration_ms: 120000,
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
                examples: vec![CapabilityExample {
                    description: "Full web vuln scan on example.com".to_string(),
                    params: serde_json::json!({"target": "example.com"}),
                }],
                attack_surface: vec![AttackSurface::Web, AttackSurface::Api],
                severity_potential: vec![
                    AgentSeverity::Critical,
                    AgentSeverity::High,
                    AgentSeverity::Medium,
                ],
                prerequisites: vec!["endpoint".to_string()],
                estimated_duration_ms: 300000,
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
                examples: vec![CapabilityExample {
                    description: "Complete security scan".to_string(),
                    params: serde_json::json!({"target": "example.com"}),
                }],
                attack_surface: vec![
                    AttackSurface::Network,
                    AttackSurface::Web,
                    AttackSurface::Api,
                ],
                severity_potential: vec![
                    AgentSeverity::Critical,
                    AgentSeverity::High,
                    AgentSeverity::Medium,
                    AgentSeverity::Low,
                ],
                prerequisites: vec!["web".to_string()],
                estimated_duration_ms: 600000,
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
                examples: vec![CapabilityExample {
                    description: "API security scan".to_string(),
                    params: serde_json::json!({"target": "example.com"}),
                }],
                attack_surface: vec![AttackSurface::Api, AttackSurface::Authentication],
                severity_potential: vec![
                    AgentSeverity::Critical,
                    AgentSeverity::High,
                    AgentSeverity::Medium,
                ],
                prerequisites: vec![],
                estimated_duration_ms: 300000,
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
                examples: vec![CapabilityExample {
                    description: "Intelligence gathering".to_string(),
                    params: serde_json::json!({"target": "example.com"}),
                }],
                attack_surface: vec![AttackSurface::Web, AttackSurface::Cloud],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 180000,
            },
        ]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(EggsecError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
