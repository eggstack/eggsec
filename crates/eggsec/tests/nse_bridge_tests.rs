#![cfg(feature = "nse")]

use eggsec::nse::capabilities::{NseCapabilityEvent, NseCapabilityKind};
use eggsec::nse::report::*;
use eggsec::nse_bridge::to_report_envelope;
use eggsec_core::types::Severity;
use eggsec_report_model::EvidenceKind as OutputEvidenceKind;

fn compatible_report_with_evidence(evidence: Vec<NseEvidenceItem>) -> NseRunReport {
    NseRunReport::new("10.0.0.1", "test_script")
        .with_evidence(evidence)
        .compute_compatibility()
}

fn empty_compatibility() -> NseCompatibilitySummary {
    NseCompatibilitySummary {
        status: NseRunCompatibilityStatus::Unknown,
        fidelity: NseRunFidelity::Unknown,
        unsupported_features: Vec::new(),
        approximations: Vec::new(),
    }
}

fn empty_output() -> NseOutputSummary {
    NseOutputSummary {
        has_output: false,
        content: String::new(),
        line_count: 0,
        truncated: false,
    }
}

fn evidence_item(id: &str, kind: NseEvidenceKind, title: &str, summary: &str) -> NseEvidenceItem {
    NseEvidenceItem {
        id: id.to_string(),
        kind,
        title: title.to_string(),
        summary: summary.to_string(),
        target: "10.0.0.1".to_string(),
        port: None,
        service: None,
        confidence: "confirmed".to_string(),
        source: "test_script".to_string(),
        raw_excerpt: None,
        references: Vec::new(),
        tags: Vec::new(),
    }
}

#[test]
fn envelope_has_evidence_manifest() {
    let evidence = vec![
        evidence_item("ev-0", NseEvidenceKind::ScriptOutput, "Output", "3 lines"),
        evidence_item(
            "ev-1",
            NseEvidenceKind::CapabilityDenial,
            "Denied",
            "blocked",
        ),
    ];

    let report = compatible_report_with_evidence(evidence);
    let envelope = to_report_envelope(&report);

    // 2 evidence findings + 1 metadata finding = 3 findings
    assert_eq!(envelope.findings.len(), 3);

    // Manifest should have total_items == number of evidence items across all findings
    // Each finding has 1 evidence item, so 2 evidence items total (metadata has none)
    let manifest = &envelope.evidence_manifest;
    assert_eq!(manifest.total_items, 2);
    assert_eq!(manifest.redacted_items, 0);
}

#[test]
fn envelope_finding_categories() {
    let evidence = vec![
        evidence_item(
            "ev-0",
            NseEvidenceKind::VulnerabilitySignal,
            "Vuln",
            "SQLi detected",
        ),
        evidence_item("ev-1", NseEvidenceKind::ScriptOutput, "Output", "result"),
        evidence_item(
            "ev-2",
            NseEvidenceKind::CompatibilityWarning,
            "Warning",
            "unsupported",
        ),
    ];

    let report = compatible_report_with_evidence(evidence);
    let envelope = to_report_envelope(&report);

    // All evidence-based findings should have nse-* category prefix
    let evidence_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.id != "metadata-nse")
        .collect();
    assert_eq!(evidence_findings.len(), 3);

    for finding in &evidence_findings {
        assert!(
            finding.category.starts_with("nse-"),
            "Finding category '{}' should start with 'nse-'",
            finding.category
        );
    }

    // Verify specific categories
    let categories: Vec<&str> = evidence_findings
        .iter()
        .map(|f| f.category.as_str())
        .collect();
    assert!(categories.contains(&"nse-vulnerability-signal"));
    assert!(categories.contains(&"nse-script-output"));
    assert!(categories.contains(&"nse-compatibility-warning"));
}

