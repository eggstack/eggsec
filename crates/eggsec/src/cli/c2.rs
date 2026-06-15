pub(crate) const C2_ABOUT: &str = r#"MODE: Defense Lab | Lab-only; authorized use only.

Simulate Command & Control (C2) operations for defense validation and purple teaming.
Covers beaconing, tasking, campaign orchestration, and OPSEC assessment.
Maps simulations to MITRE ATT&CK profiles with confidence scores.

WARNING: This is a standalone defense-lab capability. It simulates C2 operations
in dry-run mode by default. Real execution requires explicit authorization
and scope. Not for offensive use.

Supported campaign profiles:
  apt29      - APT29 (Cozy Bear) simulation with HTTP/S beacons and LOTL techniques
  carbanak   - Carbanak/FIN7 simulation with DNS beacons and financial targeting
  default    - Generic purple team campaign with mixed C2 protocols

Examples:
  eggsec c2 --target 10.0.0.1 --dry-run --json
  eggsec c2 --target 10.0.0.1 --campaign apt29 --dry-run
  eggsec c2 --target 10.0.0.1 --campaign carbanak --dry-run -o c2-report.json
  eggsec c2 --dry-run --json

Requires building with --features c2.
All dry-run operations produce complete reports with synthetic data (no side effects).
Real mode requires explicit --allow-c2 flag.
"#;

#[derive(clap::Args, Clone)]
pub struct C2Args {
    /// Target host or IP address for the C2 simulation
    #[arg(long, value_name = "TARGET")]
    pub target: Option<String>,

    /// Campaign profile (apt29, carbanak, default)
    #[arg(long, value_name = "PROFILE")]
    pub campaign: Option<String>,

    /// Plan/dry-run mode: produce complete report with synthetic data, no real C2 operations
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
