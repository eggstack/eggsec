//! Tool abstraction layer for Eggsec.
//!
//! This module provides a unified tool registry and execution framework that
//! enables programmatic access to Eggsec's security testing capabilities.
//! Tools are registered with metadata (name, description, capabilities, parameters)
//! and can be executed via the [`ToolRegistry`] or through protocol adapters
//! (MCP, OpenAI, REST API).
//!
//! # Re-export shim (intentionally stable)
//!
//! Core data types (request, response, error, history, rate-limiting) live in
//! the `eggsec-tool-core` crate and are re-exported here as both sub-modules
//! (`tool_error`, `request`, `response`, `history`, `ratelimit`) and individual
//! types for backward compatibility.
//!
//! **Local modules** — depend on engine-internal types (`PipelineReport`,
//! `EggsecConfig`, scanner/fuzzer internals, etc.) and could not be moved
//! to `eggsec-tool-core`:
//! - `finding`, `convert`, `dispatcher`, `metadata`, `openapi`, `planner`,
//!   `registry`, `scripting`, `session`, `state`, `traits`, `implementations`,
//!   `orchestrator`, `protocol`
//!
//! **Compatibility facade** — `agents` (behind `rest-api`) re-exports the
//! `eggsec-agent` crate so existing `eggsec::tool::agents::*` paths continue
//! to work.

// Re-export core data types from eggsec-tool-core as modules
// so that `crate::tool::tool_error::ToolError` etc. still work.
pub use eggsec_tool_core::history;
pub use eggsec_tool_core::ratelimit;
pub use eggsec_tool_core::request;
pub use eggsec_tool_core::response;
pub use eggsec_tool_core::tool_error;

// Local modules that depend on eggsec-internal types
pub mod convert;
pub mod dispatcher;
pub mod finding;
pub mod metadata;
pub mod openapi;
pub mod planner;
pub mod registry;
pub mod scripting;
pub mod session;
pub mod state;
pub mod traits;

pub mod implementations;

#[cfg(feature = "rest-api")]
pub mod agents {
    //! Compatibility facade for agent coordination primitives.
    //!
    //! The implementation lives in the `eggsec-agent` crate.
    pub use eggsec_agent::*;
}
pub mod orchestrator;
pub mod protocol;

// Re-export core types at tool module level for convenience
pub use eggsec_tool_core::{
    AuthConfig, AuthType, CancellationToken, CancellationTokenHandle, EndpointData, EndpointLimit,
    GlobalRateLimitStatus, PortData, PortState, RequestOptions, Scope, Target, TargetType,
    ToolError, ToolErrorType, ToolRequest, ToolResponse,
};

pub use dispatcher::ToolDispatcher;
pub use history::{ExecutionEntry, ExecutionHistory};
pub use openapi::OpenApiGenerator;
pub use orchestrator::{
    ExecutionResult, Orchestrator, StageProgress, StageResult, StageToolResult,
};
pub use planner::{ChainPlanner, ExecutionPlan, PlanRequest, PlanValidation};
pub use registry::{ToolInfo, ToolRegistry};
pub use response::{
    Finding, FindingType, ProgressUpdate, ResponseMetadata, ResponseSeverity, ResponseStatus,
    StreamEvent, StreamEventType,
};
pub use session::{
    AuthMethod, AuthenticatedSessionManager, CsrfExtractor, CsrfToken, CsrfTokenLocation,
    FormDetector, LoginExecutor, LoginForm, LoginResult, LoginSequence, LoginStep, MfaConfig,
    ResponseField, SessionState, SessionStatus, SessionVerification, SessionVerifier,
};
pub use state::{AgentSession, ScanContext, SessionManager, SessionStatus as ToolSessionStatus};
pub use traits::{AttackSurface, ToolResult};

pub fn create_default_registry() -> ToolRegistry {
    let registry = ToolRegistry::new();

    if let Err(e) = registry.register(crate::tool::implementations::recon::ReconTool::new()) {
        tracing::warn!("Failed to register tool: recon: {}", e);
    }
    if let Err(e) = registry.register(crate::tool::implementations::scanner::ScannerTool::ports()) {
        tracing::warn!("Failed to register tool: ports: {}", e);
    }
    if let Err(e) =
        registry.register(crate::tool::implementations::scanner::ScannerTool::fingerprint())
    {
        tracing::warn!("Failed to register tool: fingerprint: {}", e);
    }
    if let Err(e) =
        registry.register(crate::tool::implementations::scanner::ScannerTool::endpoints())
    {
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
    if let Err(e) = registry.register(crate::tool::implementations::search::SearchTool::new(None)) {
        tracing::warn!("Failed to register tool: search: {}", e);
    }

    registry
}
