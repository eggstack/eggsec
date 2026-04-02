use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct CiArgs {
    /// Target to scan
    #[arg(short, long)]
    pub target: Option<String>,

    /// Fail if findings at or above this severity
    #[arg(long, default_value = "high")]
    pub fail_on: String,

    /// Maximum number of findings before failing
    #[arg(long)]
    pub max_findings: Option<usize>,

    /// Compare against baseline SARIF file
    #[arg(long)]
    pub baseline: Option<String>,

    /// Quiet mode for CI output
    #[arg(short, long)]
    pub quiet: bool,

    /// Output format
    #[arg(short, long, default_value = "sarif")]
    pub format: String,

    /// Output file path
    #[arg(short, long)]
    pub output: Option<String>,
}
