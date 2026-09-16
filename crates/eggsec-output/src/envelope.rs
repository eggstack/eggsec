//! Normalized report/evidence envelope types (compatibility facade).
//!
//! The canonical contract lives in `eggsec-report-model` (Phase B); this
//! module re-exports it so existing `eggsec_output::envelope::*` paths keep
//! working. Only the `From<&AgentFinding> for FindingRecord` conversion stays
//! here because it couples the contract to the output-side agent finding type
//! (the model must not depend on rendering/analysis types).

pub use eggsec_report_model::{
    BaselineSummary, EvidenceItem, EvidenceKind, EvidenceManifest, EvidenceSource, FindingRecord,
    RedactionPolicy, RedactionState, ReportEnvelope, ToolMetadata,
};

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
