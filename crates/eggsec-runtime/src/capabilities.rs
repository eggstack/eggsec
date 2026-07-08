use serde::{Deserialize, Serialize};

use crate::request::TaskKind;

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

/// Default capabilities are conservative (daemon safe subset).
/// Use `full_lab()` for all task kinds.
impl Default for RuntimeCapabilities {
    fn default() -> Self {
        Self::daemon_conservative()
    }
}

impl RuntimeCapabilities {
    /// Create conservative capabilities suitable for the daemon.
    ///
    /// Includes only safe, non-hazardous task kinds. Excludes stress testing,
    /// packet manipulation, wireless active attacks, database pentest, traffic
    /// interception, and C2 simulation.
    pub fn daemon_conservative() -> Self {
        Self {
            task_kinds: vec![
                TaskCapability::new("port-scan", "Port scanning"),
                TaskCapability::new("endpoint-scan", "Endpoint discovery"),
                TaskCapability::new("fingerprint", "Service fingerprinting"),
                TaskCapability::new("waf", "WAF detection"),
                TaskCapability::new("pipeline", "Security assessment pipeline"),
                TaskCapability::new("recon", "Reconnaissance"),
                TaskCapability::new("load-test", "HTTP load testing"),
                TaskCapability::new("fuzz", "Fuzzing"),
                TaskCapability::new("waf-stress", "WAF stress testing"),
                TaskCapability::new("nse", "NSE script execution"),
                TaskCapability::new("hunt", "Vulnerability hunting"),
                TaskCapability::new("browser", "Headless browser testing"),
                TaskCapability::new("graphql", "GraphQL testing"),
                TaskCapability::new("oauth", "OAuth testing"),
                TaskCapability::new("auth-test", "Authentication testing"),
                TaskCapability::new("compliance", "Compliance checking"),
                TaskCapability::new("vuln", "Vulnerability scanning"),
                TaskCapability::new("storage", "Storage operations"),
                TaskCapability::new("integration", "External integrations"),
                TaskCapability::new("workflow", "Workflow execution"),
            ],
            transports: vec!["in-process".into()],
            supports_cancellation: true,
            supports_multiple_sessions: true,
            supports_multiple_active_tasks: false,
        }
    }

    /// Create capabilities listing all task kinds including hazardous/lab families.
    pub fn full_lab() -> Self {
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
            transports: vec!["in-process".into()],
            supports_cancellation: true,
            supports_multiple_sessions: true,
            supports_multiple_active_tasks: false,
        }
    }

    /// Deprecated: use `full_lab()` instead.
    #[deprecated(note = "renamed to full_lab(); use full_lab() for all task kinds")]
    pub fn full() -> Self {
        Self::full_lab()
    }

    /// Create capabilities for a no-op executor (no task kinds).
    pub fn noop() -> Self {
        Self {
            task_kinds: vec![],
            transports: vec!["in-process".into()],
            supports_cancellation: true,
            supports_multiple_sessions: true,
            supports_multiple_active_tasks: false,
        }
    }

    /// Check if a task kind is supported by these capabilities.
    pub fn supports_task_kind(&self, kind: &TaskKind) -> bool {
        let name = task_kind_name(kind);
        self.task_kinds.iter().any(|c| c.name == name)
    }
}