#[test]
fn envelope_multiple_evidence_items() {
    let evidence = vec![
        evidence_item(
            "ev-0",
            NseEvidenceKind::ServiceFingerprint,
            "Fingerprint",
            "nginx detected",
        ),
        evidence_item(
            "ev-1",
            NseEvidenceKind::CertificateInfo,
            "Cert",
            "self-signed cert",
        ),
        evidence_item(
            "ev-2",
            NseEvidenceKind::Misconfiguration,
            "Misconfig",
            "server info leaked",
        ),
    ];

    let report = compatible_report_with_evidence(evidence);
    let envelope = to_report_envelope(&report);

    // 3 evidence findings + 1 metadata = 4
    assert_eq!(envelope.findings.len(), 4);

    // Each evidence finding should have exactly 1 evidence item
    let evidence_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.id != "metadata-nse")
        .collect();
    for finding in &evidence_findings {
        assert_eq!(finding.evidence.len(), 1);
    }

    // Verify all 3 evidence items mapped
    let fingerprint = envelope
        .findings
        .iter()
        .find(|f| f.category == "nse-service-fingerprint")
        .unwrap();
    assert_eq!(fingerprint.evidence[0].kind, OutputEvidenceKind::Banner);

    let cert = envelope
        .findings
        .iter()
        .find(|f| f.category == "nse-certificate-info")
        .unwrap();
    assert_eq!(cert.evidence[0].kind, OutputEvidenceKind::Certificate);

    let misconfig = envelope
        .findings
        .iter()
        .find(|f| f.category == "nse-misconfiguration")
        .unwrap();
    assert_eq!(misconfig.evidence[0].kind, OutputEvidenceKind::Generic);
}

#[test]
fn envelope_no_circular_dependencies() {
    // Build a report with multiple evidence items and verify the bridge
    // doesn't create duplicate evidence items
    let evidence = vec![
        evidence_item("ev-0", NseEvidenceKind::ScriptOutput, "Output", "data"),
        evidence_item("ev-1", NseEvidenceKind::ScriptOutput, "Output", "more data"),
        evidence_item(
            "ev-2",
            NseEvidenceKind::CapabilityDenial,
            "Denied",
            "blocked",
        ),
    ];

    let report = compatible_report_with_evidence(evidence);
    let envelope = to_report_envelope(&report);

    // Collect all evidence IDs across all findings
    let mut all_evidence_ids: Vec<&str> = Vec::new();
    for finding in &envelope.findings {
        for ev in &finding.evidence {
            all_evidence_ids.push(&ev.id);
        }
    }

    // No duplicate evidence IDs
    let unique_count = all_evidence_ids.len();
    all_evidence_ids.sort();
    all_evidence_ids.dedup();
    assert_eq!(
        all_evidence_ids.len(),
        unique_count,
        "Found duplicate evidence IDs: {:?}",
        all_evidence_ids
    );

    // Each finding should reference exactly one evidence item
    let evidence_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.id != "metadata-nse")
        .collect();
    assert_eq!(evidence_findings.len(), 3);
    for finding in &evidence_findings {
        assert_eq!(
            finding.evidence.len(),
            1,
            "Finding '{}' has {} evidence items, expected 1",
            finding.id,
            finding.evidence.len()
        );
    }
}

// Moved from eggsec-nse/tests/evidence_tests.rs during runtime-extraction
// Milestone 002: report-envelope conversion is engine-owned.

#[test]
fn bridge_to_envelope_basic() {
    let report = NseRunReport::new("10.0.0.1", "ssl-cert")
        .with_evidence(vec![NseEvidenceItem {
            id: "nse-ev-0".to_string(),
            kind: NseEvidenceKind::ScriptOutput,
            title: "Script output captured".to_string(),
            summary: "3 lines of output".to_string(),
            target: "10.0.0.1".to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: "ssl-cert".to_string(),
            raw_excerpt: Some("test output".to_string()),
            references: Vec::new(),
            tags: vec!["output".to_string()],
        }])
        .compute_compatibility();

    let envelope = to_report_envelope(&report);

    // Should have 1 evidence finding + 1 metadata finding = 2 findings
    assert_eq!(envelope.findings.len(), 2);
    assert_eq!(envelope.domain_id.as_deref(), Some("nse"));
    assert_eq!(envelope.target.as_deref(), Some("10.0.0.1"));
}

