//! Core data types for the Slapper tool abstraction layer.
//!
//! Contains request, response, finding, error, and rate-limiting types
//! used by the tool abstraction layer. These are pure data types with
//! no dependencies on the main slapper engine.

pub mod tool_error;
pub mod request;
pub mod finding;
pub mod response;
pub mod history;
pub mod ratelimit;

// Re-export key types at crate root for convenience
pub use tool_error::{ToolError, ToolErrorType};
pub use request::{
    AuthConfig, AuthType, CancellationToken, CancellationTokenHandle, RequestOptions, Scope,
    Target, TargetType, ToolRequest,
};
pub use finding::{Finding, FindingType, ResponseSeverity};
pub use response::{
    EndpointData, PortData, PortState, ProgressUpdate, ResponseMetadata, ResponseStatus,
    StreamEvent, StreamEventType, TechnologyData, ToolResponse,
};
pub use history::ExecutionEntry;
pub use ratelimit::{EndpointLimit, RateLimitConfig, RateLimitStatus, GlobalRateLimitStatus};
