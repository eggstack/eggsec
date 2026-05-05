use crate::types::Severity;

pub(crate) const VULN_ABOUT: &str = "Vulnerability management tools

Provides CVSS scoring, exploitability assessment, prioritization, triage, and remediation.

Examples:
  slapper vuln score CVSS:3.1/AV:N/AC:L/PR:N/UI:N/S:U/C:H/I:H/A:H
  slapper vuln exploitability CVE-2021-44228
  slapper vuln prioritize --severity critical --cvss 9.8
  slapper vuln triage --title 'SQL Injection' --severity high
  slapper vuln remediate --severity critical";

#[derive(clap::Args)]
pub struct VulnArgs {
    #[command(subcommand)]
    pub command: VulnCommand,
}

#[derive(clap::Subcommand)]
pub enum VulnCommand {
    #[command(about = "Calculate CVSS score from vector")]
    Score(VulnScoreArgs),
    #[command(about = "Assess exploitability of a CVE")]
    Exploitability(VulnExploitArgs),
    #[command(about = "Prioritize vulnerabilities by risk")]
    Prioritize(VulnPrioritizeArgs),
    #[command(about = "Triage a finding")]
    Triage(VulnTriageArgs),
    #[command(about = "Get remediation guidance")]
    Remediate(VulnRemediateArgs),
}

#[derive(clap::Args)]
pub struct VulnScoreArgs {
    #[arg(help = "CVSS 3.1 vector string")]
    pub vector: String,
}

#[derive(clap::Args)]
pub struct VulnExploitArgs {
    #[arg(help = "CVE identifier (e.g., CVE-2021-44228)")]
    pub cve_id: String,
}

#[derive(clap::Args)]
pub struct VulnPrioritizeArgs {
    #[arg(long, help = "Finding title")]
    pub title: String,
    #[arg(long, help = "Severity level", value_enum)]
    pub severity: Severity,
    #[arg(long, help = "CVSS score (0.0-10.0)")]
    pub cvss: Option<f32>,
    #[arg(long, help = "Asset criticality score (0.0-10.0)")]
    pub asset_criticality: Option<f32>,
    #[arg(long, help = "Exploitability score (0.0-10.0)")]
    pub exploitability: Option<f32>,
}

#[derive(clap::Args)]
pub struct VulnTriageArgs {
    #[arg(long, help = "Finding ID")]
    pub id: Option<String>,
    #[arg(long, help = "Finding title")]
    pub title: String,
    #[arg(long, help = "Finding description")]
    pub description: Option<String>,
    #[arg(long, help = "Severity level", value_enum)]
    pub severity: Severity,
    #[arg(long, help = "CVSS score")]
    pub cvss: Option<f32>,
}

#[derive(clap::Args)]
pub struct VulnRemediateArgs {
    #[arg(long, help = "Finding ID")]
    pub id: Option<String>,
    #[arg(long, help = "Finding title")]
    pub title: String,
    #[arg(long, help = "Severity level", value_enum)]
    pub severity: Severity,
}
