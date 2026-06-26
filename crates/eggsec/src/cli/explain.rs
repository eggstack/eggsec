use clap::Args;
use std::str::FromStr;

use crate::config::{evaluate_operation_policy, OperationDescriptor, PolicyDecision};

#[derive(Args, Clone)]
pub struct PolicyExplainArgs {
    #[arg(long, help = "Target to evaluate (e.g., http://127.0.0.1:8080)")]
    pub target: Option<String>,

    #[arg(long, help = "Scan profile to evaluate")]
    pub profile: Option<String>,

    #[arg(long, help = "Scope file path")]
    pub scope: Option<String>,

    #[arg(long, help = "Output in JSON format")]
    pub json: bool,
}

#[derive(Args, Clone)]
pub struct ScopeExplainArgs {
    #[arg(long, help = "Target to evaluate (e.g., 10.0.0.5 or example.com)")]
    pub target: Option<String>,

    #[arg(long, help = "Scope file path")]
    pub scope: Option<String>,

    #[arg(long, help = "Output in JSON format")]
    pub json: bool,
}

pub fn evaluate_policy_decision(
    target: Option<&str>,
    profile_name: Option<&str>,
    scope: Option<&crate::config::Scope>,
    policy: &crate::config::ExecutionPolicy,
) -> PolicyDecision {
    use crate::cli::ScanProfile;

    let profile = profile_name
        .and_then(|s| ScanProfile::from_str(s).ok())
        .unwrap_or(ScanProfile::Quick);

    let mode = profile.operation_mode();
    let risk = profile.max_risk_budget().to_operation_risk();
    let intended_uses = profile.intended_uses();

    let mut required_features = Vec::new();
    if profile.requires_packet_inspection() {
        required_features.push("packet-inspection".to_string());
    }
    if profile.requires_nse() {
        required_features.push("nse".to_string());
    }

    let mut required_policy_flags = Vec::new();
    if profile.requires_private_scope() && target.is_some() {
        required_policy_flags.push("require_explicit_scope".to_string());
    }

    let descriptor = OperationDescriptor {
        operation: "policy-explain".to_string(),
        mode,
        risk,
        intended_uses,
        target: target.map(|s| s.to_string()),
        required_features,
        required_policy_flags,
        requires_private_or_local_target: profile.requires_private_scope(),
        requires_explicit_scope: profile.requires_private_scope(),
        required_capabilities: Vec::new(),
    };

    evaluate_operation_policy(&descriptor, policy, scope)
}
