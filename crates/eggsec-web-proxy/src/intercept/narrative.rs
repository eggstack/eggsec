//! Attack narrative generation for proxy session reports.
//!
//! Builds human-readable and machine-readable attack narratives from
//! `WebProxySessionReport` data. Narratives describe the flow of traffic,
//! manipulations performed, and correlation events in a chronological
//! story format suitable for reports and briefings.

use crate::intercept::types::WebProxySessionReport;
use serde::{Deserialize, Serialize};

/// A single event in the attack narrative timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NarrativeEvent {
    /// Sequence number within the narrative.
    pub seq: u32,
    /// Human-readable event description.
    pub description: String,
    /// Event category (flow_start, flow_end, manipulation, correlation, summary).
    pub category: String,
    /// Flow index this event relates to (if any).
    pub flow_index: Option<u64>,
    /// Severity hint (info, warning, critical).
    pub severity: String,
}

/// A complete attack narrative for a proxy session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttackNarrative {
    /// Session summary line.
    pub summary: String,
    /// Ordered narrative events.
    pub events: Vec<NarrativeEvent>,
    /// Correlation findings across loadouts.
    pub correlations: Vec<String>,
    /// Overall risk assessment.
    pub risk_assessment: String,
    /// Recommendations.
    pub recommendations: Vec<String>,
}

impl AttackNarrative {
    /// Render the narrative as a human-readable text block.
    pub fn to_text(&self) -> String {
        let mut out = String::new();
        out.push_str("=== Attack Narrative ===\n\n");
        out.push_str(&format!("Summary: {}\n\n", self.summary));

        out.push_str("--- Timeline ---\n");
        for event in &self.events {
            let marker = match event.severity.as_str() {
                "critical" => "!!!",
                "warning" => " ! ",
                _ => "   ",
            };
            out.push_str(&format!(
                "{} [{}] {}\n",
                marker, event.category, event.description
            ));
        }

        if !self.correlations.is_empty() {
            out.push_str("\n--- Cross-Loadout Correlations ---\n");
            for corr in &self.correlations {
                out.push_str(&format!("  * {}\n", corr));
            }
        }

        out.push_str(&format!("\nRisk Assessment: {}\n", self.risk_assessment));

        if !self.recommendations.is_empty() {
            out.push_str("\n--- Recommendations ---\n");
            for rec in &self.recommendations {
                out.push_str(&format!("  - {}\n", rec));
            }
        }

        out
    }
}

