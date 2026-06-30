//! Normalized report/evidence envelope types.
//!
//! This module defines a protocol-neutral contract for report and evidence data
//! that domain crates can target when converting their domain-specific types.
//! The envelope preserves report identity, finding records, evidence manifests,
//! policy summaries, and baseline summaries in a dependency-light, serializable form.
//!
//! ## Design
//!
//! Domain crates may keep domain-specific report structs internally, but they
//! convert into [`ReportEnvelope`] when producing output. The envelope does not
//! require every field to be populated — domain bridges fill what they can and
//! leave the rest as defaults.
//!
//! ## Key Types
//!
//! - [`EvidenceItem`] — A single piece of evidence with kind, source, and redaction state
//! - [`EvidenceManifest`] — Manifest of all evidence items in a report
//! - [`FindingRecord`] — A normalized finding record
//! - [`ReportEnvelope`] — The top-level report container
//! - [`BaselineSummary`] — Summary of baseline comparison results
//! - [`EvidenceKind`] — Category of evidence data
//! - [`EvidenceSource`] — Provenance of evidence
//! - [`RedactionState`] — Sensitivity classification of evidence

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Category of evidence data attached to a finding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    /// HTTP request summary.
    HttpRequest,
    /// HTTP response summary.
    HttpResponse,
    /// HTTP header.
    Header,
    /// Body snippet.
    BodySnippet,
    /// Timing measurement.
    Timing,
    /// Diff output.
    Diff,
    /// Service banner.
    Banner,
    /// DNS record.
    DnsRecord,
    /// TLS/SSL certificate.
    Certificate,
    /// Port state.
    PortState,
    /// Screenshot.
    Screenshot,
    /// File metadata.
    FileMetadata,
    /// Log line.
    LogLine,
    /// Database finding evidence.
    DatabaseFinding,
    /// Mobile manifest/config evidence.
    MobileManifest,
    /// Traffic/proxy evidence.
    TrafficCapture,
    /// Static analysis evidence.
    StaticAnalysis,
    /// Runtime/instrumentation evidence.
    RuntimeInstrumentation,
    /// Correlation evidence linking findings across domains.
    Correlation,
    /// Generic structured evidence.
    Generic,
}

impl std::fmt::Display for EvidenceKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceKind::HttpRequest => write!(f, "HTTP Request"),
            EvidenceKind::HttpResponse => write!(f, "HTTP Response"),
            EvidenceKind::Header => write!(f, "Header"),
            EvidenceKind::BodySnippet => write!(f, "Body Snippet"),
            EvidenceKind::Timing => write!(f, "Timing"),
            EvidenceKind::Diff => write!(f, "Diff"),
            EvidenceKind::Banner => write!(f, "Banner"),
            EvidenceKind::DnsRecord => write!(f, "DNS Record"),
            EvidenceKind::Certificate => write!(f, "Certificate"),
            EvidenceKind::PortState => write!(f, "Port State"),
            EvidenceKind::Screenshot => write!(f, "Screenshot"),
            EvidenceKind::FileMetadata => write!(f, "File Metadata"),
            EvidenceKind::LogLine => write!(f, "Log Line"),
            EvidenceKind::DatabaseFinding => write!(f, "Database Finding"),
            EvidenceKind::MobileManifest => write!(f, "Mobile Manifest"),
            EvidenceKind::TrafficCapture => write!(f, "Traffic Capture"),
            EvidenceKind::StaticAnalysis => write!(f, "Static Analysis"),
            EvidenceKind::RuntimeInstrumentation => write!(f, "Runtime Instrumentation"),
            EvidenceKind::Correlation => write!(f, "Correlation"),
            EvidenceKind::Generic => write!(f, "Generic"),
        }
    }
}

