use super::CommandContext;
use crate::cli::GrpcServerArgs;
use crate::error::EggsecError;
use crate::tool::protocol::grpc::start_grpc_server;
use crate::tool::protocol::grpc::GrpcService;
use crate::tool::ToolDispatcher;
use crate::tool::ToolRegistry;
use tracing::info;

#[cfg(feature = "grpc-api")]
pub async fn handle_grpc_server(ctx: &CommandContext, args: GrpcServerArgs) -> anyhow::Result<()> {
    ctx.ensure_scope(&args.host)?;
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
