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
pub mod protocol;

pub use dispatcher::ToolDispatcher;
pub use history::{ExecutionEntry, ExecutionHistory};
pub use openapi::OpenApiGenerator;
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

    registry
        .register(crate::tool::implementations::recon::ReconTool::new())
        .ok();
    registry
        .register(crate::tool::implementations::scanner::ScannerTool::ports())
        .ok();
    registry
        .register(crate::tool::implementations::scanner::ScannerTool::fingerprint())
        .ok();
    registry
        .register(crate::tool::implementations::scanner::ScannerTool::endpoints())
        .ok();
    registry
        .register(crate::tool::implementations::fuzzer::FuzzerTool::new())
        .ok();
    registry
        .register(crate::tool::implementations::loadtest::LoadTestTool::new())
        .ok();
    registry
        .register(crate::tool::implementations::waf::WafTool::detect())
        .ok();
    registry
        .register(crate::tool::implementations::waf::WafTool::bypass())
        .ok();
    registry
        .register(crate::tool::implementations::waf::WafTool::stress())
        .ok();
    registry
        .register(crate::tool::implementations::pipeline::PipelineTool::new())
        .ok();

    registry
}
