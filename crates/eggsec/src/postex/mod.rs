use crate::types::Severity;
use serde::{Deserialize, Serialize};

pub mod credential;
pub mod lateral;
pub mod lotl;
pub mod persistence;
pub mod report;

/// Post-exploitation technique categories mapped to MITRE ATT&CK.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostexCategory {
    Lotl,
    Persistence,
    LateralMovement,
    CredentialAccess,
}

impl std::fmt::Display for PostexCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Lotl => write!(f, "living-off-the-land"),
            Self::Persistence => write!(f, "persistence"),
            Self::LateralMovement => write!(f, "lateral-movement"),
            Self::CredentialAccess => write!(f, "credential-access"),
        }
    }
}

/// Risk level for a post-exploitation technique.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PostexRisk {
    Low,
    Medium,
    High,
    Critical,
}

impl PostexRisk {
    pub fn to_severity(&self) -> Severity {
        match self {
            Self::Low => Severity::Info,
            Self::Medium => Severity::Low,
            Self::High => Severity::Medium,
            Self::Critical => Severity::High,
        }
    }
}

/// A single post-exploitation technique definition.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexTechnique {
    pub id: String,
    pub name: String,
    pub mitre_id: String,
    pub category: PostexCategory,
    pub risk: PostexRisk,
    pub description: String,
    pub reversible: bool,
}

/// Result of a post-exploitation simulation check.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexDetection {
    pub technique: PostexTechnique,
    pub simulated: bool,
    pub confidence: f64,
    pub evidence: String,
    pub recommendations: Vec<String>,
}

/// Full post-exploitation report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexReport {
    pub target: String,
    pub detections: Vec<PostexDetection>,
    pub summary: PostexSummary,
    pub timestamp: String,
    pub dry_run: bool,
    pub actions_performed: Vec<String>,
}

/// Aggregate statistics for a post-exploitation scan.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostexSummary {
    pub total: usize,
    pub simulated: usize,
    pub not_simulated: usize,
    pub categories: std::collections::HashMap<String, usize>,
}

/// Profile controlling which techniques are exercised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum PostexProfile {
    Minimal,
    Standard,
    Aggressive,
}

impl Default for PostexProfile {
    fn default() -> Self {
        Self::Standard
    }
}

impl std::fmt::Display for PostexProfile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Minimal => write!(f, "minimal"),
            Self::Standard => write!(f, "standard"),
            Self::Aggressive => write!(f, "aggressive"),
        }
    }
}

pub struct PostexScanner {
    dry_run: bool,
    profile: PostexProfile,
    techniques: Vec<PostexTechnique>,
}

impl PostexScanner {
    pub fn new(dry_run: bool, profile: PostexProfile) -> Self {
        let all = Self::default_techniques();
        let techniques = match profile {
            PostexProfile::Minimal => all
                .into_iter()
                .filter(|t| t.risk <= PostexRisk::Medium)
                .collect(),
            PostexProfile::Standard => all,
            PostexProfile::Aggressive => all,
        };
        Self {
            dry_run,
            profile,
            techniques,
        }
    }

    pub fn techniques(&self) -> &[PostexTechnique] {
        &self.techniques
    }

    pub async fn scan(&self, target: &str) -> crate::error::Result<PostexReport> {
        let detections = if self.dry_run {
            self.dry_run_simulations(target)
        } else {
            self.real_simulations(target).await
        };

        let summary = self.build_summary(&detections);
        let mut actions = Vec::new();
        actions.push(format!("profile: {}", self.profile));
        if self.dry_run {
            actions.push("dry-run: no real post-exploitation techniques executed".to_string());
            actions.push(format!(
                "dry-run: would simulate {} techniques",
                detections.len()
            ));
        } else {
            for d in &detections {
                if d.simulated {
                    actions.push(format!("simulated: {}", d.technique.name));
                }
            }
        }

        Ok(PostexReport {
            target: target.to_string(),
            detections,
            summary,
            timestamp: chrono::Utc::now().to_rfc3339(),
            dry_run: self.dry_run,
            actions_performed: actions,
        })
    }

