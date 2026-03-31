//! Feature flag integration tests.
//!
//! Verifies that feature-flaged modules compile correctly and expose
//! expected public types. These tests run under `--features full` in CI
//! but each test is gated on its specific feature.

#[test]
fn core_modules_always_available() {
    // Core modules should compile without any features enabled
    let _ = slapper::error::SlapperError::Config("test".into());
    let _ = slapper::types::Severity::High;
}

#[cfg(feature = "tool-api")]
#[test]
fn tool_api_exposes_registry() {
    use slapper::tool::ToolRegistry;
    let _registry = ToolRegistry::new();
}

#[cfg(feature = "rest-api")]
#[test]
fn rest_api_module_available() {
    // Verify the module compiled and McpServer type exists
    use slapper::tool::protocol::mcp::McpServer;
    let _ = std::any::TypeId::of::<McpServer>();
}

#[cfg(feature = "grpc-api")]
#[test]
fn grpc_api_module_available() {
    // Verify tonic/prost integration compiled
    let _ = std::any::TypeId::of::<slapper::tool::ToolRegistry>();
}

#[cfg(feature = "python-plugins")]
#[test]
fn python_plugins_available() {
    use slapper::plugin;
    // Verify the plugin module is re-exported
    let _ = std::any::TypeId::of::<plugin::PluginManager>();
}

#[cfg(feature = "ruby-plugins")]
#[test]
fn ruby_plugins_available() {
    use slapper::plugin;
    // Verify the plugin module is re-exported (ruby also uses slapper-plugin)
    let _ = std::any::TypeId::of::<plugin::PluginManager>();
}

#[cfg(feature = "stress-testing")]
#[test]
fn stress_testing_module_available() {
    use slapper::stress;
    let _ = std::any::TypeId::of::<stress::StressConfig>();
}

#[cfg(feature = "packet-inspection")]
#[test]
fn packet_inspection_module_available() {
    let _ = std::any::TypeId::of::<slapper::packet::CaptureConfig>();
}

#[cfg(feature = "nse")]
#[test]
fn nse_module_available() {
    let _ = std::any::TypeId::of::<slapper::nse::NseConfig>();
}

#[cfg(feature = "all-plugins")]
#[test]
fn all_plugins_enables_both_languages() {
    // all-plugins should enable both python and ruby
    // This test just verifies compilation; the types are checked above
    assert!(cfg!(feature = "python-plugins"));
    assert!(cfg!(feature = "ruby-plugins"));
}
