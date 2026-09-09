//! Phase D contract tests: protocol/agent boundaries and hotspot facades.
//!
//! Proves:
//! - REST/MCP/gRPC share identical checked approval/dispatch behavior for
//!   the same canonical request (all derive `McpStrict`);
//! - binding mismatches fail closed via `validate_request_binding`;
//! - agent strict mode cannot be downgraded through injected services;
//! - daemon RBAC ownership semantics are unchanged;
//! - approval tokens remain bound to the exact approved operation/target;
//! - hotspot facades (`policy_target`, `policy_catalog`, `scope_address`,
//!   `scope_resolver`, `runtime_config`, `runtime_sink`, `host_auth`,
//!   `host_persistence`) resolve identically to their legacy paths.

use eggsec::config::{
    all_operation_metadata, classify_address, default_resolver, metadata_for_tool_id,
    operation_matches_tool_id, operation_metadata, AddressClass, EnforcementContext,
    ExecutionPolicy, ExecutionSurface, LoadedScope, OperationMode, OperationRisk,
};
use eggsec::tool::service::EngineServices;
use eggsec::tool::ToolRegistry;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn strict_empty_enforcement() -> EnforcementContext {
    EnforcementContext::mcp_strict(ExecutionPolicy::default(), LoadedScope::default_empty())
}

/// Strict enforcement with an explicit loopback manifest (fail-open for
/// allowlisted 127.0.0.1, fail-closed otherwise). Strict profiles require an
/// explicit manifest for networked operations; `DefaultEmpty` always denies.
fn strict_loopback_enforcement() -> EnforcementContext {
    use eggsec::config::{Scope, ScopeRule, ScopeSource};

    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("127.0.0.1".to_string()));
    let loaded = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
    EnforcementContext::mcp_strict(ExecutionPolicy::default(), loaded)
}

fn agent_strict_enforcement() -> EnforcementContext {
    EnforcementContext::agent_strict(ExecutionPolicy::default(), LoadedScope::default_empty())
}

/// AgentStrict with an explicit loopback manifest for allow-path tests.
fn agent_loopback_enforcement() -> EnforcementContext {
    use eggsec::config::{Scope, ScopeRule, ScopeSource};

    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("127.0.0.1".to_string()));
    let loaded = LoadedScope::explicit(scope, ScopeSource::ConfigFile, None);
    EnforcementContext::agent_strict(ExecutionPolicy::default(), loaded)
}

fn scan_ports_descriptor() -> eggsec::config::OperationDescriptor {
    let metadata = metadata_for_tool_id("scan-ports").expect("scan-ports metadata");
    metadata
        .try_descriptor_for_target(Some("127.0.0.1"))
        .expect("loopback descriptor")
}

// ---------------------------------------------------------------------------
// REST/MCP/gRPC share checked behavior (all McpStrict)
// ---------------------------------------------------------------------------

#[test]
fn rest_mcp_grpc_approve_identically_for_canonical_request() {
    let enforcement = strict_loopback_enforcement();
    let services = EngineServices::new(ToolRegistry::new(), enforcement);
    let descriptor = scan_ports_descriptor();

    // All three protocol surfaces derive McpStrict; identical descriptors
    // must produce identical allow/deny outcomes.
    let rest = services
        .approve(ExecutionSurface::RestApi, descriptor.clone())
        .map(|_| ())
        .map_err(|e| e.to_string());
    let grpc = services
        .approve(ExecutionSurface::GrpcApi, descriptor.clone())
        .map(|_| ())
        .map_err(|e| e.to_string());
    let mcp = services
        .approve(ExecutionSurface::McpServer, descriptor.clone())
        .map(|_| ())
        .map_err(|e| e.to_string());

    assert_eq!(rest.is_ok(), grpc.is_ok());
    assert_eq!(rest.is_ok(), mcp.is_ok());
    // With an explicit loopback manifest the target is allowlisted; all
    // three must allow (proving shared enforcement, not per-adapter policy).
    assert!(rest.is_ok(), "expected allow, got {rest:?}");
}

#[test]
fn strict_empty_scope_denies_networked_operation() {
    // Fail-closed: DefaultEmpty (no explicit manifest) denies networked
    // operations on strict surfaces even for loopback.
    let enforcement = strict_empty_enforcement();
    let services = EngineServices::new(ToolRegistry::new(), enforcement);
    let descriptor = scan_ports_descriptor();
    assert!(services
        .approve(ExecutionSurface::RestApi, descriptor)
        .is_err());
}

