pub mod auth_test;
#[cfg(feature = "headless-browser")]
pub mod browser;
#[cfg(feature = "c2")]
pub mod c2;
pub mod ci;
pub mod cluster;
pub mod config;
#[cfg(feature = "db-pentest")]
pub mod db_pentest;
pub mod doctor;
#[cfg(feature = "evasion")]
pub mod evasion;
pub mod explain;
pub mod fuzz;
#[cfg(feature = "advanced-hunting")]
pub mod hunt;
pub mod load;
#[cfg(feature = "mobile")]
pub mod mobile;
pub mod network;
pub mod notify;
pub mod plan;
#[cfg(feature = "postex")]
pub mod postex;
pub mod preflight;
pub mod recon;
pub mod report;
pub mod scan;
#[cfg(feature = "rest-api")]
pub mod serve;
pub mod storage;
pub mod stress;
pub mod vuln;
#[cfg(feature = "web-proxy")]
pub mod web_proxy;
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
pub use preflight::*;
pub use recon::*;
pub use scan::*;

#[cfg(feature = "rest-api")]
pub use agent::*;
pub use auth_test::*;
#[cfg(feature = "headless-browser")]
pub use browser::*;
#[cfg(feature = "c2")]
pub use c2::*;
#[cfg(feature = "db-pentest")]
pub use db_pentest::*;
#[cfg(feature = "evasion")]
pub use evasion::*;
#[cfg(feature = "mobile")]
pub use mobile::*;
pub use notify::*;
#[cfg(feature = "postex")]
pub use postex::*;
pub use report::*;
#[cfg(feature = "sbom")]
pub use sbom::*;
#[cfg(feature = "rest-api")]
pub use serve::*;
pub use storage::*;
#[cfg(feature = "stress-testing")]
pub use stress::*;
pub use vuln::*;
#[cfg(feature = "web-proxy")]
pub use web_proxy::*;
#[cfg(feature = "wireless")]
pub use wireless::*;

#[cfg(feature = "grpc-api")]
pub use grpc::*;

#[cfg(feature = "ai-integration")]
pub mod ai_analyze;

#[cfg(feature = "ai-integration")]
pub use ai_analyze::*;

use crate::audit::{audit_event_from_enforcement_outcome, emit_audit_event};
use crate::cli::Cli;
use crate::cli::Commands;
use crate::config::OperationDescriptor;
use crate::config::{EggsecConfig, ExecutionProfile, ExecutionSurface, Scope};
use crate::error::Result as ErrorResult;
use anyhow::Result;

pub struct CommandContext {
    pub config: EggsecConfig,
    pub scope: Scope,
    pub json: bool,
    config_path: Option<String>,
    pub notify_manager: crate::notify::NotifyManager,
    pub execution_profile: ExecutionProfile,
    /// Origin of this execution request. Derives `execution_profile` and
    /// determines whether manual overrides are honored.
    pub execution_surface: ExecutionSurface,
    pub enforcement: crate::config::EnforcementContext,
    /// Manual-only override flags. Honored exclusively for ManualPermissive.
    /// Strict profiles, CI, MCP, and agent ignore or reject overrides.
    pub manual_override: crate::config::ManualOverride,
}

impl CommandContext {
    pub fn new(config: EggsecConfig, scope: Scope, json: bool) -> Self {
        let notify_manager = crate::notify::NotifyManager::from_settings(&config);
        // Use explicit provenance when the provided scope has rules (simulates --scope / config file).
        // Tests that pass an empty/default scope will still get DefaultEmpty.
        let source = if scope.allowed_targets.is_empty() && !scope.require_explicit_scope {
            crate::config::ScopeSource::DefaultEmpty
        } else {
            crate::config::ScopeSource::CliScopeFile
        };
        let loaded_scope = crate::config::LoadedScope::explicit(scope.clone(), source, None);
        let execution_surface = ExecutionSurface::CliManual;
        let enforcement = crate::config::EnforcementContext::for_surface(
            execution_surface,
            config.execution_policy.clone(),
            loaded_scope,
        );
        Self {
            config,
            scope,
            json,
            config_path: None,
            notify_manager,
            execution_profile: execution_surface.profile(),
            execution_surface,
            enforcement,
            manual_override: crate::config::ManualOverride::default(),
        }
    }

