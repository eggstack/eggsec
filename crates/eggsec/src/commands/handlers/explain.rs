use anyhow::Result;

use crate::cli::{PolicyExplainArgs, ScopeExplainArgs};
use crate::commands::handlers::CommandContext;
use crate::config::{
    evaluate_operation_policy, load_scope, IntendedUse, OperationDescriptor, OperationMode,
    OperationRisk,
};

pub async fn handle_policy_explain(ctx: &CommandContext, args: PolicyExplainArgs) -> Result<()> {
    let scope = args.scope.as_deref().and_then(|s| {
        load_scope(Some(s))
            .map_err(|e| tracing::debug!("Failed to load scope: {}", e))
            .ok()
    });
    let decision = crate::cli::explain::evaluate_policy_decision(
        args.target.as_deref(),
        args.profile.as_deref(),
        scope.as_ref(),
        &ctx.config.execution_policy,
    );

    if args.json || ctx.json {
        println!("{}", serde_json::to_string_pretty(&decision)?);
    } else {
        println!("{}", decision.to_human_readable());
    }

    Ok(())
}

pub async fn handle_scope_explain(ctx: &CommandContext, args: ScopeExplainArgs) -> Result<()> {
    let scope = args.scope.as_deref().and_then(|s| {
        load_scope(Some(s))
            .map_err(|e| tracing::debug!("Failed to load scope: {}", e))
            .ok()
    });

    let descriptor = OperationDescriptor::new(
        "scope-explain".to_string(),
        OperationMode::StandardAssessment,
        OperationRisk::Passive,
        vec![IntendedUse::WebAssessment],
        args.target.clone(),
        vec![],
        vec![],
        false,
        false,
        Vec::new(),
    );

    let decision =
        evaluate_operation_policy(&descriptor, &ctx.config.execution_policy, scope.as_ref());

    if args.json || ctx.json {
        println!("{}", serde_json::to_string_pretty(&decision)?);
    } else {
        println!("{}", decision.to_human_readable());
    }

    Ok(())
}