#[test]
fn bridge_to_envelope_empty_evidence() {
    let report = NseRunReport::new("10.0.0.1", "test_script").compute_compatibility();

    let envelope = to_report_envelope(&report);

    // Should have only the metadata finding
    assert_eq!(envelope.findings.len(), 1);
    assert_eq!(envelope.findings[0].id, "metadata-nse");
}
#[test]
fn bridge_compatible_run_envelope() {
    let report = NseRunReport::new("10.0.0.1", "ssl-cert").compute_compatibility();

    assert_eq!(
        report.compatibility.status,
        NseRunCompatibilityStatus::Compatible
    );
    assert_eq!(report.compatibility.fidelity, NseRunFidelity::Full);

    let envelope = to_report_envelope(&report);

    // Only the metadata finding exists (no evidence findings)
    assert_eq!(envelope.findings.len(), 1);
    let metadata = &envelope.findings[0];
    assert_eq!(metadata.id, "metadata-nse");
    assert_eq!(metadata.severity, Severity::Info);
    assert!(metadata.description.contains("compatible"));

    // No CapabilityDenial findings
    let denial_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.category.contains("capability-denial"))
        .collect();
    assert!(denial_findings.is_empty());
}

#[test]
fn bridge_partial_run_envelope() {
    let summaries = vec![NseCapabilityEventSummary {
        kind: "process_exec".to_string(),
        operation: "io.popen".to_string(),
        target: Some("ls".to_string()),
        allowed: false,
        reason: Some("denied by AgentSafe policy".to_string()),
    }];

    let events = vec![NseCapabilityEvent {
        kind: NseCapabilityKind::ProcessExec,
        operation: "io.popen".to_string(),
        target: Some("ls".to_string()),
        allowed: false,
        reason: Some("denied by AgentSafe policy".to_string()),
        bytes: None,
    }];

    let evidence = extract_evidence(
        "10.0.0.1",
        "test_script",
        &summaries,
        &empty_compatibility(),
        &[],
        &empty_output(),
    );

    let report = NseRunReport::new("10.0.0.1", "test_script")
        .with_evidence(evidence)
        .with_capability_events(events)
        .compute_compatibility();

    assert_eq!(
        report.compatibility.status,
        NseRunCompatibilityStatus::Partial
    );

    let envelope = to_report_envelope(&report);

    // Should have 1 capability denial finding + 1 metadata finding
    assert_eq!(envelope.findings.len(), 2);

    let denial_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.category.contains("capability-denial"))
        .collect();
    assert_eq!(denial_findings.len(), 1);
    assert_eq!(denial_findings[0].severity, Severity::Info);

    // Metadata should reflect partial status
    let metadata = &envelope.findings[1];
    assert!(metadata.description.contains("partial"));
}

