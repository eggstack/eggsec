//! Bridge from `NseRunReport` to the normalized `ReportEnvelope`.
//!
//! Maps NSE evidence items and report metadata into the eggsec-report-model
//! normalized report envelope for cross-domain report integration.
//!
//! This is an Eggsec composition/reporting adapter, not NSE runtime
//! semantics. It lives in the engine (moved out of `eggsec-nse` during the
//! runtime-extraction Milestone 002) so the runtime crate has no inward
//! dependency on Eggsec report contracts. It follows the same pattern as
//! `eggsec-db-lab/src/bridge.rs`: domain-internal types are mapped to the
//! generic envelope types without creating a circular dependency.

use crate::nse::report::NseRunReport;
use eggsec_core::types::Severity;
use eggsec_report_model::{
    EvidenceItem as OutputEvidenceItem, EvidenceKind as OutputEvidenceKind, EvidenceSource,
    FindingRecord, RedactionState, ReportEnvelope, ToolMetadata,
};

/// Map NseEvidenceKind to report-model EvidenceKind.
fn evidence_kind_to_output(kind: &crate::nse::report::NseEvidenceKind) -> OutputEvidenceKind {
    match kind {
        crate::nse::report::NseEvidenceKind::ServiceFingerprint => OutputEvidenceKind::Banner,
        crate::nse::report::NseEvidenceKind::VersionInfo => OutputEvidenceKind::Banner,
        crate::nse::report::NseEvidenceKind::CertificateInfo => OutputEvidenceKind::Certificate,
        crate::nse::report::NseEvidenceKind::VulnerabilitySignal => OutputEvidenceKind::Generic,
        crate::nse::report::NseEvidenceKind::Misconfiguration => OutputEvidenceKind::Generic,
        crate::nse::report::NseEvidenceKind::CapabilityDenial => {
            OutputEvidenceKind::RuntimeInstrumentation
        }
        crate::nse::report::NseEvidenceKind::CompatibilityWarning => OutputEvidenceKind::LogLine,
        crate::nse::report::NseEvidenceKind::ScriptOutput => OutputEvidenceKind::Generic,
    }
}

/// Map NseEvidenceKind to severity.
fn evidence_kind_to_severity(kind: &crate::nse::report::NseEvidenceKind) -> Severity {
    match kind {
        crate::nse::report::NseEvidenceKind::VulnerabilitySignal => Severity::Medium,
        crate::nse::report::NseEvidenceKind::Misconfiguration => Severity::Medium,
        crate::nse::report::NseEvidenceKind::CapabilityDenial => Severity::Info,
        crate::nse::report::NseEvidenceKind::CompatibilityWarning => Severity::Info,
        _ => Severity::Info,
    }
}

/// Convert an NseRunReport into the normalized ReportEnvelope.
///
/// Evidence items are mapped into FindingRecords with associated EvidenceItems.
/// An execution-metadata finding is always added at Severity::Info.
/// Raw output is preserved in the original report and not duplicated here.
pub fn to_report_envelope(report: &NseRunReport) -> ReportEnvelope {
    let mut findings: Vec<FindingRecord> = Vec::new();

    // Map evidence items into findings
    for (i, ev) in report.evidence.iter().enumerate() {
        let finding_id = format!("nse-{}-{}", report.script_name, i);
        let severity = evidence_kind_to_severity(&ev.kind);
        let mut record = FindingRecord::new(
            &finding_id,
            "nse",
            &report.script_name,
            severity,
            &ev.title,
            &ev.summary,
        )
        .with_category(format!("nse-{}", ev.kind))
        .with_location(&report.target);

        let output_ev = OutputEvidenceItem::new(
            format!("{}-ev-0", finding_id),
            evidence_kind_to_output(&ev.kind),
            EvidenceSource {
                tool: "eggsec-nse".to_string(),
                module: Some(report.script_name.clone()),
                run_id: None,
            },
            &ev.summary,
        )
        .with_redaction(RedactionState::None);

        record = record.with_evidence(output_ev);

        for reference in &ev.references {
            record = record.with_reference(reference);
        }

        findings.push(record);
    }

    // Add execution metadata as info finding
    let metadata_finding = FindingRecord::new(
        "metadata-nse",
        "nse",
        &report.script_name,
        Severity::Info,
        "NSE execution metadata",
        format!(
            "target={} script={} status={} fidelity={} elapsed_secs={:.2}",
            report.target,
            report.script_name,
            report.compatibility.status,
            report.compatibility.fidelity,
            report.stats.elapsed_secs,
        ),
    )
    .with_category("nse-info")
    .with_location(&report.target);
    findings.push(metadata_finding);

    let mut envelope = ReportEnvelope::new(&report.script_name)
        .with_domain_id("nse")
        .with_target(&report.target)
        .with_tool_metadata(ToolMetadata {
            tool_name: "eggsec-nse".to_string(),
            tool_version: None,
            eggsec_version: None,
        });

    for finding in findings {
        envelope = envelope.with_finding(finding);
    }

    envelope.refresh_evidence_manifest();
    envelope
}
