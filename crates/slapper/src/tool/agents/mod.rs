mod registry;
mod delegation;
mod scheduler;
mod aggregator;
mod lifecycle;

pub use registry::{AgentRegistry, AgentInfo, AgentStatus};
pub use delegation::{DelegationRequest, DelegationResponse};
pub use scheduler::{TaskScheduler, TaskQueue, ScheduledTask, TaskPriority, TaskOutcome};
pub use aggregator::ResultAggregator;
pub use lifecycle::{LifecycleManager, LifecycleConfig, LifecycleEvent, LifecycleEventType, AgentHealth, HealthIssue};
