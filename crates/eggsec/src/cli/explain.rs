use clap::Args;

use crate::config::PolicyDecision;

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
) -> PolicyDecision {
    use crate::cli::ScanProfile;

    let profile = profile_name
        .and_then(|p| ScanProfile::from_str(p))
        .unwrap_or(ScanProfile::Quick);

    let mode = profile.operation_mode();
    let risk = profile.max_risk_budget().to_operation_risk();
    let intended_uses = profile.intended_uses();

    let mut decision = PolicyDecision::allowed("policy-explain", mode, risk, intended_uses);

    if let Some(target_str) = target {
        decision = decision.with_target(target_str, target_str);

        if let Some(scope) = scope {
            match scope.is_target_allowed(target_str) {
                Ok(true) => {
                    decision
                        .matched_scope_rules
                        .push("target in scope".to_string());
                }
                Ok(false) => {
                    decision
                        .denied_reasons
                        .push("target not in scope".to_string());
                    decision.allowed = false;
                }
                Err(e) => {
                    decision
                        .warnings
                        .push(format!("scope check error: {}", e));
                }
            }
        } else if crate::config::is_private_ip(
            &target_str
                .parse()
                .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)),
        ) {
            decision.warnings.push(
                "target is a private IP; scope file recommended for defense-lab profiles"
                    .to_string(),
            );
        }
    }

    if profile.requires_private_scope() && target.is_some() {
        decision
            .required_policy_flags
            .push("require_explicit_scope".to_string());
    }

    if profile.requires_packet_inspection() {
        decision
            .required_features
            .push("packet-inspection".to_string());
    }

    if profile.requires_nse() {
        decision.required_features.push("nse".to_string());
    }

    decision
}
