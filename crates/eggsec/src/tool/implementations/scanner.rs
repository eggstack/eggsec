use async_trait::async_trait;
use chrono::Utc;
use std::sync::Arc;

use crate::error::EggsecError;
use crate::output::AgentSeverity;
use crate::tool::response::Finding;
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

#[derive(Clone)]
pub struct ScannerTool {
    mode: ScanMode,
}

#[derive(Clone, Copy)]
pub enum ScanMode {
    Ports,
    Fingerprint,
    Endpoints,
}

impl ScannerTool {
    pub fn new() -> Self {
        Self {
            mode: ScanMode::Ports,
        }
    }

    pub fn ports() -> Self {
        Self {
            mode: ScanMode::Ports,
        }
    }

    pub fn fingerprint() -> Self {
        Self {
            mode: ScanMode::Fingerprint,
        }
    }

    pub fn endpoints() -> Self {
        Self {
            mode: ScanMode::Endpoints,
        }
    }
}

impl Default for ScannerTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for ScannerTool {
    fn id(&self) -> &'static str {
        match self.mode {
            ScanMode::Ports => "scan-ports",
            ScanMode::Fingerprint => "fingerprint",
            ScanMode::Endpoints => "scan-endpoints",
        }
    }

    fn name(&self) -> &'static str {
        match self.mode {
            ScanMode::Ports => "Port Scanner",
            ScanMode::Fingerprint => "Service Fingerprinter",
            ScanMode::Endpoints => "Endpoint Discovery",
        }
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Scanning
    }

    fn description(&self) -> &'static str {
        match self.mode {
            ScanMode::Ports => "Scan ports on target hosts to discover open services",
            ScanMode::Fingerprint => {
                "Identify services running on open ports by analyzing responses"
            }
            ScanMode::Endpoints => "Discover hidden or sensitive HTTP endpoints using wordlists",
        }
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let target = &request.target.value;

        let concurrency = request.options.concurrency.unwrap_or(50);
        let timeout = request
            .options
            .timeout_ms
            .unwrap_or(crate::constants::DEFAULT_TOOL_TIMEOUT_MS);

        let findings: std::sync::Arc<parking_lot::Mutex<Vec<Finding>>> =
            std::sync::Arc::new(parking_lot::Mutex::new(Vec::new()));
        let findings_clone = findings.clone();

        let result: Result<(), crate::error::EggsecError> = match self.mode {
            ScanMode::Ports => {
                let args = crate::cli::PortScanArgs {
                    host: target.clone(),
                    ports: "1-1000".to_string(),
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    source_ip: None,
                    spoof_range: None,
                    dry_run: false,
                    decoy: None,
                    decoy_range: None,
                    decoy_count: None,
                    decoy_mode: None,
                    include_me: false,
                    source_port: None,
                    random_source_port: false,
                    fragment: false,
                    scan_type: None,
                    packet_trace: None,
                    max_rate: None,
                    ttl: None,
                    grepable: false,
                    xml: false,
                    verbose: false,
                    quiet: false,
                    output: None,
                };
                let config = crate::config::load_config(None::<&str>).inspect_err(|e| {
                    tracing::warn!(error = %e, "Failed to load config for scanner, using defaults");
                }).unwrap_or_default();
                tokio::time::timeout(
                    std::time::Duration::from_secs(60),
                    crate::scanner::ports::run_cli_with_callback(args, &config, move |f| {
                        let mut findings = findings_clone.lock();
                        findings.push(f);
                    }),
                )
                .await
                .map_err(|e| crate::error::EggsecError::Timeout {
                    timeout_ms: 0,
                    operation: format!("Port scan timed out after 60s: {}", e),
                })?
                .map_err(|e| {
                    crate::error::EggsecError::Runtime(format!("Port scan failed: {}", e))
                })?;
                Ok(())
            }
            ScanMode::Fingerprint => {
                let args = crate::cli::FingerprintArgs {
                    host: target.clone(),
                    ports: "22,80,443,3306,5432,6379".to_string(),
                    timeout: timeout / 1000,
                    json: true,
                    udp: false,
                    verbose: false,
                    quiet: false,
                    output: None,
                    concurrency: 20,
                };
                let config = crate::config::load_config(None::<&str>).inspect_err(|e| {
                    tracing::warn!(error = %e, "Failed to load config for scanner, using defaults");
                }).unwrap_or_default();
                tokio::time::timeout(
                    std::time::Duration::from_secs(60),
                    crate::scanner::fingerprint::run_cli_with_callback(args, &config, move |f| {
                        let mut findings = findings_clone.lock();
                        findings.push(f);
                    }),
                )
                .await
                .map_err(|e| crate::error::EggsecError::Timeout {
                    timeout_ms: 0,
                    operation: format!("Fingerprint scan timed out after 60s: {}", e),
                })?
                .map_err(|e| {
                    crate::error::EggsecError::Runtime(format!("Fingerprint scan failed: {}", e))
                })?;
                Ok(())
            }
            ScanMode::Endpoints => {
                let wordlist = request
                    .params
                    .get("wordlist")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string());
                let args = crate::cli::EndpointScanArgs {
                    url: target.clone(),
                    wordlist,
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    source_ip: None,
                    spoof_range: None,
                    decoy: None,
                    decoy_range: None,
                    decoy_count: None,
                    decoy_mode: None,
                    include_me: false,
                    include_404: false,
                    verbose: false,
                    quiet: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                let config = crate::config::load_config(None::<&str>).inspect_err(|e| {
                    tracing::warn!(error = %e, "Failed to load config for scanner, using defaults");
                }).unwrap_or_default();
                tokio::time::timeout(
                    std::time::Duration::from_secs(60),
                    crate::scanner::endpoints::run_cli_with_callback(args, &config, move |f| {
                        let mut findings = findings_clone.lock();
                        findings.push(f);
                    }),
                )
                .await
                .map_err(|e| crate::error::EggsecError::Timeout {
                    timeout_ms: 0,
                    operation: format!("Endpoint scan timed out after 60s: {}", e),
                })?
                .map_err(|e| {
                    crate::error::EggsecError::Runtime(format!("Endpoint scan failed: {}", e))
                })?;
                Ok(())
            }
        };

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
            tool_id: self.id().to_string(),
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
        match self.mode {
            ScanMode::Ports => vec![
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
                    name: "scan_ports".to_string(),
                description: "Scan TCP ports on target".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "target".to_string(),
                        param_type: ParameterType::Ip,
                        required: true,
                        default: None,
                        description: "Target IP or hostname".to_string(),
                    },
                    ParameterDef {
                        name: "ports".to_string(),
                        param_type: ParameterType::String,
                        required: false,
                        default: Some(serde_json::json!("1-1000")),
                        description: "Port range or list (e.g., 1-1000, 80,443,8080)".to_string(),
                    },
                    ParameterDef {
                        name: "concurrency".to_string(),
                        param_type: ParameterType::Integer,
                        required: false,
                        default: Some(serde_json::json!(50)),
                        description: "Number of concurrent connections".to_string(),
                    },
                ],
                examples: vec![CapabilityExample {
                    description: "Scan common ports on 192.168.1.1".to_string(),
                    params: serde_json::json!({
                        "target": "192.168.1.1",
                        "ports": "1-1000"
                    }),
                }],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 60000,
            }],
            ScanMode::Fingerprint => vec![
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
                    name: "fingerprint".to_string(),
                description: "Fingerprint services on open ports".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "target".to_string(),
                        param_type: ParameterType::Ip,
                        required: true,
                        default: None,
                        description: "Target IP or hostname".to_string(),
                    },
                    ParameterDef {
                        name: "ports".to_string(),
                        param_type: ParameterType::String,
                        required: false,
                        default: Some(serde_json::json!("22,80,443,3306")),
                        description: "Comma-separated port list".to_string(),
                    },
                ],
                examples: vec![CapabilityExample {
                    description: "Fingerprint services on common ports".to_string(),
                    params: serde_json::json!({
                        "target": "192.168.1.1",
                        "ports": "22,80,443,3306"
                    }),
                }],
                attack_surface: vec![AttackSurface::Network],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec!["scan_ports".to_string()],
                estimated_duration_ms: 30000,
            }],
            ScanMode::Endpoints => vec![
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
                    name: "active-probe".to_string(),
                    description: "Active probing capability required by policy".to_string(),
                    parameters: vec![],
                    examples: vec![],
                    attack_surface: vec![AttackSurface::Web],
                    severity_potential: vec![AgentSeverity::Info],
                    prerequisites: vec![],
                    estimated_duration_ms: 0,
                },
                ToolCapability {
                    name: "scan_endpoints".to_string(),
                description: "Discover HTTP endpoints".to_string(),
                parameters: vec![
                    ParameterDef {
                        name: "target".to_string(),
                        param_type: ParameterType::Url,
                        required: true,
                        default: None,
                        description: "Target base URL".to_string(),
                    },
                    ParameterDef {
                        name: "wordlist".to_string(),
                        param_type: ParameterType::String,
                        required: false,
                        default: None,
                        description: "Path to wordlist file".to_string(),
                    },
                ],
                examples: vec![CapabilityExample {
                    description: "Discover endpoints on a web application".to_string(),
                    params: serde_json::json!({
                        "target": "https://example.com"
                    }),
                }],
                attack_surface: vec![AttackSurface::Web],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 120000,
            }],
        }
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        if request.target.value.is_empty() {
            return Err(EggsecError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
