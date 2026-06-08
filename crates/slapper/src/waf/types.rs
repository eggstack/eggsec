use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub use crate::types::Severity;

#[allow(non_camel_case_types)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum OwaspCategory {
    #[serde(rename = "A01:2021 - Broken Access Control")]
    A01_2021_BrokenAccessControl,
    #[serde(rename = "A02:2021 - Cryptographic Failures")]
    A02_2021_CryptographicFailures,
    #[serde(rename = "A03:2021 - Injection")]
    A03_2021_Injection,
    #[serde(rename = "A04:2021 - Insecure Design")]
    A04_2021_InsecureDesign,
    #[serde(rename = "A05:2021 - Security Misconfiguration")]
    A05_2021_SecurityMisconfiguration,
    #[serde(rename = "A06:2021 - Vulnerable and Outdated Components")]
    A06_2021_VulnerableComponents,
    #[serde(rename = "A07:2021 - Identification and Authentication Failures")]
    A07_2021_AuthFailures,
    #[serde(rename = "A08:2021 - Software and Data Integrity Failures")]
    A08_2021_SoftwareIntegrity,
    #[serde(rename = "A09:2021 - Security Logging and Monitoring Failures")]
    A09_2021_LoggingFailures,
    #[serde(rename = "A10:2021 - Server-Side Request Forgery (SSRF)")]
    A10_2021_SSRF,
    #[serde(rename = "A08:2023 - Weak Cryptographic Hashes")]
    A08_2023_WeakCryptography,
    #[serde(rename = "A01:2023 - Broken Object Level Authorization")]
    A01_2023_BrokenObjectLevelAuthorization,
    #[serde(rename = "A02:2023 - Broken Authentication")]
    A02_2023_BrokenAuthentication,
    #[serde(rename = "A03:2023 - Broken Object Property Level Access Control")]
    A03_2023_BrokenObjectPropertyLevelAccessControl,
    #[serde(rename = "A04:2023 - Unrestricted Resource Consumption")]
    A04_2023_UnrestrictedResourceConsumption,
    #[serde(rename = "A05:2023 - Broken Access Control")]
    A05_2023_BrokenAccessControl,
    #[serde(rename = "A06:2023 - Security Misconfiguration")]
    A06_2023_SecurityMisconfiguration,
    #[serde(rename = "A07:2023 - Insecure Design")]
    A07_2023_InsecureDesign,
    #[serde(rename = "A08:2023 - Software and Data Integrity Failures")]
    A08_2023_SoftwareIntegrityFailures,
    #[serde(rename = "A09:2023 - Security Logging and Monitoring Failures")]
    A09_2023_LoggingMonitoring,
    #[serde(rename = "A10:2023 - SSRF")]
    A10_2023_SSRF,
}

impl std::fmt::Display for OwaspCategory {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OwaspCategory::A01_2021_BrokenAccessControl => {
                write!(f, "A01:2021 - Broken Access Control")
            }
            OwaspCategory::A02_2021_CryptographicFailures => {
                write!(f, "A02:2021 - Cryptographic Failures")
            }
            OwaspCategory::A03_2021_Injection => write!(f, "A03:2021 - Injection"),
            OwaspCategory::A04_2021_InsecureDesign => write!(f, "A04:2021 - Insecure Design"),
            OwaspCategory::A05_2021_SecurityMisconfiguration => {
                write!(f, "A05:2021 - Security Misconfiguration")
            }
            OwaspCategory::A06_2021_VulnerableComponents => {
                write!(f, "A06:2021 - Vulnerable and Outdated Components")
            }
            OwaspCategory::A07_2021_AuthFailures => {
                write!(f, "A07:2021 - Identification and Authentication Failures")
            }
            OwaspCategory::A08_2021_SoftwareIntegrity => {
                write!(f, "A08:2021 - Software and Data Integrity Failures")
            }
            OwaspCategory::A09_2021_LoggingFailures => {
                write!(f, "A09:2021 - Security Logging and Monitoring Failures")
            }
            OwaspCategory::A10_2021_SSRF => {
                write!(f, "A10:2021 - Server-Side Request Forgery (SSRF)")
            }
            OwaspCategory::A08_2023_WeakCryptography => {
                write!(f, "A08:2023 - Weak Cryptographic Hashes")
            }
            OwaspCategory::A01_2023_BrokenObjectLevelAuthorization => {
                write!(f, "A01:2023 - Broken Object Level Authorization")
            }
            OwaspCategory::A02_2023_BrokenAuthentication => {
                write!(f, "A02:2023 - Broken Authentication")
            }
            OwaspCategory::A03_2023_BrokenObjectPropertyLevelAccessControl => {
                write!(f, "A03:2023 - Broken Object Property Level Access Control")
            }
            OwaspCategory::A04_2023_UnrestrictedResourceConsumption => {
                write!(f, "A04:2023 - Unrestricted Resource Consumption")
            }
            OwaspCategory::A05_2023_BrokenAccessControl => {
                write!(f, "A05:2023 - Broken Access Control")
            }
            OwaspCategory::A06_2023_SecurityMisconfiguration => {
                write!(f, "A06:2023 - Security Misconfiguration")
            }
            OwaspCategory::A07_2023_InsecureDesign => write!(f, "A07:2023 - Insecure Design"),
            OwaspCategory::A08_2023_SoftwareIntegrityFailures => {
                write!(f, "A08:2023 - Software and Data Integrity Failures")
            }
            OwaspCategory::A09_2023_LoggingMonitoring => {
                write!(f, "A09:2023 - Security Logging and Monitoring Failures")
            }
            OwaspCategory::A10_2023_SSRF => write!(f, "A10:2023 - SSRF"),
        }
    }
}

