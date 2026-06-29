//! C2 (Command & Control) framework (feature-gated behind `c2`).
//!
//! Defense-lab-only module for realistic purple teaming and defense validation.
//! Provides beaconing agents, tasking, campaign orchestration, and OPSEC scoring
//! for MITRE ATT&CK profile simulation. Builds on Phase 1 (evasion) and
//! Phase 2 (postex/LOTL) capabilities.
//!
//! Safety: all operations are either dry-run (synthetic results only) or
//! perform simulated C2 activities within authorized lab environments.
//! Real execution requires explicit `--allow-c2` policy flag.
//! Standalone defense-lab surface. No MCP/agent/TUI/pipeline integration.

pub mod agent;
pub mod beacon;
pub mod campaign;
pub mod opsec;
pub mod tasking;

use crate::error::Result;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Report {
    pub target: String,
    pub campaign: C2Campaign,
    pub beacon_results: Vec<BeaconResult>,
    pub task_results: Vec<TaskResult>,
    pub opsec_assessment: OpsecAssessment,
    pub summary: C2Summary,
    pub timestamp: String,
    pub dry_run: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attack_graph: Option<campaign::AttackGraph>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeline: Option<campaign::CampaignTimeline>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Campaign {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mitre_profile: String,
    pub phases: Vec<CampaignPhase>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CampaignPhase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub mitre_techniques: Vec<String>,
    pub order: u32,
}

impl CampaignPhase {
    /// Returns the MITRE technique IDs (same as `mitre_techniques`).
    pub fn mitre_technique_ids(&self) -> Vec<String> {
        self.mitre_techniques.clone()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BeaconResult {
    pub protocol: BeaconProtocol,
    pub interval_ms: u64,
    pub jitter_percent: u32,
    pub success: bool,
    pub evidence: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BeaconProtocol {
    Http,
    Https,
    Dns,
    Tcp,
    Custom,
}

impl BeaconProtocol {
    pub fn as_str(&self) -> &str {
        match self {
            BeaconProtocol::Http => "http",
            BeaconProtocol::Https => "https",
            BeaconProtocol::Dns => "dns",
            BeaconProtocol::Tcp => "tcp",
            BeaconProtocol::Custom => "custom",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskResult {
    pub task_type: TaskType,
    pub status: TaskStatus,
    pub output: Option<String>,
    pub mitre_technique: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskType {
    Recon,
    Execute,
    Exfil,
    Persist,
    Lateral,
    Evade,
    SelfDestruct,
}

impl TaskType {
    pub fn as_str(&self) -> &str {
        match self {
            TaskType::Recon => "recon",
            TaskType::Execute => "execute",
            TaskType::Exfil => "exfil",
            TaskType::Persist => "persist",
            TaskType::Lateral => "lateral",
            TaskType::Evade => "evade",
            TaskType::SelfDestruct => "self-destruct",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Completed,
    Failed,
    Simulated,
    Denied,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsecAssessment {
    pub score: u32,
    pub max_score: u32,
    pub findings: Vec<OpsecFinding>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpsecFinding {
    pub category: OpsecCategory,
    pub severity: OpsecSeverity,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpsecCategory {
    ParentSpoofing,
    Timestomping,
    LogTampering,
    ProcessMasquerading,
    BurnMechanism,
    DecoyActivity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OpsecSeverity {
    Info,
    Low,
    Medium,
    High,
}

impl OpsecSeverity {
    pub fn to_severity(&self) -> Severity {
        match self {
            OpsecSeverity::Info => Severity::Info,
            OpsecSeverity::Low => Severity::Low,
            OpsecSeverity::Medium => Severity::Medium,
            OpsecSeverity::High => Severity::High,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct C2Summary {
    pub total_beacons: usize,
    pub successful_beacons: usize,
    pub total_tasks: usize,
    pub completed_tasks: usize,
    pub opsec_score: u32,
    pub opsec_max: u32,
}

pub struct C2Scanner {
    dry_run: bool,
    campaign: C2Campaign,
}

impl C2Scanner {
    pub fn new(dry_run: bool, campaign_profile: &str) -> Self {
        Self {
            dry_run,
            campaign: Self::default_campaign(campaign_profile),
        }
    }

    fn default_campaign(profile: &str) -> C2Campaign {
        match profile {
            "apt29" => C2Campaign {
                id: "c2-campaign-apt29".to_string(),
                name: "APT29 Simulation".to_string(),
                description:
                    "Simulated APT29-style campaign with HTTP/S beacons and LOTL techniques"
                        .to_string(),
                mitre_profile: "APT29 (Cozy Bear)".to_string(),
                phases: vec![
                    CampaignPhase {
                        id: "phase-1".to_string(),
                        name: "Initial Access & Beacon".to_string(),
                        description: "Establish C2 channel via HTTP/S beacon".to_string(),
                        mitre_techniques: vec!["T1071.001".to_string(), "T1573".to_string()],
                        order: 1,
                    },
                    CampaignPhase {
                        id: "phase-2".to_string(),
                        name: "Persistence & Evasion".to_string(),
                        description: "Establish persistence and evade detection".to_string(),
                        mitre_techniques: vec!["T1547.001".to_string(), "T1070.006".to_string()],
                        order: 2,
                    },
                    CampaignPhase {
                        id: "phase-3".to_string(),
                        name: "Lateral Movement".to_string(),
                        description: "Move laterally within the network".to_string(),
                        mitre_techniques: vec!["T1021.002".to_string(), "T1570".to_string()],
                        order: 3,
                    },
                    CampaignPhase {
                        id: "phase-4".to_string(),
                        name: "Data Exfiltration".to_string(),
                        description: "Exfiltrate target data via encrypted channels".to_string(),
                        mitre_techniques: vec!["T1041".to_string(), "T1573.002".to_string()],
                        order: 4,
                    },
                ],
            },
            "carbanak" => C2Campaign {
                id: "c2-campaign-carbanak".to_string(),
                name: "Carbanak Simulation".to_string(),
                description:
                    "Simulated Carbanak-style campaign with DNS beacons and financial targeting"
                        .to_string(),
                mitre_profile: "Carbanak/FIN7".to_string(),
                phases: vec![
                    CampaignPhase {
                        id: "phase-1".to_string(),
                        name: "DNS Beacon Establishment".to_string(),
                        description: "Establish C2 channel via DNS tunneling".to_string(),
                        mitre_techniques: vec!["T1071.004".to_string(), "T1001".to_string()],
                        order: 1,
                    },
                    CampaignPhase {
                        id: "phase-2".to_string(),
                        name: "Credential Harvesting".to_string(),
                        description: "Harvest credentials for lateral movement".to_string(),
                        mitre_techniques: vec!["T1003".to_string(), "T1555".to_string()],
                        order: 2,
                    },
                    CampaignPhase {
                        id: "phase-3".to_string(),
                        name: "Financial System Access".to_string(),
                        description: "Access financial systems and ATMs".to_string(),
                        mitre_techniques: vec!["T1021.002".to_string(), "T1565.001".to_string()],
                        order: 3,
                    },
                ],
            },
            _ => C2Campaign {
                id: "c2-campaign-default".to_string(),
                name: "Generic Purple Team Campaign".to_string(),
                description: "Default campaign with mixed C2 protocols and techniques".to_string(),
                mitre_profile: "Generic APT".to_string(),
                phases: vec![
                    CampaignPhase {
                        id: "phase-1".to_string(),
                        name: "C2 Establishment".to_string(),
                        description: "Establish command and control channel".to_string(),
                        mitre_techniques: vec!["T1071".to_string()],
                        order: 1,
                    },
                    CampaignPhase {
                        id: "phase-2".to_string(),
                        name: "Post-Exploitation".to_string(),
                        description: "Execute post-exploitation activities".to_string(),
                        mitre_techniques: vec!["T1059".to_string(), "T1053".to_string()],
                        order: 2,
                    },
                ],
            },
        }
    }

    pub async fn scan(&self, target: &str) -> Result<C2Report> {
        let timestamp = chrono::Utc::now().to_rfc3339();

        let (beacon_results, task_results, opsec_assessment) = if self.dry_run {
            self.dry_run_simulation().await
        } else {
            self.real_simulation(target).await
        };

        let successful_beacons = beacon_results.iter().filter(|b| b.success).count();
        let completed_tasks = task_results
            .iter()
            .filter(|t| t.status == TaskStatus::Completed)
            .count();

        let summary = C2Summary {
            total_beacons: beacon_results.len(),
            successful_beacons,
            total_tasks: task_results.len(),
            completed_tasks,
            opsec_score: opsec_assessment.score,
            opsec_max: opsec_assessment.max_score,
        };

        let attack_graph = Some(campaign::build_attack_graph(
            &self.campaign.id,
            &self.campaign.name,
            &self.campaign.phases,
        ));
        let timeline = Some(campaign::build_timeline(
            &self.campaign.id,
            &self.campaign.name,
            &self.campaign.phases,
        ));

        Ok(C2Report {
            target: target.to_string(),
            campaign: self.campaign.clone(),
            beacon_results,
            task_results,
            opsec_assessment,
            summary,
            timestamp,
            dry_run: self.dry_run,
            attack_graph,
            timeline,
        })
    }

    async fn dry_run_simulation(&self) -> (Vec<BeaconResult>, Vec<TaskResult>, OpsecAssessment) {
        let beacon_results = self::beacon::simulate_beacons(&self.campaign, "dry-run", true).await;
        let task_results = self::tasking::simulate_tasks(&self.campaign, "dry-run", true).await;
        let opsec_assessment = self::opsec::simulate_opsec_assessment();

        (beacon_results, task_results, opsec_assessment)
    }

    async fn real_simulation(
        &self,
        target: &str,
    ) -> (Vec<BeaconResult>, Vec<TaskResult>, OpsecAssessment) {
        let beacon_results = self::beacon::simulate_beacons(&self.campaign, target, false).await;
        let task_results = self::tasking::simulate_tasks(&self.campaign, target, false).await;
        let opsec_assessment = self::opsec::simulate_opsec_assessment();

        (beacon_results, task_results, opsec_assessment)
    }

    pub fn campaign(&self) -> &C2Campaign {
        &self.campaign
    }
}

fn opsec_category_for(category: &OpsecCategory) -> String {
    match category {
        OpsecCategory::ParentSpoofing => "c2-parent-spoofing".to_string(),
        OpsecCategory::Timestomping => "c2-timestomping".to_string(),
        OpsecCategory::LogTampering => "c2-log-tampering".to_string(),
        OpsecCategory::ProcessMasquerading => "c2-process-masquerading".to_string(),
        OpsecCategory::BurnMechanism => "c2-burn-mechanism".to_string(),
        OpsecCategory::DecoyActivity => "c2-decoy-activity".to_string(),
    }
}

pub fn to_scan_report_data(report: &C2Report) -> crate::output::convert::ScanReportData {
    use crate::output::convert::FindingData;

    let mut findings: Vec<FindingData> = Vec::new();

    // Beacon results as findings
    for beacon in &report.beacon_results {
        findings.push(FindingData {
            title: format!("C2 Beacon ({})", beacon.protocol.as_str()),
            severity: if beacon.success {
                Severity::Medium.as_str().to_string()
            } else {
                Severity::Low.as_str().to_string()
            },
            category: "c2-beacon".to_string(),
            description: format!(
                "Beacon via {} (interval: {}ms, jitter: {}%) - {}",
                beacon.protocol.as_str(),
                beacon.interval_ms,
                beacon.jitter_percent,
                if beacon.success {
                    "successful"
                } else {
                    "failed"
                }
            ),
            location: report.target.clone(),
            evidence: beacon.evidence.clone(),
            remediation: Some(
                "Monitor for anomalous outbound connections with periodic timing patterns"
                    .to_string(),
            ),
            cwe_ids: Vec::new(),
        });
    }

    // Task results as findings
    for task in &report.task_results {
        let severity = match task.status {
            TaskStatus::Completed => Severity::High,
            TaskStatus::Simulated => Severity::Medium,
            TaskStatus::Failed => Severity::Low,
            TaskStatus::Denied => Severity::Info,
        };
        findings.push(FindingData {
            title: format!("C2 Task: {}", task.task_type.as_str()),
            severity: severity.as_str().to_string(),
            category: format!("c2-task-{}", task.task_type.as_str()),
            description: format!(
                "Task type: {} - status: {:?}{}",
                task.task_type.as_str(),
                task.status,
                task.output
                    .as_deref()
                    .map_or_else(String::new, |o| format!(" - {}", o))
            ),
            location: report.target.clone(),
            evidence: task.output.clone(),
            remediation: Some("Implement detection for C2 task execution patterns".to_string()),
            cwe_ids: Vec::new(),
        });
    }

    // OPSEC findings
    for finding in &report.opsec_assessment.findings {
        findings.push(FindingData {
            title: format!("OPSEC: {}", finding.description),
            severity: finding.severity.to_severity().as_str().to_string(),
            category: opsec_category_for(&finding.category),
            description: finding.description.clone(),
            location: report.target.clone(),
            evidence: None,
            remediation: Some(finding.recommendation.clone()),
            cwe_ids: Vec::new(),
        });
    }

    // Campaign summary finding
    findings.push(FindingData {
        title: "C2 Campaign Summary".to_string(),
        severity: Severity::Info.as_str().to_string(),
        category: "c2-summary".to_string(),
        description: format!(
            "Campaign '{}' ({}): {}/{} beacons successful, {}/{} tasks completed, OPSEC score: {}/{}",
            report.campaign.name,
            report.campaign.mitre_profile,
            report.summary.successful_beacons,
            report.summary.total_beacons,
            report.summary.completed_tasks,
            report.summary.total_tasks,
            report.summary.opsec_score,
            report.summary.opsec_max,
        ),
        location: report.target.clone(),
        evidence: Some(report.campaign.description.clone()),
        remediation: None,
        cwe_ids: Vec::new(),
    });

    crate::output::convert::ScanReportData {
        target: report.target.clone(),
        scan_type: "c2".to_string(),
        timestamp: report.timestamp.clone(),
        findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

pub async fn run_cli(
    args: crate::cli::C2Args,
    _config: &crate::config::EggsecConfig,
) -> Result<()> {
    let campaign_profile = args.campaign.as_deref().unwrap_or("default");
    let scanner = C2Scanner::new(args.dry_run, campaign_profile);

    if !args.quiet {
        if args.dry_run {
            eprintln!("DRY-RUN: planning mode (no real C2 operations performed).");
        } else {
            eprintln!("NOTE: Defense-lab only. Performing real C2 simulation.");
        }
        eprintln!(
            "Campaign: {} ({})",
            scanner.campaign().name,
            scanner.campaign().mitre_profile
        );
        eprintln!(
            "Phases: {} | Techniques: {}",
            scanner.campaign().phases.len(),
            scanner
                .campaign()
                .phases
                .iter()
                .flat_map(|p| &p.mitre_techniques)
                .count()
        );
    }

    let target = args.target.as_deref().unwrap_or("localhost");
    let report = scanner.scan(target).await?;

    let output = if args.json {
        serde_json::to_string_pretty(&report)?
    } else {
        let mut buf = String::new();
        if report.dry_run {
            buf.push_str("DRY-RUN: no real C2 operations performed\n\n");
        }
        buf.push_str(&format!("C2 Campaign Report - Target: {}\n", report.target));
        buf.push_str(&format!(
            "Campaign: {} ({})\n",
            report.campaign.name, report.campaign.mitre_profile
        ));
        buf.push_str(&format!(
            "Beacons: {}/{} successful | Tasks: {}/{} completed | OPSEC: {}/{}\n\n",
            report.summary.successful_beacons,
            report.summary.total_beacons,
            report.summary.completed_tasks,
            report.summary.total_tasks,
            report.summary.opsec_score,
            report.summary.opsec_max,
        ));

        for phase in &report.campaign.phases {
            buf.push_str(&format!(
                "  Phase {}: {} - {}\n",
                phase.order, phase.name, phase.description
            ));
            buf.push_str(&format!(
                "    MITRE: {}\n",
                phase.mitre_techniques.join(", ")
            ));
        }
        buf.push('\n');

        // Attack graph critical path
        if let Some(ref graph) = report.attack_graph {
            buf.push_str(&format!(
                "Attack Graph: {} nodes, critical path: {}\n",
                graph.nodes.len(),
                graph.critical_path.join(" -> ")
            ));
            buf.push('\n');
        }

        // Timeline summary
        if let Some(ref timeline) = report.timeline {
            buf.push_str(&format!(
                "Timeline: {} phases, {} techniques\n",
                timeline.total_phases, timeline.total_techniques
            ));
            for entry in &timeline.entries {
                buf.push_str(&format!(
                    "  [{}] Phase {}: {} ({})\n",
                    &entry.timestamp[11..19],
                    entry.phase_order,
                    entry.technique_id,
                    entry.phase_name
                ));
            }
            buf.push('\n');
        }

        for beacon in &report.beacon_results {
            let status = if beacon.success { "OK" } else { "FAIL" };
            buf.push_str(&format!(
                "  [{}] Beacon {} ({}ms, {}% jitter)\n",
                status,
                beacon.protocol.as_str(),
                beacon.interval_ms,
                beacon.jitter_percent
            ));
            if let Some(ref evidence) = beacon.evidence {
                buf.push_str(&format!("    Evidence: {}\n", evidence));
            }
        }
        buf.push('\n');

        for task in &report.task_results {
            buf.push_str(&format!(
                "  [{:?}] Task: {}\n",
                task.status,
                task.task_type.as_str()
            ));
            if let Some(ref output) = task.output {
                buf.push_str(&format!("    Output: {}\n", output));
            }
        }
        buf.push('\n');

        buf.push_str(&format!(
            "OPSEC Score: {}/{}\n",
            report.opsec_assessment.score, report.opsec_assessment.max_score
        ));
        for finding in &report.opsec_assessment.findings {
            buf.push_str(&format!(
                "  [{}] {} - {}\n",
                format!("{:?}", finding.severity).to_lowercase(),
                finding.description,
                finding.recommendation
            ));
        }

        buf
    };

    if let Some(ref output_file) = args.output {
        tokio::fs::write(output_file, &output).await?;
        if !args.quiet {
            eprintln!("Results written to {}", output_file);
        }
    } else {
        println!("{}", output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beacon_protocol_as_str() {
        assert_eq!(BeaconProtocol::Http.as_str(), "http");
        assert_eq!(BeaconProtocol::Https.as_str(), "https");
        assert_eq!(BeaconProtocol::Dns.as_str(), "dns");
        assert_eq!(BeaconProtocol::Tcp.as_str(), "tcp");
        assert_eq!(BeaconProtocol::Custom.as_str(), "custom");
    }

    #[test]
    fn test_task_type_as_str() {
        assert_eq!(TaskType::Recon.as_str(), "recon");
        assert_eq!(TaskType::Execute.as_str(), "execute");
        assert_eq!(TaskType::Exfil.as_str(), "exfil");
        assert_eq!(TaskType::Persist.as_str(), "persist");
        assert_eq!(TaskType::Lateral.as_str(), "lateral");
        assert_eq!(TaskType::Evade.as_str(), "evade");
        assert_eq!(TaskType::SelfDestruct.as_str(), "self-destruct");
    }

    #[test]
    fn test_opsec_severity_to_severity() {
        assert_eq!(OpsecSeverity::Info.to_severity(), Severity::Info);
        assert_eq!(OpsecSeverity::Low.to_severity(), Severity::Low);
        assert_eq!(OpsecSeverity::Medium.to_severity(), Severity::Medium);
        assert_eq!(OpsecSeverity::High.to_severity(), Severity::High);
    }

    #[test]
    fn test_c2_scanner_creation() {
        let scanner = C2Scanner::new(true, "apt29");
        assert!(scanner.dry_run);
        assert_eq!(scanner.campaign().name, "APT29 Simulation");
        assert_eq!(scanner.campaign().phases.len(), 4);
    }

    #[test]
    fn test_default_campaign_profiles() {
        let apt29 = C2Scanner::new(true, "apt29");
        assert_eq!(apt29.campaign().mitre_profile, "APT29 (Cozy Bear)");

        let carbanak = C2Scanner::new(true, "carbanak");
        assert_eq!(carbanak.campaign().mitre_profile, "Carbanak/FIN7");

        let default = C2Scanner::new(true, "unknown");
        assert_eq!(default.campaign().mitre_profile, "Generic APT");
    }

    #[tokio::test]
    async fn test_dry_run_scan_produces_results() {
        let scanner = C2Scanner::new(true, "apt29");
        let report = scanner.scan("test-target").await.unwrap();
        assert!(report.dry_run);
        assert_eq!(report.target, "test-target");
        assert!(!report.beacon_results.is_empty());
        assert!(!report.task_results.is_empty());
        assert!(report.opsec_assessment.score <= report.opsec_assessment.max_score);
    }

    #[tokio::test]
    async fn test_dry_run_scan_summary_counts() {
        let scanner = C2Scanner::new(true, "default");
        let report = scanner.scan("localhost").await.unwrap();
        assert_eq!(report.summary.total_beacons, report.beacon_results.len());
        assert_eq!(
            report.summary.successful_beacons,
            report.beacon_results.iter().filter(|b| b.success).count()
        );
        assert_eq!(report.summary.total_tasks, report.task_results.len());
        assert_eq!(
            report.summary.completed_tasks,
            report
                .task_results
                .iter()
                .filter(|t| t.status == TaskStatus::Completed)
                .count()
        );
    }

    #[test]
    fn test_opsec_category_for_mapping() {
        assert_eq!(
            opsec_category_for(&OpsecCategory::ParentSpoofing),
            "c2-parent-spoofing"
        );
        assert_eq!(
            opsec_category_for(&OpsecCategory::Timestomping),
            "c2-timestomping"
        );
        assert_eq!(
            opsec_category_for(&OpsecCategory::LogTampering),
            "c2-log-tampering"
        );
        assert_eq!(
            opsec_category_for(&OpsecCategory::ProcessMasquerading),
            "c2-process-masquerading"
        );
        assert_eq!(
            opsec_category_for(&OpsecCategory::BurnMechanism),
            "c2-burn-mechanism"
        );
        assert_eq!(
            opsec_category_for(&OpsecCategory::DecoyActivity),
            "c2-decoy-activity"
        );
    }

    #[test]
    fn test_to_scan_report_data_bridge() {
        let report = C2Report {
            target: "test-target".to_string(),
            campaign: C2Campaign {
                id: "test".to_string(),
                name: "Test Campaign".to_string(),
                description: "Test".to_string(),
                mitre_profile: "Test".to_string(),
                phases: Vec::new(),
            },
            beacon_results: vec![BeaconResult {
                protocol: BeaconProtocol::Https,
                interval_ms: 60000,
                jitter_percent: 25,
                success: true,
                evidence: Some("test evidence".to_string()),
            }],
            task_results: vec![TaskResult {
                task_type: TaskType::Recon,
                status: TaskStatus::Completed,
                output: Some("recon output".to_string()),
                mitre_technique: Some("T1071".to_string()),
            }],
            opsec_assessment: OpsecAssessment {
                score: 85,
                max_score: 100,
                findings: Vec::new(),
            },
            summary: C2Summary {
                total_beacons: 1,
                successful_beacons: 1,
                total_tasks: 1,
                completed_tasks: 1,
                opsec_score: 85,
                opsec_max: 100,
            },
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dry_run: true,
            attack_graph: None,
            timeline: None,
        };
        let bridge = to_scan_report_data(&report);
        assert_eq!(bridge.target, "test-target");
        assert_eq!(bridge.scan_type, "c2");
        // 1 beacon + 1 task + 1 summary = 3 findings
        assert_eq!(bridge.findings.len(), 3);
    }

    #[test]
    fn test_to_scan_report_data_empty() {
        let report = C2Report {
            target: "empty".to_string(),
            campaign: C2Campaign {
                id: "empty".to_string(),
                name: "Empty".to_string(),
                description: "Empty".to_string(),
                mitre_profile: "None".to_string(),
                phases: Vec::new(),
            },
            beacon_results: Vec::new(),
            task_results: Vec::new(),
            opsec_assessment: OpsecAssessment {
                score: 0,
                max_score: 100,
                findings: Vec::new(),
            },
            summary: C2Summary {
                total_beacons: 0,
                successful_beacons: 0,
                total_tasks: 0,
                completed_tasks: 0,
                opsec_score: 0,
                opsec_max: 100,
            },
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dry_run: true,
            attack_graph: None,
            timeline: None,
        };
        let bridge = to_scan_report_data(&report);
        // Only the summary finding
        assert_eq!(bridge.findings.len(), 1);
        assert_eq!(bridge.findings[0].title, "C2 Campaign Summary");
    }

    #[test]
    fn test_serialization_roundtrip() {
        let report = C2Report {
            target: "test".to_string(),
            campaign: C2Campaign {
                id: "test".to_string(),
                name: "Test".to_string(),
                description: "Test".to_string(),
                mitre_profile: "Test".to_string(),
                phases: Vec::new(),
            },
            beacon_results: Vec::new(),
            task_results: Vec::new(),
            opsec_assessment: OpsecAssessment {
                score: 0,
                max_score: 100,
                findings: Vec::new(),
            },
            summary: C2Summary {
                total_beacons: 0,
                successful_beacons: 0,
                total_tasks: 0,
                completed_tasks: 0,
                opsec_score: 0,
                opsec_max: 100,
            },
            timestamp: "2024-01-01T00:00:00Z".to_string(),
            dry_run: true,
            attack_graph: None,
            timeline: None,
        };
        let json = serde_json::to_string(&report).unwrap();
        let deserialized: C2Report = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.target, report.target);
        assert_eq!(deserialized.dry_run, report.dry_run);
    }

    #[test]
    fn test_beacon_protocol_serialization() {
        let json = serde_json::to_string(&BeaconProtocol::Https).unwrap();
        assert_eq!(json, "\"https\"");
    }

    #[test]
    fn test_task_type_serialization() {
        let json = serde_json::to_string(&TaskType::SelfDestruct).unwrap();
        assert_eq!(json, "\"self_destruct\"");
    }

    #[test]
    fn test_opsec_category_serialization() {
        let json = serde_json::to_string(&OpsecCategory::ProcessMasquerading).unwrap();
        assert_eq!(json, "\"process_masquerading\"");
    }

    #[tokio::test]
    async fn test_real_simulation_produces_different_evidence_than_dry_run() {
        let dry_scanner = C2Scanner::new(true, "apt29");
        let dry_report = dry_scanner.scan("test-target").await.unwrap();

        // Real simulation against an unreachable target produces different evidence
        let real_scanner = C2Scanner::new(false, "apt29");
        let real_report = real_scanner.scan("127.0.0.1:1").await.unwrap();

        // Dry-run beacons always succeed; real beacons fail against unreachable target
        assert!(
            dry_report.beacon_results.iter().all(|b| b.success),
            "dry-run beacons should always succeed"
        );
        assert!(
            real_report.beacon_results.iter().all(|b| !b.success),
            "real beacons should fail against unreachable target"
        );

        // Evidence should differ: dry-run has "dry-run:" prefix, real has "real:" or error
        let dry_evidence = dry_report.beacon_results[0].evidence.as_ref().unwrap();
        let real_evidence = real_report.beacon_results[0].evidence.as_ref().unwrap();
        assert!(
            dry_evidence.contains("dry-run"),
            "dry-run evidence should contain 'dry-run'"
        );
        assert!(
            !real_evidence.contains("dry-run"),
            "real evidence should NOT contain 'dry-run'"
        );
    }

    #[tokio::test]
    async fn test_real_simulation_report_marked_not_dry_run() {
        let scanner = C2Scanner::new(false, "apt29");
        let report = scanner.scan("127.0.0.1:1").await.unwrap();
        assert!(
            !report.dry_run,
            "real simulation report should have dry_run=false"
        );
    }

    #[tokio::test]
    async fn test_dry_run_scan_always_succeeds() {
        let scanner = C2Scanner::new(true, "apt29");
        let report = scanner
            .scan("totally-invalid-target-that-does-not-exist")
            .await
            .unwrap();
        assert!(report.dry_run);
        assert!(
            report.beacon_results.iter().all(|b| b.success),
            "dry-run beacons should always succeed regardless of target"
        );
        assert!(
            report
                .task_results
                .iter()
                .all(|t| t.status == TaskStatus::Completed || t.status == TaskStatus::Simulated),
            "dry-run tasks should always complete or be simulated"
        );
    }
}