/// Build an attack narrative from a proxy session report.
pub fn build_narrative(report: &WebProxySessionReport) -> AttackNarrative {
    let mut events = Vec::new();
    let mut seq = 0u32;

    let total_flows = report.flows.len();
    let https_count = report.https_intercepted;
    let http_count = report.http_logged;
    let manipulation_count = report.manipulations.len();
    let blocked = report.blocked;
    let redacted = report.redacted;

    // Session overview event
    seq += 1;
    events.push(NarrativeEvent {
        seq,
        description: format!(
            "Session started on {} (dry_run={}) with {} flows captured ({} HTTPS, {} HTTP)",
            report.listen_addr, report.dry_run, total_flows, https_count, http_count
        ),
        category: "session_start".to_string(),
        flow_index: None,
        severity: "info".to_string(),
    });

    // Notable flows (errors, redirects, auth patterns)
    for flow in &report.flows {
        if flow.response_status >= 400 || flow.response_status == 301 || flow.response_status == 302
        {
            seq += 1;
            let status_desc = match flow.response_status {
                301 | 302 => format!("redirect ({})", flow.response_status),
                401 => "authentication required (401)".to_string(),
                403 => "forbidden (403)".to_string(),
                404 => "not found (404)".to_string(),
                500 => "internal server error (500)".to_string(),
                s if s >= 400 => format!("client error ({})", s),
                s => format!("status {}", s),
            };
            let severity = if flow.response_status == 401 || flow.response_status == 403 {
                "warning".to_string()
            } else if flow.response_status >= 500 {
                "critical".to_string()
            } else {
                "info".to_string()
            };
            events.push(NarrativeEvent {
                seq,
                description: format!(
                    "{} {}{}{} -> {}",
                    flow.method, flow.host, flow.path, status_desc, flow.response_status
                ),
                category: "flow_notable".to_string(),
                flow_index: Some(flow.index),
                severity,
            });
        }

        // Check for auth-related headers
        if flow.request_headers.iter().any(|(k, _)| {
            k.eq_ignore_ascii_case("authorization") || k.eq_ignore_ascii_case("cookie")
        }) {
            seq += 1;
            events.push(NarrativeEvent {
                seq,
                description: format!(
                    "{} {}{} carries authentication credentials",
                    flow.method, flow.host, flow.path
                ),
                category: "flow_auth".to_string(),
                flow_index: Some(flow.index),
                severity: "info".to_string(),
            });
        }
    }

    // Manipulations
    for m in &report.manipulations {
        seq += 1;
        let direction_str = match m.direction {
            crate::intercept::types::ProxyFlowDirection::Request => "request",
            crate::intercept::types::ProxyFlowDirection::Response => "response",
        };
        let before_summary = m
            .before
            .as_deref()
            .map(|b| truncate_narrative(b, 40))
            .unwrap_or_else(|| "(none)".to_string());
        let after_summary = m
            .after
            .as_deref()
            .map(|a| truncate_narrative(a, 40))
            .unwrap_or_else(|| "(none)".to_string());
        events.push(NarrativeEvent {
            seq,
            description: format!(
                "Manipulated {} {} on flow #{}: \"{}\" -> \"{}\" ({})",
                direction_str, m.field, m.flow_index, before_summary, after_summary, m.reason
            ),
            category: "manipulation".to_string(),
            flow_index: Some(m.flow_index),
            severity: "warning".to_string(),
        });
    }

    // Correlations
    let mut correlations = Vec::new();
    for corr in &report.correlation_refs {
        seq += 1;
        let desc = format!(
            "[{:?}] {} (confidence: {:.0}%)",
            corr.source,
            corr.description,
            corr.confidence * 100.0
        );
        correlations.push(desc.clone());
        events.push(NarrativeEvent {
            seq,
            description: desc,
            category: "correlation".to_string(),
            flow_index: None,
            severity: "warning".to_string(),
        });
    }

    // Summary event
    seq += 1;
    events.push(NarrativeEvent {
        seq,
        description: format!(
            "Session complete: {} flows, {} manipulations, {} blocked, {} redacted, {} correlations",
            total_flows, manipulation_count, blocked, redacted, correlations.len()
        ),
        category: "session_end".to_string(),
        flow_index: None,
        severity: "info".to_string(),
    });

    // Risk assessment
    let risk_assessment = build_risk_assessment(report);

    // Recommendations
    let recommendations = build_recommendations(report);

    let summary = format!(
        "Captured {} flows on {} ({} HTTPS, {} HTTP) with {} manipulations{}.",
        total_flows,
        report.listen_addr,
        https_count,
        http_count,
        manipulation_count,
        if blocked > 0 {
            format!(", {} blocked", blocked)
        } else {
            String::new()
        }
    );

    AttackNarrative {
        summary,
        events,
        correlations,
        risk_assessment,
        recommendations,
    }
}

fn build_risk_assessment(report: &WebProxySessionReport) -> String {
    let mut findings: Vec<String> = Vec::new();

    if report.manipulations.iter().any(|m| {
        let field_lower = m.field.to_lowercase();
        field_lower.contains("authorization")
            || field_lower.contains("cookie")
            || field_lower.contains("token")
    }) {
        findings.push("authentication credential manipulation detected".to_string());
    }

    let error_flows = report
        .flows
        .iter()
        .filter(|f| f.response_status >= 500)
        .count();
    if error_flows > 0 {
        findings.push(format!("{} server errors observed", error_flows));
    }

    let auth_required = report
        .flows
        .iter()
        .filter(|f| f.response_status == 401)
        .count();
    if auth_required > 0 {
        findings.push(format!("{} 401 Unauthorized responses", auth_required));
    }

    if !report.correlation_refs.is_empty() {
        findings.push(format!(
            "{} cross-loadout correlations found",
            report.correlation_refs.len()
        ));
    }

    if findings.is_empty() {
        "Low risk. Standard traffic interception with no notable security events.".to_string()
    } else {
        format!("Elevated risk: {}.", findings.join("; "))
    }
}

fn build_recommendations(report: &WebProxySessionReport) -> Vec<String> {
    let mut recs = Vec::new();

    if report.manipulations.iter().any(|m| {
        let field_lower = m.field.to_lowercase();
        field_lower.contains("authorization") || field_lower.contains("token")
    }) {
        recs.push("Review authentication token manipulations for credential exposure.".to_string());
    }

    if report.flows.iter().any(|f| f.response_status == 403) {
        recs.push("Investigate 403 Forbidden responses for access control issues.".to_string());
    }

    if report.flows.iter().any(|f| f.response_status >= 500) {
        recs.push("Review server error responses for potential information leakage.".to_string());
    }

    if !report.correlation_refs.is_empty() {
        recs.push(
            "Cross-loadout correlations detected; review linked findings for attack chains."
                .to_string(),
        );
    }

    if report
        .flows
        .iter()
        .any(|f| f.path.contains("admin") || f.path.contains("debug"))
    {
        recs.push(
            "Sensitive paths (admin/debug) accessed; verify authorization requirements."
                .to_string(),
        );
    }

    if recs.is_empty() {
        recs.push("No immediate action required. Review traffic for anomalies.".to_string());
    }

    recs
}

