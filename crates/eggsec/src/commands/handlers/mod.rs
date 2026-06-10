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
use crate::config::{EggsecConfig, ExecutionProfile, Scope};
use crate::error::Result as ErrorResult;
use anyhow::Result;

pub struct CommandContext {
    pub config: EggsecConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
    pub notify_manager: crate::notify::NotifyManager,
    pub execution_profile: ExecutionProfile,
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
            execution_profile: ExecutionProfile::ManualPermissive,
        }
    }

    pub fn with_config_path(mut self, path: Option<String>) -> Self {
        self.config_path = path;
        self
    }

    pub fn with_execution_profile(mut self, profile: ExecutionProfile) -> Self {
        self.execution_profile = profile;
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
    /// Wraps the shared [`evaluate_operation_policy`] evaluator with
    /// profile-aware enforcement via [`evaluate_enforcement`]. Returns the
    /// [`PolicyDecision`] on allow, or an error with denial details on deny.
    pub fn evaluate_and_enforce_operation(
        &self,
        descriptor: OperationDescriptor,
    ) -> Result<crate::config::PolicyDecision> {
        let outcome = crate::config::evaluate_enforcement(
            &descriptor,
            &self.config.execution_policy,
            Some(&self.scope),
            self.execution_profile,
        );

        match &outcome {
            crate::config::EnforcementOutcome::Allow(decision) => Ok(decision.clone()),
            crate::config::EnforcementOutcome::Warn(decision) => {
                for warning in &decision.warnings {
                    tracing::warn!(warning = %warning, "Policy warning");
                }
                Ok(decision.clone())
            }
            crate::config::EnforcementOutcome::Deny(decision) => {
                if self.json {
                    let json = serde_json::to_string(decision)
                        .unwrap_or_else(|_| "unable to serialize decision".to_string());
                    anyhow::bail!("{}", json);
                } else {
                    anyhow::bail!("{}", decision.to_human_readable());
                }
            }
        }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        ExecutionPolicy, IntendedUse, OperationDescriptor, OperationMode, OperationRisk, Scope,
    };

    fn make_ctx(policy: ExecutionPolicy, scope: Scope, json: bool) -> CommandContext {
        let config = EggsecConfig {
            execution_policy: policy,
            ..Default::default()
        };
        CommandContext::new(config, scope, json)
    }

    fn localhost_scope() -> Scope {
        Scope {
            allowed_targets: vec![crate::config::ScopeRule::new("127.0.0.1".to_string())],
            ..Default::default()
        }
    }

    fn descriptor(operation: &str, risk: OperationRisk) -> OperationDescriptor {
        OperationDescriptor {
            operation: operation.to_string(),
            mode: OperationMode::StandardAssessment,
            risk,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        }
    }

    #[test]
    fn safe_active_allowed_by_default() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("scan", OperationRisk::SafeActive));
        assert!(result.is_ok());
    }

    #[test]
    fn intrusive_denied_by_default() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive));
        assert!(result.is_err());
    }

    #[test]
    fn intrusive_allowed_when_enabled() {
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive));
        assert!(result.is_ok());
    }

    #[test]
    fn stress_test_denied_without_policy_flag() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("stress", OperationRisk::StressTest));
        assert!(result.is_err());
    }

    #[test]
    fn stress_test_allowed_with_policy_flag() {
        let policy = ExecutionPolicy {
            allow_stress_testing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("stress", OperationRisk::StressTest));
        assert!(result.is_ok());
    }

    #[test]
    fn raw_packet_denied_without_policy_flag() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("packet", OperationRisk::RawPacket));
        assert!(result.is_err());
    }

    #[test]
    fn raw_packet_allowed_with_policy_flag() {
        let policy = ExecutionPolicy {
            allow_raw_packets: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("packet", OperationRisk::RawPacket));
        assert!(result.is_ok());
    }

    #[test]
    fn load_test_denied_without_policy_flag() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("load", OperationRisk::LoadTest));
        assert!(result.is_err());
    }

    #[test]
    fn load_test_allowed_with_policy_flag() {
        let policy = ExecutionPolicy {
            allow_load_testing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("load", OperationRisk::LoadTest));
        assert!(result.is_ok());
    }

    #[test]
    fn remote_execution_denied_by_default() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("exec", OperationRisk::RemoteExecution));
        assert!(result.is_err());
    }

    #[test]
    fn remote_execution_allowed_with_policy_flag() {
        let policy = ExecutionPolicy {
            allow_remote_execution: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("exec", OperationRisk::RemoteExecution));
        assert!(result.is_ok());
    }

    #[test]
    fn json_mode_denial_includes_structured_data() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), true);
        let err = ctx
            .evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive))
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("\"allowed\":false"),
            "denial should contain allowed:false: {}",
            msg
        );
        assert!(
            msg.contains("\"operation_risk\""),
            "denial should contain operation_risk: {}",
            msg
        );
        assert!(
            msg.contains("\"denied_reasons\""),
            "denial should contain denied_reasons: {}",
            msg
        );
        assert!(
            msg.contains("\"decision_id\""),
            "denial should contain decision_id: {}",
            msg
        );
    }

    #[test]
    fn human_mode_denial_is_readable() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let err = ctx
            .evaluate_and_enforce_operation(descriptor("stress", OperationRisk::StressTest))
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("DENIED"),
            "human denial should contain DENIED: {}",
            msg
        );
    }

    #[test]
    fn denied_public_target_out_of_scope() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_err());
    }

    #[test]
    fn allowed_target_passes_scope_check() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_ok());
    }

    #[test]
    fn exploit_adjacent_denied_by_default() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("proxy", OperationRisk::ExploitAdjacent));
        assert!(result.is_err());
    }

    #[test]
    fn credential_testing_denied_by_default() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result = ctx.evaluate_and_enforce_operation(descriptor(
            "auth-test",
            OperationRisk::CredentialTesting,
        ));
        assert!(result.is_err());
    }

    #[test]
    fn credential_testing_allowed_with_policy_flag() {
        let policy = ExecutionPolicy {
            allow_credential_testing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let result = ctx.evaluate_and_enforce_operation(descriptor(
            "auth-test",
            OperationRisk::CredentialTesting,
        ));
        assert!(result.is_ok());
    }

    #[test]
    fn manual_permissive_execution_profile() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        assert_eq!(ctx.execution_profile, ExecutionProfile::ManualPermissive);
    }

    #[test]
    fn with_execution_profile_sets_profile() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_execution_profile(ExecutionProfile::McpStrict);
        assert_eq!(ctx.execution_profile, ExecutionProfile::McpStrict);
    }

    #[test]
    fn mcp_strict_denies_requires_explicit_scope_without_scope() {
        let ctx = make_ctx(ExecutionPolicy::default(), Scope::default(), false)
            .with_execution_profile(ExecutionProfile::McpStrict);
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_err());
    }

    #[test]
    fn agent_strict_denies_requires_explicit_scope_without_scope() {
        let ctx = make_ctx(ExecutionPolicy::default(), Scope::default(), false)
            .with_execution_profile(ExecutionProfile::AgentStrict);
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("127.0.0.1".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: true,
            required_capabilities: Vec::new(),
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_err());
    }
}
