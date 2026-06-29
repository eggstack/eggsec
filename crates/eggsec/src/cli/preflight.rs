use clap::Parser;

const PREFLIGHT_ABOUT: &str = r#"Preview enforcement decision for an operation without executing it.

Shows the same policy/scope/capability decision that dispatch would use,
including required confirmation classes and suggested CLI flags.

Examples:
  eggsec preflight scan-ports --target 192.168.1.1
  eggsec preflight fuzz --target https://example.com/api --json
  eggsec preflight waf-detect --target https://example.com
  eggsec preflight stress --target 10.0.0.1 --allow-high-risk
  eggsec preflight scan-ports --target 10.0.0.1 --profile ci
"#;

#[derive(Debug, Clone, Parser)]
#[command(about = "Preview enforcement decision without executing", long_about = PREFLIGHT_ABOUT)]
pub struct PreflightArgs {
    /// Operation to evaluate (e.g., scan-ports, fuzz, waf-detect, stress, recon)
    pub operation: String,

    /// Target to evaluate against
    #[arg(long)]
    pub target: Option<String>,

    /// Output in JSON format
    #[arg(long)]
    pub json: bool,

    /// Enforcement profile to simulate. Overrides the default CLI manual profile.
    /// Options: manual (default), ci, mcp, agent, guarded.
    #[arg(long, value_name = "PROFILE")]
    pub profile: Option<PreflightProfile>,
}

/// Supported enforcement profiles for preflight simulation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum PreflightProfile {
    /// CLI manual permissive (default)
    Manual,
    /// CI strict (deterministic, no overrides)
    Ci,
    /// MCP strict (automated agent)
    Mcp,
    /// Security agent strict
    Agent,
    /// CLI manual guarded (strict scope)
    Guarded,
}