    pub fn with_config_path(mut self, path: Option<String>) -> Self {
        self.config_path = path;
        self
    }

    /// Set execution profile directly. Transitional — prefer [`Self::with_execution_surface`]
    /// for new call sites so that the surface-to-profile derivation is centralized.
    #[deprecated(
        note = "Transitional — prefer with_execution_surface() for new call sites so that surface-to-profile derivation is centralized. Test-only usage."
    )]
    pub fn with_execution_profile(mut self, profile: ExecutionProfile) -> Self {
        self.execution_profile = profile;
        self.enforcement = match profile {
            ExecutionProfile::ManualPermissive => {
                crate::config::EnforcementContext::manual_permissive(
                    self.config.execution_policy.clone(),
                    self.enforcement.loaded_scope.clone(),
                )
            }
            ExecutionProfile::ManualGuarded => crate::config::EnforcementContext::manual_guarded(
                self.config.execution_policy.clone(),
                self.enforcement.loaded_scope.clone(),
            ),
            ExecutionProfile::CiStrict => crate::config::EnforcementContext::ci_strict(
                self.config.execution_policy.clone(),
                self.enforcement.loaded_scope.clone(),
            ),
            ExecutionProfile::McpStrict => crate::config::EnforcementContext::mcp_strict(
                self.config.execution_policy.clone(),
                self.enforcement.loaded_scope.clone(),
            ),
            ExecutionProfile::AgentStrict => crate::config::EnforcementContext::agent_strict(
                self.config.execution_policy.clone(),
                self.enforcement.loaded_scope.clone(),
            ),
        };
        self
    }

    /// Set execution surface, deriving both the profile and enforcement context
    /// from the surface. This is the preferred way to configure entrypoint semantics.
    pub fn with_execution_surface(mut self, surface: ExecutionSurface) -> Self {
        self.execution_surface = surface;
        self.execution_profile = surface.profile();
        self.enforcement = crate::config::EnforcementContext::for_surface(
            surface,
            self.config.execution_policy.clone(),
            self.enforcement.loaded_scope.clone(),
        );
        self
    }

    pub fn config_path(&self) -> Option<&str> {
        self.config_path.as_deref()
    }

    pub fn with_loaded_scope(mut self, loaded_scope: crate::config::LoadedScope) -> Self {
        self.scope = loaded_scope.scope.clone();
        self.enforcement = crate::config::EnforcementContext::for_surface(
            self.execution_surface,
            self.config.execution_policy.clone(),
            loaded_scope,
        );
        self
    }

    /// Attach manual override flags. Only effective for ManualPermissive.
    pub fn with_manual_override(mut self, manual_override: crate::config::ManualOverride) -> Self {
        self.manual_override = manual_override;
        self
    }

    /// Build an `OperationDescriptor` from the command registry for the given
    /// command ID and target. Returns `None` if the command has no registry
    /// entry with operation metadata (config/helper/server commands).
    ///
    /// This is the preferred way to build descriptors for registry-backed
    /// commands. It delegates to `OperationMetadata::descriptor_for_target()`
    /// to ensure consistency with the canonical metadata source.
    pub fn describe_from_registry(
        &self,
        command_id: &str,
        target: Option<String>,
    ) -> Option<OperationDescriptor> {
        crate::commands::registry::build_descriptor_for_command(command_id, target)
    }

    #[deprecated(
        note = "Unused legacy helper. Scope checks are centralized in EnforcementContext::evaluate()."
    )]
    pub fn ensure_scope_url(&self, url: &str) -> ErrorResult<()> {
        crate::utils::check_scope_from_url(&self.scope, url)
    }

    #[deprecated(
        note = "Unused legacy helper. Scope checks are centralized in EnforcementContext::evaluate()."
    )]
    pub fn ensure_scope(&self, target: &str) -> ErrorResult<()> {
        crate::utils::check_scope(&self.scope, target)
    }

    /// Evaluate an operation against the current execution policy and scope.
    ///
    /// Wraps the shared [`evaluate_operation_policy`] evaluator with
    /// profile-aware enforcement via [`evaluate_enforcement`]. Returns the
    /// [`PolicyDecision`] on allow, or an error with denial details on deny.
    ///
    /// `RequireConfirmation` (produced only under ManualPermissive for operator-discretion
    /// cases) is converted to proceed only if matching manual override flags are present.
    /// Strict profiles, CI, MCP, and agent paths never proceed on `RequireConfirmation`.
    pub fn evaluate_and_enforce_operation(
        &self,
        descriptor: OperationDescriptor,
    ) -> Result<crate::config::PolicyDecision> {
        let outcome = self.enforcement.evaluate(&descriptor);

        match &outcome {
            crate::config::EnforcementOutcome::Allow(decision) => {
                let event = audit_event_from_enforcement_outcome(
                    self.execution_surface,
                    &self.enforcement,
                    &descriptor,
                    &outcome,
                    false,
                    false,
                    None,
                    &[],
                    None,
                    None,
                );
                emit_audit_event(&event);
                Ok(decision.clone())
            }
            crate::config::EnforcementOutcome::Warn(decision) => {
                let event = audit_event_from_enforcement_outcome(
                    self.execution_surface,
                    &self.enforcement,
                    &descriptor,
                    &outcome,
                    false,
                    false,
                    None,
                    &[],
                    None,
                    None,
                );
                emit_audit_event(&event);
                for warning in &decision.warnings {
                    tracing::warn!(warning = %warning, "Policy warning");
                }
                Ok(decision.clone())
            }
            crate::config::EnforcementOutcome::RequireConfirmation(decision) => {
                if self.execution_profile != ExecutionProfile::ManualPermissive {
                    // Automated / guarded profiles: treat confirmation as hard denial
                    if self.json {
                        let json = serde_json::to_string(decision)
                            .unwrap_or_else(|_| "unable to serialize decision".to_string());
                        anyhow::bail!("{}", json);
                    } else {
                        anyhow::bail!("{}", decision.to_human_readable());
                    }
                }
                // Compute required confirmation classes from the decision
                let required: Vec<crate::config::ConfirmationClass> =
                    crate::config::confirmation_classes_for(
                        &descriptor,
                        decision,
                        &self.config.execution_policy,
                    );
                let permitted = required.iter().all(|c| self.manual_override.permits(*c));
                if permitted {
                    // Audit the override (stable kebab-case class strings, deduped, deterministic order)
                    let classes_vec: Vec<String> =
                        crate::config::confirmation_class_strings(&required);
                    tracing::warn!(
                        operation = %decision.operation,
                        target = ?decision.target_original,
                        classes = ?classes_vec,
                        reason = ?self.manual_override.reason,
                        "manual enforcement override accepted"
                    );
                    let mut out = decision.clone();
                    if !out.manual_override_used {
                        out = out.with_manual_override_record(
                            self.manual_override.reason.clone(),
                            classes_vec,
                        );
                    }
                    let event = audit_event_from_enforcement_outcome(
                        self.execution_surface,
                        &self.enforcement,
                        &descriptor,
                        &outcome,
                        true,
                        false,
                        Some(&self.manual_override),
                        &required,
                        None,
                        None,
                    );
                    emit_audit_event(&event);
                    Ok(out)
                } else {
                    // Explain exactly which flags are needed (dedicated flags for private/redirect)
                    let needed: Vec<&str> = required
                        .iter()
                        .filter_map(|c| match c {
                            crate::config::ConfirmationClass::OutOfScope => {
                                if !self.manual_override.allow_out_of_scope {
                                    Some("--allow-out-of-scope")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::ExplicitExclusion => {
                                if !self.manual_override.allow_explicit_exclusion {
                                    Some("--allow-excluded-target")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::HighRisk => {
                                if !self.manual_override.allow_high_risk {
                                    Some("--allow-high-risk")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::NonBaselineCapability => {
                                if !self.manual_override.allow_nonbaseline_capability {
                                    Some("--allow-nonbaseline-capability")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::PrivateResolution => {
                                if !self.manual_override.allow_private_resolution {
                                    Some("--allow-private-resolution")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::CrossHostRedirect => {
                                if !self.manual_override.allow_cross_host_redirect {
                                    Some("--allow-cross-host-redirect")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::TargetExpansion => {
                                if !self.manual_override.allow_out_of_scope {
                                    Some("--allow-out-of-scope")
                                } else {
                                    None
                                }
                            }
                            crate::config::ConfirmationClass::TrafficInterception => {
                                if !self.manual_override.allow_web_proxy {
                                    Some("--allow-web-proxy")
                                } else {
                                    None
                                }
                            }
                        })
                        .collect();
                    let classes_list = classes_str(&required);
                    let msg = if needed.is_empty() {
                        if self.manual_override.assume_yes {
                            format!(
                                "manual confirmation required for: {}. --yes alone does not permit these classes. Re-run with the appropriate --allow-* flag(s) and optionally --manual-override-reason",
                                classes_list
                            )
                        } else {
                            "manual confirmation required; re-run with --yes or the appropriate --allow-* flag(s) and optionally --manual-override-reason".to_string()
                        }
                    } else {
                        let base = format!(
                            "manual confirmation required for: {}. Re-run with {} (and optionally --manual-override-reason)",
                            classes_list,
                            needed.join(" ")
                        );
                        if self.manual_override.assume_yes
                            && required.iter().any(|c| {
                                !matches!(
                                    *c,
                                    crate::config::ConfirmationClass::OutOfScope
                                        | crate::config::ConfirmationClass::TargetExpansion
                                )
                            })
                        {
                            format!("{}. --yes alone does not permit these classes.", base)
                        } else {
                            base
                        }
                    };
                    let event = audit_event_from_enforcement_outcome(
                        self.execution_surface,
                        &self.enforcement,
                        &descriptor,
                        &outcome,
                        false,
                        false,
                        None,
                        &required,
                        None,
                        None,
                    );
                    emit_audit_event(&event);
                    anyhow::bail!("{}", msg);
                }
            }
            crate::config::EnforcementOutcome::Deny(decision) => {
                let event = audit_event_from_enforcement_outcome(
                    self.execution_surface,
                    &self.enforcement,
                    &descriptor,
                    &outcome,
                    false,
                    false,
                    None,
                    &[],
                    None,
                    None,
                );
                emit_audit_event(&event);
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

fn classes_str(classes: &[crate::config::ConfirmationClass]) -> String {
    classes
        .iter()
        .map(|c| c.as_str().to_string())
        .collect::<Vec<_>>()
        .join(", ")
}

pub async fn handle_command(cli: Cli, ctx: &CommandContext) -> Result<()> {
    // ── Dispatch bridge: validate registry metadata before execution ──
    //
    // For commands backed by the registry (those with an operation_id),
    // verify that the registry entry resolves to valid OperationMetadata
    // before dispatching to the handler. This ensures registry metadata
    // stays consistent with the canonical source and catches stale entries
    // at runtime.
    //
    // Non-registry commands (config, helper, server) skip this check.
    // Enforcement is NOT performed here — it remains in each handler via
    // `evaluate_and_enforce_operation()`.
    if let Some(ref command) = cli.command {
        let command_id = command.command_id();
        if let Some(registry_entry) = crate::commands::registry::lookup_command(command_id) {
            if let Some(op_id) = registry_entry.operation_id {
                // Validate registry metadata resolves to OperationMetadata.
                // A failure here means the registry entry is stale.
                let metadata_valid = crate::config::metadata_for_tool_id(op_id).is_some();
                if !metadata_valid {
                    tracing::warn!(
                        command_id,
                        op_id,
                        "Command registry entry has stale operation_id — \
                         metadata not found. Falling back to handler."
                    );
                } else {
                    tracing::trace!(
                        command_id,
                        op_id,
                        category = registry_entry.category.as_str(),
                        "Dispatch bridge: registry entry validated"
                    );
                }
            }
        }
    }

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
        Some(Commands::Preflight(args)) => handle_preflight(ctx, args).await,
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
        #[cfg(feature = "web-proxy")]
        Some(Commands::ProxyIntercept(args)) => web_proxy::handle_proxy_intercept(ctx, args).await,
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
        #[cfg(feature = "evasion")]
        Some(Commands::Evasion(args)) => handle_evasion(ctx, args).await,
        #[cfg(feature = "postex")]
        Some(Commands::Postex(args)) => handle_postex(ctx, args).await,
        #[cfg(feature = "c2")]
        Some(Commands::C2(args)) => handle_c2(ctx, args).await,
        #[cfg(feature = "mobile")]
        Some(Commands::Mobile(args)) => handle_mobile(ctx, args).await,
        #[cfg(feature = "db-pentest")]
        Some(Commands::Db(args)) => handle_db_pentest(ctx, args).await,
        #[cfg(feature = "headless-browser")]
        Some(Commands::Browser(args)) => handle_browser(ctx, args).await,
        #[cfg(feature = "grpc-api")]
        Some(Commands::Grpc(args)) => handle_grpc_server(ctx, args).await,
        Some(Commands::Vuln(args)) => handle_vuln(ctx, args).await,
        Some(Commands::Storage(args)) => handle_storage(ctx, args).await,
        // Daemon client commands are dispatched from main.rs before reaching handle_command.
        // These arms satisfy the exhaustive match but should never be reached.
        #[cfg(feature = "daemon-client")]
        Some(Commands::Daemon(_)) | Some(Commands::Session(_)) | Some(Commands::Task(_)) => {
            anyhow::bail!(
                "daemon client commands are handled by the CLI binary, not the command handler"
            )
        }
    }
}

async fn handle_no_command(_cli: &Cli) -> Result<()> {
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
        Capability, ExecutionPolicy, ExecutionProfile, IntendedUse, OperationDescriptor,
        OperationMode, OperationRisk, Scope, ScopeRule,
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
        // Under ManualPermissive (default), high-risk with policy flag still requires operator confirmation.
        // Provide matching override to proceed (per 2026-06-10 manual discretion model).
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
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
        // High-risk under permissive requires confirmation.
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
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
        // ManualPermissive high-risk requires confirmation; attach override.
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
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
        // High-risk under permissive requires confirmation.
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
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
        // High-risk under permissive requires confirmation.
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
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
        // ManualPermissive: out-of-scope with positive rules now yields RequireConfirmation
        // (not immediate hard error). The test asserts an error path for the default (no-override) case.
        let err = ctx.evaluate_and_enforce_operation(desc).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("confirmation required")
                || msg.contains("--allow-out-of-scope")
                || msg.contains("DENIED")
        );
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
        // High-risk (credential) under permissive requires confirmation.
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
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

    #[test]
    fn command_context_enforcement_context_is_built() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        assert_eq!(
            ctx.enforcement.execution_profile,
            ExecutionProfile::ManualPermissive
        );
    }

    #[test]
    fn command_context_with_loaded_scope_updates_enforcement() {
        use crate::config::{LoadedScope, ScopeSource};
        let loaded = LoadedScope::explicit(
            localhost_scope(),
            ScopeSource::CliScopeFile,
            Some("/path/to/scope.toml".to_string()),
        );
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_loaded_scope(loaded);
        assert!(ctx.enforcement.loaded_scope.is_explicit_manifest());
        assert_eq!(
            ctx.enforcement.loaded_scope.source,
            ScopeSource::CliScopeFile
        );
    }

    #[test]
    fn command_context_with_mcp_strict_profile_builds_mcp_enforcement() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_execution_profile(ExecutionProfile::McpStrict);
        assert_eq!(
            ctx.enforcement.execution_profile,
            ExecutionProfile::McpStrict
        );
    }

    // --- 2026-06-10 manual discretion ergonomics focused tests (CommandContext) ---

    #[test]
    fn assume_yes_permits_out_of_scope_and_target_expansion_but_not_high_risk_or_exclusion() {
        // --yes (assume_yes) permits OutOfScope/TargetExpansion only.
        let mo = crate::config::ManualOverride {
            assume_yes: true,
            ..Default::default()
        };
        assert!(mo.permits(crate::config::ConfirmationClass::OutOfScope));
        assert!(mo.permits(crate::config::ConfirmationClass::TargetExpansion));
        assert!(!mo.permits(crate::config::ConfirmationClass::HighRisk));
        assert!(!mo.permits(crate::config::ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(crate::config::ConfirmationClass::NonBaselineCapability));
        assert!(!mo.permits(crate::config::ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(crate::config::ConfirmationClass::CrossHostRedirect));
    }

    #[test]
    fn yes_alone_does_not_permit_high_risk() {
        // To reach RequireConfirmation for HighRisk, the policy must permit the risk tier.
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                assume_yes: true,
                ..Default::default()
            },
        );
        // Intrusive + policy flag => confirmable under permissive; --yes alone does not satisfy HighRisk.
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive));
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("--allow-high-risk"),
            "error should mention dedicated --allow-high-risk, got: {}",
            msg
        );
        assert!(
            msg.contains("--yes alone does not permit these classes"),
            "should note that --yes alone is insufficient for high-risk: {}",
            msg
        );
    }

    #[test]
    fn yes_alone_does_not_permit_explicit_exclusion() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            excluded_targets: vec![ScopeRule::new("93.184.216.34".to_string())],
            ..Default::default()
        };
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false).with_manual_override(
            crate::config::ManualOverride {
                assume_yes: true,
                ..Default::default()
            },
        );
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("--allow-excluded-target"),
            "error should mention --allow-excluded-target, got: {}",
            msg
        );
    }

    #[test]
    fn allow_high_risk_permits_high_risk_without_explicit_exclusion() {
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                ..Default::default()
            },
        );
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive));
        assert!(result.is_ok());
        // Explicit exclusion should still require its own flag.
        let mo = &ctx.manual_override;
        assert!(mo.permits(crate::config::ConfirmationClass::HighRisk));
        assert!(!mo.permits(crate::config::ConfirmationClass::ExplicitExclusion));
    }

    #[test]
    fn allow_excluded_target_permits_explicit_exclusion_without_high_risk() {
        let scope = Scope {
            allowed_targets: vec![ScopeRule::new("*".to_string())],
            excluded_targets: vec![ScopeRule::new("93.184.216.34".to_string())],
            ..Default::default()
        };
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false).with_manual_override(
            crate::config::ManualOverride {
                allow_explicit_exclusion: true,
                ..Default::default()
            },
        );
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_ok());
        let mo = &ctx.manual_override;
        assert!(mo.permits(crate::config::ConfirmationClass::ExplicitExclusion));
        assert!(!mo.permits(crate::config::ConfirmationClass::HighRisk));
    }

    #[test]
    fn allow_nonbaseline_capability_permits_nonbaseline() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_manual_override(crate::config::ManualOverride {
                allow_nonbaseline_capability: true,
                ..Default::default()
            });
        let mut desc = descriptor("fuzz", OperationRisk::SafeActive);
        desc.required_capabilities = vec![Capability::IntrusiveFuzz];
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_ok());
    }

    #[test]
    fn allow_private_resolution_permits_private_resolution_class() {
        let mo = crate::config::ManualOverride {
            allow_private_resolution: true,
            ..Default::default()
        };
        assert!(mo.permits(crate::config::ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(crate::config::ConfirmationClass::CrossHostRedirect));
        // Dedicated; allow_out_of_scope does not cover it.
        let mo2 = crate::config::ManualOverride {
            allow_out_of_scope: true,
            ..Default::default()
        };
        assert!(!mo2.permits(crate::config::ConfirmationClass::PrivateResolution));
    }

    #[test]
    fn allow_cross_host_redirect_permits_cross_host_class() {
        let mo = crate::config::ManualOverride {
            allow_cross_host_redirect: true,
            ..Default::default()
        };
        assert!(mo.permits(crate::config::ConfirmationClass::CrossHostRedirect));
        assert!(!mo.permits(crate::config::ConfirmationClass::PrivateResolution));
        let mo2 = crate::config::ManualOverride {
            allow_out_of_scope: true,
            ..Default::default()
        };
        assert!(!mo2.permits(crate::config::ConfirmationClass::CrossHostRedirect));
    }

    #[test]
    fn allow_out_of_scope_does_not_permit_private_or_cross_host() {
        let mo = crate::config::ManualOverride {
            allow_out_of_scope: true,
            ..Default::default()
        };
        assert!(!mo.permits(crate::config::ConfirmationClass::PrivateResolution));
        assert!(!mo.permits(crate::config::ConfirmationClass::CrossHostRedirect));
        assert!(mo.permits(crate::config::ConfirmationClass::OutOfScope));
    }

    #[test]
    fn command_context_error_messages_list_exact_dedicated_flags() {
        // High-risk missing override
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false);
        let err = ctx
            .evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive))
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("--allow-high-risk"),
            "high-risk error should list --allow-high-risk: {}",
            msg
        );

        // Out of scope missing override
        let ctx2 = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let public_desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let err2 = ctx2
            .evaluate_and_enforce_operation(public_desc)
            .unwrap_err();
        let msg2 = err2.to_string();
        assert!(
            msg2.contains("--allow-out-of-scope"),
            "out-of-scope error should list --allow-out-of-scope: {}",
            msg2
        );

        // Private resolution would list its dedicated flag (covered via permits + classification path; explicit msg test uses reachable classes).
    }

    #[test]
    fn successful_override_records_stable_kebab_case_classes_on_decision_no_debug_no_dups() {
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                allow_high_risk: true,
                assume_yes: true, // extra that should be deduped/not affect high-risk class
                ..Default::default()
            },
        );
        let decision = ctx
            .evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive))
            .expect("override should permit");
        assert!(decision.manual_override_used);
        // Classes must be kebab-case stable strings, deduped, order-preserving first-seen.
        assert!(
            decision
                .manual_override_classes
                .contains(&"high-risk".to_string()),
            "audit classes should contain kebab 'high-risk', got: {:?}",
            decision.manual_override_classes
        );
        // No Debug formatting like "HighRisk"
        assert!(
            !decision
                .manual_override_classes
                .iter()
                .any(|s| s.contains("HighRisk") || s.contains("ConfirmationClass")),
            "must not contain Debug names: {:?}",
            decision.manual_override_classes
        );
        // assume_yes does not add high-risk class; only the required one(s) for this path.
    }

    #[test]
    fn manual_guarded_with_all_overrides_still_denies_require_confirmation() {
        // Use out-of-scope (canonical confirmable under permissive with positive scope rules).
        // Under Guarded the enforcement produces Deny (or RequireConfirmation which CommandContext hard-denies).
        // All overrides attached; they must not be honored.
        let scope = localhost_scope(); // positive rule -> out-of-scope target will be confirmable only for permissive
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false)
            .with_execution_profile(ExecutionProfile::ManualGuarded)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                allow_explicit_exclusion: true,
                allow_high_risk: true,
                allow_nonbaseline_capability: true,
                allow_private_resolution: true,
                allow_cross_host_redirect: true,
                assume_yes: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(
            result.is_err(),
            "ManualGuarded must treat RequireConfirmation as hard deny even with all overrides"
        );
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("DENIED")
                || msg.contains("denied")
                || msg.contains("not allowed")
                || msg.contains("Scope")
                || msg.contains("scope"),
            "should be denial: {}",
            msg
        );
    }

    #[test]
    fn ci_strict_with_all_overrides_still_denies_require_confirmation() {
        let scope = localhost_scope();
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false)
            .with_execution_profile(ExecutionProfile::CiStrict)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                allow_explicit_exclusion: true,
                allow_high_risk: true,
                allow_nonbaseline_capability: true,
                allow_private_resolution: true,
                allow_cross_host_redirect: true,
                assume_yes: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(
            result.is_err(),
            "CiStrict must treat RequireConfirmation as hard deny even with all overrides"
        );
    }

    #[test]
    fn mcp_strict_via_command_context_ignores_overrides_and_denies_require_confirmation() {
        let scope = localhost_scope();
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false)
            .with_execution_profile(ExecutionProfile::McpStrict)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                assume_yes: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(
            result.is_err(),
            "McpStrict must deny (no override path) even with matching flags"
        );
        // Negative: does not surface confirmation, surfaces denial.
        let msg = result.unwrap_err().to_string();
        assert!(
            !msg.contains("confirmation required"),
            "strict should not mention confirmation: {}",
            msg
        );
    }

    #[test]
    fn agent_strict_via_command_context_ignores_overrides_and_denies_require_confirmation() {
        let scope = localhost_scope();
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false)
            .with_execution_profile(ExecutionProfile::AgentStrict)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                assume_yes: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(
            result.is_err(),
            "AgentStrict must deny (no override path) even with matching flags"
        );
    }

    #[test]
    fn out_of_scope_with_allow_out_of_scope_succeeds_and_records_kebab_class() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = ctx
            .evaluate_and_enforce_operation(desc)
            .expect("allow-out-of-scope should permit");
        assert!(decision.manual_override_used);
        assert!(
            decision
                .manual_override_classes
                .contains(&"out-of-scope".to_string())
                || decision
                    .manual_override_classes
                    .contains(&"target-expansion".to_string()),
            "should record out-of-scope or target-expansion kebab class: {:?}",
            decision.manual_override_classes
        );
    }

    #[test]
    fn mobile_static_safe_active_allowed_by_default() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let result = ctx
            .evaluate_and_enforce_operation(descriptor("mobile-static", OperationRisk::SafeActive));
        assert!(result.is_ok());
    }

    #[test]
    fn manual_permissive_high_risk_with_both_yes_and_allow_flag_succeeds() {
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                assume_yes: true,
                allow_high_risk: true,
                ..Default::default()
            },
        );
        let result =
            ctx.evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive));
        assert!(result.is_ok());
        let decision = result.unwrap();
        assert!(decision.manual_override_used);
        assert!(
            decision
                .manual_override_classes
                .contains(&"high-risk".to_string()),
            "audit classes should contain high-risk: {:?}",
            decision.manual_override_classes
        );
    }

    #[test]
    fn manual_permissive_out_of_scope_no_override_error_suggests_allow_flag() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false);
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let err = ctx.evaluate_and_enforce_operation(desc).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("--allow-out-of-scope"),
            "error should suggest --allow-out-of-scope: {}",
            msg
        );
    }

    #[test]
    fn manual_permissive_high_risk_no_override_error_explains_yes_insufficient() {
        let policy = ExecutionPolicy {
            allow_intrusive_fuzzing: true,
            ..Default::default()
        };
        let ctx = make_ctx(policy, localhost_scope(), false).with_manual_override(
            crate::config::ManualOverride {
                assume_yes: true,
                ..Default::default()
            },
        );
        let err = ctx
            .evaluate_and_enforce_operation(descriptor("fuzz", OperationRisk::Intrusive))
            .unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("--yes alone does not permit"),
            "error should explain --yes is insufficient: {}",
            msg
        );
        assert!(
            msg.contains("--allow-high-risk"),
            "error should suggest --allow-high-risk: {}",
            msg
        );
    }

    #[test]
    fn manual_guarded_all_overrides_cannot_override_scope_miss() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_execution_profile(ExecutionProfile::ManualGuarded)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                allow_high_risk: true,
                assume_yes: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(
            result.is_err(),
            "ManualGuarded must deny regardless of overrides"
        );
    }

    #[test]
    fn agent_strict_overrides_ignored_and_deny_message_unconditional() {
        let scope = localhost_scope();
        let ctx = make_ctx(ExecutionPolicy::default(), scope, false)
            .with_execution_profile(ExecutionProfile::AgentStrict)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                allow_high_risk: true,
                allow_private_resolution: true,
                allow_cross_host_redirect: true,
                assume_yes: true,
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let result = ctx.evaluate_and_enforce_operation(desc);
        assert!(result.is_err(), "AgentStrict must deny unconditionally");
        let msg = result.unwrap_err().to_string();
        assert!(
            !msg.contains("--allow-"),
            "AgentStrict denial should not suggest manual override flags: {}",
            msg
        );
    }

    #[test]
    fn successful_out_of_scope_override_records_audit_fields() {
        let ctx = make_ctx(ExecutionPolicy::default(), localhost_scope(), false)
            .with_manual_override(crate::config::ManualOverride {
                allow_out_of_scope: true,
                reason: Some("authorized pentest".to_string()),
                ..Default::default()
            });
        let desc = OperationDescriptor {
            operation: "scan".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::SafeActive,
            intended_uses: vec![IntendedUse::WebAssessment],
            target: Some("93.184.216.34".to_string()),
            required_features: Vec::new(),
            required_policy_flags: Vec::new(),
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: Vec::new(),
        };
        let decision = ctx
            .evaluate_and_enforce_operation(desc)
            .expect("override should permit");
        assert!(decision.manual_override_used);
        assert_eq!(
            decision.manual_override_reason.as_deref(),
            Some("authorized pentest")
        );
        assert!(
            !decision.manual_override_classes.is_empty(),
            "audit classes should be non-empty"
        );
        assert!(
            decision
                .manual_override_classes
                .contains(&"out-of-scope".to_string())
                || decision
                    .manual_override_classes
                    .contains(&"target-expansion".to_string()),
            "should record the appropriate class: {:?}",
            decision.manual_override_classes
        );
    }
}
