use serde::{Deserialize, Serialize};

use crate::utils::strip_controls;
use crate::waf::types::{OwaspCategory, Severity};

use super::super::payloads::Payload;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzResult {
    pub payload: Payload,
    pub status_code: u16,
    pub response_time_ms: u64,
    pub response_length: Option<u64>,
    pub is_waf_blocked: bool,
    pub is_anomaly: bool,
    pub is_redos_suspected: bool,
    pub leaks_found: Vec<String>,
    pub error: Option<String>,
    pub owasp_category: Option<String>,
    pub detected_severity: Severity,
}

impl FuzzResult {
    pub fn is_vulnerable(&self) -> bool {
        !self.leaks_found.is_empty()
            || self.is_waf_blocked
            || self.is_anomaly
            || self.is_redos_suspected
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OwaspSummary {
    pub a01_broken_access_control: usize,
    pub a02_cryptographic_failures: usize,
    pub a03_injection: usize,
    pub a04_insecure_design: usize,
    pub a05_security_misconfiguration: usize,
    pub a06_vulnerable_components: usize,
    pub a07_auth_failures: usize,
    pub a08_software_integrity: usize,
    pub a09_logging_failures: usize,
    pub a10_ssrf: usize,
}

impl OwaspSummary {
    pub fn from_results(results: &[FuzzResult]) -> Self {
        let mut summary = OwaspSummary {
            a01_broken_access_control: 0,
            a02_cryptographic_failures: 0,
            a03_injection: 0,
            a04_insecure_design: 0,
            a05_security_misconfiguration: 0,
            a06_vulnerable_components: 0,
            a07_auth_failures: 0,
            a08_software_integrity: 0,
            a09_logging_failures: 0,
            a10_ssrf: 0,
        };

        for result in results {
            let category =
                OwaspCategory::from_payload_type(&result.payload.payload_type.to_string());
            match category {
                OwaspCategory::A01_2021_BrokenAccessControl
                | OwaspCategory::A01_2023_BrokenObjectLevelAuthorization
                | OwaspCategory::A05_2023_BrokenAccessControl => {
                    summary.a01_broken_access_control += 1;
                }
                OwaspCategory::A02_2021_CryptographicFailures
                | OwaspCategory::A08_2023_WeakCryptography => {
                    summary.a02_cryptographic_failures += 1;
                }
                OwaspCategory::A03_2021_Injection
                | OwaspCategory::A03_2023_BrokenObjectPropertyLevelAccessControl => {
                    summary.a03_injection += 1;
                }
                OwaspCategory::A04_2021_InsecureDesign
                | OwaspCategory::A07_2023_InsecureDesign
                | OwaspCategory::A04_2023_UnrestrictedResourceConsumption => {
                    summary.a04_insecure_design += 1;
                }
                OwaspCategory::A05_2021_SecurityMisconfiguration
                | OwaspCategory::A06_2023_SecurityMisconfiguration => {
                    summary.a05_security_misconfiguration += 1;
                }
                OwaspCategory::A06_2021_VulnerableComponents => {
                    summary.a06_vulnerable_components += 1;
                }
                OwaspCategory::A07_2021_AuthFailures
                | OwaspCategory::A02_2023_BrokenAuthentication => {
                    summary.a07_auth_failures += 1;
                }
                OwaspCategory::A08_2021_SoftwareIntegrity
                | OwaspCategory::A08_2023_SoftwareIntegrityFailures => {
                    summary.a08_software_integrity += 1;
                }
                OwaspCategory::A09_2021_LoggingFailures
                | OwaspCategory::A09_2023_LoggingMonitoring => {
                    summary.a09_logging_failures += 1;
                }
                OwaspCategory::A10_2021_SSRF | OwaspCategory::A10_2023_SSRF => {
                    summary.a10_ssrf += 1;
                }
            }
        }

        summary
    }
}

impl std::fmt::Display for OwaspSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "OWASP Top 10")?;
        writeln!(
            f,
            "\tA01-BrokenAccessControl: {}",
            self.a01_broken_access_control
        )?;
        writeln!(
            f,
            "\tA02-CryptographicFailures: {}",
            self.a02_cryptographic_failures
        )?;
        writeln!(f, "\tA03-Injection: {}", self.a03_injection)?;
        writeln!(f, "\tA04-InsecureDesign: {}", self.a04_insecure_design)?;
        writeln!(
            f,
            "\tA05-SecurityMisconfiguration: {}",
            self.a05_security_misconfiguration
        )?;
        writeln!(
            f,
            "\tA06-VulnerableComponents: {}",
            self.a06_vulnerable_components
        )?;
        writeln!(f, "\tA07-AuthFailures: {}", self.a07_auth_failures)?;
        writeln!(
            f,
            "\tA08-SoftwareIntegrity: {}",
            self.a08_software_integrity
        )?;
        writeln!(f, "\tA09-LoggingFailures: {}", self.a09_logging_failures)?;
        writeln!(f, "\tA10-SSRF: {}", self.a10_ssrf)?;
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzSession {
    pub target_url: String,
    pub mode: String,
    pub payload_type: String,
    pub total_payloads: usize,
    pub successful_requests: usize,
    pub failed_requests: usize,
    pub waf_bypasses: usize,
    pub potential_leaks: usize,
    pub time_anomalies: usize,
    pub redos_suspected: usize,
    pub duration_ms: u64,
    pub total_requests: usize,
    pub findings: usize,
    pub results: Vec<FuzzResult>,
    pub owasp_summary: OwaspSummary,
    pub baseline: Option<BaselineResponse>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BaselineResponse {
    pub status_code: u16,
    pub response_time_ms: u64,
    pub content_length: Option<u64>,
    pub headers: std::collections::HashMap<String, String>,
}

impl std::fmt::Display for FuzzSession {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Fuzz Results")?;
        writeln!(f, "target: {}", strip_controls(&self.target_url, 60))?;
        writeln!(f, "mode: {} | payloads: {}", self.mode, self.total_payloads)?;
        writeln!(
            f,
            "requests: {} success / {} failed",
            self.successful_requests, self.failed_requests
        )?;
        writeln!(
            f,
            "waf_bypasses: {} | leaks: {} | anomalies: {} | redos: {}",
            self.waf_bypasses, self.potential_leaks, self.time_anomalies, self.redos_suspected
        )?;
        writeln!(f, "duration: {}ms", self.duration_ms)?;
        writeln!(f, "{}", self.owasp_summary)?;

        let critical_results: Vec<_> = self
            .results
            .iter()
            .filter(|r| r.is_waf_blocked || r.is_anomaly || !r.leaks_found.is_empty())
            .take(10)
            .collect();

        if !critical_results.is_empty() {
            writeln!(f, "findings")?;
            for result in critical_results {
                let severity = if result.is_redos_suspected {
                    "CRITICAL"
                } else if !result.leaks_found.is_empty() {
                    "HIGH"
                } else if result.is_anomaly {
                    "MEDIUM"
                } else {
                    "INFO"
                };

                writeln!(
                    f,
                    "\t[{}] {} | {} | {}ms",
                    severity,
                    result.status_code,
                    strip_controls(&result.payload.description, 40),
                    result.response_time_ms
                )?;

                if !result.leaks_found.is_empty() {
                    for leak in result.leaks_found.iter().take(2) {
                        writeln!(f, "\t\tleak: {}", strip_controls(leak, 50))?;
                    }
                }
            }
        }

        Ok(())
    }
}