/// Provenance of evidence.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceSource {
    /// Tool or module that produced the evidence (e.g. "eggsec-mobile-lab", "eggsec-db-lab").
    pub tool: String,
    /// Module or sub-component (e.g. "static-analysis", "dynamic-instrumentation").
    pub module: Option<String>,
    /// Optional run/session identifier.
    pub run_id: Option<String>,
}

/// Sensitivity classification for evidence.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionState {
    /// Evidence contains no sensitive data; full content is safe to include.
    #[default]
    None,
    /// Evidence is fully redacted; only a placeholder is included.
    FullyRedacted,
    /// Evidence is partially redacted; sensitive fields are masked.
    PartiallyRedacted,
    /// Evidence is summarized; original content is replaced with a summary.
    Summarized,
}

/// Manifest-level redaction policy describing how evidence should be treated.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RedactionPolicy {
    /// No redaction applied; all evidence is included as-is.
    #[default]
    None,
    /// Redact all evidence items regardless of individual state.
    RedactAll,
    /// Redact only items marked as sensitive; leave others intact.
    RedactSensitive,
    /// Summarize all evidence items rather than including raw content.
    SummarizeAll,
    /// Domain-specific redaction logic; individual item states take precedence.
    DomainSpecific,
}

/// A single piece of evidence supporting a finding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceItem {
    /// Unique identifier for this evidence item.
    pub id: String,
    /// Category of evidence data.
    pub kind: EvidenceKind,
    /// Provenance of the evidence.
    pub source: EvidenceSource,
    /// Human-readable summary of what this evidence shows.
    pub summary: String,
    /// Optional structured data reference (file path, URL, or inline JSON).
    pub data_ref: Option<String>,
    /// Sensitivity classification.
    pub redaction: RedactionState,
    /// Optional collection timestamp.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub collected_at: Option<DateTime<Utc>>,
}

impl EvidenceItem {
    /// Create a new evidence item.
    pub fn new(
        id: impl Into<String>,
        kind: EvidenceKind,
        source: EvidenceSource,
        summary: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            kind,
            source,
            summary: summary.into(),
            data_ref: None,
            redaction: RedactionState::default(),
            collected_at: None,
        }
    }

    /// Set the data reference.
    pub fn with_data_ref(mut self, data_ref: impl Into<String>) -> Self {
        self.data_ref = Some(data_ref.into());
        self
    }

    /// Set the redaction state.
    pub fn with_redaction(mut self, redaction: RedactionState) -> Self {
        self.redaction = redaction;
        self
    }

    /// Set the collection timestamp.
    pub fn with_collected_at(mut self, collected_at: DateTime<Utc>) -> Self {
        self.collected_at = Some(collected_at);
        self
    }
}

/// Manifest of all evidence items in a report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceManifest {
    /// Unique identifier for this manifest.
    pub bundle_id: String,
    /// Operation ID that produced this evidence.
    pub operation_id: String,
    /// Domain ID, if applicable.
    pub domain_id: Option<String>,
    /// Target or artifact identity.
    pub target: Option<String>,
    /// When this manifest was generated.
    pub generated_at: DateTime<Utc>,
    /// Total number of evidence items.
    pub total_items: usize,
    /// Number of redacted items.
    pub redacted_items: usize,
    /// Manifest-level redaction policy governing how evidence is treated.
    pub redaction_policy: RedactionPolicy,
    /// Producer version (e.g. "0.1.0").
    pub producer_version: Option<String>,
    /// Optional policy/enforcement correlation ID.
    pub policy_correlation_id: Option<String>,
}

impl Default for EvidenceManifest {
    fn default() -> Self {
        Self {
            bundle_id: String::new(),
            operation_id: String::new(),
            domain_id: None,
            target: None,
            generated_at: Utc::now(),
            total_items: 0,
            redacted_items: 0,
            redaction_policy: RedactionPolicy::default(),
            producer_version: None,
            policy_correlation_id: None,
        }
    }
}

