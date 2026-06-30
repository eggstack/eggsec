#![cfg(feature = "web-proxy")]

/// Smoke test proving the main crate adapter re-exports domain crate types.
#[test]
fn proxy_types_reexported_from_main_crate() {
    use eggsec::proxy::{ProxyConfig, ProxyEntry, ProxyPool, ProxyType};
    use eggsec::proxy::intercept::types::{ProxyFlow, WebProxySessionReport};
    use eggsec::proxy::intercept::{CertGenerator, InterceptConfig, InterceptMode};

    // Verify adapter re-exports compile and types are accessible
    let _ = std::mem::size_of::<ProxyEntry>();
    let _ = std::mem::size_of::<ProxyConfig>();
    let _ = std::mem::size_of::<ProxyFlow>();
    let _ = std::mem::size_of::<WebProxySessionReport>();
    let _ = std::mem::size_of::<InterceptConfig>();
    let _ = InterceptMode::Monitor;
}

/// Verify intercept submodule re-exports work.
#[test]
fn intercept_types_reexported() {
    use eggsec::proxy::intercept::types::{
        BudgetUsage, FlowAction, InterceptSession, ManipulationRecord, ProxyFlowDirection,
    };
    use eggsec::proxy::intercept::protocols::{
        GrpcMethodType, Http2StreamState, WebSocketOpcode,
    };
    use eggsec::proxy::intercept::correlation::{
        CorrelationContext, CorrelationEngine, CorrelationSource,
    };

    // Verify types are accessible via adapter paths
    let _ = std::mem::size_of::<BudgetUsage>();
    let _ = std::mem::size_of::<ProxyFlowDirection>();
    let _ = std::mem::size_of::<GrpcMethodType>();
    let _ = std::mem::size_of::<Http2StreamState>();
    let _ = std::mem::size_of::<WebSocketOpcode>();
}

/// Verify ProxyManager is accessible from main crate.
#[test]
fn proxy_manager_reexported() {
    use eggsec::proxy::{HealthCheckConfig, HealthChecker, ProxyManager, ProxyRotator};

    // Verify types compile
    let _ = std::mem::size_of::<ProxyManager>();
    let _ = std::mem::size_of::<ProxyRotator>();
    let _ = std::mem::size_of::<HealthChecker>();
}
