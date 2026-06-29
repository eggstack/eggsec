use crate::commands::handlers::CommandContext;
use crate::config::{LoadedScope, Scope, ScopeSource};
use anyhow::Result;

/// Resolve the loaded scope for REST API execution.
///
/// Scope precedence for REST:
/// - If `--scope-file` is provided to `serve`, that file takes precedence
///   (REST-specific scope override).
/// - Otherwise, the loaded top-level scope from the CLI/context is inherited.
///
/// This matches the CLI shape where `eggsec --scope global.toml serve --scope-file rest.toml`
/// uses the REST-specific `scope_file`.
fn resolve_rest_loaded_scope(
    ctx: &CommandContext,
    args: &crate::cli::ServeArgs,
) -> Result<LoadedScope> {
    if let Some(ref scope_file) = args.scope_file {
        let scope = Scope::from_file(scope_file)?;
        Ok(LoadedScope {
            scope,
            source: ScopeSource::CliScopeFile,
            path: Some(scope_file.to_string()),
        })
    } else {
        Ok(ctx.enforcement.loaded_scope.clone())
    }
}

#[cfg(feature = "rest-api")]
pub async fn handle_serve(ctx: &CommandContext, args: crate::cli::ServeArgs) -> Result<()> {
    use crate::config::{EnforcementContext, ExecutionSurface};
    use crate::distributed::TlsConfig;
    use crate::tool::{create_default_registry, protocol::rest::create_router};
    use axum::serve;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use tokio::net::TcpListener;

    let loaded_scope = resolve_rest_loaded_scope(ctx, &args)?;

    let enforcement = EnforcementContext::for_surface(
        ExecutionSurface::RestApi,
        ctx.config.execution_policy.clone(),
        loaded_scope,
    );

    let tls_config = match (&args.tls_cert, &args.tls_key) {
        (Some(ref cert), Some(ref key)) => Some(TlsConfig {
            cert_path: PathBuf::from(cert),
            key_path: PathBuf::from(key),
        }),
        _ => None,
    };

    let registry = create_default_registry();
    let router = create_router(
        registry,
        args.api_key.clone(),
        enforcement,
        tls_config.clone(),
    );

    let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
        .parse()
        .map_err(|e| anyhow::anyhow!("Invalid address {}:{} - {}", args.bind, args.port, e))?;

    if tls_config.is_some() {
        tracing::info!("Starting HTTPS server on {}", addr);
    } else {
        tracing::info!("Starting HTTP server on {}", addr);
    }

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    let make_service = router.into_make_service();
    serve(listener, make_service)
        .await
        .map_err(|e| anyhow::anyhow!("Server error: {}", e))?;

    Ok(())
}

#[cfg(feature = "rest-api")]
pub async fn handle_mcp_serve(ctx: &CommandContext, args: crate::cli::McpServeArgs) -> Result<()> {
    use crate::tool::create_default_registry;
    use crate::tool::protocol::mcp::{create_mcp_router, run_stdio, McpProfile};
    use axum::serve;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    let registry = create_default_registry();

    let profile = match args.profile.as_str() {
        "coding-agent" => McpProfile::CodingAgent,
        _ => McpProfile::OpsAgent,
    };

    let enforcement = crate::config::EnforcementContext::for_surface(
        crate::config::ExecutionSurface::McpServer,
        ctx.config.execution_policy.clone(),
        ctx.enforcement.loaded_scope.clone(),
    );

    if args.stdio {
        tracing::info!(
            "Starting MCP server in STDIO mode (profile: {})",
            args.profile
        );
        run_stdio(registry, args.api_key, profile, enforcement).await;
        Ok(())
    } else {
        let router = create_mcp_router(registry, args.api_key.clone(), profile, enforcement).await;

        let addr: SocketAddr = format!("{}:{}", args.bind, args.port)
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid address {}:{} - {}", args.bind, args.port, e))?;

        tracing::info!("Starting MCP server on {}", addr);

        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

        serve(listener, router)
            .await
            .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

        Ok(())
    }
}