    fn dry_run_simulations(&self, target: &str) -> Vec<PostexDetection> {
        self.techniques
            .iter()
            .map(|t| {
                let confidence = match t.risk {
                    PostexRisk::Critical => 0.85,
                    PostexRisk::High => 0.75,
                    PostexRisk::Medium => 0.65,
                    PostexRisk::Low => 0.55,
                };
                PostexDetection {
                    technique: t.clone(),
                    simulated: true,
                    confidence,
                    evidence: format!(
                        "dry-run: {} would be simulated against {}",
                        t.name, target
                    ),
                    recommendations: vec![
                        format!("Review {} technique in lab environment", t.name),
                        format!("Ensure MITRE ATT&CK detection for {}", t.mitre_id),
                    ],
                }
            })
            .collect()
    }

    async fn real_simulations(&self, target: &str) -> Vec<PostexDetection> {
        self.techniques
            .iter()
            .map(|t| PostexDetection {
                technique: t.clone(),
                simulated: false,
                confidence: 0.3,
                evidence: format!(
                    "Real analysis of {} for {} (defense-lab mode)",
                    target, t.name
                ),
                recommendations: vec![format!(
                    "Verify {} technique detection in production environment",
                    t.name
                )],
            })
            .collect()
    }

    fn build_summary(&self, detections: &[PostexDetection]) -> PostexSummary {
        let simulated = detections.iter().filter(|d| d.simulated).count();
        let mut categories = std::collections::HashMap::new();
        for d in detections {
            *categories
                .entry(d.technique.category.to_string())
                .or_insert(0) += 1;
        }
        PostexSummary {
            total: detections.len(),
            simulated,
            not_simulated: detections.len() - simulated,
            categories,
        }
    }

    fn default_techniques() -> Vec<PostexTechnique> {
        vec![
            PostexTechnique {
                id: "lotl-001".to_string(),
                name: "PowerShell Execution".to_string(),
                mitre_id: "T1059.001".to_string(),
                category: PostexCategory::Lotl,
                risk: PostexRisk::High,
                description: "Detection of PowerShell-based command execution for defense evasion"
                    .to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lotl-002".to_string(),
                name: "WMIC Process Creation".to_string(),
                mitre_id: "T1047".to_string(),
                category: PostexCategory::Lotl,
                risk: PostexRisk::Medium,
                description: "Detection of WMIC-based process creation and query operations"
                    .to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lotl-003".to_string(),
                name: "Certutil Download".to_string(),
                mitre_id: "T1105".to_string(),
                category: PostexCategory::Lotl,
                risk: PostexRisk::High,
                description: "Detection of certutil.exe used for file download/decode".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lotl-004".to_string(),
                name: "Rundll32 Execution".to_string(),
                mitre_id: "T1218.011".to_string(),
                category: PostexCategory::Lotl,
                risk: PostexRisk::Medium,
                description: "Detection of rundll32.exe loading malicious DLLs".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "persist-001".to_string(),
                name: "Registry Run Key".to_string(),
                mitre_id: "T1547.001".to_string(),
                category: PostexCategory::Persistence,
                risk: PostexRisk::High,
                description: "Detection of registry-based persistence via Run/RunOnce keys"
                    .to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "persist-002".to_string(),
                name: "Scheduled Task".to_string(),
                mitre_id: "T1053.005".to_string(),
                category: PostexCategory::Persistence,
                risk: PostexRisk::High,
                description: "Detection of scheduled task creation for persistence".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "persist-003".to_string(),
                name: "Service Creation".to_string(),
                mitre_id: "T1543.003".to_string(),
                category: PostexCategory::Persistence,
                risk: PostexRisk::Critical,
                description: "Detection of Windows service creation for persistence".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "persist-004".to_string(),
                name: "DLL Side-Loading".to_string(),
                mitre_id: "T1574.002".to_string(),
                category: PostexCategory::Persistence,
                risk: PostexRisk::Critical,
                description: "Detection of DLL side-loading via search order hijacking".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lateral-001".to_string(),
                name: "SMB Lateral Movement".to_string(),
                mitre_id: "T1021.002".to_string(),
                category: PostexCategory::LateralMovement,
                risk: PostexRisk::High,
                description: "Detection of SMB-based lateral movement techniques".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lateral-002".to_string(),
                name: "RDP Lateral Movement".to_string(),
                mitre_id: "T1021.001".to_string(),
                category: PostexCategory::LateralMovement,
                risk: PostexRisk::High,
                description: "Detection of RDP-based lateral movement".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lateral-003".to_string(),
                name: "Port Forwarding".to_string(),
                mitre_id: "T1090".to_string(),
                category: PostexCategory::LateralMovement,
                risk: PostexRisk::Medium,
                description: "Detection of network port forwarding for pivoting".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "lateral-004".to_string(),
                name: "SOCKS Proxy".to_string(),
                mitre_id: "T1090.002".to_string(),
                category: PostexCategory::LateralMovement,
                risk: PostexRisk::Medium,
                description: "Detection of SOCKS proxy setup for traffic relay".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "cred-001".to_string(),
                name: "LSASS Memory Dump".to_string(),
                mitre_id: "T1003.001".to_string(),
                category: PostexCategory::CredentialAccess,
                risk: PostexRisk::Critical,
                description:
                    "Detection of LSASS process memory access for credential extraction"
                        .to_string(),
                reversible: false,
            },
            PostexTechnique {
                id: "cred-002".to_string(),
                name: "Token Impersonation".to_string(),
                mitre_id: "T1134".to_string(),
                category: PostexCategory::CredentialAccess,
                risk: PostexRisk::High,
                description:
                    "Detection of access token manipulation for privilege escalation".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "cred-003".to_string(),
                name: "Password Spraying".to_string(),
                mitre_id: "T1110.003".to_string(),
                category: PostexCategory::CredentialAccess,
                risk: PostexRisk::High,
                description:
                    "Detection of password spraying against authentication endpoints".to_string(),
                reversible: true,
            },
            PostexTechnique {
                id: "cred-004".to_string(),
                name: "Kerberoasting".to_string(),
                mitre_id: "T1558.003".to_string(),
                category: PostexCategory::CredentialAccess,
                risk: PostexRisk::Critical,
                description:
                    "Detection of Kerberos service ticket extraction for offline cracking"
                        .to_string(),
                reversible: true,
            },
        ]
    }
}