impl OwaspCategory {
    pub fn from_payload_type(payload_type: &str) -> Self {
        match payload_type.to_lowercase().as_str() {
            "sqli" | "sql" | "sql injection" => OwaspCategory::A03_2021_Injection,
            "xss" | "cross-site scripting" | "cross site scripting" => {
                OwaspCategory::A03_2021_Injection
            }
            "ssrf" | "server-side request forgery" => OwaspCategory::A10_2021_SSRF,
            "traversal"
            | "lfi"
            | "rfi"
            | "path traversal"
            | "local file inclusion"
            | "remote file inclusion" => OwaspCategory::A01_2021_BrokenAccessControl,
            "redirect" | "open redirect" => OwaspCategory::A01_2021_BrokenAccessControl,
            "headers" | "header injection" => OwaspCategory::A03_2021_Injection,
            "compression" | "compression bomb" => OwaspCategory::A04_2021_InsecureDesign,
            "redos" | "regex dos" | "denial of service" => OwaspCategory::A04_2021_InsecureDesign,
            _ => OwaspCategory::A05_2021_SecurityMisconfiguration,
        }
    }

}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    pub id: String,
    pub title: String,
    pub description: String,
    pub severity: Severity,
    pub owasp_category: OwaspCategory,
    pub waf_detected: Option<String>,
    pub bypass_successful: bool,
    pub technique: String,
    pub payload: String,
    pub response_status: u16,
    pub timestamp: DateTime<Utc>,
}

impl Finding {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        title: String,
        description: String,
        severity: Severity,
        owasp_category: OwaspCategory,
        waf_detected: Option<String>,
        bypass_successful: bool,
        technique: String,
        payload: String,
        response_status: u16,
    ) -> Self {
        let id = uuid::Uuid::new_v4().to_string();
        Self {
            id,
            title,
            description,
            severity,
            owasp_category,
            waf_detected,
            bypass_successful,
            technique,
            payload,
            response_status,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResults {
    pub target: String,
    pub timestamp: DateTime<Utc>,
    pub duration_ms: u64,
    pub waf_detection: Option<crate::waf::detector::WafDetectionResult>,
    pub findings: Vec<Finding>,
    pub summary: ScanSummary,
}

impl ScanResults {
    pub fn new(
        target: String,
        duration_ms: u64,
        waf_detection: Option<crate::waf::detector::WafDetectionResult>,
        findings: Vec<Finding>,
    ) -> Self {
        let summary = ScanSummary::from_findings(&findings);
        Self {
            target,
            timestamp: Utc::now(),
            duration_ms,
            waf_detection,
            findings,
            summary,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanSummary {
    pub total_findings: usize,
    pub critical: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub bypass_success_rate: f32,
}

impl ScanSummary {
    pub fn from_findings(findings: &[Finding]) -> Self {
        let total = findings.len();
        let critical = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Critical))
            .count();
        let high = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::High))
            .count();
        let medium = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Medium))
            .count();
        let low = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Low))
            .count();
        let info = findings
            .iter()
            .filter(|f| matches!(f.severity, Severity::Info))
            .count();

        let bypass_count = findings.iter().filter(|f| f.bypass_successful).count();
        let bypass_success_rate = if total > 0 {
            (bypass_count as f32 / total as f32) * 100.0
        } else {
            0.0
        };

        Self {
            total_findings: total,
            critical,
            high,
            medium,
            low,
            info,
            bypass_success_rate,
        }
    }
}

impl std::fmt::Display for ScanSummary {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Scan Summary")?;
        writeln!(
            f,
            "Total Findings: {}  Critical: {}  High: {}  Medium: {}  Low: {}  Info: {}",
            self.total_findings, self.critical, self.high, self.medium, self.low, self.info
        )?;
        writeln!(f, "Bypass Success Rate: {:.1}%", self.bypass_success_rate)?;
        Ok(())
    }
}
