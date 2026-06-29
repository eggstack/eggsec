use async_trait::async_trait;

use crate::tool::response::{Finding, FindingType, ResponseSeverity};
use crate::tool::traits::{
    AttackSurface, CapabilityExample, ParameterDef, ParameterType, SecurityTool, ToolCapability,
    ToolCategory,
};
use crate::tool::{ToolRequest, ToolResponse, ToolResult};

/// MCP tool for C2 campaign simulation against authorized lab targets.
///
/// Only compiled when `c2-mcp` feature is enabled.
/// The `EnforcementContext` pre-dispatch gate in `handle_tools_call` ensures
/// policy enforcement (scope, risk, capabilities) before this tool is reached.
///
/// Safety: always forces dry_run=true regardless of user input.
#[derive(Clone)]
pub struct C2Tool;

impl C2Tool {
    pub fn new() -> Self {
        Self
    }
}

impl Default for C2Tool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl SecurityTool for C2Tool {
    fn id(&self) -> &'static str {
        "c2"
    }

    fn name(&self) -> &'static str {
        "C2 Campaign Simulation"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Pipeline
    }

    fn description(&self) -> &'static str {
        "C2 (Command & Control) campaign simulation for authorized lab environments. \
         Simulates beaconing, tasking, attack graphs, and OPSEC scoring. \
         Supports APT29, Carbanak/FIN7, and generic campaign profiles. \
         Always runs in dry-run mode for safety (no real C2 operations)."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let target = request
            .params
            .get("target")
            .and_then(|v| v.as_str())
            .unwrap_or("localhost")
            .to_string();

        let campaign = request
            .params
            .get("campaign")
            .and_then(|v| v.as_str())
            .unwrap_or("default")
            .to_string();

        let config = crate::config::load_config(None).map_err(|e| {
            crate::error::EggsecError::Config(format!("Failed to load config: {}", e))
        })?;

        let scanner = crate::c2::C2Scanner::new(true, &campaign);

        let report =
            scanner
                .scan(&target)
                .await
                .map_err(|e| crate::error::EggsecError::ScanFailed {
                    stage: "c2".to_string(),
                    error: e.to_string(),
                })?;

        let findings: Vec<Finding> = report
            .beacon_results
            .iter()
            .enumerate()
            .map(|(i, b)| Finding {
                id: format!("c2-beacon-{}", i),
                finding_type: FindingType::Misconfiguration,
                severity: if b.success {
                    ResponseSeverity::Medium
                } else {
                    ResponseSeverity::Low
                },
                title: format!("C2 Beacon ({})", b.protocol.as_str()),
                description: format!(
                    "Beacon via {} (interval: {}ms, jitter: {}%) - {}",
                    b.protocol.as_str(),
                    b.interval_ms,
                    b.jitter_percent,
                    if b.success { "successful" } else { "failed" }
                ),
                location: target.clone(),
                evidence: b.evidence.clone(),
                cve_ids: vec![],
                remediation: Some(
                    "Monitor for anomalous outbound connections with periodic timing patterns"
                        .to_string(),
                ),
                references: vec![],
                metadata: {
                    let mut m = rustc_hash::FxHashMap::default();
                    m.insert(
                        "protocol".to_string(),
                        serde_json::Value::String(b.protocol.as_str().to_string()),
                    );
                    m.insert(
                        "category".to_string(),
                        serde_json::Value::String("c2-beacon".to_string()),
                    );
                    m
                },
            })
            .chain(report.task_results.iter().enumerate().map(|(i, t)| {
                let severity = match t.status {
                    crate::c2::TaskStatus::Completed => ResponseSeverity::High,
                    crate::c2::TaskStatus::Simulated => ResponseSeverity::Medium,
                    crate::c2::TaskStatus::Failed => ResponseSeverity::Low,
                    crate::c2::TaskStatus::Denied => ResponseSeverity::Info,
                };
                Finding {
                    id: format!("c2-task-{}", i),
                    finding_type: FindingType::Misconfiguration,
                    severity,
                    title: format!("C2 Task: {}", t.task_type.as_str()),
                    description: format!(
                        "Task type: {} - status: {:?}{}",
                        t.task_type.as_str(),
                        t.status,
                        t.output
                            .as_deref()
                            .map_or_else(String::new, |o| format!(" - {}", o))
                    ),
                    location: target.clone(),
                    evidence: t.output.clone(),
                    cve_ids: vec![],
                    remediation: Some(
                        "Implement detection for C2 task execution patterns".to_string(),
                    ),
                    references: vec![],
                    metadata: {
                        let mut m = rustc_hash::FxHashMap::default();
                        m.insert(
                            "category".to_string(),
                            serde_json::Value::String(format!("c2-task-{}", t.task_type.as_str())),
                        );
                        if let Some(ref technique) = t.mitre_technique {
                            m.insert(
                                "mitre_technique".to_string(),
                                serde_json::Value::String(technique.clone()),
                            );
                        }
                        m
                    },
                }
            }))
            .chain(
                report
                    .opsec_assessment
                    .findings
                    .iter()
                    .enumerate()
                    .map(|(i, f)| Finding {
                        id: format!("c2-opsec-{}", i),
                        finding_type: FindingType::Misconfiguration,
                        severity: match f.severity {
                            crate::c2::OpsecSeverity::Info => ResponseSeverity::Info,
                            crate::c2::OpsecSeverity::Low => ResponseSeverity::Low,
                            crate::c2::OpsecSeverity::Medium => ResponseSeverity::Medium,
                            crate::c2::OpsecSeverity::High => ResponseSeverity::High,
                        },
                        title: format!("OPSEC: {}", f.description),
                        description: f.description.clone(),
                        location: target.clone(),
                        evidence: None,
                        cve_ids: vec![],
                        remediation: Some(f.recommendation.clone()),
                        references: vec![],
                        metadata: {
                            let mut m = rustc_hash::FxHashMap::default();
                            m.insert(
                                "category".to_string(),
                                serde_json::Value::String(
                                    format!("{:?}", f.category).to_lowercase(),
                                ),
                            );
                            m
                        },
                    }),
            )
            .collect();

        let result = serde_json::json!({
            "target": report.target,
            "campaign": {
                "id": report.campaign.id,
                "name": report.campaign.name,
                "mitre_profile": report.campaign.mitre_profile,
                "phases": report.campaign.phases.len(),
            },
            "dry_run": report.dry_run,
            "beacons": {
                "total": report.summary.total_beacons,
                "successful": report.summary.successful_beacons,
            },
            "tasks": {
                "total": report.summary.total_tasks,
                "completed": report.summary.completed_tasks,
            },
            "opsec": {
                "score": report.summary.opsec_score,
                "max": report.summary.opsec_max,
            },
            "attack_graph": report.attack_graph.as_ref().map(|g| {
                serde_json::json!({
                    "nodes": g.nodes.len(),
                    "critical_path": g.critical_path,
                })
            }),
            "timeline": report.timeline.as_ref().map(|t| {
                serde_json::json!({
                    "phases": t.total_phases,
                    "techniques": t.total_techniques,
                })
            }),
        });

        let _ = config;

        Ok(ToolResponse::success(request.id.clone(), self.id(), result).with_findings(findings))
    }

    fn validate(&self, request: &ToolRequest) -> ToolResult<()> {
        // Target is optional; defaults to "localhost"
        if let Some(target) = request.params.get("target").and_then(|v| v.as_str()) {
            if target.is_empty() {
                return Err(crate::error::EggsecError::Config(
                    "Parameter 'target' must not be empty".to_string(),
                ));
            }
        }
        if let Some(campaign) = request.params.get("campaign").and_then(|v| v.as_str()) {
            let valid = ["apt29", "carbanak", "default"];
            if !valid.contains(&campaign) {
                return Err(crate::error::EggsecError::Config(format!(
                    "Invalid campaign '{}'. Valid profiles: apt29, carbanak, default",
                    campaign
                )));
            }
        }
        Ok(())
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![ToolCapability {
            name: "c2-simulation".to_string(),
            description: "C2 campaign simulation with beacons, tasking, and OPSEC scoring"
                .to_string(),
            parameters: vec![
                ParameterDef {
                    name: "target".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(serde_json::json!("localhost")),
                    description: "Target for the C2 simulation".to_string(),
                },
                ParameterDef {
                    name: "campaign".to_string(),
                    param_type: ParameterType::String,
                    required: false,
                    default: Some(serde_json::json!("default")),
                    description: "Campaign profile: apt29, carbanak, or default".to_string(),
                },
            ],
            examples: vec![
                CapabilityExample {
                    description: "Dry-run APT29 campaign (always safe)".to_string(),
                    params: serde_json::json!({
                        "target": "10.0.0.1",
                        "campaign": "apt29"
                    }),
                },
                CapabilityExample {
                    description: "Dry-run Carbanak campaign".to_string(),
                    params: serde_json::json!({
                        "target": "10.0.0.0/24",
                        "campaign": "carbanak"
                    }),
                },
            ],
            attack_surface: vec![AttackSurface::Network],
            severity_potential: vec![
                crate::output::AgentSeverity::Medium,
                crate::output::AgentSeverity::High,
            ],
            prerequisites: vec![
                "Authorized lab environment only".to_string(),
                "Always runs in dry-run mode (no real C2 operations)".to_string(),
            ],
            estimated_duration_ms: 5_000,
        }]
    }
}