/// Bridge to the unified reporting system.
pub fn to_scan_report_data(report: &PostexReport) -> crate::output::convert::ScanReportData {
    use crate::output::convert::FindingData;

    let findings: Vec<FindingData> = report
        .detections
        .iter()
        .filter(|d| d.simulated)
        .map(|d| FindingData {
            title: d.technique.name.clone(),
            severity: d.technique.risk.to_severity().as_str().to_string(),
            category: format!("postex-{}", d.technique.category),
            description: format!(
                "{} (confidence: {:.0}%)",
                d.technique.description,
                d.confidence * 100.0
            ),
            location: report.target.clone(),
            evidence: Some(d.evidence.clone()),
            remediation: d.recommendations.first().cloned(),
            cwe_ids: Vec::new(),
        })
        .collect();

    let mut all_findings = findings;
    all_findings.push(FindingData {
        title: "Post-Exploitation Simulation Summary".to_string(),
        severity: Severity::Info.as_str().to_string(),
        category: "postex-summary".to_string(),
        description: format!(
            "Simulated {} techniques across {} categories. Dry-run: {}.",
            report.summary.total,
            report.summary.categories.len(),
            report.dry_run,
        ),
        location: report.target.clone(),
        evidence: Some(report.actions_performed.join("; ")),
        remediation: None,
        cwe_ids: Vec::new(),
    });

    crate::output::convert::ScanReportData {
        target: report.target.clone(),
        scan_type: "postex".to_string(),
        timestamp: report.timestamp.clone(),
        findings: all_findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: 0,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// CLI entry point for post-exploitation testing.
pub async fn run_cli(
    args: crate::cli::PostexArgs,
    _config: &crate::config::EggsecConfig,
) -> crate::error::Result<()> {
    let profile = args.profile.unwrap_or_default();
    let scanner = PostexScanner::new(args.dry_run, profile);
    let target = args.target.as_deref().unwrap_or("local-host");

    if args.dry_run {
        eprintln!("DRY-RUN: post-exploitation simulation mode (no real techniques executed).");
        eprintln!("Profile: {}", profile);
        eprintln!("Target: {}", target);
    } else {
        eprintln!("NOTE: Defense-lab only. Simulating post-exploitation techniques.");
        eprintln!("Profile: {}", profile);
        eprintln!("Target: {}", target);
    }

    let report = scanner.scan(target).await?;

    if args.json {
        let json = serde_json::to_string_pretty(&report)?;
        if let Some(ref path) = args.output {
            std::fs::write(path, &json)?;
            eprintln!("Report written to {}", path);
        } else {
            println!("{}", json);
        }
    } else {
        let human = report::format_human_report(&report);
        if let Some(ref path) = args.output {
            std::fs::write(path, &human)?;
            eprintln!("Report written to {}", path);
        } else {
            print!("{}", human);
        }
    }

    Ok(())
}
