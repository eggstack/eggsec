use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct PlanArgs {
    /// Target to plan for
    #[arg(short, long)]
    pub target: Option<String>,

    /// Output file
    #[arg(short = 'o', long)]
    pub output: Option<String>,

    /// Output format (table, json)
    #[arg(short, long, default_value = "table")]
    pub format: String,

    /// Scan profile (quick, endpoint, web, waf, full, api, recon, stealth,
    /// deep, vuln, auth, defense-lab, synvoid-local, waf-regression,
    /// protocol-edge, nse-safe)
    #[arg(short, long, default_value = "quick")]
    pub profile: String,

    /// Scope file path
    #[arg(long)]
    pub scope: Option<String>,

    /// Enable verbose output
    #[arg(short, long)]
    pub verbose: bool,

    /// Suppress non-essential output
    #[arg(short, long)]
    pub quiet: bool,
}
