pub mod ci;
pub mod scan;
pub mod fuzz;
pub mod load;
pub mod cluster;
pub mod recon;
pub mod network;
pub mod plan;
pub mod plugin;
pub mod report;
pub mod stress;
pub mod notify;
pub mod auth_test;
#[cfg(feature = "sbom")]
pub mod sbom;
#[cfg(feature = "rest-api")]
pub mod agent;

pub use ci::*;
pub use scan::*;
pub use fuzz::*;
pub use load::*;
pub use cluster::*;
pub use recon::*;
pub use network::*;
pub use plan::*;

#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub use plugin::*;
pub use report::*;
#[cfg(feature = "stress-testing")]
pub use stress::*;
pub use notify::*;
#[cfg(feature = "sbom")]
pub use sbom::*;
pub use auth_test::*;
#[cfg(feature = "rest-api")]
pub use agent::*;

#[cfg(feature = "ai-integration")]
pub mod ai_analyze;

#[cfg(feature = "ai-integration")]
pub use ai_analyze::*;

use anyhow::Result;
use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::{SlapperConfig, Scope};
use crate::error::Result as ErrorResult;

pub struct CommandContext {
    pub config: SlapperConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
}

impl CommandContext {
    pub fn new(config: SlapperConfig, scope: Scope, json: bool) -> Self {
        Self {
            config,
            scope,
            json,
            config_path: None,
        }
    }

    pub fn with_config_path(mut self, path: Option<String>) -> Self {
        self.config_path = path;
        self
    }

    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref()
    }

    pub fn ensure_scope_url(&self, url: &str) -> ErrorResult<()> {
        crate::utils::check_scope_from_url(&self.scope, url)
    }

    pub fn ensure_scope(&self, target: &str) -> ErrorResult<()> {
        crate::utils::check_scope(&self.scope, target)
    }
}

pub async fn handle_command(cli: Cli, ctx: &CommandContext) -> Result<()> {
    match cli.command {
        None => handle_no_command(&cli).await,
        Some(Commands::Load(args)) => handle_load(ctx, args).await,
        Some(Commands::ScanPorts(args)) => handle_scan_ports(ctx, args).await,
        Some(Commands::ScanEndpoints(args)) => handle_scan_endpoints(ctx, args).await,
        Some(Commands::Fingerprint(args)) => handle_fingerprint(ctx, args).await,
        #[cfg(feature = "nse")]
        Some(Commands::Nse(args)) => handle_nse(ctx, args).await,
        #[cfg(not(feature = "nse"))]
        Some(Commands::Nse(_)) => anyhow::bail!("NSE support requires the 'nse' feature"),
        Some(Commands::Fuzz(args)) => handle_fuzz(ctx, args).await,
        Some(Commands::WafStress(args)) => handle_waf_stress(ctx, args).await,
        Some(Commands::Waf(args)) => handle_waf(ctx, args).await,
        Some(Commands::Scan(args)) => handle_scan(ctx, args).await,
        Some(Commands::Resume(args)) => handle_resume(args).await,
        Some(Commands::Recon(args)) => handle_recon(ctx, args).await,
        Some(Commands::Plan(args)) => handle_plan(ctx, args).await,
        Some(Commands::Ci(args)) => handle_ci(ctx, args).await,
        Some(Commands::Graphql(args)) => handle_graphql(ctx, args).await,
        Some(Commands::OAuth(args)) => handle_oauth(ctx, args).await,
        Some(Commands::AuthTest(args)) => handle_auth_test(ctx, args).await,
        #[cfg(feature = "sbom")]
        Some(Commands::Sbom(args)) => handle_sbom(ctx, args).await,
        Some(Commands::Packet(args)) => handle_packet(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Icmp(args)) => handle_icmp(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Traceroute(args)) => handle_traceroute(ctx, args).await,
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        Some(Commands::Plugin(args)) => handle_plugin(ctx, args).await,
        #[cfg(not(any(feature = "python-plugins", feature = "ruby-plugins")))]
        Some(Commands::Plugin(_)) => anyhow::bail!("Plugin support requires the 'python-plugins' or 'ruby-plugins' feature"),
        Some(Commands::Report(args)) => handle_report(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Stress(args)) => handle_stress(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Proxy(args)) => handle_proxy(ctx, args).await,
        Some(Commands::Cluster(args)) => handle_cluster(ctx, args).await,
        Some(Commands::Notify(args)) => handle_notify(ctx, args).await,
        Some(Commands::Remote(args)) => handle_remote(ctx, args).await,
        Some(Commands::Exec(args)) => handle_exec(ctx, args).await,
        #[cfg(feature = "rest-api")]
        Some(Commands::Serve(args)) => handle_serve(ctx, args).await,
        #[cfg(feature = "rest-api")]
        Some(Commands::McpServe(args)) => handle_mcp_serve(ctx, args).await,
        #[cfg(feature = "rest-api")]
        Some(Commands::Agent(args)) => handle_agent(ctx, args).await,
        #[cfg(feature = "ai-integration")]
        Some(Commands::AiAnalyze(args)) => handle_ai_analyze(args).await,
    }
}

async fn handle_no_command(cli: &Cli) -> Result<()> {
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        crate::tui::run(cli.config.clone())?;
    } else {
        println!("No command specified and not running in interactive terminal.");
        println!("Available commands:");
        println!("  slapper load <url>          - Run HTTP load test");
        println!("  slapper scan-ports <host>   - Scan ports");
        println!("  slapper scan-endpoints <url> - Discover endpoints");
        println!("  slapper fuzz <url>          - Fuzz target");
        println!("  slapper recon <target>      - Reconnaissance");
        println!("  slapper --help             - Show all commands");
        println!("\nTo launch TUI, run from an interactive terminal.");
    }
    Ok(())
}
