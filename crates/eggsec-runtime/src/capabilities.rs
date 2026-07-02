use serde::{Deserialize, Serialize};

/// Capability descriptor for a single task kind.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCapability {
    pub name: String,
    pub description: String,
}

impl TaskCapability {
    pub fn new(name: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: description.into(),
        }
    }
}

/// Runtime capabilities describing what the runtime supports.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RuntimeCapabilities {
    pub task_kinds: Vec<TaskCapability>,
    pub transports: Vec<String>,
    pub supports_cancellation: bool,
    pub supports_multiple_sessions: bool,
    pub supports_multiple_active_tasks: bool,
}

impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self {
            task_kinds: vec![
                TaskCapability::new("load-test", "HTTP load testing"),
                TaskCapability::new("stress-test", "Network stress testing"),
                TaskCapability::new("port-scan", "Port scanning"),
                TaskCapability::new("endpoint-scan", "Endpoint discovery"),
                TaskCapability::new("fingerprint", "Service fingerprinting"),
                TaskCapability::new("fuzz", "Fuzzing"),
                TaskCapability::new("waf", "WAF detection"),
                TaskCapability::new("waf-stress", "WAF stress testing"),
                TaskCapability::new("pipeline", "Security assessment pipeline"),
                TaskCapability::new("recon", "Reconnaissance"),
                TaskCapability::new("packet-capture", "Packet capture"),
                TaskCapability::new("traceroute", "Packet traceroute"),
                TaskCapability::new("packet-send", "Packet sending"),
                TaskCapability::new("graphql", "GraphQL testing"),
                TaskCapability::new("oauth", "OAuth testing"),
                TaskCapability::new("auth-test", "Authentication testing"),
                TaskCapability::new("hunt", "Vulnerability hunting"),
                TaskCapability::new("compliance", "Compliance checking"),
                TaskCapability::new("storage", "Storage operations"),
                TaskCapability::new("integration", "External integrations"),
                TaskCapability::new("workflow", "Workflow execution"),
                TaskCapability::new("vuln", "Vulnerability scanning"),
                TaskCapability::new("intercept", "Traffic interception"),
            ],
            transports: vec!["stdio".into(), "unix-socket".into()],
            supports_cancellation: true,
            supports_multiple_sessions: false,
            supports_multiple_active_tasks: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_roundtrip() {
        let caps = RuntimeCapabilities::default();
        let json = serde_json::to_string(&caps).unwrap();
        let deserialized: RuntimeCapabilities = serde_json::from_str(&json).unwrap();
        assert_eq!(caps.task_kinds.len(), deserialized.task_kinds.len());
        assert!(deserialized.supports_cancellation);
    }

    #[test]
    fn default_capabilities_have_task_kinds() {
        let caps = RuntimeCapabilities::default();
        assert!(!caps.task_kinds.is_empty());
        let names: Vec<_> = caps.task_kinds.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"port-scan"));
        assert!(names.contains(&"load-test"));
    }
}
