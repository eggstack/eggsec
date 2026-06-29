use crate::cli::preflight::PreflightArgs;
use crate::commands::handlers::CommandContext;
use crate::config::{metadata_for_tool_id, preflight_operation};
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

    let result = preflight_operation(
        ctx.execution_surface,
        &ctx.enforcement,
        descriptor,
        Some(&ctx.manual_override),
    );

    if args.json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", result.to_human_readable());
    }

    Ok(())
}