/// Map a `TaskKind` variant to its capability name string.
fn task_kind_name(kind: &TaskKind) -> &'static str {
    match kind {
        TaskKind::LoadTest(_) => "load-test",
        TaskKind::StressTest(_) => "stress-test",
        TaskKind::PortScan(_) => "port-scan",
        TaskKind::EndpointScan(_) => "endpoint-scan",
        TaskKind::Fingerprint(_) => "fingerprint",
        TaskKind::Fuzz(_) => "fuzz",
        TaskKind::Waf(_) => "waf",
        TaskKind::WafStress(_) => "waf-stress",
        TaskKind::Pipeline(_) => "pipeline",
        TaskKind::Recon(_) => "recon",
        TaskKind::PacketCapture(_) => "packet-capture",
        TaskKind::PacketTraceroute(_) => "traceroute",
        TaskKind::PacketSend(_) => "packet-send",
        TaskKind::GraphQl(_) => "graphql",
        TaskKind::OAuth(_) => "oauth",
        TaskKind::AuthTest(_) => "auth-test",
        TaskKind::Nse(_) => "nse",
        TaskKind::Hunt(_) => "hunt",
        TaskKind::Browser(_) => "browser",
        TaskKind::Compliance(_) => "compliance",
        TaskKind::Storage(_) => "storage",
        TaskKind::Integrations(_) => "integration",
        TaskKind::Workflow(_) => "workflow",
        TaskKind::Vuln(_) => "vuln",
        TaskKind::Wireless(_) => "wireless",
        TaskKind::WirelessActive(_) => "wireless-active",
        TaskKind::DbPentest(_) => "db-pentest",
        TaskKind::Intercept(_) => "intercept",
        TaskKind::C2(_) => "c2",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::*;

    #[test]
    fn capabilities_roundtrip() {
        let caps = RuntimeCapabilities::daemon_conservative();
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
        assert!(!caps.supports_multiple_active_tasks);
    }

    #[test]
    fn default_capabilities_no_unimplemented_transports() {
        let caps = RuntimeCapabilities::default();
        assert_eq!(caps.transports, vec!["in-process"]);
    }

    #[test]
    fn default_capabilities_sessions_supported() {
        let caps = RuntimeCapabilities::default();
        assert!(caps.supports_multiple_sessions);
    }

    #[test]
    fn daemon_conservative_includes_safe_kinds() {
        let caps = RuntimeCapabilities::daemon_conservative();
        let names: Vec<_> = caps.task_kinds.iter().map(|c| c.name.as_str()).collect();
        for expected in &[
            "port-scan",
            "endpoint-scan",
            "fingerprint",
            "waf",
            "pipeline",
            "recon",
            "load-test",
            "fuzz",
            "waf-stress",
            "nse",
            "hunt",
            "browser",
            "graphql",
            "oauth",
            "auth-test",
            "compliance",
            "vuln",
            "storage",
            "integration",
            "workflow",
        ] {
            assert!(
                names.contains(expected),
                "Missing safe task kind: {expected}"
            );
        }
    }

    #[test]
    fn daemon_conservative_excludes_hazardous_kinds() {
        let caps = RuntimeCapabilities::daemon_conservative();
        let names: Vec<_> = caps.task_kinds.iter().map(|c| c.name.as_str()).collect();
        for excluded in &[
            "stress-test",
            "packet-send",
            "wireless",
            "wireless-active",
            "db-pentest",
            "intercept",
            "c2",
            "packet-capture",
            "traceroute",
        ] {
            assert!(
                !names.contains(excluded),
                "Hazardous kind should not be in conservative: {excluded}"
            );
        }
    }

    #[test]
    fn full_lab_has_all_task_kinds() {
        let caps = RuntimeCapabilities::full_lab();
        let names: Vec<_> = caps.task_kinds.iter().map(|c| c.name.as_str()).collect();
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

    #[test]
    fn supports_task_kind_positive() {
        let caps = RuntimeCapabilities::daemon_conservative();
        assert!(caps.supports_task_kind(&TaskKind::PortScan(PortScanParams {
            target: "10.0.0.1".into(),
            ports: None,
            scan_type: None,
            timeout_ms: None,
        })));
        assert!(caps.supports_task_kind(&TaskKind::Recon(ReconParams {
            target: "example.com".into(),
            modules: None,
        })));
    }

    #[test]
    fn supports_task_kind_negative() {
        let caps = RuntimeCapabilities::daemon_conservative();
        assert!(
            !caps.supports_task_kind(&TaskKind::StressTest(StressTestParams {
                target: "10.0.0.1".into(),
                flood_type: "syn".into(),
                duration_secs: None,
                threads: None,
            }))
        );
        assert!(
            !caps.supports_task_kind(&TaskKind::Wireless(WirelessParams {
                interface: None,
                duration_secs: None,
            }))
        );
    }
}