impl EvidenceManifest {
    /// Build a manifest from a list of evidence items with the given redaction policy.
    pub fn with_redaction_policy(
        operation_id: impl Into<String>,
        items: &[EvidenceItem],
        policy: RedactionPolicy,
    ) -> Self {
        let redacted_items = items
            .iter()
            .filter(|i| i.redaction != RedactionState::None)
            .count();
        Self {
            bundle_id: uuid::Uuid::new_v4().to_string(),
            operation_id: operation_id.into(),
            domain_id: None,
            target: None,
            generated_at: Utc::now(),
            total_items: items.len(),
            redacted_items,
            redaction_policy: policy,
            producer_version: None,
            policy_correlation_id: None,
        }
    }

    /// Build a manifest from a list of evidence items with default redaction policy.
    pub fn from_items(operation_id: impl Into<String>, items: &[EvidenceItem]) -> Self {
        Self::with_redaction_policy(operation_id, items, RedactionPolicy::default())
    }
}

/// A normalized finding record.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FindingRecord {
    /// Unique identifier for this finding.
    pub id: String,
    /// Domain that produced this finding (e.g. "db-pentest", "mobile-static").
    pub domain: String,
    /// Operation ID that produced this finding.
    pub operation_id: String,
    /// Severity rating.
    pub severity: eggsec_core::types::Severity,
    /// Short human-readable title.
    pub title: String,
    /// Detailed description of the finding.
    pub description: String,
    /// Supporting evidence items.
    pub evidence: Vec<EvidenceItem>,
    /// Recommended remediation, if available.
    pub remediation: Option<String>,
    /// References (CWE, OWASP, CVE, URLs).
    pub references: Vec<String>,
    /// Category or classification string (domain-specific).
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub category: String,
    /// Location or endpoint where the finding was observed.
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub location: String,
}

impl FindingRecord {
    /// Create a minimal finding record.
    pub fn new(
        id: impl Into<String>,
        domain: impl Into<String>,
        operation_id: impl Into<String>,
        severity: eggsec_core::types::Severity,
        title: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self {
            id: id.into(),
            domain: domain.into(),
            operation_id: operation_id.into(),
            severity,
            title: title.into(),
            description: description.into(),
            evidence: Vec::new(),
            remediation: None,
            references: Vec::new(),
            category: String::new(),
            location: String::new(),
        }
    }

    /// Add an evidence item.
    pub fn with_evidence(mut self, evidence: EvidenceItem) -> Self {
        self.evidence.push(evidence);
        self
    }

    /// Set the remediation.
    pub fn with_remediation(mut self, remediation: impl Into<String>) -> Self {
        self.remediation = Some(remediation.into());
        self
    }

    /// Add a reference (CWE, OWASP, CVE, or URL).
    pub fn with_reference(mut self, reference: impl Into<String>) -> Self {
        self.references.push(reference.into());
        self
    }

    /// Set the category.
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = category.into();
        self
    }

    /// Set the location.
    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = location.into();
        self
    }
}

/// Summary of baseline comparison results.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct BaselineSummary {
    /// Unique identifier for this baseline comparison.
    pub baseline_id: String,
    /// Source of the baseline (e.g. "db-pentest", "mobile-dynamic").
    pub baseline_source: String,
    /// When this comparison was performed.
    pub compared_at: DateTime<Utc>,
    /// Number of new findings since baseline.
    pub added: usize,
    /// Number of resolved findings since baseline.
    pub resolved: usize,
    /// Number of unchanged findings.
    pub unchanged: usize,
    /// Per-severity delta counts (severity name -> count change).
    pub severity_deltas: std::collections::HashMap<String, i64>,
    /// Whether this comparison indicates a regression.
    pub is_regression: bool,
    /// Whether this comparison indicates improvement.
    pub is_improvement: bool,
    /// Optional human-readable summary.
    pub summary: Option<String>,
}

