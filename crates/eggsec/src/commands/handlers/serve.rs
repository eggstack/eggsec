use crate::commands::handlers::CommandContext;
use crate::config::{LoadedScope, ScopeSource};
use crate::policy_bridge::resolver::load_scope_from_file;
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
        let scope = load_scope_from_file(scope_file)?;
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
fn parse_mcp_profile(profile: &str) -> Result<crate::tool::protocol::mcp::McpProfile, String> {
    match profile {
        "ops-agent" => Ok(crate::tool::protocol::mcp::McpProfile::OpsAgent),
        "coding-agent" => Ok(crate::tool::protocol::mcp::McpProfile::CodingAgent),
        _ => Err(format!(
            "Unsupported MCP profile '{}'; expected 'ops-agent' or 'coding-agent'",
            profile
        )),
    }
}

#[cfg(feature = "rest-api")]
fn validate_mcp_tool_selection(
    registry: &crate::tool::ToolRegistry,
    profile: crate::tool::protocol::mcp::McpProfile,
    selected: &[String],
) -> Result<(), String> {
    if selected.is_empty() {
        return Err("MCP requires at least one explicit --allow-tool selection".to_string());
    }

    let available_ids: Vec<String> = registry.list().into_iter().map(|tool| tool.id).collect();
    let exposed_operation_ids: Vec<&str> =
        crate::tool::registration::mcp_tool_registrations(profile.as_str())
            .into_iter()
            .map(|registration| registration.operation_id)
            .collect();

    for tool_id in selected {
        let is_registered = available_ids.iter().any(|available| available == tool_id);
        let is_mcp_exposable = crate::config::metadata_for_tool_id(tool_id)
            .is_some_and(|metadata| exposed_operation_ids.contains(&metadata.id));
        if tool_id.trim().is_empty() || !is_registered || !is_mcp_exposable {
            return Err(format!(
                "MCP tool '{}' is not registered and MCP-exposable for profile '{}'",
                tool_id,
                profile.as_str()
            ));
        }
    }
    Ok(())
}

#[cfg(feature = "rest-api")]
fn validate_mcp_bind(
    addr: &std::net::SocketAddr,
    api_key: Option<&str>,
    allow_public: bool,
) -> Result<(), String> {
    if addr.ip().is_loopback() {
        return Ok(());
    }

    if !allow_public {
        return Err(format!(
            "Refusing non-loopback MCP bind at {addr}: pass --allow-public-bind only if you accept the risk of remote task submission and session access; a public bind also requires --api-key."
        ));
    }

    if !api_key.is_some_and(|key| !key.trim().is_empty()) {
        return Err(format!(
            "Refusing non-loopback MCP bind at {addr}: --allow-public-bind requires a non-empty --api-key; public exposure risks remote task submission and session access."
        ));
    }

    tracing::warn!(%addr, "MCP server bound to a non-loopback address");
    Ok(())
}

#[cfg(feature = "rest-api")]
pub async fn handle_mcp_serve(ctx: &CommandContext, args: crate::cli::McpServeArgs) -> Result<()> {
    use crate::tool::create_default_registry;
    use crate::tool::protocol::mcp::{
        create_mcp_router_with_allowed_tools, run_stdio_with_allowed_tools,
    };
    use axum::serve;
    use std::net::SocketAddr;
    use tokio::net::TcpListener;

    let registry = create_default_registry();

    let profile = parse_mcp_profile(&args.profile).map_err(anyhow::Error::msg)?;

    validate_mcp_tool_selection(&registry, profile, &args.allow_tools)
        .map_err(anyhow::Error::msg)?;

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
        run_stdio_with_allowed_tools(
            registry,
            args.api_key,
            profile,
            enforcement,
            args.allow_tools,
        )
        .await;
        return Ok(());
    }

    let bind_ip = args
        .bind
        .parse::<std::net::IpAddr>()
        .map_err(|e| anyhow::anyhow!("Invalid bind address {} - {}", args.bind, e))?;
    let addr = SocketAddr::new(bind_ip, args.port);
    validate_mcp_bind(&addr, args.api_key.as_deref(), args.allow_public_bind)
        .map_err(anyhow::Error::msg)?;

    let router = create_mcp_router_with_allowed_tools(
        registry,
        args.api_key.clone(),
        profile,
        enforcement,
        args.allow_tools,
    )
    .await;

    tracing::info!("Starting MCP server on {}", addr);

    let listener = TcpListener::bind(addr)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to bind to {}: {}", addr, e))?;

    serve(listener, router)
        .await
        .map_err(|e| anyhow::anyhow!("MCP server error: {}", e))?;

    Ok(())
}

