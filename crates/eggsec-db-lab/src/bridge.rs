//! Bridge from DbPentestReport (local defense-lab type) to unified ScanReportData.
//! Auto-wired in commands/handlers/report.rs when `db-pentest` feature is present.
//! Produces findings with `db-postgres-*` / `db-mysql-*` categories.
//! Phase 4: includes correlation metadata in the info finding description.

use crate::types::DbPentestReport;
use eggsec_output::convert::{FindingData, ScanReportData};

pub fn to_scan_report_data_db(result: &DbPentestReport) -> ScanReportData {
    let findings: Vec<FindingData> = result
        .findings
        .iter()
        .map(|f| FindingData {
            title: f.title.clone(),
            severity: f.severity.as_str().to_string(),
            category: f.category.clone(),
            description: f.description.clone(),
            location: result.target.clone(),
            evidence: f.evidence.clone(),
            remediation: Some(f.recommendation.clone()),
            cwe_ids: Vec::new(),
        })
        .collect();

    // Correlation summary string for bridged info finding
    let correlation_summary = result.correlation.as_ref().map_or(String::new(), |c| {
        if c.correlations.is_empty() {
            String::new()
        } else {
            format!(
                " | correlated: {} (avg conf {}%)",
                c.summary.total_correlations, c.summary.avg_confidence
            )
        }
    });

    // Note: extra metadata (budgets, actions, db_type, dry_run, correlation) is carried in the human/JSON native report
    // and visible via the info finding we add below for traceability in bridged formats.
    let mut all_findings = findings;
    let evidence_with_correlation = if correlation_summary.is_empty() {
        result.actions_performed.join("; ")
    } else {
        format!(
            "{};{}",
            result.actions_performed.join("; "),
            correlation_summary.trim_start_matches(" | ")
        )
    };
    all_findings.push(FindingData {
        title: "DB pentest execution metadata".to_string(),
        severity: "info".to_string(),
        category: format!("db-{}-info", result.db_type),
        description: format!(
            "db_type={} scan_type={} queries_executed={} dry_run={} manifest_matched={} duration_ms={}{}",
            result.db_type, result.scan_type, result.queries_executed, result.dry_run, result.manifest_matched, result.duration_ms, correlation_summary
        ),
        location: result.target.clone(),
        evidence: Some(evidence_with_correlation),
        remediation: None,
        cwe_ids: Vec::new(),
    });

    ScanReportData {
        target: result.target.clone(),
        scan_type: result.scan_type.clone(),
        timestamp: result.timestamp.clone(),
        findings: all_findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: result.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

/// Convert a DbPentestReport into the normalized ReportEnvelope.
///
/// This produces the new normalized envelope alongside the existing `to_scan_report_data_db()`
/// bridge. DB-specific details (db_type, dry_run, queries_executed, correlation, compliance)
/// are preserved in findings' evidence items and the envelope's metadata.
pub fn to_report_envelope(result: &DbPentestReport) -> eggsec_output::envelope::ReportEnvelope {
    use eggsec_output::envelope::{
        BaselineSummary, EvidenceItem, EvidenceKind, EvidenceSource, FindingRecord, RedactionState,
    };

    let mut findings: Vec<FindingRecord> = result
        .findings
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let finding_id = format!("db-{}-{}", result.db_type, i);
            let mut record = FindingRecord::new(
                &finding_id,
                "db-pentest",
                "db-pentest",
                f.severity,
                &f.title,
                &f.description,
            )
            .with_category(&f.category)
            .with_location(&result.target)
            .with_remediation(&f.recommendation);

            if let Some(ref evidence_text) = f.evidence {
                let ev_id = format!("{}-ev-{}", finding_id, 0);
                let ev_source = EvidenceSource {
                    tool: "eggsec-db-lab".to_string(),
                    module: Some("db-pentest".to_string()),
                    run_id: None,
                };
                record = record.with_evidence(
                    EvidenceItem::new(
                        ev_id,
                        EvidenceKind::DatabaseFinding,
                        ev_source,
                        evidence_text,
                    )
                    .with_data_ref(evidence_text.clone())
                    .with_redaction(RedactionState::PartiallyRedacted),
                );
            }
            record
        })
        .collect();

    // Add correlation evidence if present
    if let Some(ref correlation) = result.correlation {
        if !correlation.correlations.is_empty() {
            let ev_id = format!("db-{}-correlation", result.db_type);
            let summary = format!(
                "{} correlations found (avg confidence {}%)",
                correlation.summary.total_correlations, correlation.summary.avg_confidence
            );
            let corr_finding = FindingRecord::new(
                &ev_id,
                "db-pentest",
                "db-pentest",
                eggsec_core::types::Severity::Info,
                "DB correlation summary",
                summary.clone(),
            )
            .with_category(format!("db-{}-correlation", result.db_type))
            .with_location(&result.target)
            .with_evidence(
                EvidenceItem::new(
                    format!("{}-ev-0", ev_id),
                    EvidenceKind::Correlation,
                    EvidenceSource {
                        tool: "eggsec-db-lab".to_string(),
                        module: Some("correlation-engine".to_string()),
                        run_id: None,
                    },
                    summary,
                )
                .with_redaction(RedactionState::None),
            );
            findings.push(corr_finding);
        }
    }

    // Add execution metadata as a summary finding
    let metadata_finding = FindingRecord::new(
        "metadata-db-pentest",
        "db-pentest",
        "db-pentest",
        eggsec_core::types::Severity::Info,
        "DB pentest execution metadata",
        format!(
            "db_type={} scan_type={} queries_executed={} dry_run={} manifest_matched={} duration_ms={}",
            result.db_type, result.scan_type, result.queries_executed, result.dry_run, result.manifest_matched, result.duration_ms
        ),
    )
    .with_category(format!("db-{}-info", result.db_type))
    .with_location(&result.target);
    findings.push(metadata_finding);

    let mut envelope = eggsec_output::envelope::ReportEnvelope::new("db-pentest")
        .with_domain_id("db-pentest")
        .with_target(&result.target)
        .with_tool_metadata(eggsec_output::envelope::ToolMetadata {
            tool_name: "eggsec-db-lab".to_string(),
            tool_version: None,
            eggsec_version: None,
        });

    for finding in findings {
        envelope = envelope.with_finding(finding);
    }

    // Add baseline summary if available
    if let Some(ref regression_summary) = result.regression_summary {
        let mut baseline = BaselineSummary::new("db-pentest");
        baseline.summary = Some(regression_summary.clone());
        envelope = envelope.with_baseline(baseline);
    }

    envelope.refresh_evidence_manifest();
    envelope
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::DbFinding;
    use eggsec_core::types::Severity;

    #[test]
    fn bridge_produces_valid_scan_report_data() {
        let mut r = DbPentestReport::new("postgres://u@h:5432/db", "postgres");
        r.findings.push(DbFinding {
            category: "db-postgres-misconfig-dangerous-extension".to_string(),
            severity: Severity::High,
            title: "Dangerous ext".to_string(),
            description: "desc".to_string(),
            recommendation: "revoke".to_string(),
            evidence: None,
            db_type: "postgres".to_string(),
            target_host: "h".to_string(),
        });
        let srd = to_scan_report_data_db(&r);
        assert!(srd.scan_type.contains("db"));
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category.contains("db-postgres")));
        // Roundtrip
        let _ = serde_json::to_string(&srd).unwrap();
    }
}
