use crate::cli::preflight::{PreflightArgs, PreflightProfile};
use crate::commands::handlers::CommandContext;
use crate::config::{
    metadata_for_tool_id, preflight_operation, EnforcementContext, ExecutionSurface,
};
use anyhow::Result;

pub async fn handle_preflight(ctx: &CommandContext, args: PreflightArgs) -> Result<()> {
    let metadata = metadata_for_tool_id(&args.operation).ok_or_else(|| {
        anyhow::anyhow!(
            "Unknown operation '{}'. Use 'eggsec policy-explain' for profile-based analysis, \
             or check available tool IDs with the REST API.",
            args.operation
        )
    })?;

    let descriptor = metadata.descriptor_for_target(args.target.clone());

    // If a profile override is specified, build a fresh EnforcementContext for that surface.
    // This lets users simulate CI-strict, MCP, agent, etc. from the CLI preflight command.
    let (surface, enforcement) = match args.profile {
        Some(profile) => {
            let surface = match profile {
                PreflightProfile::Manual => ExecutionSurface::CliManual,
                PreflightProfile::Ci => ExecutionSurface::Ci,
                PreflightProfile::Mcp => ExecutionSurface::McpServer,
                PreflightProfile::Agent => ExecutionSurface::SecurityAgent,
                PreflightProfile::Guarded => ExecutionSurface::CliManualStrict,
            };
            let scope = ctx.enforcement.loaded_scope.clone();
            let enforcement = EnforcementContext::for_surface(
                surface,
                ctx.config.execution_policy.clone(),
                scope,
            );
            (surface, enforcement)
        }
        None => (ctx.execution_surface, ctx.enforcement.clone()),
    };

    let manual_override = if surface.honors_manual_override() {
        Some(&ctx.manual_override)
    } else {
        None
    };

    let result = preflight_operation(surface, &enforcement, descriptor, manual_override);

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.to_human_readable());
    }

    Ok(())
}
