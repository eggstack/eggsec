//! Canonical finding and evidence schema.
//!
//! This module defines the canonical [`Finding`] model and related types
//! for representing security findings across all Slapper modules.
//!
//! ## Design
//!
//! The canonical `Finding` is a superset of what individual modules produce.
//! Existing module-specific types (e.g. `tool::finding::Finding`,
//! `output::agent::AgentFinding`, `workflow::finding::Finding`) are NOT
//! migrated yet - this module defines the target schema for future unification.
//!
//! ## Key Types
//!
//! - [`Finding`] - The canonical finding record
//! - [`Confidence`] - Confidence level of a finding
//! - [`EvidenceKind`] - Category of evidence data
//! - [`Evidence`] - A piece of supporting evidence
//! - [`AffectedAsset`] - The asset affected by a finding
//! - [`FindingLocation`] - Where the finding was observed
//! - [`Reproduction`] - Steps to reproduce the finding
//! - [`FindingType`] - High-level classification of the finding
//! - [`FindingSource`] - Which tool/module produced the finding

pub mod lifecycle;
pub mod store;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Confidence level for a finding.
///
/// This is distinct from `Severity` (which rates impact).
/// Confidence rates how sure we are that the finding is a true positive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Confidence {
    Confirmed,
    High,
    Medium,
    Low,
    Informational,
}

impl Confidence {
    /// Numeric score from 0.0 (Informational) to 1.0 (Confirmed).
    pub fn score(&self) -> f32 {
        match self {
            Confidence::Confirmed => 1.0,
            Confidence::High => 0.75,
            Confidence::Medium => 0.5,
            Confidence::Low => 0.25,
            Confidence::Informational => 0.0,
        }
    }

    /// Derive confidence from a ratio of successful detections to total tests.
    pub fn from_ratio(found: usize, tested: usize) -> Self {
        if tested == 0 {
            return Confidence::Informational;
        }
        let ratio = found as f32 / tested as f32;
        match ratio {
            r if r >= 0.9 => Confidence::Confirmed,
            r if r >= 0.6 => Confidence::High,
            r if r >= 0.3 => Confidence::Medium,
            r if r > 0.0 => Confidence::Low,
            _ => Confidence::Informational,
        }
    }
}

impl std::fmt::Display for Confidence {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Confidence::Confirmed => write!(f, "confirmed"),
            Confidence::High => write!(f, "high"),
            Confidence::Medium => write!(f, "medium"),
            Confidence::Low => write!(f, "low"),
            Confidence::Informational => write!(f, "informational"),
        }
    }
}

/// Category of evidence data attached to a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    HttpRequest,
    HttpResponse,
    Header,
    BodySnippet,
    Timing,
    Diff,
    Banner,
    DnsRecord,
    Certificate,
    PortState,
    Screenshot,
    FilePath,
    LogLine,
}

impl std::fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceKind::HttpRequest => write!(f, "http_request"),
            EvidenceKind::HttpResponse => write!(f, "http_response"),
            EvidenceKind::Header => write!(f, "header"),
            EvidenceKind::BodySnippet => write!(f, "body_snippet"),
            EvidenceKind::Timing => write!(f, "timing"),
            EvidenceKind::Diff => write!(f, "diff"),
            EvidenceKind::Banner => write!(f, "banner"),
            EvidenceKind::DnsRecord => write!(f, "dns_record"),
            EvidenceKind::Certificate => write!(f, "certificate"),
            EvidenceKind::PortState => write!(f, "port_state"),
            EvidenceKind::Screenshot => write!(f, "screenshot"),
            EvidenceKind::FilePath => write!(f, "file_path"),
            EvidenceKind::LogLine => write!(f, "log_line"),
        }
    }
}

/// A piece of evidence supporting a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Evidence {
    /// What kind of evidence this is.
    pub kind: EvidenceKind,
    /// Whether the data has been redacted of sensitive content.
    pub redacted: bool,
    /// Human-readable summary of what this evidence shows.
    pub summary: String,
    /// The actual evidence data (structure depends on `kind`).
    pub data: serde_json::Value,
}

impl Evidence {
    /// Create a new evidence entry, marking it as not redacted.
    pub fn new(kind: EvidenceKind, summary: impl Into<String>, data: serde_json::Value) -> Self {
        Self {
            kind,
            redacted: false,
            summary: summary.into(),
            data,
        }
    }

    /// Create a redacted evidence entry with a placeholder.
    pub fn redacted(kind: EvidenceKind, summary: impl Into<String>) -> Self {
        Self {
            kind,
            redacted: true,
            summary: summary.into(),
            data: serde_json::Value::Null,
        }
    }
}

/// The asset affected by a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AffectedAsset {
    /// Type of asset (e.g. "web_application", "host", "api_endpoint", "container").
    pub asset_type: String,
    /// Primary identifier (e.g. URL, hostname, IP, container ID).
    pub identifier: String,
    /// Hostname or IP, if applicable.
    pub host: Option<String>,
    /// Port number, if applicable.
    pub port: Option<u16>,
    /// Protocol (e.g. "https", "tcp", "grpc").
    pub protocol: Option<String>,
}

