pub mod scan;
pub mod fuzz;
pub mod load;
pub mod cluster;
pub mod recon;
pub mod network;
pub mod plugin;
pub mod report;
pub mod stress;
pub mod notify;

pub use scan::*;
pub use fuzz::*;
pub use load::*;
pub use cluster::*;
pub use recon::*;
pub use network::*;
#[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
pub use plugin::*;
pub use report::*;
#[cfg(feature = "stress-testing")]
pub use stress::*;
pub use notify::*;

use anyhow::Result;
use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::{SlapperConfig, Scope};

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

    pub fn ensure_scope_url(&self, url: &str) -> Result<()> {
        crate::utils::check_scope_from_url(&self.scope, url)
    }

    pub fn ensure_scope(&self, target: &str) -> Result<()> {
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
        Some(Commands::Fuzz(args)) => handle_fuzz(ctx, args).await,
        Some(Commands::WafStress(args)) => handle_waf_stress(ctx, args).await,
        Some(Commands::Waf(args)) => handle_waf(ctx, args).await,
        Some(Commands::Scan(args)) => handle_scan(ctx, args).await,
        Some(Commands::Resume(args)) => handle_resume(args).await,
        Some(Commands::Recon(args)) => handle_recon(ctx, args).await,
        Some(Commands::Graphql(args)) => handle_graphql(ctx, args).await,
        Some(Commands::OAuth(args)) => handle_oauth(ctx, args).await,
        Some(Commands::Packet(args)) => handle_packet(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Icmp(args)) => handle_icmp(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Traceroute(args)) => handle_traceroute(ctx, args).await,
        #[cfg(any(feature = "python-plugins", feature = "ruby-plugins"))]
        Some(Commands::Plugin(args)) => handle_plugin(ctx, args).await,
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
        #[cfg(feature = "mcp-server")]
        Some(Commands::McpServe(args)) => handle_mcp_serve(ctx, args).await,
    }
}
