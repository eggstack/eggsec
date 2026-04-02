use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct PlanArgs {
    /// Target to plan for
    #[arg(short, long)]
    pub target: Option<String>,

    /// Output format
    #[arg(short, long, default_value = "table")]
    pub format: String,

    /// Profile to use (quick, default, thorough)
    #[arg(short, long, default_value = "default")]
    pub profile: String,
}