#[cfg(all(test, feature = "rest-api"))]
mod mcp_bind_tests {
    use super::{parse_mcp_profile, validate_mcp_bind, validate_mcp_tool_selection};

    #[test]
    fn mcp_bind_admission_matrix() {
        let cases = [
            ("127.0.0.1:8081", false, None, true),
            ("[::1]:8081", false, None, true),
            ("0.0.0.0:8081", false, Some("key"), false),
            ("0.0.0.0:8081", true, None, false),
            ("0.0.0.0:8081", true, Some(""), false),
            ("0.0.0.0:8081", true, Some("   "), false),
            ("0.0.0.0:8081", true, Some("key"), true),
            ("[::]:8081", false, Some("key"), false),
            ("192.168.1.10:8081", true, Some("key"), true),
        ];

        for (address, allow_public, api_key, expected_ok) in cases {
            let addr = address
                .parse::<std::net::SocketAddr>()
                .expect("test address must be valid");
            let result = validate_mcp_bind(&addr, api_key, allow_public);
            assert_eq!(
                result.is_ok(),
                expected_ok,
                "unexpected admission decision for {address}, allow_public={allow_public}, api_key={api_key:?}"
            );
        }
    }

    #[test]
    fn mcp_bind_errors_are_actionable_and_do_not_disclose_key() {
        let addr = "0.0.0.0:8081"
            .parse::<std::net::SocketAddr>()
            .expect("test address must be valid");
        let secret = "test-secret-key";

        let opt_in_error = validate_mcp_bind(&addr, Some(secret), false)
            .expect_err("public bind without opt-in must be rejected");
        assert_eq!(
            opt_in_error,
            format!(
                "Refusing non-loopback MCP bind at {addr}: pass --allow-public-bind only if you accept the risk of remote task submission and session access; a public bind also requires --api-key."
            )
        );
        assert!(!opt_in_error.contains(secret));

        let key_error = validate_mcp_bind(&addr, Some("   "), true)
            .expect_err("whitespace-only key must be rejected");
        assert_eq!(
            key_error,
            format!(
                "Refusing non-loopback MCP bind at {addr}: --allow-public-bind requires a non-empty --api-key; public exposure risks remote task submission and session access."
            )
        );
        assert!(!key_error.contains(secret));
    }

    #[test]
    fn mcp_tool_selection_requires_known_exposable_operator_ids() {
        let registry = crate::tool::create_default_registry();
        let profile = crate::tool::protocol::mcp::McpProfile::OpsAgent;
        assert!(validate_mcp_tool_selection(&registry, profile, &[])
            .unwrap_err()
            .contains("--allow-tool"));
        for runtime_id in ["scan", "load", "scan-ports"] {
            assert!(
                validate_mcp_tool_selection(&registry, profile, &[runtime_id.to_string()]).is_ok(),
                "registered runtime tool {runtime_id} should resolve through canonical MCP metadata"
            );
        }
        for canonical_id in ["pipeline", "load-test"] {
            assert!(
                validate_mcp_tool_selection(&registry, profile, &[canonical_id.to_string()]).is_err(),
                "canonical operation {canonical_id} must not substitute for a selected runtime registry ID"
            );
        }
        assert!(validate_mcp_tool_selection(
            &registry,
            profile,
            &["not-a-registered-tool".to_string()]
        )
        .is_err());
    }

    #[test]
    fn mcp_profile_parser_rejects_unknown_names() {
        use crate::tool::protocol::mcp::McpProfile;
        assert_eq!(
            parse_mcp_profile("ops-agent").unwrap(),
            McpProfile::OpsAgent
        );
        assert_eq!(
            parse_mcp_profile("coding-agent").unwrap(),
            McpProfile::CodingAgent
        );
        assert!(parse_mcp_profile("typo")
            .unwrap_err()
            .contains("Unsupported MCP profile"));
    }
}
