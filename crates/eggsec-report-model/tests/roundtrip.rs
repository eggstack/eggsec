//! Serialization compatibility tests for `eggsec-report-model`.
//!
//! Independent of `eggsec-output`: these fixtures pin the JSON shapes that
//! domain bridges and report consumers rely on (defaults, aliases, skip
//! rules, enum rename conventions, timestamp formats). The moved types are
//! verbatim from `eggsec-output`, so any shape change here is a contract
//! break and must fail loudly.

use eggsec_core::types::Severity;
use eggsec_report_model::*;

fn sample_report() -> ScanReportData {
    ScanReportData {
        target: "https://example.com".to_string(),
        scan_type: "full".to_string(),
        timestamp: "2026-09-16T00:00:00Z".to_string(),
        findings: vec![FindingData {
            title: "Reflected XSS".to_string(),
            severity: "High".to_string(),
            category: "xss".to_string(),
            description: "desc".to_string(),
            location: "/search?q=1".to_string(),
            evidence: Some("payload reflected".to_string()),
            remediation: Some("encode output".to_string()),
            cwe_ids: vec!["CWE-79".to_string()],
        }],
        open_ports: vec![PortData {
            port: 443,
            status: "open".to_string(),
            protocol: Some("tcp".to_string()),
            state: Some("open".to_string()),
            service: Some("https".to_string()),
            version: None,
            banner: None,
        }],
        services: vec![ServiceData {
            service: "https".to_string(),
            version: None,
            banner: None,
        }],
        duration_ms: 1500,
        wireless_networks: vec![],
        policy_summary: None,
    }
}

#[test]
fn scan_report_roundtrip_preserves_shape() {
    let report = sample_report();
    let json = serde_json::to_string_pretty(&report).unwrap();
    let back: ScanReportData = serde_json::from_str(&json).unwrap();
    assert_eq!(back.target, "https://example.com");
    assert_eq!(back.findings.len(), 1);
    assert_eq!(back.findings[0].cwe_ids, vec!["CWE-79".to_string()]);
    assert_eq!(back.open_ports[0].port, 443);
    // skip_serializing rules: empty wireless/policy are omitted on the wire.
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(value.get("wireless_networks").is_none());
    assert!(value.get("policy_summary").is_none());
}

#[test]
fn finding_cve_alias_still_deserializes() {
    // Pre-move `FindingData` accepted `cve_ids` as an alias for `cwe_ids`.
    let json = r#"{"title":"t","severity":"High","category":"c","description":"d","location":"l","evidence":null,"remediation":null,"cve_ids":["CVE-2024-1"]}"#;
    let finding: FindingData = serde_json::from_str(json).unwrap();
    assert_eq!(finding.cwe_ids, vec!["CVE-2024-1".to_string()]);
}

#[test]
fn wireless_defaults_apply_when_absent() {
    let json = r#"{"ssid":"s","bssid":"b","channel":6,"security_type":"WPA2","signal_strength":-50,"last_seen":"2026-09-16T00:00:00Z"}"#;
    let net: WirelessNetworkReportData = serde_json::from_str(json).unwrap();
    assert!(!net.wps_enabled);
    assert!(!net.is_hidden);
    assert!(!net.transition_mode);
}

#[test]
fn policy_summary_roundtrip() {
    let summary = PolicySummary {
        operation_mode: "defense-lab".to_string(),
        max_risk: "intrusive".to_string(),
        total_decisions: 1,
        denied_count: 0,
        warning_count: 1,
        denied_reasons: vec![],
        warnings: vec!["private-ip".to_string()],
    };
    let json = serde_json::to_string(&summary).unwrap();
    assert!(json.contains("\"defense-lab\""));
    let back: PolicySummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back.warnings, vec!["private-ip".to_string()]);
    assert_eq!(back.total_decisions, 1);
}

#[test]
fn diff_summary_roundtrip() {
    let diff = DiffSummary {
        total_new: 5,
        total_resolved: 3,
        total_escalated: 1,
        total_deescalated: 2,
        net_change: 2,
    };
    let json = serde_json::to_string(&diff).unwrap();
    let back: DiffSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(back.net_change, 2);
    assert_eq!(back.total_new, 5);
}

#[test]
fn evidence_enums_use_snake_case() {
    let kind_json = serde_json::to_string(&EvidenceKind::HttpRequest).unwrap();
    assert_eq!(kind_json, "\"http_request\"");
    let state_json = serde_json::to_string(&RedactionState::PartiallyRedacted).unwrap();
    assert_eq!(state_json, "\"partially_redacted\"");
    let policy_json = serde_json::to_string(&RedactionPolicy::RedactSensitive).unwrap();
    assert_eq!(policy_json, "\"redact_sensitive\"");
}

#[test]
fn evidence_kind_display_is_intrinsic() {
    assert_eq!(EvidenceKind::HttpRequest.to_string(), "HTTP Request");
    assert_eq!(
        EvidenceKind::DatabaseFinding.to_string(),
        "Database Finding"
    );
}

#[test]
fn finding_record_builders_and_severity_roundtrip() {
    let record = FindingRecord::new(
        "f-1",
        "db-pentest",
        "db-pentest",
        Severity::Critical,
        "Title",
        "Desc",
    )
    .with_category("cat")
    .with_location("loc")
    .with_remediation("fix")
    .with_reference("CWE-89")
    .with_evidence(EvidenceItem::new(
        "ev-1",
        EvidenceKind::DatabaseFinding,
        EvidenceSource {
            tool: "eggsec-db-lab".to_string(),
            module: Some("db-pentest".to_string()),
            run_id: None,
        },
        "summary",
    ));
    let envelope = ReportEnvelope::new("db-pentest")
        .with_domain_id("db-pentest")
        .with_target("t")
        .with_finding(record);
    let json = envelope.to_json().unwrap();
    let back = ReportEnvelope::from_json(&json).unwrap();
    assert_eq!(back.findings[0].severity, Severity::Critical);
    assert_eq!(back.findings[0].references, vec!["CWE-89".to_string()]);
}

#[test]
fn envelope_manifest_counts_redacted_items() {
    let source = || EvidenceSource {
        tool: "t".to_string(),
        module: None,
        run_id: None,
    };
    let items = vec![
        EvidenceItem::new("a", EvidenceKind::Generic, source(), "a"),
        EvidenceItem::new("b", EvidenceKind::Generic, source(), "b")
            .with_redaction(RedactionState::FullyRedacted),
    ];
    let manifest = EvidenceManifest::from_items("op-1", &items);
    assert_eq!(manifest.total_items, 2);
    assert_eq!(manifest.redacted_items, 1);
}

#[test]
fn baseline_flags_compute_from_counts() {
    let mut summary = BaselineSummary::new("db-pentest");
    summary.added = 3;
    summary.resolved = 0;
    summary.compute_flags();
    assert!(summary.is_regression);
    assert!(!summary.is_improvement);
}

#[test]
fn envelope_refresh_rebuilds_manifest_from_findings() {
    let item = EvidenceItem::new(
        "ev-1",
        EvidenceKind::HttpRequest,
        EvidenceSource {
            tool: "t".to_string(),
            module: None,
            run_id: None,
        },
        "s",
    );
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
