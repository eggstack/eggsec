use super::CommandContext;
use crate::cli::GrpcServerArgs;
use crate::config::OperationDescriptor;
use crate::error::EggsecError;
use crate::tool::protocol::grpc::start_grpc_server;
use crate::tool::protocol::grpc::GrpcService;
use crate::tool::ToolDispatcher;
use crate::tool::ToolRegistry;
use tracing::info;

#[cfg(feature = "grpc-api")]
pub async fn handle_grpc_server(ctx: &CommandContext, args: GrpcServerArgs) -> anyhow::Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor {
        operation: "grpc-server".to_string(),
        mode: crate::config::OperationMode::StandardAssessment,
        risk: crate::config::OperationRisk::SafeActive,
        intended_uses: vec![crate::config::IntendedUse::WebAssessment],
        target: Some(args.host.clone()),
        required_features: vec!["grpc-api".to_string()],
        required_policy_flags: Vec::new(),
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: Vec::new(),
    })?;
    info!("Starting gRPC server on {}:{}", args.host, args.port);

    let registry = ToolRegistry::new();
    let service = GrpcService::new(registry.clone(), args.api_key);

    start_grpc_server(&args.host, args.port, service)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

#[cfg(not(feature = "grpc-api"))]
pub async fn handle_grpc_server(_ctx: &CommandContext, _args: GrpcServerArgs) -> Result<()> {
    Err(crate::error::EggsecError::Config(
        "gRPC API is not enabled. Compile with --features grpc-api".to_string(),
    ))
}
