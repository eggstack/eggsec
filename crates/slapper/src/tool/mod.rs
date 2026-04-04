pub mod convert;
pub mod dispatcher;
pub mod history;
pub mod openapi;
pub mod planner;
pub mod ratelimit;
pub mod registry;
pub mod request;
pub mod response;
pub mod state;
pub mod traits;

pub mod implementations;

#[cfg(feature = "rest-api")]
pub mod agents;
pub mod orchestrator;
pub mod protocol;

pub use dispatcher::ToolDispatcher;
pub use history::{ExecutionEntry, ExecutionHistory};
pub use openapi::OpenApiGenerator;
pub use orchestrator::{ExecutionResult, Orchestrator, StageProgress, StageResult, StageToolResult};
pub use planner::{ChainPlanner, ExecutionPlan, PlanRequest, PlanValidation};
pub use ratelimit::{RateLimitConfig, RateLimiter, RateLimitStatus};
pub use registry::{ToolInfo, ToolRegistry};
pub use request::{
    CancellationToken, CancellationTokenHandle, RequestOptions, Target, TargetType, ToolRequest,
};
pub use response::{
    ProgressUpdate, ResponseMetadata, ResponseStatus, StreamEvent, StreamEventType, ToolError,
    ToolErrorType, ToolResponse,
};
pub use state::{AgentSession, ScanContext, SessionManager, SessionStatus};
pub use traits::{
    validate_parameters, AttackSurface, CapabilityExample, ParameterDef, ParameterType,
    SecurityTool, ToolCapability, ToolCategory, ToolResult,
};

pub fn create_default_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();

    if let Err(e) = registry.register(crate::tool::implementations::recon::ReconTool::new()) {
        tracing::warn!("Failed to register tool: recon: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::scanner::ScannerTool::ports()) {
        tracing::warn!("Failed to register tool: ports: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::scanner::ScannerTool::fingerprint()) {
        tracing::warn!("Failed to register tool: fingerprint: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::scanner::ScannerTool::endpoints()) {
        tracing::warn!("Failed to register tool: endpoints: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::fuzzer::FuzzerTool::new()) {
        tracing::warn!("Failed to register tool: fuzzer: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::loadtest::LoadTestTool::new()) {
        tracing::warn!("Failed to register tool: loadtest: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::waf::WafTool::detect()) {
        tracing::warn!("Failed to register tool: waf_detect: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::waf::WafTool::bypass()) {
        tracing::warn!("Failed to register tool: waf_bypass: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::waf::WafTool::stress()) {
        tracing::warn!("Failed to register tool: waf_stress: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::pipeline::PipelineTool::new()) {
        tracing::warn!("Failed to register tool: pipeline: {}", e);
    }

    registry
}
