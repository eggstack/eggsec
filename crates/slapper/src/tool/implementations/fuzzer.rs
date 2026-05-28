use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::error::SlapperError;
use crate::output::AgentSeverity;
use crate::tool::response::Finding;
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

#[derive(Clone)]
pub struct FuzzerTool;

impl FuzzerTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for FuzzerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for FuzzerTool {
    fn id(&self) -> &'static str {
        "fuzz"
    }

    fn name(&self) -> &'static str {
        "Security Fuzzer"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Fuzzing
    }

    fn description(&self) -> &'static str {
        "Test applications for vulnerabilities using various security payloads including SQL injection, XSS, SSRF, and more."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let target = &request.target.value;

        let params = &request.params;

        let payload_types = params
            .get("types")
            .and_then(|v| v.as_str())
            .unwrap_or("xss,sqli")
            .to_string();

        let concurrency = request.options.concurrency.unwrap_or(10);
        let timeout = request.options.timeout_ms.unwrap_or(30000);

        // Parse additional fuzzing options from params
        let mutate = params
            .get("mutate")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let mutation_count = params
            .get("mutation_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(3) as usize;
        let session = params
            .get("session")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let waf_fingerprint = params
            .get("waf_fingerprint")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        let method = params
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_string();

        let param = params
            .get("param")
            .and_then(|v| v.as_str())
            .map(String::from);

        let args = crate::cli::FuzzArgs {
            url: target.clone(),
            payload_type: payload_types,
            mode: crate::cli::FuzzMode::Sequential,
            mutate,
            mutation_count,
            grammar_fuzz: params
                .get("grammar_fuzz")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            grammar_type: params
                .get("grammar_type")
                .and_then(|v| v.as_str())
                .map(String::from),
            adaptive_rate: params
                .get("adaptive_rate")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            session,
            diffing: params
                .get("diffing")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            capture_baseline: params
                .get("capture_baseline")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            enhanced_redos: params
                .get("enhanced_redos")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            waf_fingerprint,
            chaining: params
                .get("chaining")
                .and_then(|v| v.as_bool())
                .unwrap_or(false),
            chain_file: params
                .get("chain_file")
                .and_then(|v| v.as_str())
                .map(String::from),
            method,
            param,
            concurrency,
            timeout: timeout / 1000,
            json: true,
            output: None,
            verbose: false,
            quiet: false,
            format: None,
            target: None,
            jwt_token: None,
            oauth_issuer: None,
            oauth_client_id: None,
            oauth_client_secret: None,
            idor_base_id: None,
            idor_user_ids: None,
            ssti_param: None,
            graphql_introspection: true,
            graphql_depth_bypass: true,
            graphql_alias_overload: true,
            oauth_redirect: true,
            oauth_scope: true,
            oauth_state: true,
            oauth_grant: true,
            schema: None,
            discover_only: false,
            auto_discover_schema: false,
            calibrate: false,
            fc: None,
            fs: None,
            fw: None,
            fl: None,
            ft: None,
            fr: None,
            common: crate::cli::CommonHttpArgs::default(),
        };

        let findings: std::sync::Arc<parking_lot::Mutex<Vec<Finding>>> =
            std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let findings_clone = findings.clone();
        let result = tokio::time::timeout(
            std::time::Duration::from_secs(60),
            crate::fuzzer::run_cli_with_callback(args, move |finding| {
                let mut f = findings_clone.lock();
                f.push(finding);
            }),
        )
        .await
        .map_err(|e| crate::error::SlapperError::Timeout { timeout_ms: 0, operation: format!(
            "Fuzzing timed out after 60s: {}",
            e
        ) })?
        .map_err(|e| crate::error::SlapperError::Runtime(format!(
            "Fuzzing failed: {}",
            e
        )))?;
        let findings = match std::sync::Arc::try_unwrap(findings) {
            Ok(inner) => inner.into_inner(),
            Err(e) => {
                tracing::warn!("Callback still referenced, using empty result: Arc still has {} references", Arc::strong_count(&e));
                Vec::new()
            }
        };

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;
        let findings_count = findings.len();

        match result {
            Ok(_) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: "fuzz".to_string(),
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
            }),
            Err(e) => Ok(ToolResponse {
                request_id: request.id,
                tool_id: "fuzz".to_string(),
                status: crate::tool::ResponseStatus::Failed,
                results: serde_json::json!({}),
                metadata: crate::tool::ResponseMetadata {
                    started_at,
                    completed_at,
                    duration_ms,
                    targets_scanned: 0,
                    findings_count,
                },
                errors: vec![crate::tool::ToolError::new(
                    "EXECUTION_ERROR",
                    e.to_string(),
                )],
                findings,
            }),
        }
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability {
                name: "sqli".to_string(),
                description: "Test for SQL injection vulnerabilities".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL with parameter".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Test for SQL injection".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com/search?q=test"
                    }),
                }],
                attack_surface: vec![AttackSurface::Web, AttackSurface::Api],
                severity_potential: vec![AgentSeverity::Critical, AgentSeverity::High],
                prerequisites: vec![],
                estimated_duration_ms: 60000,
            },
            ToolCapability {
                name: "xss".to_string(),
                description: "Test for Cross-Site Scripting vulnerabilities".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL with parameter".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Test for XSS vulnerabilities".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com/search?q=test"
                    }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::High, AgentSeverity::Medium],
                prerequisites: vec![],
                estimated_duration_ms: 60000,
            },
            ToolCapability {
                name: "ssrf".to_string(),
                description: "Test for Server-Side Request Forgery".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL with parameter".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Test for SSRF vulnerabilities".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com/fetch?url=test"
                    }),
                }],
                attack_surface: vec![
                    AttackSurface::Web,
                    AttackSurface::Api,
                    AttackSurface::Internal,
                ],
                severity_potential: vec![AgentSeverity::Critical, AgentSeverity::High],
                prerequisites: vec!["recon".to_string()],
                estimated_duration_ms: 30000,
            },
            ToolCapability {
                name: "graphql".to_string(),
                description: "Test GraphQL endpoints".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "GraphQL endpoint URL".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Test GraphQL for vulnerabilities".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com/graphql"
                    }),
                }],
                attack_surface: vec![AttackSurface::Api],
                severity_potential: vec![AgentSeverity::High, AgentSeverity::Medium],
                prerequisites: vec![],
                estimated_duration_ms: 45000,
            },
            ToolCapability {
                name: "jwt".to_string(),
                description: "Test JWT vulnerabilities".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Test JWT tokens for vulnerabilities".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com/api/login"
                    }),
                }],
                attack_surface: vec![AttackSurface::Authentication],
                severity_potential: vec![AgentSeverity::Critical, AgentSeverity::High],
                prerequisites: vec![],
                estimated_duration_ms: 30000,
            },
            ToolCapability {
                name: "all".to_string(),
                description: "Run all payload types".to_string(),
                parameters: vec![ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL".to_string(),
                }],
                examples: vec![CapabilityExample {
                    description: "Run all fuzzing tests".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com/search?q=test"
                    }),
                }],
                attack_surface: vec![
                    AttackSurface::Web,
                    AttackSurface::Api,
                    AttackSurface::Authentication,
                ],
                severity_potential: vec![
                    AgentSeverity::Critical,
                    AgentSeverity::High,
                    AgentSeverity::Medium,
                    AgentSeverity::Low,
                ],
                prerequisites: vec![],
                estimated_duration_ms: 300000,
            },
        ]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }

        if request.target.target_type != crate::tool::TargetType::Url {
            return Err(SlapperError::Validation(
                "Fuzzer requires a URL target".to_string(),
            ));
        }

        Ok(())
    }
}
