//! Integration tests for the normalized report/evidence envelope model.
//! Covers Phase 9 Work Item 8: report/evidence consistency tests.

use eggsec_core::types::Severity;
use eggsec_output::envelope::*;

#[test]
fn evidence_manifest_serialization_roundtrip() {
    let source = EvidenceSource {
        tool: "test-tool".to_string(),
        module: Some("test-module".to_string()),
        run_id: Some("run-1".to_string()),
    };
    let items = vec![
        EvidenceItem::new(
            "ev-1",
            EvidenceKind::HttpRequest,
            source.clone(),
            "request data",
        )
        .with_redaction(RedactionState::None),
        EvidenceItem::new("ev-2", EvidenceKind::DatabaseFinding, source, "db data")
            .with_redaction(RedactionState::FullyRedacted),
    ];
    let manifest = EvidenceManifest::from_items("op-1", &items);

    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: EvidenceManifest = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.total_items, 2);
    assert_eq!(deserialized.redacted_items, 1);
    assert_eq!(deserialized.operation_id, "op-1");
    assert_eq!(deserialized.redaction_policy, RedactionPolicy::None);
}

#[test]
fn redaction_state_preserved_through_serialization() {
    let source = EvidenceSource {
        tool: "test".to_string(),
        module: None,
        run_id: None,
    };
    let item = EvidenceItem::new("ev-1", EvidenceKind::Generic, source, "sensitive data")
        .with_redaction(RedactionState::PartiallyRedacted);

    let json = serde_json::to_string(&item).unwrap();
    let deserialized: EvidenceItem = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.redaction, RedactionState::PartiallyRedacted);
}

#[test]
fn baseline_summary_serialization_roundtrip() {
    let mut summary = BaselineSummary::new("db-pentest");
    summary.added = 5;
    summary.resolved = 2;
    summary.unchanged = 10;
    summary.is_regression = true;
    summary.is_improvement = false;
    summary.severity_deltas.insert("high".to_string(), 3);
    summary.severity_deltas.insert("low".to_string(), -1);
    summary.summary = Some("3 new high findings".to_string());

    let json = serde_json::to_string(&summary).unwrap();
    let deserialized: BaselineSummary = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.added, 5);
    assert_eq!(deserialized.resolved, 2);
    assert_eq!(deserialized.unchanged, 10);
    assert!(deserialized.is_regression);
    assert!(!deserialized.is_improvement);
    assert_eq!(deserialized.severity_deltas.get("high"), Some(&3));
    assert_eq!(deserialized.severity_deltas.get("low"), Some(&-1));
    assert_eq!(deserialized.summary.as_deref(), Some("3 new high findings"));
}

#[test]
fn finding_record_preserves_severity_and_evidence() {
    let source = EvidenceSource {
        tool: "test".to_string(),
        module: None,
        run_id: None,
    };
    let evidence = EvidenceItem::new("ev-1", EvidenceKind::HttpRequest, source, "request data")
        .with_data_ref("https://example.com/api");

    let record = FindingRecord::new(
        "f-1",
        "db-pentest",
        "db-check",
        Severity::Critical,
        "SQL Injection",
        "Unparameterized query allows SQL injection",
    )
    .with_evidence(evidence)
    .with_remediation("Use parameterized queries")
    .with_reference("CWE-89")
    .with_category("db-postgres-sqli")
    .with_location("localhost:5432");

    let json = serde_json::to_string(&record).unwrap();
    let deserialized: FindingRecord = serde_json::from_str(&json).unwrap();

    assert_eq!(deserialized.severity, Severity::Critical);
    assert_eq!(deserialized.evidence.len(), 1);
    assert_eq!(deserialized.evidence[0].kind, EvidenceKind::HttpRequest);
    assert_eq!(
        deserialized.evidence[0].data_ref.as_deref(),
        Some("https://example.com/api")
    );
    assert_eq!(
        deserialized.remediation.as_deref(),
        Some("Use parameterized queries")
    );
    assert_eq!(deserialized.references, vec!["CWE-89"]);
    assert_eq!(deserialized.category, "db-postgres-sqli");
    assert_eq!(deserialized.location, "localhost:5432");
}

#[test]
fn report_envelope_full_roundtrip() {
    let source = EvidenceSource {
        tool: "eggsec-db-lab".to_string(),
        module: Some("db-pentest".to_string()),
        run_id: None,
    };

    let finding = FindingRecord::new(
        "f-1",
        "db-pentest",
        "db-pentest",
        Severity::High,
        "Dangerous Extension",
        "Postgres extension allows arbitrary code execution",
    )
    .with_evidence(
        EvidenceItem::new(
            "ev-1",
            EvidenceKind::DatabaseFinding,
            source,
            "extension pg_exec",
        )
        .with_redaction(RedactionState::PartiallyRedacted),
    )
    .with_remediation("Revoke dangerous extensions")
    .with_reference("CWE-94")
    .with_category("db-postgres-misconfig-dangerous-extension");

    let mut baseline = BaselineSummary::new("db-pentest");
    baseline.added = 1;
    baseline.resolved = 0;
    baseline.unchanged = 5;
    baseline.is_regression = true;

    let envelope = ReportEnvelope::new("db-pentest")
        .with_domain_id("db-pentest")
        .with_target("localhost:5432")
        .with_finding(finding)
        .with_baseline(baseline)
        .with_tool_metadata(ToolMetadata {
            tool_name: "eggsec-db-lab".to_string(),
            tool_version: None,
            eggsec_version: Some("0.1.0".to_string()),
        });

    let json = envelope.to_json().unwrap();
    let deserialized = ReportEnvelope::from_json(&json).unwrap();

    assert_eq!(deserialized.operation_id, "db-pentest");
    assert_eq!(deserialized.domain_id.as_deref(), Some("db-pentest"));
    assert_eq!(deserialized.target.as_deref(), Some("localhost:5432"));
    assert_eq!(deserialized.findings.len(), 1);
    assert_eq!(deserialized.findings[0].severity, Severity::High);
    assert!(deserialized.baseline.is_some());
    assert!(deserialized.tool_metadata.is_some());

    let baseline = deserialized.baseline.unwrap();
    assert!(baseline.is_regression);
    assert_eq!(baseline.added, 1);
}