#[test]
fn bridge_capability_denial_evidence_severity() {
    let summaries = vec![
        NseCapabilityEventSummary {
            kind: "process_exec".to_string(),
            operation: "io.popen".to_string(),
            target: Some("ls".to_string()),
            allowed: false,
            reason: Some("denied by policy".to_string()),
        },
        NseCapabilityEventSummary {
            kind: "filesystem_write".to_string(),
            operation: "io.write".to_string(),
            target: Some("/tmp/test".to_string()),
            allowed: false,
            reason: Some("denied by policy".to_string()),
        },
    ];

    let events = vec![
        NseCapabilityEvent {
            kind: NseCapabilityKind::ProcessExec,
            operation: "io.popen".to_string(),
            target: Some("ls".to_string()),
            allowed: false,
            reason: Some("denied by policy".to_string()),
            bytes: None,
        },
        NseCapabilityEvent {
            kind: NseCapabilityKind::FilesystemWrite,
            operation: "io.write".to_string(),
            target: Some("/tmp/test".to_string()),
            allowed: false,
            reason: Some("denied by policy".to_string()),
            bytes: None,
        },
    ];

    let evidence = extract_evidence(
        "10.0.0.1",
        "test_script",
        &summaries,
        &empty_compatibility(),
        &[],
        &empty_output(),
    );

    let report = NseRunReport::new("10.0.0.1", "test_script")
        .with_evidence(evidence)
        .with_capability_events(events)
        .compute_compatibility();

    let envelope = to_report_envelope(&report);

    // All CapabilityDenial findings must be Severity::Info
    for finding in &envelope.findings {
        if finding.category.contains("capability-denial") {
            assert_eq!(
                finding.severity,
                Severity::Info,
                "CapabilityDenial finding '{}' must be Info, not {:?}",
                finding.id,
                finding.severity
            );
        }
    }
}

#[test]
fn bridge_rule_error_evidence() {
    let rules = vec![NseRuleEvaluationReport {
        kind: "portrule".to_string(),
        evaluated: false,
        matched: false,
        exactness: "exact".to_string(),
        error: Some("lua runtime error: attempt to call nil".to_string()),
        summary: "rule error: lua runtime error".to_string(),
        unsupported: None,
        host_context_source: None,
        port_context_source: None,
        service_context_available: None,
        fidelity_reason: None,
    }];

    let evidence = extract_evidence(
        "10.0.0.1",
        "test_script",
        &[],
        &empty_compatibility(),
        &rules,
        &empty_output(),
    );

    let report = NseRunReport::new("10.0.0.1", "test_script")
        .with_evidence(evidence)
        .with_rules(rules)
        .compute_compatibility();

    let envelope = to_report_envelope(&report);

    // Should have 1 rule-error finding + 1 metadata finding
    let rule_error_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.category.contains("compatibility-warning"))
        .collect();
    assert_eq!(rule_error_findings.len(), 1);

    let finding = &rule_error_findings[0];
    assert_eq!(finding.severity, Severity::Info);
    assert!(finding.description.contains("lua runtime error"));

    // The finding should have an evidence item of kind LogLine
    assert!(!finding.evidence.is_empty());
    assert_eq!(finding.evidence[0].kind, OutputEvidenceKind::LogLine);
}

#[test]
fn bridge_raw_output_evidence() {
    let report = NseRunReport::new("10.0.0.1", "http-server-header")
        .with_output("HTTP/1.1 200 OK\nServer: nginx/1.18.0")
        .with_evidence(vec![NseEvidenceItem {
            id: "nse-ev-0".to_string(),
            kind: NseEvidenceKind::ScriptOutput,
            title: "Script output captured".to_string(),
            summary: "2 lines of output".to_string(),
            target: "10.0.0.1".to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: "http-server-header".to_string(),
            raw_excerpt: Some("HTTP/1.1 200 OK\nServer: nginx/1.18.0".to_string()),
            references: Vec::new(),
            tags: vec!["output".to_string()],
        }])
        .compute_compatibility();

    let envelope = to_report_envelope(&report);

    let output_findings: Vec<_> = envelope
        .findings
        .iter()
        .filter(|f| f.category.contains("script-output"))
        .collect();
    assert_eq!(output_findings.len(), 1);

    let finding = &output_findings[0];
    assert_eq!(finding.severity, Severity::Info);

    // Evidence item should preserve raw excerpt
    assert!(!finding.evidence.is_empty());
    let ev = &finding.evidence[0];
    assert_eq!(ev.kind, OutputEvidenceKind::Generic);
}