/// Where within the affected asset the finding was observed.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FindingLocation {
    /// Full URL, if applicable.
    pub url: Option<String>,
    /// Path component (e.g. "/api/users").
    pub path: Option<String>,
    /// Parameter name, if applicable.
    pub parameter: Option<String>,
    /// Header name, if applicable.
    pub header: Option<String>,
    /// HTTP method, if applicable.
    pub method: Option<String>,
    /// Line number in a file, if applicable.
    pub line: Option<u32>,
    /// File path, if applicable.
    pub file: Option<String>,
}

/// Steps to reproduce the finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reproduction {
    /// Ordered steps to reproduce.
    pub steps: Vec<String>,
    /// What was expected to happen.
    pub expected: Option<String>,
    /// What actually happened.
    pub actual: Option<String>,
}

/// High-level classification of a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FindingType {
    Vulnerability,
    Misconfiguration,
    InformationLeak,
    PolicyViolation,
    AssetDiscovery,
    ServiceDetection,
    WafDetection,
    FuzzResult,
    ScanResult,
}

impl std::fmt::Display for FindingType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FindingType::Vulnerability => write!(f, "vulnerability"),
            FindingType::Misconfiguration => write!(f, "misconfiguration"),
            FindingType::InformationLeak => write!(f, "information_leak"),
            FindingType::PolicyViolation => write!(f, "policy_violation"),
            FindingType::AssetDiscovery => write!(f, "asset_discovery"),
            FindingType::ServiceDetection => write!(f, "service_detection"),
            FindingType::WafDetection => write!(f, "waf_detection"),
            FindingType::FuzzResult => write!(f, "fuzz_result"),
            FindingType::ScanResult => write!(f, "scan_result"),
        }
    }
}

/// Which tool and module produced a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingSource {
    /// Tool name (e.g. "slapper", "nmap", "custom-plugin").
    pub tool: String,
    /// Module within the tool (e.g. "fuzzer", "scanner", "waf").
    pub module: String,
    /// Optional run/session identifier for grouping.
    pub run_id: Option<String>,
}

/// The canonical finding record.
///
/// This is the unified schema that all Slapper modules should eventually produce.
/// It is a superset of existing module-specific types and includes fields for
/// every kind of security finding the toolkit can produce.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finding {
    /// Unique identifier for this finding instance.
    pub id: String,
    /// Stable fingerprint for deduplication across scan runs.
    pub fingerprint: String,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description of the finding.
    pub description: String,
    /// Severity rating (uses the canonical `Severity` from `types`).
    pub severity: crate::types::Severity,
    /// How confident we are this is a true positive.
    pub confidence: Confidence,
    /// High-level classification.
    pub finding_type: FindingType,
    /// CWE identifier (e.g. "CWE-79"), if applicable.
    pub cwe: Option<String>,
    /// OWASP category (e.g. "A03:2021-Injection"), if applicable.
    pub owasp: Option<String>,
    /// CVE identifier (e.g. "CVE-2024-1234"), if applicable.
    pub cve: Option<String>,
    /// The affected asset.
    pub affected_asset: AffectedAsset,
    /// Where within the asset the finding was observed.
    pub location: FindingLocation,
    /// Supporting evidence.
    pub evidence: Vec<Evidence>,
    /// Steps to reproduce, if available.
    pub reproduction: Option<Reproduction>,
    /// Recommended remediation, if available.
    pub remediation: Option<String>,
    /// When this finding was discovered.
    pub discovered_at: DateTime<Utc>,
    /// Which tool/module produced this finding.
    pub source: FindingSource,
    /// Freeform tags for filtering and grouping.
    pub tags: Vec<String>,
    /// Additional metadata as key-value pairs.
    pub metadata: serde_json::Value,
}

impl Finding {
    /// Generate a stable fingerprint for this finding.
    ///
    /// The fingerprint is deterministic across scan runs when the same issue
    /// is rediscovered on the same asset. It hashes the finding type, asset
    /// identifier, location, CWE, and normalized title.
    pub fn compute_fingerprint(&self) -> String {
        use sha2::{Digest, Sha256};

        let mut hasher = Sha256::new();

        hasher.update(self.affected_asset.asset_type.as_bytes());
        hasher.update(self.affected_asset.identifier.to_lowercase().as_bytes());
        hasher.update(format!("{:?}", self.finding_type).as_bytes());

        if let Some(ref path) = self.location.path {
            hasher.update(path.to_lowercase().as_bytes());
        }
        if let Some(ref param) = self.location.parameter {
            hasher.update(param.to_lowercase().as_bytes());
        }
        if let Some(ref cwe) = self.cwe {
            hasher.update(cwe.as_bytes());
        }

        hasher.update(self.title.to_lowercase().trim().as_bytes());

        hex::encode(hasher.finalize())
    }

