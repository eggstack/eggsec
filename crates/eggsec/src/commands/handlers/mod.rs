pub mod auth_test;
#[cfg(feature = "headless-browser")]
pub mod browser;
pub mod ci;
pub mod cluster;
pub mod config;
pub mod doctor;
pub mod explain;
pub mod fuzz;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
pub mod load;
pub mod network;
pub mod notify;
pub mod plan;
pub mod recon;
pub mod report;
pub mod scan;
#[cfg(feature = "rest-api")]
pub mod serve;
pub mod storage;
pub mod stress;
pub mod vuln;
#[cfg(feature = "wireless")]
pub mod wireless;
pub use config::*;
pub use doctor::*;
pub use explain::*;
#[cfg(feature = "rest-api")]
pub mod agent;
#[cfg(feature = "grpc-api")]
pub mod grpc;
#[cfg(feature = "sbom")]
pub mod sbom;

pub use ci::*;
pub use cluster::*;
pub use fuzz::*;
#[cfg(feature = "advanced-hunting")]
pub use hunt::*;
pub use load::*;
pub use network::*;
pub use plan::*;
pub use recon::*;
pub use scan::*;

#[cfg(feature = "rest-api")]
pub use agent::*;
pub use auth_test::*;
#[cfg(feature = "headless-browser")]
pub use browser::*;
pub use notify::*;
pub use report::*;
#[cfg(feature = "sbom")]
pub use sbom::*;
#[cfg(feature = "rest-api")]
pub use serve::*;
pub use storage::*;
#[cfg(feature = "stress-testing")]
pub use stress::*;
pub use vuln::*;
#[cfg(feature = "wireless")]
pub use wireless::*;

#[cfg(feature = "grpc-api")]
pub use grpc::*;

#[cfg(feature = "ai-integration")]
pub mod ai_analyze;

#[cfg(feature = "ai-integration")]
pub use ai_analyze::*;

use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::OperationDescriptor;
use crate::config::evaluate_operation_policy;
use crate::config::{Scope, EggsecConfig};
use crate::error::Result as ErrorResult;
use anyhow::Result;

pub struct CommandContext {
    pub config: EggsecConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
    pub notify_manager: crate::notify::NotifyManager,
}

impl CommandContext {
    pub fn new(config: EggsecConfig, scope: Scope, json: bool) -> Self {
        let notify_manager = crate::notify::NotifyManager::from_settings(&config);
        Self {
            config,
            scope,
            json,
            config_path: None,
            notify_manager,
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

    /// Evaluate an operation against the current execution policy and scope.
    ///
    /// Wraps the shared [`evaluate_operation_policy`] evaluator. Returns the
    /// [`PolicyDecision`] on allow, or an error with denial details on deny.
    pub fn evaluate_and_enforce_operation(
        &self,
        descriptor: OperationDescriptor,
    ) -> Result<crate::config::PolicyDecision> {
        let decision = evaluate_operation_policy(
            &descriptor,
            &self.config.execution_policy,
            Some(&self.scope),
        );

        if !decision.allowed {
            if self.json {
                let json = serde_json::to_string(&decision)
                    .unwrap_or_else(|_| "unable to serialize decision".to_string());
                anyhow::bail!("{}", json);
            } else {
                anyhow::bail!("{}", decision.to_human_readable());
            }
        }

        Ok(decision)
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
        #[cfg(feature = "advanced-hunting")]
        Some(Commands::Hunt(args)) => handle_hunt(ctx, args).await,
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
        Some(Commands::PolicyExplain(args)) => handle_policy_explain(ctx, args).await,
        Some(Commands::ScopeExplain(args)) => handle_scope_explain(ctx, args).await,
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
        #[cfg(feature = "wireless")]
        Some(Commands::Wireless(args)) => handle_wireless(ctx, args).await,
        #[cfg(feature = "headless-browser")]
        Some(Commands::Browser(args)) => handle_browser(ctx, args).await,
        #[cfg(feature = "grpc-api")]
        Some(Commands::Grpc(args)) => handle_grpc_server(ctx, args).await,
        Some(Commands::Vuln(args)) => handle_vuln(ctx, args).await,
        Some(Commands::Storage(args)) => handle_storage(ctx, args).await,
    }
}

async fn handle_no_command(cli: &Cli) -> Result<()> {
    if std::io::IsTerminal::is_terminal(&std::io::stdout()) {
        // TUI launch is handled by the binary via eggsec-tui.
        // This path should not be reached when using the binary.
        anyhow::bail!(
            "TUI launch requested but eggsec-tui is not available. \
             Run from the eggsec binary or install eggsec-tui."
        );
    } else {
        println!("No command specified and not running in interactive terminal.");
        println!("Run 'eggsec --help' for available commands.");
        println!("\nTo launch TUI, run from an interactive terminal.");
    }
    Ok(())
}
