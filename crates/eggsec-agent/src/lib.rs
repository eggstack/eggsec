//! Agent coordination primitives for Eggsec.
//!
//! This crate owns the registry, scheduling, lifecycle, communication,
//! delegation, and aggregation implementations extracted from `eggsec`.
//! The lifecycle manager performs callback health checks through the
//! scope-aware [`eggsec_transport::HttpTransport`] contract: the owning
//! process injects the transport plus its [`eggsec_transport::NetworkAuthority`]
//! at composition time, so this crate never constructs an unrestricted HTTP
//! client (Phase D migration).

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
