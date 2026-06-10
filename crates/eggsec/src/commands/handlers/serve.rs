use crate::commands::handlers::CommandContext;
use anyhow::Result;

#[cfg(feature = "rest-api")]
pub async fn handle_serve(_ctx: &CommandContext, args: crate::cli::ServeArgs) -> Result<()> {
    use crate::config::Scope;
    use crate::distributed::TlsConfig;
    use crate::tool::{create_default_registry, protocol::rest::create_router};
    use axum::serve;
    use std::net::SocketAddr;
    use std::path::PathBuf;
    use tokio::net::TcpListener;

    let scope = if let Some(ref scope_file) = args.scope_file {
        Some(Scope::from_file(scope_file)?)
    } else {
        None
    };

    let tls_config = match (&args.tls_cert, &args.tls_key) {
        (Some(ref cert), Some(ref key)) => Some(TlsConfig {
            cert_path: PathBuf::from(cert),
            key_path: PathBuf::from(key),
        }),
        _ => None,
    };

    let registry = create_default_registry();
    let router = create_router(registry, args.api_key.clone(), scope, tls_config.clone());

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

    let enforcement = crate::config::EnforcementContext::mcp_strict(
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
