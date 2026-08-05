use super::CommandContext;
use crate::cli::GrpcServerArgs;
use crate::config::{
    EnforcementContext, ExecutionPolicy, ExecutionSurface, LoadedScope, OperationDescriptor,
};
use crate::tool::protocol::grpc::start_grpc_server;
use crate::tool::protocol::grpc::GrpcService;
use crate::tool::ToolRegistry;
use tracing::info;

#[cfg(feature = "grpc-api")]
pub async fn handle_grpc_server(ctx: &CommandContext, args: GrpcServerArgs) -> anyhow::Result<()> {
    ctx.evaluate_and_enforce_operation(OperationDescriptor::new(
        "grpc-server".to_string(),
        crate::config::OperationMode::StandardAssessment,
        crate::config::OperationRisk::SafeActive,
        vec![crate::config::IntendedUse::WebAssessment],
        Some(args.host.clone()),
        vec!["grpc-api".to_string()],
        Vec::new(),
        false,
        false,
        Vec::new(),
    ))?;
    info!("Starting gRPC server on {}:{}", args.host, args.port);

    let registry = ToolRegistry::new();
    let enforcement = EnforcementContext::for_surface(
        ExecutionSurface::GrpcApi,
        ExecutionPolicy::default(),
        LoadedScope::default_empty(),
    );
    let service = GrpcService::new(registry.clone(), enforcement, args.api_key);

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
