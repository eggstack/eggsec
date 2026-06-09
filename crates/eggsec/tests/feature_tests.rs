//! Feature flag integration tests.
//!
//! Verifies that feature-flaged modules compile correctly and expose
//! expected public types. These tests run under `--features full` in CI
//! but each test is gated on its specific feature.

#[test]
fn core_modules_always_available() {
    // Core modules should compile without any features enabled
    let _ = eggsec::error::EggsecError::Config("test".into());
    let _ = eggsec::types::Severity::High;
}

#[cfg(feature = "tool-api")]
#[test]
fn tool_api_exposes_registry() {
    use eggsec::tool::ToolRegistry;
    let _registry = ToolRegistry::new();
}

#[cfg(feature = "rest-api")]
#[test]
fn rest_api_module_available() {
    // Verify the module compiled and McpServer type exists
    use eggsec::tool::protocol::mcp::McpServer;
    let _ = std::any::TypeId::of::<McpServer>();
}

#[cfg(feature = "grpc-api")]
#[test]
fn grpc_api_module_available() {
    // Verify tonic/prost integration compiled
    let _ = std::any::TypeId::of::<eggsec::tool::ToolRegistry>();
}

#[cfg(feature = "stress-testing")]
#[test]
fn stress_testing_module_available() {
    use eggsec::stress;
    let _ = std::any::TypeId::of::<stress::StressConfig>();
}

#[cfg(feature = "packet-inspection")]
#[test]
fn packet_inspection_module_available() {
    let _ = std::any::TypeId::of::<eggsec::packet::CaptureConfig>();
}

#[cfg(feature = "nse")]
#[test]
fn nse_module_available() {
    let _ = std::any::TypeId::of::<eggsec::nse::NseConfig>();
}