impl BaselineSummary {
    /// Create a new baseline summary.
    pub fn new(baseline_source: impl Into<String>) -> Self {
        Self {
            baseline_id: uuid::Uuid::new_v4().to_string(),
            baseline_source: baseline_source.into(),
            compared_at: Utc::now(),
            added: 0,
            resolved: 0,
            unchanged: 0,
            severity_deltas: std::collections::HashMap::new(),
            is_regression: false,
            is_improvement: false,
            summary: None,
        }
    }

    /// Set the summary text.
    pub fn with_summary(mut self, summary: impl Into<String>) -> Self {
        self.summary = Some(summary.into());
        self
    }

    /// Compute regression/improvement flags from counts.
    pub fn compute_flags(&mut self) {
        self.is_regression = self.added > 0 && self.resolved == 0;
        self.is_improvement = self.resolved > 0 && self.added == 0;
    }
}

/// Tool/version metadata for a report.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolMetadata {
    /// Tool name (e.g. "eggsec").
    pub tool_name: String,
    /// Tool version.
    pub tool_version: Option<String>,
    /// Eggsec version.
    pub eggsec_version: Option<String>,
}

/// The normalized report envelope.
///
/// This is the top-level container that domain bridges produce from their
/// domain-specific types. It preserves report identity, finding records,
/// evidence manifests, policy summaries, and baseline summaries.
///
/// Not every field needs to be populated. Domain bridges fill what they can
/// and leave the rest as defaults.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportEnvelope {
    /// Unique report identifier (deterministic or generated).
    pub report_id: String,
    /// Operation ID that produced this report.
    pub operation_id: String,
    /// Domain ID, if applicable.
    pub domain_id: Option<String>,
    /// Target or local artifact identifier.
    pub target: Option<String>,
    /// When this report was generated.
    pub generated_at: DateTime<Utc>,
    /// Finding records.
    pub findings: Vec<FindingRecord>,
    /// Evidence manifest.
    pub evidence_manifest: EvidenceManifest,
    /// Policy/enforcement summary, if available.
    pub policy_summary: Option<super::PolicySummary>,
    /// Baseline/regression summary, if available.
    pub baseline: Option<BaselineSummary>,
    /// Tool/version metadata, if available.
    pub tool_metadata: Option<ToolMetadata>,
}

impl ReportEnvelope {
    /// Create a new empty report envelope.
    pub fn new(operation_id: impl Into<String>) -> Self {
        let op_id = operation_id.into();
        Self {
            report_id: uuid::Uuid::new_v4().to_string(),
            operation_id: op_id.clone(),
            domain_id: None,
            target: None,
            generated_at: Utc::now(),
            findings: Vec::new(),
            evidence_manifest: EvidenceManifest {
                operation_id: op_id,
                ..Default::default()
            },
            policy_summary: None,
            baseline: None,
            tool_metadata: None,
        }
    }

    /// Set the domain ID.
    pub fn with_domain_id(mut self, domain_id: impl Into<String>) -> Self {
        self.domain_id = Some(domain_id.into());
        self
    }

    /// Set the target.
    pub fn with_target(mut self, target: impl Into<String>) -> Self {
        self.target = Some(target.into());
        self
    }

    /// Add a finding record.
    pub fn with_finding(mut self, finding: FindingRecord) -> Self {
        self.findings.push(finding);
        self
    }

    /// Set the policy summary.
    pub fn with_policy_summary(mut self, policy_summary: super::PolicySummary) -> Self {
        self.policy_summary = Some(policy_summary);
        self
    }

    /// Set the baseline summary.
    pub fn with_baseline(mut self, baseline: BaselineSummary) -> Self {
        self.baseline = Some(baseline);
        self
    }

    /// Set the tool metadata.
    pub fn with_tool_metadata(mut self, tool_metadata: ToolMetadata) -> Self {
        self.tool_metadata = Some(tool_metadata);
        self
    }

