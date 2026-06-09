use anyhow::Result;

use crate::cli::{PolicyExplainArgs, ScopeExplainArgs};
use crate::commands::handlers::CommandContext;
use crate::config::{
    evaluate_operation_policy, load_scope, IntendedUse, OperationDescriptor, OperationMode,
    OperationRisk, PolicyDecision,
};

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

    let descriptor = OperationDescriptor {
        operation: "scope-explain".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Passive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: args.target.clone(),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
    };

    let policy = crate::config::ExecutionPolicy::default();
    let decision = evaluate_operation_policy(&descriptor, &policy, scope.as_ref());

    if args.json || ctx.json {
        println!("{}", serde_json::to_string_pretty(&decision)?);
    } else {
        println!("{}", decision.to_human_readable());
    }

    Ok(())
}
