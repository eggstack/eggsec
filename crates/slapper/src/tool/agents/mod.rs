mod aggregator;
mod communication;
mod delegation;
mod lifecycle;
mod registry;
mod scheduler;

pub use aggregator::ResultAggregator;
pub use communication::{
    AgentCapability, AgentMessage, CapabilityAdvertisement, HealthMetrics, HealthStatus,
    InterAgentChannel, MessageType, MultiAgentCoordinator,
};
pub use delegation::{DelegationRequest, DelegationResponse};
pub use lifecycle::{
    AgentHealth, HealthIssue, LifecycleConfig, LifecycleEvent, LifecycleEventType, LifecycleManager,
};
pub use registry::{AgentInfo, AgentRegistry, AgentStatus};
pub use scheduler::{
    ScheduledTask, TaskOutcome, TaskPriority, TaskQueue, TaskScheduler, TaskStatus,
};
