use crate::cli::GrpcServerArgs;
use crate::error::SlapperError;
use crate::tool::ToolRegistry;
use crate::tool::protocol::grpc::GrpcService;
use crate::tool::protocol::grpc::start_grpc_server;
use crate::tool::ToolDispatcher;
use tracing::info;

#[cfg(feature = "grpc-api")]
pub async fn handle_grpc_server(args: GrpcServerArgs) -> anyhow::Result<()> {
    info!("Starting gRPC server on {}:{}", args.host, args.port);

    let registry = ToolRegistry::new();
    let service = GrpcService::new(registry.clone(), args.api_key);

    start_grpc_server(&args.host, args.port, service)
        .await
        .map_err(|e| anyhow::anyhow!(e))?;

    Ok(())
}

#[cfg(not(feature = "grpc-api"))]
pub async fn handle_grpc_server(_args: GrpcServerArgs) -> Result<()> {
    Err(crate::error::SlapperError::Config(
        "gRPC API is not enabled. Compile with --features grpc-api".to_string(),
    ))
}
