use crate::container::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapeDetectionResult {
    pub target: String,
    pub escape_risks: Vec<EscapeRisk>,
    pub risk_level: EscapeRiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EscapeRisk {
    pub risk_type: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum EscapeRiskLevel {
    None,
    Low,
    Medium,
    High,
    Critical,
}

pub struct EscapeDetector;

impl Default for EscapeDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl EscapeDetector {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_docker_config(&self, config: &str) -> EscapeDetectionResult {
        let mut risks = Vec::new();
        let lower = config.to_lowercase();

        if lower.contains("\"privileged\": true") || lower.contains("privileged: true") {
            risks.push(EscapeRisk {
                risk_type: "Privileged Container".to_string(),
                severity: Severity::Critical,
                description: "Container runs in privileged mode with full host access".to_string(),
                recommendation: "Remove privileged flag and use specific capabilities instead"
                    .to_string(),
            });
        }

        if lower.contains("hostpath") || lower.contains("host_path") {
            risks.push(EscapeRisk {
                risk_type: "HostPath Mount".to_string(),
                severity: Severity::High,
                description: "Container mounts host filesystem path".to_string(),
                recommendation: "Avoid hostPath mounts; use PersistentVolumeClaims instead"
                    .to_string(),
            });
        }

        if lower.contains("hostnetwork: true")
            || lower.contains("\"hostnetwork\": true")
            || lower.contains("host_network: true")
        {
            risks.push(EscapeRisk {
                risk_type: "Host Network".to_string(),
                severity: Severity::High,
                description: "Container shares host network namespace".to_string(),
                recommendation: "Disable hostNetwork unless absolutely required".to_string(),
            });
        }

        if lower.contains("hostpid: true")
            || lower.contains("\"hostpid\": true")
            || lower.contains("host_pid: true")
        {
            risks.push(EscapeRisk {
                risk_type: "Host PID".to_string(),
                severity: Severity::High,
                description: "Container shares host PID namespace".to_string(),
                recommendation: "Disable hostPID to prevent process visibility on host".to_string(),
            });
        }

        if lower.contains("hostipc: true")
            || lower.contains("\"hostipc\": true")
            || lower.contains("host_ipc: true")
        {
            risks.push(EscapeRisk {
                risk_type: "Host IPC".to_string(),
                severity: Severity::Medium,
                description: "Container shares host IPC namespace".to_string(),
                recommendation: "Disable hostIPC unless required for shared memory".to_string(),
            });
        }

        let dangerous_caps = [
            "SYS_ADMIN",
            "NET_ADMIN",
            "SYS_PTRACE",
            "DAC_READ_SEARCH",
            "SYS_MODULE",
        ];
        for cap in &dangerous_caps {
            if upper_contains(&lower, cap) {
                risks.push(EscapeRisk {
                    risk_type: format!("Dangerous capability: {}", cap),
                    severity: Severity::High,
                    description: format!("Container has {} capability", cap),
                    recommendation: format!("Remove {} capability unless required", cap),
                });
            }
        }

        if lower.contains("docker.sock") || lower.contains("containerd.sock") {
            risks.push(EscapeRisk {
                risk_type: "Container Runtime Socket".to_string(),
                severity: Severity::Critical,
                description: "Container runtime socket mounted inside container".to_string(),
                recommendation: "Never mount container runtime sockets in containers".to_string(),
            });
        }

        let risk_level = Self::calculate_risk_level(&risks);

        if !risks.is_empty() {
            tracing::debug!(
                "Escape analysis found {} risk(s) (level: {:?})",
                risks.len(),
                risk_level
            );
        }

        EscapeDetectionResult {
            target: "docker-config".to_string(),
            escape_risks: risks,
            risk_level,
        }
    }

    pub fn analyze_k8s_pod_spec(&self, pod_spec: &str) -> EscapeDetectionResult {
        self.analyze_docker_config(pod_spec)
    }

    fn calculate_risk_level(risks: &[EscapeRisk]) -> EscapeRiskLevel {
        if risks.iter().any(|r| r.severity == Severity::Critical) {
            EscapeRiskLevel::Critical
        } else if risks.iter().any(|r| r.severity == Severity::High) {
            EscapeRiskLevel::High
        } else if risks.iter().any(|r| r.severity == Severity::Medium) {
            EscapeRiskLevel::Medium
        } else if !risks.is_empty() {
            EscapeRiskLevel::Low
        } else {
            EscapeRiskLevel::None
        }
    }
}

fn upper_contains(haystack: &str, needle: &str) -> bool {
    haystack.contains(&needle.to_lowercase())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_detector_creation() {
        let detector = EscapeDetector::new();
        let _ = detector;
    }

    #[test]
    fn test_detect_privileged_container() {
        let detector = EscapeDetector::new();
        let config = r#"{"privileged": true, "image": "ubuntu"}"#;
        let result = detector.analyze_docker_config(config);
        assert!(result
            .escape_risks
            .iter()
            .any(|r| r.risk_type == "Privileged Container"));
        assert_eq!(result.risk_level, EscapeRiskLevel::Critical);
    }

    #[test]
    fn test_detect_hostpath_mount() {
        let detector = EscapeDetector::new();
        let config = r#"volumes: [{hostPath: {path: /}}]"#;
        let result = detector.analyze_docker_config(config);
        assert!(result
            .escape_risks
            .iter()
            .any(|r| r.risk_type == "HostPath Mount"));
    }

    #[test]
    fn test_detect_docker_socket() {
        let detector = EscapeDetector::new();
        let config = r#"volumes: [{name: docker-sock, hostPath: {path: /var/run/docker.sock}}]"#;
        let result = detector.analyze_docker_config(config);
        assert!(result
            .escape_risks
            .iter()
            .any(|r| r.risk_type == "Container Runtime Socket"));
    }

    #[test]
    fn test_detect_dangerous_capabilities() {
        let detector = EscapeDetector::new();
        let config = r#"securityContext: {capabilities: {add: [SYS_ADMIN]}}"#;
        let result = detector.analyze_docker_config(config);
        assert!(result
            .escape_risks
            .iter()
            .any(|r| r.risk_type.contains("SYS_ADMIN")));
    }

    #[test]
    fn test_clean_config() {
        let detector = EscapeDetector::new();
        let config = r#"image: nginx:latest, ports: [80]"#;
        let result = detector.analyze_docker_config(config);
        assert!(result.escape_risks.is_empty());
        assert_eq!(result.risk_level, EscapeRiskLevel::None);
    }

    #[test]
    fn test_risk_level_calculation() {
        let risks = vec![EscapeRisk {
            risk_type: "Test".to_string(),
            severity: Severity::High,
            description: "Test".to_string(),
            recommendation: "Test".to_string(),
        }];
        assert_eq!(
            EscapeDetector::calculate_risk_level(&risks),
            EscapeRiskLevel::High
        );
    }
}