#[test]
fn bridge_weak_evidence_not_high_severity() {
    let summaries = vec![NseCapabilityEventSummary {
        kind: "process_exec".to_string(),
        operation: "io.popen".to_string(),
        target: Some("ls".to_string()),
        allowed: false,
        reason: Some("denied".to_string()),
    }];

    let events = vec![NseCapabilityEvent {
        kind: NseCapabilityKind::ProcessExec,
        operation: "io.popen".to_string(),
        target: Some("ls".to_string()),
        allowed: false,
        reason: Some("denied".to_string()),
        bytes: None,
    }];

    let rules = vec![NseRuleEvaluationReport {
        kind: "portrule".to_string(),
        evaluated: false,
        matched: false,
        exactness: "exact".to_string(),
        error: Some("timeout".to_string()),
        summary: "rule error: timeout".to_string(),
        unsupported: None,
        host_context_source: None,
        port_context_source: None,
        service_context_available: None,
        fidelity_reason: None,
    }];

    let output = NseOutputSummary {
        has_output: true,
        content: "some output".to_string(),
        line_count: 1,
        truncated: false,
    };

    let compat = NseCompatibilitySummary {
        status: NseRunCompatibilityStatus::Partial,
        fidelity: NseRunFidelity::Minimal,
        unsupported_features: vec!["nmap.socket".to_string()],
        approximations: vec!["portrule: synthetic context".to_string()],
    };

    let evidence = extract_evidence(
        "10.0.0.1",
        "test_script",
        &summaries,
        &compat,
        &rules,
        &output,
    );

    let report = NseRunReport::new("10.0.0.1", "test_script")
        .with_evidence(evidence)
        .with_capability_events(events)
        .with_rules(rules)
        .compute_compatibility();

    let envelope = to_report_envelope(&report);

    // No finding should have severity > Info except VulnerabilitySignal and Misconfiguration
    // Since we don't include VulnerabilitySignal or Misconfiguration evidence, all should be Info
    for finding in &envelope.findings {
        assert!(
            finding.severity == Severity::Info,
            "Finding '{}' has unexpected severity {:?} (should be Info for weak evidence)",
            finding.id,
            finding.severity
        );
    }
}

#[test]
fn bridge_envelope_metadata_fields() {
    let report = NseRunReport::new("10.0.0.1", "ssl-cert").compute_compatibility();

    let envelope = to_report_envelope(&report);

    assert_eq!(envelope.domain_id.as_deref(), Some("nse"));
    assert_eq!(envelope.target.as_deref(), Some("10.0.0.1"));

    let tool = envelope.tool_metadata.as_ref().unwrap();
    assert_eq!(tool.tool_name, "eggsec-nse");
    assert!(tool.tool_version.is_none());
    assert!(tool.eggsec_version.is_none());
}

#[test]
fn live_canonical_report_bridges_to_nse_envelope() {
    // End-to-end through the engine facade: canonical runtime execution
    // produces a report the engine-owned bridge converts (replaces the
    // pre-extraction runtime smoke/local envelope assertions).
    use eggsec::nse::run::{execute_nse_run, NseRunRequest};
    use eggsec::nse::{NseScriptSource, ResolvedNseExecutionProfile};

    let profile = ResolvedNseExecutionProfile::manual_permissive(Some("127.0.0.1"));
    let request = NseRunRequest::new(
        "127.0.0.1",
        NseScriptSource::InlineManual {
            label: "bridge-live".to_string(),
            content: "hostrule = function(host) return true end\naction = function(host, port) return 'live-ok' end".to_string(),
        },
        profile,
    );
    let report = execute_nse_run(request).expect("facade execution succeeds");

    let envelope = to_report_envelope(&report);
    assert_eq!(envelope.domain_id.as_deref(), Some("nse"));
    assert_eq!(envelope.target.as_deref(), Some("127.0.0.1"));
    assert!(
        !envelope.findings.is_empty(),
        "live report with output must yield envelope findings"
    );
    assert!(
        envelope.findings.iter().any(|f| f.id == "metadata-nse"),
        "envelope must include execution metadata finding"
    );
}
