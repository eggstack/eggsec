use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct AiAnalyzeArgs {
    /// Input findings file (JSON)
    #[arg(short, long)]
    pub input: Option<String>,

    /// Output file
    #[arg(short, long)]
    pub output: Option<String>,

    /// Analysis type (severity, exploitability, attack-chain, remediation)
    #[arg(short, long, default_value = "full")]
    pub analysis_type: String,
}
