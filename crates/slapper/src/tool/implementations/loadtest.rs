use async_trait::async_trait;
use chrono::Utc;

use crate::error::SlapperError;
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

#[derive(Clone)]
pub struct LoadTestTool;

impl LoadTestTool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LoadTestTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for LoadTestTool {
    fn id(&self) -> &'static str {
        "load"
    }

    fn name(&self) -> &'static str {
        "Load Tester"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::LoadTest
    }

    fn description(&self) -> &'static str {
        "Run HTTP load tests to measure server performance and gather metrics under concurrent load."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let target = &request.target.value;

        let params = &request.params;

        let requests = params
            .get("requests")
            .and_then(|v| v.as_u64())
            .unwrap_or(100);

        let concurrency = params
            .get("concurrency")
            .and_then(|v| v.as_u64())
            .unwrap_or(10) as usize;

        let args = crate::cli::LoadArgs {
            url: target.clone(),
            requests,
            concurrency,
            method: "GET".to_string(),
            body: None,
            headers: vec![],
            timeout: 30,
            json: true,
            verbose: false,
            quiet: false,
            output: None,
            common: crate::cli::CommonHttpArgs::default(),
        };

        let config = crate::config::load_config(None::<&str>)
            .inspect_err(|e| {
                tracing::warn!(error = %e, "Failed to load config for loadtest, using defaults");
            })
            .unwrap_or_default();
        tokio::time::timeout(
            std::time::Duration::from_secs(60),
            crate::loadtest::run_cli(args, &config),
        )
        .await
        .map_err(|e| crate::error::SlapperError::Timeout {
            timeout_ms: 0,
            operation: format!("Load test timed out after 60s: {}", e),
        })?
        .map_err(|e| crate::error::SlapperError::Runtime(format!("Load test failed: {}", e)))?;

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        Ok(ToolResponse {
            request_id: request.id,
            tool_id: "load".to_string(),
            status: crate::tool::ResponseStatus::Success,
            results: serde_json::json!({ "target": target, "requests": requests, "concurrency": concurrency }),
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
            name: "http_load_test".to_string(),
            description: "Run HTTP load test".to_string(),
            parameters: vec![
                ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::Url,
                    required: true,
                    default: None,
                    description: "Target URL".to_string(),
                },
                ParameterDef {
                    name: "requests".to_string(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(serde_json::json!(100)),
                    description: "Total number of requests".to_string(),
                },
                ParameterDef {
                    name: "concurrency".to_string(),
                    param_type: ParameterType::Integer,
                    required: false,
                    default: Some(serde_json::json!(10)),
                    description: "Number of concurrent connections".to_string(),
                },
            ],
            examples: vec![CapabilityExample {
                description: "Load test with 1000 requests".to_string(),
                params: serde_json::json!({
                    "target": "https://example.com/api",
                    "requests": 1000,
                    "concurrency": 20
                }),
            }],
            attack_surface: vec![AttackSurface::Web, AttackSurface::Api],
            severity_potential: vec![],
            prerequisites: vec![],
            estimated_duration_ms: 120000,
        }]
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(SlapperError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
