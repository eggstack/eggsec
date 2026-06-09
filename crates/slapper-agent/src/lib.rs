//! Agent coordination primitives for Slapper.
//!
//! This crate owns the registry, scheduling, lifecycle, communication,
//! delegation, and aggregation implementations extracted from `slapper`.
//! The lifecycle manager uses `reqwest` for callback health checks.

pub mod aggregator;
pub mod communication;
pub mod delegation;
pub mod lifecycle;
pub mod registry;
pub mod scheduler;

pub use aggregator::{
    AggregatedError, AggregatedResult, ResultAggregator, StageSummary, ToolSummary,
};
pub use communication::{
    AgentCapability, AgentMessage, CapabilityAdvertisement, CapabilityParam, HealthMetrics,
    HealthStatus, InterAgentChannel, InterAgentError, MessageType, MultiAgentCoordinator,
    TaskStatusUpdate,
};
pub use delegation::{DelegationRequest, DelegationResponse};
pub use lifecycle::{
    AgentHealth, HealthIssue, LifecycleConfig, LifecycleEvent, LifecycleEventType, LifecycleManager,
};
pub use registry::{AgentInfo, AgentRegistry, AgentStatus};
pub use scheduler::{
    ScheduledTask, TaskOutcome, TaskPriority, TaskQueue, TaskScheduler, TaskStatus,
};