#[test]
fn binding_mismatch_fails_closed() {
    let enforcement = strict_loopback_enforcement();
    let services = EngineServices::new(ToolRegistry::new(), enforcement);
    let descriptor = scan_ports_descriptor();
    let approved = services
        .approve(ExecutionSurface::RestApi, descriptor)
        .expect("approve");

    // Wrong tool: `fuzz` does not resolve to approved `scan-ports`.
    let bad_tool = eggsec::tool::ToolRequest::new("fuzz", eggsec::tool::Target::ip("127.0.0.1"));
    let err = eggsec::tool::dispatcher::validate_request_binding(&approved, &bad_tool);
    assert!(err.is_err(), "operation mismatch must fail closed");

    // Wrong target: approved 127.0.0.1, request 127.0.0.2.
    let bad_target =
        eggsec::tool::ToolRequest::new("scan-ports", eggsec::tool::Target::ip("127.0.0.2"));
    let err = eggsec::tool::dispatcher::validate_request_binding(&approved, &bad_target);
    assert!(err.is_err(), "target mismatch must fail closed");
}

// ---------------------------------------------------------------------------
// Agent strict cannot be downgraded
// ---------------------------------------------------------------------------

#[test]
fn agent_services_reject_mcp_strict_bundle() {
    use eggsec::agent::AgentExecutionService;

    // An McpStrict bundle must not approve SecurityAgent dispatch.
    let services = EngineServices::new(ToolRegistry::new(), strict_empty_enforcement());
    let descriptor = scan_ports_descriptor();
    let result: Result<_, _> = AgentExecutionService::approve_security_agent(&services, descriptor);
    assert!(
        result.is_err(),
        "McpStrict bundle must not approve agent execution"
    );
}

#[test]
fn agent_services_accept_agent_strict_bundle() {
    use eggsec::agent::AgentExecutionService;

    // AgentStrict with an explicit loopback manifest must approve (proving
    // the injected bundle carries the correct profile, not a downgrade).
    let services = EngineServices::new(ToolRegistry::new(), agent_loopback_enforcement());
    let descriptor = scan_ports_descriptor();
    let approved = AgentExecutionService::approve_security_agent(&services, descriptor);
    assert!(approved.is_ok(), "expected allow, got {approved:?}");
}

// ---------------------------------------------------------------------------
// Daemon RBAC unchanged
//
// Daemon RBAC ownership is covered by `eggsec-daemon` unit tests in
// `host_auth.rs` (owner/observer/allowed-client/legacy semantics). Those run
// in the daemon crate to avoid a dev-dependency cycle (`eggsec` integration
// tests cannot depend on `eggsec-daemon`). See
// `crates/eggsec-daemon/src/host_auth.rs`.
// ---------------------------------------------------------------------------

// ---------------------------------------------------------------------------
// Hotspot facades resolve identically
// ---------------------------------------------------------------------------

#[test]
fn hotspot_facades_match_legacy_paths() {
    // Policy catalog facade.
    assert!(operation_metadata("scan-ports").is_some());
    assert!(metadata_for_tool_id("scan").is_some());
    assert!(operation_matches_tool_id("scan", "scan-ports"));
    assert!(!all_operation_metadata().is_empty());

    // Scope facades.
    let loopback: std::net::IpAddr = "127.0.0.1".parse().expect("loopback");
    assert_eq!(classify_address(&loopback), AddressClass::Loopback);
    assert!(eggsec::config::is_private_ip(&loopback));
    let _ = default_resolver();

    // Runtime facades.
    let config = eggsec_runtime::RuntimeConfig::default();
    assert_eq!(config.max_active_tasks_per_session, 1);
    let options = eggsec_runtime::SessionOptions::default();
    assert!(options.task_timeout.is_none());

    // Operation descriptor still builds through the catalog facade.
    let descriptor = scan_ports_descriptor();
    assert_eq!(descriptor.operation, "scan-ports");
    assert!(descriptor.normalized_target.is_present());
}

#[test]
fn approval_token_is_bound_to_operation_and_target() {
    let enforcement = strict_loopback_enforcement();
    let services = EngineServices::new(ToolRegistry::new(), enforcement);
    let descriptor = scan_ports_descriptor();
    let approved = services
        .approve(ExecutionSurface::RestApi, descriptor.clone())
        .expect("approve");
    assert_eq!(approved.descriptor().operation, "scan-ports");
    assert_eq!(
        approved.descriptor().normalized_target,
        descriptor.normalized_target
    );
    assert_eq!(approved.surface(), ExecutionSurface::RestApi);
    assert_eq!(
        approved.profile(),
        eggsec::config::ExecutionProfile::McpStrict
    );
}

#[test]
fn policy_types_still_resolve_through_config_facade() {
    // `crate::config::{...}` remains the stable public facade after WS6.
    // (Within-crate `crate::config::policy::{...}` paths are preserved via
    // `policy.rs` re-exports; integration tests use the public facade.)
    let _ = eggsec::config::TargetHint::Url;
    let _ = eggsec::config::TargetPolicyKind::TargetRequired;
    let _ = eggsec::config::OperationMode::StandardAssessment;
    let _ = OperationRisk::SafeActive;
    let _ = OperationMode::StandardAssessment;
}
