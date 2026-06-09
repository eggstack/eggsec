//! Agent coordination primitives for Slapper.

pub mod aggregator;
pub mod communication;
pub mod delegation;
pub mod lifecycle;
pub mod registry;
pub mod scheduler;

pub use registry::{AgentInfo, AgentRegistry, AgentStatus};
pub use scheduler::{ScheduledTask, TaskPriority, TaskScheduler, TaskStatus};
pub use lifecycle::{AgentHealth, LifecycleConfig, LifecycleManager};
pub use communication::{InterAgentChannel, MultiAgentCoordinator};
pub use delegation::{DelegationRequest, DelegationResponse};
pub use aggregator::{AggregatedResult, ResultAggregator};
