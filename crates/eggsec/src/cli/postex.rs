pub(crate) const POSTEX_ABOUT: &str = r#"MODE: Defense Lab | Lab-only; authorized use only.

Simulate post-exploitation techniques for defense validation and purple teaming.
Covers Living-Off-The-Land (LOTL), persistence, lateral movement, and credential access.
Maps simulations to MITRE ATT&CK IDs with confidence scores.

WARNING: This is a standalone defense-lab capability. It simulates post-exploitation
techniques in dry-run mode by default. Real execution requires explicit authorization
and scope. Not for offensive use.

Examples:
  eggsec postex --target 10.0.0.1 --dry-run --json
  eggsec postex --target 10.0.0.1 --profile minimal --dry-run
  eggsec postex --target 10.0.0.1 --profile aggressive --dry-run -o postex-report.json
  eggsec postex --category lotl --dry-run --json
  eggsec postex --category persistence --target 10.0.0.1 --dry-run

Requires building with --features postex.
All dry-run operations produce complete reports with synthetic data (no side effects).
Real mode requires explicit --allow-postex flag.
"#;

#[derive(clap::Args, Clone)]
pub struct PostexArgs {
    /// Target host or IP address for the simulation
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Category of techniques to simulate (lotl, persistence, lateral, credential)
    #[arg(long, value_name = "CATEGORY")]
    pub category: Option<String>,

    /// Technique profile (minimal, standard, aggressive)
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<crate::postex::PostexProfile>,

    /// Plan/dry-run mode: produce complete report with synthetic data, no real techniques
    #[arg(long)]
    pub dry_run: bool,

    /// Output results as JSON
    #[arg(long)]
    pub json: bool,

    /// Write output to file instead of stdout
    #[arg(short, long, value_name = "FILE")]
    pub output: Option<String>,

    /// Suppress non-essential output
    #[arg(long, short = 'q')]
    pub quiet: bool,
}
