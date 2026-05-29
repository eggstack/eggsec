use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::error::SlapperError;
use crate::output::AgentSeverity;
use crate::tool::response::Finding;
use crate::tool::traits::{
    validate_parameters, AttackSurface, CapabilityExample, ParameterDef, ParameterType,
    SecurityTool, ToolCapability, ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

#[derive(Clone)]
pub struct ReconTool;

impl ReconTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for ReconTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for ReconTool {
    fn id(&self) -> &'static str {
        "recon"
    }

    fn name(&self) -> &'static str {
        "Reconnaissance"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Recon
    }

    fn description(&self) -> &'static str {
        "Gather comprehensive intelligence about a target including DNS, technology stack, subdomains, SSL, and more."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();

        let target = &request.target.value;
        let params = &request.params;

        let findings: std::sync::Arc<parking_lot::Mutex<Vec<Finding>>> =
            std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let findings_clone = findings.clone();

        let no_tech = params
            .get("no_tech")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_dns = params
            .get("no_dns")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_geo = params
            .get("no_geo")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_whois = params
            .get("no_whois")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_subdomains = params
            .get("no_subdomains")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_ssl = params
            .get("no_ssl")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let no_cve = params
            .get("no_cve")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let args = crate::cli::ReconArgs {
            target: target.clone(),
            no_tech,
            no_dns,
            no_geo,
            no_whois,
            no_subdomains,
            no_ssl,
            no_dns_records: params
                .get("no_dns_records")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_js: params
                .get("no_js")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_content: params
                .get("no_content")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_cloud: params
                .get("no_cloud")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_wayback: params
                .get("no_wayback")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_cors: params
                .get("no_cors")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_threat: params
                .get("no_threat")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_cve,
            no_email: params
                .get("no_email")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            no_takeover: params
                .get("no_takeover")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            concurrency: request.options.concurrency,
            json: true,
            quiet: true,
            verbose: false,
            output: None,
        };

        let config = crate::config::load_config(None::<&str>)
            .inspect_err(|e| {
                tracing::warn!(error = %e, "Failed to load config for recon, using defaults");
            })
            .unwrap_or_default();

        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            crate::recon::run_cli_with_callback(args, &config, move |f| {
                let mut findings = findings_clone.lock();
                findings.push(f);
            }),
        )
        .await
        .map_err(|e| crate::error::SlapperError::Timeout {
            timeout_ms: 0,
            operation: format!("Recon timed out after 60s: {}", e),
        })?
        .map_err(|e| crate::error::SlapperError::Runtime(format!("Recon failed: {}", e)))?;

        let findings = match std::sync::Arc::try_unwrap(findings) {
            Ok(inner) => inner.into_inner(),
            Err(e) => {
                tracing::warn!(
                    "Callback still referenced, using empty result: Arc still has {} references",
                    Arc::strong_count(&e)
                );
                Vec::new()
            }
        };
        let findings_count = findings.len();

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        Ok(ToolResponse {
            request_id: request.id,
            tool_id: "recon".to_string(),
            status: crate::tool::ResponseStatus::Success,
            results: serde_json::json!({ "target": target }),
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
                name: "full_recon".to_string(),
                description: "Perform comprehensive reconnaissance".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "target".to_string(),
                        param_type: ParameterType::Domain,
                        required: true,
                        default: None,
                        description: "Target domain or URL".to_string(),
                    },
                    ParameterDef {
                        name: "concurrency".to_string(),
                        param_type: ParameterType::Integer,
                        required: false,
                        default: Some(serde_json::json!(20)),
                        description: "Number of concurrent requests".to_string(),
                    },
                ],
                examples: vec![CapabilityExample {
                    description: "Full recon on example.com".to_string(),
                    params: serde_json::json!({
                        "target": "example.com",
                        "concurrency": 20
                    }),
                }],
                attack_surface: vec![
                    AttackSurface::Web,
                    AttackSurface::Network,
                    AttackSurface::Cloud,
                ],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 180000,
            },
            ToolCapability {
                name: "dns".to_string(),
                description: "Perform DNS enumeration".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "DNS enumeration for example.com".to_string(),
                    params: serde_json::json!({
                        "target": "example.com"
                    }),
                }],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 60000,
            },
            ToolCapability {
                name: "tech_detection".to_string(),
                description: "Detect technology stack".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Detect tech stack on https://example.com".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com"
                    }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 30000,
            },
            ToolCapability {
                name: "subdomain_enum".to_string(),
                description: "Enumerate subdomains".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Domain,
                    required: true,
                    default: None,
                    description: "Target domain".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Find subdomains of example.com".to_string(),
                    params: serde_json::json!({
                        "target": "example.com"
                    }),
                }],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec!["dns".to_string()],
                estimated_duration_ms: 120000,
            },
        ]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }

        let capability_name = request
            .params
            .get("_capability")
            .and_then(|v| v.as_str())
            .map(String::from);

        if let Some(cap_name) = capability_name {
            let cap = self.capabilities().into_iter().find(|c| c.name == cap_name);

            if let Some(cap) = cap {
                validate_parameters(&request.params, &cap.parameters)?;
            }
        }

        Ok(())
    }
}
