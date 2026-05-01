use crate::container::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CisBenchmarkResult {
    pub benchmark_version: String,
    pub total_checks: usize,
    pub passed: usize,
    pub failed: usize,
    pub warnings: usize,
    pub checks: Vec<CisCheck>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CisCheck {
    pub id: String,
    pub description: String,
    pub severity: Severity,
    pub status: CisCheckStatus,
    pub recommendation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum CisCheckStatus {
    Pass,
    Fail,
    Warn,
}

pub struct CisBenchmarkChecker;

impl Default for CisBenchmarkChecker {
    fn default() -> Self {
        Self::new()
    }
}

impl CisBenchmarkChecker {
    pub fn new() -> Self {
        Self
    }

    pub fn check_docker(&self, docker_info: &str) -> CisBenchmarkResult {
        let checks = self.docker_checks(docker_info);
        let passed = checks
            .iter()
            .filter(|c| c.status == CisCheckStatus::Pass)
            .count();
        let failed = checks
            .iter()
            .filter(|c| c.status == CisCheckStatus::Fail)
            .count();
        let warnings = checks
            .iter()
            .filter(|c| c.status == CisCheckStatus::Warn)
            .count();

        CisBenchmarkResult {
            benchmark_version: "CIS Docker Benchmark 1.6.0".to_string(),
            total_checks: checks.len(),
            passed,
            failed,
            warnings,
            checks,
        }
    }

    pub fn check_kubernetes(&self, k8s_config: &str) -> CisBenchmarkResult {
        let checks = self.kubernetes_checks(k8s_config);
        let passed = checks
            .iter()
            .filter(|c| c.status == CisCheckStatus::Pass)
            .count();
        let failed = checks
            .iter()
            .filter(|c| c.status == CisCheckStatus::Fail)
            .count();
        let warnings = checks
            .iter()
            .filter(|c| c.status == CisCheckStatus::Warn)
            .count();

        CisBenchmarkResult {
            benchmark_version: "CIS Kubernetes Benchmark 1.8.0".to_string(),
            total_checks: checks.len(),
            passed,
            failed,
            warnings,
            checks,
        }
    }

    fn docker_checks(&self, info: &str) -> Vec<CisCheck> {
        let lower = info.to_lowercase();
        let mut checks = Vec::new();

        checks.push(CisCheck {
            id: "1.1".to_string(),
            description: "Do not run containers as root".to_string(),
            severity: Severity::High,
            status: if lower.contains("user")
                && !lower.contains("user root")
                && !lower.contains("user 0")
            {
                CisCheckStatus::Pass
            } else {
                CisCheckStatus::Fail
            },
            recommendation: "Run containers as non-root user".to_string(),
        });

        checks.push(CisCheck {
            id: "1.2".to_string(),
            description: "Do not use privileged containers".to_string(),
            severity: Severity::Critical,
            status: if lower.contains("privileged") && lower.contains("true") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Remove --privileged flag".to_string(),
        });

        checks.push(CisCheck {
            id: "1.3".to_string(),
            description: "Do not mount sensitive host directories".to_string(),
            severity: Severity::High,
            status: if lower.contains("/etc") || lower.contains("/proc") || lower.contains("/sys") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Avoid mounting sensitive host paths".to_string(),
        });

        checks.push(CisCheck {
            id: "1.4".to_string(),
            description: "Do not use host network mode".to_string(),
            severity: Severity::High,
            status: if lower.contains("hostnetwork") || lower.contains("host_network") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Disable host network mode".to_string(),
        });

        checks.push(CisCheck {
            id: "1.5".to_string(),
            description: "Limit memory usage".to_string(),
            severity: Severity::Medium,
            status: if lower.contains("memory") || lower.contains("mem_limit") {
                CisCheckStatus::Pass
            } else {
                CisCheckStatus::Warn
            },
            recommendation: "Set memory limits for containers".to_string(),
        });

        checks.push(CisCheck {
            id: "1.6".to_string(),
            description: "Set CPU shares".to_string(),
            severity: Severity::Low,
            status: if lower.contains("cpu_shares") || lower.contains("cpus") {
                CisCheckStatus::Pass
            } else {
                CisCheckStatus::Warn
            },
            recommendation: "Set CPU resource limits".to_string(),
        });

        checks.push(CisCheck {
            id: "1.7".to_string(),
            description: "Do not map container ports to privileged host ports".to_string(),
            severity: Severity::Medium,
            status: if lower.contains("-p 0:") || lower.contains("-p 1:") || lower.contains("-p 2:")
            {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Use non-privileged host ports (>1024)".to_string(),
        });

        checks
    }

    fn kubernetes_checks(&self, config: &str) -> Vec<CisCheck> {
        let lower = config.to_lowercase();
        let mut checks = Vec::new();

        checks.push(CisCheck {
            id: "5.1.1".to_string(),
            description: "Do not admit containers with privileged security context".to_string(),
            severity: Severity::Critical,
            status: if lower.contains("privileged: true") || lower.contains("\"privileged\": true")
            {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Set securityContext.privileged to false".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.2".to_string(),
            description: "Do not admit containers with allowPrivilegeEscalation".to_string(),
            severity: Severity::High,
            status: if lower.contains("allowprivilegeescalation: true") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Set allowPrivilegeEscalation to false".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.3".to_string(),
            description: "Minimize admission of containers with root user".to_string(),
            severity: Severity::High,
            status: if lower.contains("runasuser: 0") || lower.contains("run_as_user: 0") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Set runAsNonRoot: true".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.4".to_string(),
            description: "Do not admit containers with added capabilities".to_string(),
            severity: Severity::High,
            status: if lower.contains("capabilities") && lower.contains("add") {
                CisCheckStatus::Warn
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Remove added capabilities".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.5".to_string(),
            description: "Minimize admission of containers with hostPath volumes".to_string(),
            severity: Severity::High,
            status: if lower.contains("hostpath") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Use PersistentVolumeClaims instead of hostPath".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.6".to_string(),
            description: "Minimize admission of containers with hostNetwork".to_string(),
            severity: Severity::High,
            status: if lower.contains("hostnetwork: true") || lower.contains("host_network: true") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Disable hostNetwork".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.7".to_string(),
            description: "Minimize admission of containers with hostPID".to_string(),
            severity: Severity::High,
            status: if lower.contains("hostpid: true") || lower.contains("host_pid: true") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Disable hostPID".to_string(),
        });

        checks.push(CisCheck {
            id: "5.1.8".to_string(),
            description: "Minimize admission of containers with hostIPC".to_string(),
            severity: Severity::Medium,
            status: if lower.contains("hostipc: true") || lower.contains("host_ipc: true") {
                CisCheckStatus::Fail
            } else {
                CisCheckStatus::Pass
            },
            recommendation: "Disable hostIPC".to_string(),
        });

        checks
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cis_checker_creation() {
        let checker = CisBenchmarkChecker::new();
        let _ = checker;
    }

    #[test]
    fn test_docker_checks_privileged_fail() {
        let checker = CisBenchmarkChecker::new();
        let result = checker.check_docker("privileged: true");
        assert!(result
            .checks
            .iter()
            .any(|c| c.id == "1.2" && c.status == CisCheckStatus::Fail));
    }

    #[test]
    fn test_docker_checks_clean_pass() {
        let checker = CisBenchmarkChecker::new();
        let result = checker.check_docker("image: nginx:latest, user: appuser");
        assert!(result
            .checks
            .iter()
            .any(|c| c.id == "1.2" && c.status == CisCheckStatus::Pass));
    }

    #[test]
    fn test_k8s_checks_privileged_fail() {
        let checker = CisBenchmarkChecker::new();
        let result = checker.check_kubernetes("privileged: true");
        assert!(result
            .checks
            .iter()
            .any(|c| c.id == "5.1.1" && c.status == CisCheckStatus::Fail));
    }

    #[test]
    fn test_benchmark_result_summary() {
        let checker = CisBenchmarkChecker::new();
        let result = checker.check_docker("privileged: true\nuser root\nhostPath: /etc");
        assert!(result.failed > 0);
        assert!(result.total_checks > 0);
    }

    #[test]
    fn test_cis_check_creation() {
        let check = CisCheck {
            id: "1.1".to_string(),
            description: "Test check".to_string(),
            severity: Severity::High,
            status: CisCheckStatus::Pass,
            recommendation: "Test recommendation".to_string(),
        };
        assert_eq!(check.id, "1.1");
        assert_eq!(check.status, CisCheckStatus::Pass);
    }
}
