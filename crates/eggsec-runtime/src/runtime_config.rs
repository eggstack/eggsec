//! Runtime configuration (Phase D WS7).
//!
//! Cohesive module extracted from `runtime.rs`: submission/lifecycle tuning
//! only (timeouts, capacities, capabilities, session options). No task
//! registry, event, or cancellation logic here.
//!
//! Stable facade: `runtime.rs` re-exports everything here.

use std::time::Duration;

/// Configuration for the runtime.
#[derive(Debug, Clone)]
pub struct RuntimeConfig {
    /// Default timeout for tasks. None means no timeout.
    pub default_task_timeout: Option<Duration>,
    /// Maximum active tasks per session.
    pub max_active_tasks_per_session: usize,
    /// Capacity of the event broadcast channel.
    pub event_channel_capacity: usize,
    /// Capabilities advertised by this runtime. Determines which task kinds
    /// sessions report as available. Use `RuntimeCapabilities::noop()` for
    /// daemons without a real executor.
    pub capabilities: crate::capabilities::RuntimeCapabilities,
}

impl Default for RuntimeConfig {
    fn default() -> Self {
        Self {
            default_task_timeout: Some(Duration::from_secs(300)),
            max_active_tasks_per_session: 1,
            event_channel_capacity: 256,
            capabilities: crate::capabilities::RuntimeCapabilities::full_lab(),
        }
    }
}

/// Options for creating a session.
#[derive(Debug, Clone, Default)]
pub struct SessionOptions {
    /// Override for the default task timeout for this session.
    pub task_timeout: Option<Duration>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_has_sane_timeouts_and_capacity() {
        let config = RuntimeConfig::default();
        assert_eq!(config.default_task_timeout, Some(Duration::from_secs(300)));
        assert_eq!(config.max_active_tasks_per_session, 1);
        assert_eq!(config.event_channel_capacity, 256);
    }

    #[test]
    fn session_options_default_has_no_override() {
        let options = SessionOptions::default();
        assert!(options.task_timeout.is_none());
    }
}
