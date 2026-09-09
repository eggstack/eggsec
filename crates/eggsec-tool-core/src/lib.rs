//! Core data types for the Eggsec tool abstraction layer.
//!
//! Contains request, response, finding, error, and rate-limiting types
//! used by the tool abstraction layer. These are pure data types with
//! no dependencies on the main eggsec engine.

pub mod finding;
pub mod history;
pub mod operation_request;
pub mod ratelimit;
pub mod request;
pub mod response;
pub mod tool_error;

// Re-export key types at crate root for convenience
pub use finding::{Finding, FindingType, ResponseSeverity};
pub use history::ExecutionEntry;
pub use ratelimit::{EndpointLimit, GlobalRateLimitStatus, RateLimitConfig, RateLimitStatus};
#[allow(deprecated)]
pub use request::Scope;
pub use request::{
    AuthConfig, AuthType, CancellationToken, CancellationTokenHandle, RequestOptions, ScopeSpec,
    Target, TargetType, ToolRequest, ToolScopeSpec,
};
pub use response::{
    EndpointData, PortData, PortState, ProgressUpdate, ResponseMetadata, ResponseStatus,
    StreamEvent, StreamEventType, TechnologyData, ToolResponse,
};
pub use tool_error::{ToolError, ToolErrorType};
