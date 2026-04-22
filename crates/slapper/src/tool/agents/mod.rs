mod registry;
mod delegation;
mod scheduler;
mod aggregator;
mod lifecycle;
mod communication;

pub use registry::{AgentRegistry, AgentInfo, AgentStatus};
pub use delegation::{DelegationRequest, DelegationResponse};
pub use scheduler::{TaskScheduler, TaskQueue, ScheduledTask, TaskPriority, TaskOutcome};
pub use aggregator::ResultAggregator;
pub use lifecycle::{LifecycleManager, LifecycleConfig, LifecycleEvent, LifecycleEventType, AgentHealth, HealthIssue};
pub use communication::{InterAgentChannel, MultiAgentCoordinator, CapabilityAdvertisement, AgentCapability, HealthMetrics, HealthStatus, AgentMessage, MessageType};
