use clap::Parser;

#[derive(Debug, Clone, Parser)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: Option<AgentCommand>,

    /// Portfolio file path (JSON)
    #[arg(long)]
    pub portfolio: Option<String>,

    /// Memory directory for longitudinal storage
    #[arg(long, default_value = "~/.config/slapper/memory")]
    pub memory_dir: String,

    /// Poll interval in seconds
    #[arg(long, default_value = "60")]
    pub poll_interval: u64,

    /// Enable AI integration
    #[arg(long)]
    pub with_ai: bool,

    /// AI config file path
    #[arg(long)]
    pub ai_config: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub enum AgentCommand {
    /// Run the autonomous agent
    Run(RunArgs),
    /// Manage targets in the portfolio
    Targets(TargetsArgs),
    /// Manage skills
    Skills(SkillsArgs),
    /// Show agent status
    Status,
}

#[derive(Debug, Clone, Parser)]
pub struct RunArgs {
    /// Run once and exit (don't loop)
    #[arg(long)]
    pub once: bool,
}

#[derive(Debug, Clone, Parser)]
pub struct TargetsArgs {
    #[command(subcommand)]
    pub command: TargetsCommand,
}

#[derive(Debug, Clone, Parser)]
pub enum TargetsCommand {
    /// List all targets
    List,
    /// Add a new target
    Add(AddTargetArgs),
    /// Update a target
    Update(UpdateTargetArgs),
    /// Remove a target
    Remove { id: String },
    /// Enable a target
    Enable { id: String },
    /// Disable a target
    Disable { id: String },
}

#[derive(Debug, Clone, Parser)]
pub struct AddTargetArgs {
    /// Target ID
    pub id: String,
    /// Target URL
    pub target: String,
    /// Target type
    #[arg(long, default_value = "url")]
    pub target_type: String,
    /// Schedule (cron expression)
    #[arg(long)]
    pub schedule: Option<String>,
    /// Priority
    #[arg(long, default_value = "normal")]
    pub priority: String,
}

#[derive(Debug, Clone, Parser)]
pub struct UpdateTargetArgs {
    /// Target ID to update
    pub id: String,
    /// New target URL
    #[arg(long)]
    pub target: Option<String>,
    /// New schedule (cron expression)
    #[arg(long)]
    pub schedule: Option<String>,
    /// New priority
    #[arg(long)]
    pub priority: Option<String>,
    /// Scan depth (quick, normal, deep)
    #[arg(long)]
    pub scan_depth: Option<String>,
}

#[derive(Debug, Clone, Parser)]
pub struct SkillsArgs {
    #[command(subcommand)]
    pub command: SkillsCommand,
}

#[derive(Debug, Clone, Parser)]
pub enum SkillsCommand {
    /// List available skills
    List,
    /// Load skills from directory
    Load { path: String },
    /// Show skill details
    Show { name: String },
}
