// Adapter layer: re-exports from eggsec-web-proxy domain crate.
// All domain logic lives in the domain crate. This module provides
// backward-compatible paths for TUI, CLI, and other consumers.

pub use eggsec_web_proxy::intercept;
pub use eggsec_web_proxy::*;
pub use eggsec_web_proxy::{
    HealthCheckConfig, HealthChecker, ProxiedConnection, ProxyConfig, ProxyEntry, ProxyHealth,
    ProxyManager, ProxyPool, ProxyRotator, ProxyType,
};