    /// Set the manifest-level redaction policy.
    pub fn with_redaction_policy(mut self, policy: RedactionPolicy) -> Self {
        self.evidence_manifest.redaction_policy = policy;
        self
    }

    /// Rebuild the evidence manifest from the current findings.
    /// Preserves the existing redaction policy, bundle_id, and producer_version.
    pub fn refresh_evidence_manifest(&mut self) {
        let all_evidence: Vec<EvidenceItem> = self
            .findings
            .iter()
            .flat_map(|f| f.evidence.iter().cloned())
            .collect();
        let old_policy = self.evidence_manifest.redaction_policy;
        let old_producer_version = self.evidence_manifest.producer_version.clone();
        self.evidence_manifest = EvidenceManifest::from_items(&self.operation_id, &all_evidence);
        self.evidence_manifest.domain_id = self.domain_id.clone();
        self.evidence_manifest.target = self.target.clone();
        self.evidence_manifest.redaction_policy = old_policy;
        self.evidence_manifest.producer_version = old_producer_version;
    }

    /// Serialize the envelope to pretty-printed JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Deserialize an envelope from JSON.
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

impl Default for ReportEnvelope {
    fn default() -> Self {
        Self::new("unknown")
    }
}

/// Convert an `AgentFinding` to a `FindingRecord`.
impl From<&super::AgentFinding> for FindingRecord {
    fn from(f: &super::AgentFinding) -> Self {
        let mut record = FindingRecord::new(
            &f.id,
            &f.vulnerability_type,
            &f.tool_id,
            f.severity,
            &f.title,
            &f.description,
        );
        record.location = f.endpoint.clone();
        record.category = f.vulnerability_type.clone();
        for cwe in &f.cwe_ids {
            record = record.with_reference(cwe);
        }
        if !f.remediation.summary.is_empty() {
            record = record.with_remediation(&f.remediation.summary);
        }
        if let Some(ref request) = f.evidence.request {
            if !request.is_empty() {
                let ev_id = format!("{}-evidence-0", f.id);
                let source = EvidenceSource {
                    tool: f.tool_id.clone(),
                    module: None,
                    run_id: None,
                };
                record = record.with_evidence(
                    EvidenceItem::new(ev_id, EvidenceKind::HttpRequest, source, request)
                        .with_data_ref(request.clone()),
                );
            }
        }
        record
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_core::types::Severity;

    #[test]
    fn evidence_item_creation() {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let item = EvidenceItem::new("ev-1", EvidenceKind::HttpRequest, source, "test evidence");
        assert_eq!(item.id, "ev-1");
        assert_eq!(item.kind, EvidenceKind::HttpRequest);
        assert_eq!(item.redaction, RedactionState::None);
        assert!(item.data_ref.is_none());
    }

    #[test]
    fn evidence_item_with_redaction() {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let item = EvidenceItem::new("ev-2", EvidenceKind::Generic, source, "redacted")
            .with_redaction(RedactionState::FullyRedacted);
        assert_eq!(item.redaction, RedactionState::FullyRedacted);
    }

    #[test]
    fn finding_record_creation() {
        let record = FindingRecord::new(
            "f-1",
            "db-pentest",
            "db-check",
            Severity::High,
            "Test Finding",
            "Description",
        );
        assert_eq!(record.id, "f-1");
        assert_eq!(record.domain, "db-pentest");
        assert_eq!(record.severity, Severity::High);
        assert!(record.evidence.is_empty());
    }

    #[test]
    fn finding_record_with_evidence() {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let item = EvidenceItem::new("ev-1", EvidenceKind::DatabaseFinding, source, "test");
        let record = FindingRecord::new(
            "f-1",
            "db-pentest",
            "db-check",
            Severity::Medium,
            "Finding",
            "Desc",
        )
        .with_evidence(item);
        assert_eq!(record.evidence.len(), 1);
    }

    #[test]
    fn evidence_manifest_from_items() {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let items = vec![
            EvidenceItem::new("ev-1", EvidenceKind::HttpRequest, source.clone(), "a"),
            EvidenceItem::new("ev-2", EvidenceKind::Generic, source, "b")
                .with_redaction(RedactionState::FullyRedacted),
        ];
        let manifest = EvidenceManifest::from_items("op-1", &items);
        assert_eq!(manifest.total_items, 2);
        assert_eq!(manifest.redacted_items, 1);
        assert_eq!(manifest.operation_id, "op-1");
        assert_eq!(manifest.redaction_policy, RedactionPolicy::None);
    }

    #[test]
    fn evidence_manifest_with_redaction_policy() {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let items = vec![
            EvidenceItem::new("ev-1", EvidenceKind::DatabaseFinding, source.clone(), "a"),
            EvidenceItem::new("ev-2", EvidenceKind::Generic, source, "b")
                .with_redaction(RedactionState::FullyRedacted),
        ];
        let manifest =
            EvidenceManifest::with_redaction_policy("op-1", &items, RedactionPolicy::RedactAll);
        assert_eq!(manifest.total_items, 2);
        assert_eq!(manifest.redacted_items, 1);
        assert_eq!(manifest.redaction_policy, RedactionPolicy::RedactAll);
    }

    #[test]
    fn baseline_summary_defaults() {
        let mut summary = BaselineSummary::new("db-pentest");
        assert_eq!(summary.baseline_source, "db-pentest");
        assert!(!summary.is_regression);
        assert!(!summary.is_improvement);
        summary.added = 3;
        summary.resolved = 0;
        summary.compute_flags();
        assert!(summary.is_regression);
    }

    #[test]
    fn report_envelope_creation() {
        let envelope = ReportEnvelope::new("scan-ports")
            .with_domain_id("scanner")
            .with_target("10.0.0.1");
        assert_eq!(envelope.operation_id, "scan-ports");
        assert_eq!(envelope.domain_id.as_deref(), Some("scanner"));
        assert_eq!(envelope.target.as_deref(), Some("10.0.0.1"));
        assert!(envelope.findings.is_empty());
    }

    #[test]
    fn report_envelope_serialization_roundtrip() {
        let envelope = ReportEnvelope::new("test-op")
            .with_domain_id("test-domain")
            .with_finding(FindingRecord::new(
                "f-1",
                "test",
                "test-op",
                Severity::High,
                "Title",
                "Desc",
            ));
        let json = envelope.to_json().unwrap();
        let deserialized = ReportEnvelope::from_json(&json).unwrap();
        assert_eq!(deserialized.operation_id, "test-op");
        assert_eq!(deserialized.findings.len(), 1);
        assert_eq!(deserialized.findings[0].severity, Severity::High);
    }

    #[test]
    fn refresh_evidence_manifest() {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let item = EvidenceItem::new("ev-1", EvidenceKind::HttpRequest, source, "test");
        let finding =
            FindingRecord::new("f-1", "test", "op-1", Severity::Low, "F", "D").with_evidence(item);
        let mut envelope = ReportEnvelope::new("op-1")
            .with_domain_id("test")
            .with_target("host")
            .with_finding(finding);
        envelope.refresh_evidence_manifest();
        assert_eq!(envelope.evidence_manifest.total_items, 1);
        assert_eq!(
            envelope.evidence_manifest.domain_id.as_deref(),
            Some("test")
        );
    }

    #[test]
    fn severity_preserved_in_roundtrip() {
        let record = FindingRecord::new("f-1", "test", "op-1", Severity::Critical, "Title", "Desc");
        let envelope = ReportEnvelope::new("op-1").with_finding(record);
        let json = envelope.to_json().unwrap();
        let deserialized = ReportEnvelope::from_json(&json).unwrap();
        assert_eq!(deserialized.findings[0].severity, Severity::Critical);
    }
}
