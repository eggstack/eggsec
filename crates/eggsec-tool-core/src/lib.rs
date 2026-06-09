//! Core data types for the Eggsec tool abstraction layer.
//!
//! Contains request, response, finding, error, and rate-limiting types
//! used by the tool abstraction layer. These are pure data types with
//! no dependencies on the main eggsec engine.

pub mod finding;
pub mod history;
pub mod ratelimit;
pub mod request;
pub mod response;
pub mod tool_error;

// Re-export key types at crate root for convenience
pub use finding::{Finding, FindingType, ResponseSeverity};
pub use history::ExecutionEntry;
pub use ratelimit::{EndpointLimit, GlobalRateLimitStatus, RateLimitConfig, RateLimitStatus};
pub use request::{
    AuthConfig, AuthType, CancellationToken, CancellationTokenHandle, RequestOptions, Scope,
    Target, TargetType, ToolRequest,
};
pub use response::{
    EndpointData, PortData, PortState, ProgressUpdate, ResponseMetadata, ResponseStatus,
    StreamEvent, StreamEventType, TechnologyData, ToolResponse,
};
pub use tool_error::{ToolError, ToolErrorType};
