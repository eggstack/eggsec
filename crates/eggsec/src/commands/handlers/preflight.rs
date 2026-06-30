use crate::cli::preflight::{PreflightArgs, PreflightProfile};
use crate::commands::handlers::CommandContext;
use crate::config::{
    metadata_for_tool_id, preflight_operation, EnforcementContext, ExecutionSurface,
};
use crate::domain::domain_descriptor_by_id;
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
        // Enrich JSON output with domain metadata if available.
        let mut value = serde_json::to_value(&result)?;
        if let Some(domain) = domain_descriptor_by_id(&args.operation) {
            if let Some(obj) = value.as_object_mut() {
                obj.insert(
                    "domain".to_string(),
                    serde_json::json!({
                        "id": domain.id,
                        "display_name": domain.display_name,
                        "description": domain.description,
                        "category": domain.category.to_string(),
                    }),
                );
            }
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
    } else {
        let mut output = result.to_human_readable();
        // Append domain metadata to human-readable output if available.
        if let Some(domain) = domain_descriptor_by_id(&args.operation) {
            output.push_str(&format!(
                "\nDomain: {} ({})\nDescription: {}",
                domain.display_name, domain.category, domain.description
            ));
        }
        println!("{}", output);
    }

    Ok(())
}