    /// Recompute and store the fingerprint.
    pub fn refresh_fingerprint(&mut self) {
        self.fingerprint = self.compute_fingerprint();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_finding() -> Finding {
        Finding {
            id: "test-1".to_string(),
            fingerprint: String::new(),
            title: "Test Finding".to_string(),
            description: "A test finding".to_string(),
            severity: crate::types::Severity::Medium,
            confidence: Confidence::High,
            finding_type: FindingType::Vulnerability,
            cwe: Some("CWE-79".to_string()),
            owasp: None,
            cve: None,
            affected_asset: AffectedAsset {
                asset_type: "web_application".to_string(),
                identifier: "https://example.com".to_string(),
                host: Some("example.com".to_string()),
                port: Some(443),
                protocol: Some("https".to_string()),
            },
            location: FindingLocation {
                url: Some("https://example.com/search".to_string()),
                path: Some("/search".to_string()),
                parameter: Some("q".to_string()),
                header: None,
                method: Some("GET".to_string()),
                line: None,
                file: None,
            },
            evidence: vec![],
            reproduction: None,
            remediation: None,
            discovered_at: Utc::now(),
            source: FindingSource {
                tool: "test".to_string(),
                module: "test".to_string(),
                run_id: None,
            },
            tags: vec![],
            metadata: serde_json::Value::Null,
        }
    }

    #[test]
    fn fingerprint_is_stable() {
        let f1 = create_test_finding();
        let f2 = create_test_finding();
        assert_eq!(f1.compute_fingerprint(), f2.compute_fingerprint());
    }

    #[test]
    fn different_findings_have_different_fingerprints() {
        let mut f1 = create_test_finding();
        let mut f2 = create_test_finding();
        f2.location.path = Some("/different".to_string());
        assert_ne!(f1.compute_fingerprint(), f2.compute_fingerprint());
    }

    #[test]
    fn fingerprint_changes_with_title() {
        let mut f1 = create_test_finding();
        let mut f2 = create_test_finding();
        f2.title = "Different Title".to_string();
        assert_ne!(f1.compute_fingerprint(), f2.compute_fingerprint());
    }

    #[test]
    fn fingerprint_changes_with_cwe() {
        let mut f1 = create_test_finding();
        let mut f2 = create_test_finding();
        f2.cwe = Some("CWE-89".to_string());
        assert_ne!(f1.compute_fingerprint(), f2.compute_fingerprint());
    }

    #[test]
    fn fingerprint_changes_with_finding_type() {
        let mut f1 = create_test_finding();
        let mut f2 = create_test_finding();
        f2.finding_type = FindingType::Misconfiguration;
        assert_ne!(f1.compute_fingerprint(), f2.compute_fingerprint());
    }

    #[test]
    fn fingerprint_is_case_insensitive_for_title() {
        let mut f1 = create_test_finding();
        let mut f2 = create_test_finding();
        f2.title = "test finding".to_string();
        assert_eq!(f1.compute_fingerprint(), f2.compute_fingerprint());
    }

    #[test]
    fn confidence_scores() {
        assert_eq!(Confidence::Confirmed.score(), 1.0);
        assert_eq!(Confidence::High.score(), 0.75);
        assert_eq!(Confidence::Medium.score(), 0.5);
        assert_eq!(Confidence::Low.score(), 0.25);
        assert_eq!(Confidence::Informational.score(), 0.0);
    }

    #[test]
    fn confidence_from_ratio() {
        assert_eq!(Confidence::from_ratio(10, 10), Confidence::Confirmed);
        assert_eq!(Confidence::from_ratio(7, 10), Confidence::High);
        assert_eq!(Confidence::from_ratio(4, 10), Confidence::Medium);
        assert_eq!(Confidence::from_ratio(1, 10), Confidence::Low);
        assert_eq!(Confidence::from_ratio(0, 10), Confidence::Informational);
        assert_eq!(Confidence::from_ratio(0, 0), Confidence::Informational);
    }

    #[test]
    fn finding_serializes() {
        let finding = create_test_finding();
        let json = serde_json::to_string(&finding).unwrap();
        assert!(json.contains("test-1"));
        assert!(json.contains("CWE-79"));
    }

    #[test]
    fn finding_deserializes() {
        let finding = create_test_finding();
        let json = serde_json::to_string(&finding).unwrap();
        let deserialized: Finding = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, finding.id);
        assert_eq!(deserialized.title, finding.title);
        assert_eq!(deserialized.severity, finding.severity);
    }

    #[test]
    fn refresh_fingerprint() {
        let mut finding = create_test_finding();
        assert!(finding.fingerprint.is_empty());
        finding.refresh_fingerprint();
        assert!(!finding.fingerprint.is_empty());
        assert_eq!(finding.fingerprint, finding.compute_fingerprint());
    }

    #[test]
    fn evidence_new() {
        let ev = Evidence::new(
            EvidenceKind::HttpResponse,
            "200 OK",
            serde_json::json!({"status": 200}),
        );
        assert!(!ev.redacted);
        assert_eq!(ev.kind, EvidenceKind::HttpResponse);
    }

    #[test]
    fn evidence_redacted() {
        let ev = Evidence::redacted(EvidenceKind::HttpRequest, "Sensitive request");
        assert!(ev.redacted);
        assert_eq!(ev.data, serde_json::Value::Null);
    }
}
