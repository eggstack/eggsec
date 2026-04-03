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
        let timeout = request.options.timeout_ms.unwrap_or(30000);

        let result = match self.mode {
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
                    output: None,
                };
                let config = crate::config::load_config(None::<&str>).unwrap_or_default();
                crate::scanner::ports::run_cli(args, &config).await
            }
            ScanMode::Fingerprint => {
                let args = crate::cli::FingerprintArgs {
                    host: target.clone(),
                    ports: "22,80,443,3306,5432,6379".to_string(),
                    timeout: timeout / 1000,
                    json: true,
                    udp: false,
                    verbose: false,
                    output: None,
                    concurrency: 20,
                };
                let config = crate::config::load_config(None::<&str>).unwrap_or_default();
                crate::scanner::fingerprint::run_cli(args, &config).await
            }
            ScanMode::Endpoints => {
                let args = crate::cli::EndpointScanArgs {
                    url: target.clone(),
                    wordlist: None,
                    concurrency,
                    timeout: timeout / 1000,
                    json: true,
                    spoof_ip: None,
                    spoof_range: None,
                    decoy: None,
                    decoy_range: None,
                    decoy_count: None,
                    decoy_mode: None,
                    include_me: false,
                    include_404: false,
                    verbose: false,
                    output: None,
                    common: crate::cli::CommonHttpArgs::default(),
                };
                let config = crate::config::load_config(None::<&str>).unwrap_or_default();
                crate::scanner::endpoints::run_cli(args, &config).await
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
        match self.mode {
            ScanMode::Ports => vec![ToolCapability {
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
            ScanMode::Fingerprint => vec![ToolCapability {
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
            ScanMode::Endpoints => vec![ToolCapability {
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
            return Err(SlapperError::Validation("Target is required".to_string()));
        }
        Ok(())
    }
}
