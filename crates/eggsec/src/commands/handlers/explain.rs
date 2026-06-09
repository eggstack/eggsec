use anyhow::Result;

use crate::cli::{PolicyExplainArgs, ScopeExplainArgs};
use crate::commands::handlers::CommandContext;
use crate::config::{load_scope, OperationMode, OperationRisk, IntendedUse, PolicyDecision};

pub async fn handle_policy_explain(ctx: &CommandContext, args: PolicyExplainArgs) -> Result<()> {
    let scope = args
        .scope
        .as_deref()
        .and_then(|s| load_scope(Some(s)).ok());
    let decision = crate::cli::explain::evaluate_policy_decision(
        args.target.as_deref(),
        args.profile.as_deref(),
        scope.as_ref(),
    );

    if args.json || ctx.json {
        println!("{}", serde_json::to_string_pretty(&decision)?);
    } else {
        println!("{}", decision.to_human_readable());
    }

    Ok(())
}

pub async fn handle_scope_explain(ctx: &CommandContext, args: ScopeExplainArgs) -> Result<()> {
    let scope = args
        .scope
        .as_deref()
        .and_then(|s| load_scope(Some(s)).ok());

    let mut decision = PolicyDecision::allowed(
        "scope-explain",
        OperationMode::StandardAssessment,
        OperationRisk::Passive,
        vec![IntendedUse::WebAssessment],
    );

    if let Some(ref target) = args.target {
        decision = decision.with_target(target, target);

        if let Some(ref scope) = scope {
            match scope.is_target_allowed(target) {
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
        } else {
            decision
                .warnings
                .push("no scope file provided; using default scope rules".to_string());
            if crate::config::is_private_ip(
                &target
                    .parse()
                    .unwrap_or(std::net::IpAddr::V4(std::net::Ipv4Addr::UNSPECIFIED)),
            ) {
                decision
                    .warnings
                    .push("target is a private IP address".to_string());
            }
        }
    } else {
        decision
            .warnings
            .push("no target specified".to_string());
    }

    if args.json || ctx.json {
        println!("{}", serde_json::to_string_pretty(&decision)?);
    } else {
        println!("{}", decision.to_human_readable());
    }

    Ok(())
}
