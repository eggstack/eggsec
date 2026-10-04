use anyhow::Result;

use crate::cli::{PolicyExplainArgs, ScopeExplainArgs};
use crate::commands::handlers::CommandContext;
use crate::config::{
    evaluate_operation_policy, load_scope, IntendedUse, OperationDescriptor, OperationMode,
    OperationRisk,
};

/// Load an operator-supplied scope file, failing closed.
///
/// A scope file that cannot be read must never degrade to "no scope": these
/// commands exist to answer whether a target is authorized, so a typo in
/// `--scope` previously flipped `policy-explain` from DENIED to ALLOWED with
/// no diagnostic. Mirrors the global scope load in `main`, which propagates
/// its error.
fn load_explain_scope(path: Option<&str>) -> Result<Option<crate::config::Scope>> {
    match path {
        Some(p) => {
            let scope = load_scope(Some(p))
                .map_err(|e| anyhow::anyhow!("Failed to load scope file '{}': {}", p, e))?;
            Ok(Some(scope))
        }
        None => Ok(None),
    }
}

pub async fn handle_policy_explain(ctx: &CommandContext, args: PolicyExplainArgs) -> Result<()> {
    let scope = load_explain_scope(args.scope.as_deref())?;
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
    let scope = load_explain_scope(args.scope.as_deref())?;

    // HelperOnly: scope-explain is a read-only diagnostic, no OperationMetadata (Phase C non-goal)
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