#[test]
fn evidence_item_with_collected_at() {
    let source = EvidenceSource {
        tool: "test".to_string(),
        module: None,
        run_id: None,
    };
    let now = chrono::Utc::now();
    let item = EvidenceItem::new("ev-1", EvidenceKind::Timing, source, "500ms response time")
        .with_collected_at(now);

    let json = serde_json::to_string(&item).unwrap();
    let deserialized: EvidenceItem = serde_json::from_str(&json).unwrap();

    assert!(deserialized.collected_at.is_some());
}

#[test]
fn finding_record_empty_evidence() {
    let record = FindingRecord::new(
        "f-1",
        "test",
        "test-op",
        Severity::Info,
        "Info Finding",
        "Just information",
    );

    let json = serde_json::to_string(&record).unwrap();
    let deserialized: FindingRecord = serde_json::from_str(&json).unwrap();

    assert!(deserialized.evidence.is_empty());
    assert!(deserialized.remediation.is_none());
    assert!(deserialized.references.is_empty());
}

#[test]
fn baseline_summary_compute_flags() {
    let mut summary = BaselineSummary::new("test");
    summary.added = 3;
    summary.resolved = 0;
    summary.compute_flags();
    assert!(summary.is_regression);
    assert!(!summary.is_improvement);

    let mut summary2 = BaselineSummary::new("test");
    summary2.added = 0;
    summary2.resolved = 5;
    summary2.compute_flags();
    assert!(!summary2.is_regression);
    assert!(summary2.is_improvement);

    let mut summary3 = BaselineSummary::new("test");
    summary3.added = 2;
    summary3.resolved = 3;
    summary3.compute_flags();
    assert!(!summary3.is_regression);
    assert!(!summary3.is_improvement);
}

#[test]
fn tool_metadata_preserved() {
    let envelope = ReportEnvelope::new("test-op").with_tool_metadata(ToolMetadata {
        tool_name: "eggsec-mobile-lab".to_string(),
        tool_version: Some("0.2.0".to_string()),
        eggsec_version: Some("0.1.0".to_string()),
    });

    let json = envelope.to_json().unwrap();
    let deserialized = ReportEnvelope::from_json(&json).unwrap();

    let meta = deserialized.tool_metadata.unwrap();
    assert_eq!(meta.tool_name, "eggsec-mobile-lab");
    assert_eq!(meta.tool_version.as_deref(), Some("0.2.0"));
    assert_eq!(meta.eggsec_version.as_deref(), Some("0.1.0"));
}

#[test]
fn redaction_policy_roundtrip() {
    let policies = [
        RedactionPolicy::None,
        RedactionPolicy::RedactAll,
        RedactionPolicy::RedactSensitive,
        RedactionPolicy::SummarizeAll,
        RedactionPolicy::DomainSpecific,
    ];

    for policy in policies {
        let source = EvidenceSource {
            tool: "test".to_string(),
            module: None,
            run_id: None,
        };
        let items = vec![EvidenceItem::new(
            "ev-1",
            EvidenceKind::Generic,
            source,
            "data",
        )];
        let manifest = EvidenceManifest::with_redaction_policy("op-1", &items, policy);
        let json = serde_json::to_string(&manifest).unwrap();
        let deserialized: EvidenceManifest = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.redaction_policy, policy);
    }
}

#[test]
fn evidence_manifest_default_redaction_policy() {
    let manifest = EvidenceManifest {
        bundle_id: "b-1".to_string(),
        operation_id: "op-1".to_string(),
        ..Default::default()
    };
    assert_eq!(manifest.redaction_policy, RedactionPolicy::None);
    assert_eq!(manifest.total_items, 0);
    assert_eq!(manifest.redacted_items, 0);
}

#[test]
fn evidence_manifest_with_redaction_policy_preserves_field() {
    let source = EvidenceSource {
        tool: "test".to_string(),
        module: None,
        run_id: None,
    };
    let items = vec![
        EvidenceItem::new("ev-1", EvidenceKind::DatabaseFinding, source.clone(), "a"),
        EvidenceItem::new("ev-2", EvidenceKind::Generic, source, "b")
            .with_redaction(RedactionState::PartiallyRedacted),
    ];
    let manifest =
        EvidenceManifest::with_redaction_policy("op-1", &items, RedactionPolicy::RedactSensitive);

    let json = serde_json::to_string(&manifest).unwrap();
    let deserialized: EvidenceManifest = serde_json::from_str(&json).unwrap();

    assert_eq!(
        deserialized.redaction_policy,
        RedactionPolicy::RedactSensitive
    );
    assert_eq!(deserialized.total_items, 2);
    assert_eq!(deserialized.redacted_items, 1);
}
