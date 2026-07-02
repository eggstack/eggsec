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
                TaskCapability::new("nse", "NSE script execution"),
                TaskCapability::new("hunt", "Vulnerability hunting"),
                TaskCapability::new("browser", "Headless browser testing"),
                TaskCapability::new("compliance", "Compliance checking"),
                TaskCapability::new("storage", "Storage operations"),
                TaskCapability::new("integration", "External integrations"),
                TaskCapability::new("workflow", "Workflow execution"),
                TaskCapability::new("vuln", "Vulnerability scanning"),
                TaskCapability::new("wireless", "WiFi reconnaissance"),
                TaskCapability::new("wireless-active", "WiFi active attacks"),
                TaskCapability::new("db-pentest", "Database security testing"),
                TaskCapability::new("intercept", "Traffic interception"),
                TaskCapability::new("c2", "C2 simulation"),
            ],
            // Only transports that are currently implemented. The runtime
            // executes tasks in-process; no daemon or IPC transports exist yet.
            transports: vec!["in-process".into()],
            supports_cancellation: true,
            // Sessions are independent (tests prove multiple sessions work).
            supports_multiple_sessions: true,
            // RuntimeConfig::default() sets max_active_tasks_per_session: 1
            // and submit() always cancels existing active tasks.
            supports_multiple_active_tasks: false,
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

    #[test]
    fn default_capabilities_single_active_task() {
        let caps = RuntimeCapabilities::default();
        // RuntimeConfig::default() sets max_active_tasks_per_session: 1
        // and submit() always cancels existing active tasks.
        assert!(!caps.supports_multiple_active_tasks);
    }

    #[test]
    fn default_capabilities_no_unimplemented_transports() {
        let caps = RuntimeCapabilities::default();
        // Only "in-process" transport is currently implemented.
        // No daemon or IPC transports exist yet.
        assert_eq!(caps.transports, vec!["in-process"]);
    }

    #[test]
    fn default_capabilities_sessions_supported() {
        let caps = RuntimeCapabilities::default();
        // Tests prove multiple independent sessions work.
        assert!(caps.supports_multiple_sessions);
    }

    #[test]
    fn default_capabilities_cover_all_task_kinds() {
        let caps = RuntimeCapabilities::default();
        let names: Vec<_> = caps.task_kinds.iter().map(|c| c.name.as_str()).collect();
        // Core task kinds (always compiled)
        for expected in &[
            "load-test",
            "stress-test",
            "port-scan",
            "endpoint-scan",
            "fingerprint",
            "fuzz",
            "waf",
            "waf-stress",
            "pipeline",
            "recon",
            "packet-capture",
            "traceroute",
            "packet-send",
            "graphql",
            "oauth",
            "auth-test",
            "nse",
            "hunt",
            "browser",
            "compliance",
            "storage",
            "integration",
            "workflow",
            "vuln",
            "wireless",
            "wireless-active",
            "db-pentest",
            "intercept",
            "c2",
        ] {
            assert!(names.contains(expected), "Missing task kind: {expected}");
        }
    }
}
