pub mod auth_test;
pub mod ci;
pub mod cluster;
pub mod config;
pub mod doctor;
pub mod fuzz;
pub mod load;
pub mod network;
pub mod notify;
pub mod plan;
pub mod recon;
pub mod report;
pub mod scan;
pub mod storage;
pub mod stress;
pub mod vuln;
pub use config::*;
pub use doctor::*;
#[cfg(feature = "rest-api")]
pub mod agent;
#[cfg(feature = "grpc-api")]
pub mod grpc;
#[cfg(feature = "sbom")]
pub mod sbom;

pub use ci::*;
pub use cluster::*;
pub use fuzz::*;
pub use load::*;
pub use network::*;
pub use plan::*;
pub use recon::*;
pub use scan::*;

#[cfg(feature = "rest-api")]
pub use agent::*;
pub use auth_test::*;
pub use notify::*;
pub use report::*;
#[cfg(feature = "sbom")]
pub use sbom::*;
pub use storage::*;
#[cfg(feature = "stress-testing")]
pub use stress::*;
pub use vuln::*;

#[cfg(feature = "grpc-api")]
pub use grpc::*;

#[cfg(feature = "ai-integration")]
pub mod ai_analyze;

#[cfg(feature = "ai-integration")]
pub use ai_analyze::*;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::OperationRisk;
use crate::config::{Scope, SlapperConfig};
use crate::error::Result as ErrorResult;
use anyhow::Result;

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

    /// Validate that an operation is allowed by the current execution policy.
    ///
    /// Checks scope (if a target is provided), the risk level against the
    /// policy, and blocks high-risk operations in non-interactive mode.
    pub fn enforce_operation_policy(
        &self,
        risk: OperationRisk,
        target: Option<&str>,
    ) -> Result<()> {
        if let Some(target_str) = target {
            self.ensure_scope(target_str)?;
        }

        if !risk.is_allowed_by(&self.config.execution_policy) {
            anyhow::bail!(
                "Operation risk level '{}' is not allowed by current policy. \
                 Enable it in your config file under [execution_policy].",
                risk
            );
        }

        if risk > self.config.execution_policy.max_risk_without_confirm && self.json {
            anyhow::bail!(
                "Operation risk level '{}' exceeds maximum allowed without confirmation \
                 in non-interactive mode.",
                risk
            );
        }

        Ok(())
    }
}

pub async fn handle_command(cli: Cli, ctx: &CommandContext) -> Result<()> {
    match cli.command {
        None => handle_no_command(&cli).await,
        // Keep this match exhaustive: no wildcard arm.
        // This guarantees compile-time sync with `cli::Commands` variants.
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
        Some(Commands::Resume(args)) => handle_resume(ctx, args).await,
        Some(Commands::Recon(args)) => handle_recon(ctx, args).await,
        Some(Commands::Plan(args)) => handle_plan(ctx, args).await,
        Some(Commands::Ci(args)) => handle_ci(ctx, args).await,
        Some(Commands::Config(args)) => handle_config(ctx, args).await,
        Some(Commands::Doctor) => handle_doctor(ctx).await,
        Some(Commands::Graphql(args)) => handle_graphql(ctx, args).await,
        Some(Commands::OAuth(args)) => handle_oauth(ctx, args).await,
        Some(Commands::AuthTest(args)) => handle_auth_test(ctx, args).await,
        #[cfg(feature = "sbom")]
        Some(Commands::Sbom(args)) => handle_sbom(ctx, args).await,
        #[cfg(feature = "packet-inspection")]
        Some(Commands::Packet(args)) => handle_packet(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Icmp(args)) => handle_icmp(ctx, args).await,
        #[cfg(feature = "stress-testing")]
        Some(Commands::Traceroute(args)) => handle_traceroute(ctx, args).await,
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
        Some(Commands::CodeggMcp(args)) => {
            let mcp_args = crate::cli::McpServeArgs {
                port: args.port,
                bind: args.bind,
                api_key: args.api_key,
                stdio: args.stdio,
                profile: args.profile,
            };
            handle_mcp_serve(ctx, mcp_args).await
        }
        #[cfg(feature = "rest-api")]
        Some(Commands::Agent(args)) => handle_agent(ctx, args).await,
        #[cfg(feature = "ai-integration")]
        Some(Commands::AiAnalyze(args)) => handle_ai_analyze(ctx, args).await,
        #[cfg(feature = "grpc-api")]
        Some(Commands::Grpc(args)) => handle_grpc_server(ctx, args).await,
        Some(Commands::Vuln(args)) => handle_vuln(ctx, args).await,
        Some(Commands::Storage(args)) => handle_storage(ctx, args).await,
    }
}

async fn handle_no_command(cli: &Cli) -> Result<()> {
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        crate::tui::run(cli.config.clone())?;
    } else {
        println!("No command specified and not running in interactive terminal.");
        println!("Run 'slapper --help' for available commands.");
        println!("\nTo launch TUI, run from an interactive terminal.");
    }
    Ok(())
}
