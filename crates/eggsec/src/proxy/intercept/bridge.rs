//! Bridge from WebProxySessionReport (local defense-lab type) to unified ScanReportData.
//! Auto-wired in commands/handlers/report.rs when feature is present.
//! Produces findings with `proxy-intercept-flow` and `web-traffic-summary` categories.

use super::types::WebProxySessionReport;
use crate::output::convert::{FindingData, ScanReportData};

pub fn to_scan_report_data_proxy(report: &WebProxySessionReport) -> ScanReportData {
    let findings: Vec<FindingData> = report
        .flows
        .iter()
        .map(|flow| FindingData {
            title: format!("{} {} {}", flow.method, flow.host, flow.path),
            severity: "info".to_string(),
            category: "proxy-intercept-flow".to_string(),
            description: format!(
                "method={} host={} path={} https={} status={} redacted={}",
                flow.method,
                flow.host,
                flow.path,
                flow.is_https,
                flow.response_status,
                flow.redaction_applied.is_some()
            ),
            location: report.listen_addr.clone(),
            evidence: flow.request_body.clone(),
            remediation: None,
            cwe_ids: Vec::new(),
        })
        .collect();

    let mut all_findings = findings;

    for manip in &report.manipulations {
        let finding_type = if manip.field.starts_with("header:") {
            "header-modification"
        } else if manip.field == "body" {
            "body-modification"
        } else if manip.field == "path" {
            "path-modification"
        } else {
            "proxy-manipulation"
        };

        all_findings.push(FindingData {
            title: format!("Proxy manipulation: {} on flow #{}", manip.field, manip.flow_index),
            severity: "info".to_string(),
            category: format!("proxy-manipulation-{}", finding_type),
            description: format!(
                "field={} direction={:?} before={:?} after={:?} reason={} timestamp={}",
                manip.field,
                manip.direction,
                manip.before.as_ref().map(|s| s.len().min(100)),
                manip.after.as_ref().map(|s| s.len().min(100)),
                manip.reason,
                manip.timestamp
            ),
            location: format!("flow_{}", manip.flow_index),
            evidence: manip.after.clone(),
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    all_findings.push(FindingData {
        title: "Web proxy intercept session metadata".to_string(),
        severity: "info".to_string(),
        category: "web-traffic-summary".to_string(),
        description: format!(
            "listen_addr={} total_flows={} manipulations={} https_intercepted={} redacted={} blocked={} dry_run={} duration_ms={}",
            report.listen_addr,
            report.flows.len(),
            report.manipulations.len(),
            report.https_intercepted,
            report.redacted,
            report.blocked,
            report.dry_run,
            report.duration_ms
        ),
        location: report.listen_addr.clone(),
        evidence: None,
        remediation: None,
        cwe_ids: Vec::new(),
    });

    ScanReportData {
        target: report.listen_addr.clone(),
        scan_type: "web-proxy-intercept".to_string(),
        timestamp: report.ended_at.clone(),
        findings: all_findings,
        open_ports: Vec::new(),
        services: Vec::new(),
        duration_ms: report.duration_ms,
        wireless_networks: Vec::new(),
        policy_summary: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::proxy::intercept::types::{ManipulationRecord, ProxyFlow, ProxyFlowDirection};

    #[test]
    fn bridge_produces_valid_scan_report_data() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        r.started_at = "2026-01-01T00:00:00Z".to_string();
        r.ended_at = "2026-01-01T00:01:00Z".to_string();
        r.duration_ms = 60_000;
        r.flows.push(ProxyFlow {
            index: 1,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: true,
            duration_ms: 120,
            request_body_size: 0,
            response_body_size: 0,
            started_at: "2026-01-01T00:00:01Z".to_string(),
            completed_at: "2026-01-01T00:00:01Z".to_string(),
            redaction_applied: None,
        });

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.scan_type, "web-proxy-intercept");
        assert_eq!(srd.target, "127.0.0.1:8080");
        assert_eq!(srd.findings.len(), 2);
        assert!(srd.findings.iter().any(|f| f.category == "proxy-intercept-flow"));
        assert!(srd.findings.iter().any(|f| f.category == "web-traffic-summary"));
        assert_eq!(srd.duration_ms, 60_000);
        assert_eq!(srd.timestamp, "2026-01-01T00:01:00Z");
    }

    #[test]
    fn bridge_roundtrip_serialization() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        r.ended_at = "2026-06-12T00:00:00Z".to_string();

        let srd = to_scan_report_data_proxy(&r);
        let j = serde_json::to_string(&srd).unwrap();
        let back: ScanReportData = serde_json::from_str(&j).unwrap();
        assert_eq!(back.scan_type, "web-proxy-intercept");
        assert_eq!(back.findings.len(), 1);
        assert_eq!(back.findings[0].category, "web-traffic-summary");
    }

    #[test]
    fn empty_report_produces_correct_structure() {
        let r = WebProxySessionReport::new("0.0.0.0:9090", false);
        let srd = to_scan_report_data_proxy(&r);

        assert_eq!(srd.target, "0.0.0.0:9090");
        assert_eq!(srd.findings.len(), 1);
        assert_eq!(srd.findings[0].category, "web-traffic-summary");
        assert!(srd.open_ports.is_empty());
        assert!(srd.services.is_empty());
        assert!(srd.wireless_networks.is_empty());
        assert!(srd.policy_summary.is_none());
        assert_eq!(srd.duration_ms, 0);
    }

    #[test]
    fn bridge_includes_manipulation_findings() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        r.started_at = "2026-01-01T00:00:00Z".to_string();
        r.ended_at = "2026-01-01T00:01:00Z".to_string();
        r.duration_ms = 60_000;

        r.manipulations.push(ManipulationRecord {
            flow_index: 0,
            direction: ProxyFlowDirection::Request,
            field: "header:Authorization".to_string(),
            before: Some("Bearer old-token".to_string()),
            after: Some("Bearer new-token".to_string()),
            reason: "Testing token refresh".to_string(),
            timestamp: "2026-01-01T00:00:30Z".to_string(),
        });

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.findings.len(), 2);
        assert!(srd.findings.iter().any(|f| f.category.contains("proxy-manipulation-header")));
    }
}