fn truncate_narrative(s: &str, max: usize) -> String {
    if s.len() <= max {
        s.to_string()
    } else {
        format!("{}...", &s[..max.saturating_sub(3)])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::intercept::types::*;
    use std::collections::HashMap;

    fn sample_report() -> WebProxySessionReport {
        let mut report = WebProxySessionReport::new("127.0.0.1:8080", false);
        report.flows.push(ProxyFlow {
            index: 0,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: {
                let mut h = HashMap::new();
                h.insert("Authorization".to_string(), "Bearer token123".to_string());
                h
            },
            request_body: None,
            response_status: 200,
            response_headers: HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 150,
            request_body_size: 0,
            response_body_size: 1024,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
        report.manipulations.push(ManipulationRecord {
            flow_index: 0,
            direction: ProxyFlowDirection::Request,
            field: "header:Authorization".to_string(),
            before: Some("Bearer old".to_string()),
            after: Some("Bearer new".to_string()),
            reason: "Token refresh".to_string(),
            timestamp: chrono::Utc::now().to_rfc3339(),
        });
        report
    }

    #[test]
    fn test_build_narrative_basic() {
        let report = sample_report();
        let narrative = build_narrative(&report);
        assert!(!narrative.summary.is_empty());
        assert!(!narrative.events.is_empty());
        assert!(!narrative.risk_assessment.is_empty());
        assert!(!narrative.recommendations.is_empty());
    }

    #[test]
    fn test_narrative_to_text() {
        let report = sample_report();
        let narrative = build_narrative(&report);
        let text = narrative.to_text();
        assert!(text.contains("Attack Narrative"));
        assert!(text.contains("example.com"));
        assert!(text.contains("Manipulated"));
    }

    #[test]
    fn test_narrative_auth_detection() {
        let report = sample_report();
        let narrative = build_narrative(&report);
        assert!(narrative.events.iter().any(|e| e.category == "flow_auth"));
    }

    #[test]
    fn test_narrative_manipulation_event() {
        let report = sample_report();
        let narrative = build_narrative(&report);
        assert!(narrative
            .events
            .iter()
            .any(|e| e.category == "manipulation"));
    }

    #[test]
    fn test_narrative_empty_report() {
        let report = WebProxySessionReport::new("0.0.0.0:9090", true);
        let narrative = build_narrative(&report);
        assert!(narrative.summary.contains("0 flows"));
        assert_eq!(narrative.correlations.len(), 0);
    }

    #[test]
    fn test_narrative_risk_auth_manipulation() {
        let report = sample_report();
        let narrative = build_narrative(&report);
        assert!(
            narrative
                .risk_assessment
                .contains("authentication credential manipulation")
                || narrative.risk_assessment.contains("Elevated risk"),
            "Expected risk assessment about auth manipulation, got: {}",
            narrative.risk_assessment
        );
    }

    #[test]
    fn test_narrative_serialization_roundtrip() {
        let report = sample_report();
        let narrative = build_narrative(&report);
        let json = serde_json::to_string(&narrative).unwrap();
        let deserialized: AttackNarrative = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.events.len(), narrative.events.len());
        assert_eq!(deserialized.summary, narrative.summary);
    }

    #[test]
    fn test_narrative_with_correlations() {
        use crate::intercept::correlation::{CorrelationReference, CorrelationSource};

        let mut report = sample_report();
        report.correlation_refs.push(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-finding-1",
            "SQLi in proxy-modified request",
        ));
        let narrative = build_narrative(&report);
        assert_eq!(narrative.correlations.len(), 1);
        assert!(narrative.correlations[0].contains("DbPentest"));
        assert!(narrative.events.iter().any(|e| e.category == "correlation"));
    }

    #[test]
    fn test_narrative_error_flows() {
        let mut report = WebProxySessionReport::new("127.0.0.1:8080", false);
        report.flows.push(ProxyFlow {
            index: 0,
            method: "GET".to_string(),
            url: "https://example.com/fail".to_string(),
            host: "example.com".to_string(),
            path: "/fail".to_string(),
            request_headers: HashMap::new(),
            request_body: None,
            response_status: 500,
            response_headers: HashMap::new(),
            response_body: None,
            is_https: true,
            duration_ms: 100,
            request_body_size: 0,
            response_body_size: 0,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
        let narrative = build_narrative(&report);
        assert!(narrative.risk_assessment.contains("server errors"));
    }
}
