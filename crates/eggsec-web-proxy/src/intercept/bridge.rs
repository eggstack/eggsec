//! Bridge from WebProxySessionReport (local defense-lab type) to unified ScanReportData.
//! Auto-wired in commands/handlers/report.rs when feature is present.
//! Produces findings with `proxy-intercept-flow`, `proxy-websocket-session`,
//! `proxy-http2-session`, `proxy-grpc-session`, `proxy-correlation-summary`,
//! and `web-traffic-summary` categories.

use super::types::WebProxySessionReport;
use eggsec_output::convert::{FindingData, ScanReportData};

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
            title: format!(
                "Proxy manipulation: {} on flow #{}",
                manip.field, manip.flow_index
            ),
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

    for ws in &report.ws_sessions {
        all_findings.push(FindingData {
            title: format!("WebSocket session: {} {}", ws.host, ws.path),
            severity: "info".to_string(),
            category: "proxy-websocket-session".to_string(),
            description: format!(
                "host={} path={} client_messages={} server_messages={} total_bytes={} secure={}",
                ws.host,
                ws.path,
                ws.client_message_count,
                ws.server_message_count,
                ws.total_bytes,
                ws.is_secure
            ),
            location: ws.url.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    for h2 in &report.http2_sessions {
        all_findings.push(FindingData {
            title: format!("HTTP/2 session: {}", h2.host),
            severity: "info".to_string(),
            category: "proxy-http2-session".to_string(),
            description: format!(
                "host={} stream_count={} secure={}",
                h2.host,
                h2.streams.len(),
                h2.is_secure
            ),
            location: h2.host.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    for grpc in &report.grpc_sessions {
        all_findings.push(FindingData {
            title: format!("gRPC session: {}", grpc.host),
            severity: "info".to_string(),
            category: "proxy-grpc-session".to_string(),
            description: format!(
                "host={} call_count={} secure={}",
                grpc.host,
                grpc.calls.len(),
                grpc.is_secure
            ),
            location: grpc.host.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    if let Some(ref corr) = report.correlation {
        all_findings.push(FindingData {
            title: "Proxy correlation summary".to_string(),
            severity: "info".to_string(),
            category: "proxy-correlation-summary".to_string(),
            description: format!(
                "total_references={} unique_sources={} correlated_flows={}",
                corr.summary.total_references,
                corr.summary.unique_sources,
                corr.summary.correlated_flows
            ),
            location: report.listen_addr.clone(),
            evidence: None,
            remediation: None,
            cwe_ids: Vec::new(),
        });
    }

    let ws_count = report.ws_sessions.len();
    let h2_count = report.http2_sessions.len();
    let grpc_count = report.grpc_sessions.len();
    let corr_refs = report
        .correlation
        .as_ref()
        .map(|c| c.summary.total_references)
        .unwrap_or(0);

    all_findings.push(FindingData {
        title: "Web proxy intercept session metadata".to_string(),
        severity: "info".to_string(),
        category: "web-traffic-summary".to_string(),
        description: format!(
            "listen_addr={} total_flows={} manipulations={} https_intercepted={} redacted={} blocked={} dry_run={} duration_ms={} ws_sessions={} http2_sessions={} grpc_sessions={} correlation_references={}",
            report.listen_addr,
            report.flows.len(),
            report.manipulations.len(),
            report.https_intercepted,
            report.redacted,
            report.blocked,
            report.dry_run,
            report.duration_ms,
            ws_count,
            h2_count,
            grpc_count,
            corr_refs
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
    use crate::intercept::correlation::{
        CorrelationContext, CorrelationReference, CorrelationSource,
    };
    use crate::intercept::protocols::{
        GrpcCall, GrpcSession, Http2Session, Http2Stream, WebSocketSession,
    };
    use crate::intercept::types::{ManipulationRecord, ProxyFlow, ProxyFlowDirection};

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
            protocol: "http1".to_string(),
        });

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.scan_type, "web-proxy-intercept");
        assert_eq!(srd.target, "127.0.0.1:8080");
        assert_eq!(srd.findings.len(), 2);
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "proxy-intercept-flow"));
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "web-traffic-summary"));
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
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category.contains("proxy-manipulation-header")));
    }

    #[test]
    fn bridge_includes_websocket_session_findings() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        r.ws_sessions.push(WebSocketSession::new(
            "wss://example.com/chat",
            "example.com",
            "/chat",
            true,
        ));

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.findings.len(), 2);
        let ws_finding = srd
            .findings
            .iter()
            .find(|f| f.category == "proxy-websocket-session")
            .unwrap();
        assert!(ws_finding.description.contains("host=example.com"));
        assert!(ws_finding.description.contains("path=/chat"));
    }

    #[test]
    fn bridge_includes_http2_session_findings() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        let mut h2 = Http2Session::new("api.example.com", true);
        h2.add_stream(Http2Stream::new(1, "GET", "/data"));
        h2.add_stream(Http2Stream::new(3, "POST", "/upload"));
        r.http2_sessions.push(h2);

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.findings.len(), 2);
        let h2_finding = srd
            .findings
            .iter()
            .find(|f| f.category == "proxy-http2-session")
            .unwrap();
        assert!(h2_finding.description.contains("host=api.example.com"));
        assert!(h2_finding.description.contains("stream_count=2"));
    }

    #[test]
    fn bridge_includes_grpc_session_findings() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        let mut grpc = GrpcSession::new("grpc.example.com", true);
        grpc.add_call(GrpcCall::new(
            "/pkg.Svc/Method1",
            crate::intercept::protocols::GrpcMethodType::Unary,
        ));
        grpc.add_call(GrpcCall::new(
            "/pkg.Svc/Method2",
            crate::intercept::protocols::GrpcMethodType::ServerStreaming,
        ));
        r.grpc_sessions.push(grpc);

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.findings.len(), 2);
        let grpc_finding = srd
            .findings
            .iter()
            .find(|f| f.category == "proxy-grpc-session")
            .unwrap();
        assert!(grpc_finding.description.contains("host=grpc.example.com"));
        assert!(grpc_finding.description.contains("call_count=2"));
    }

    #[test]
    fn bridge_includes_correlation_summary() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        let mut ctx = CorrelationContext::new();
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-1",
            "DB finding",
        ));
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-1",
            "Auth finding",
        ));
        r.correlation = Some(ctx);

        let srd = to_scan_report_data_proxy(&r);
        assert_eq!(srd.findings.len(), 2);
        let corr_finding = srd
            .findings
            .iter()
            .find(|f| f.category == "proxy-correlation-summary")
            .unwrap();
        assert!(corr_finding.description.contains("total_references=2"));
        assert!(corr_finding.description.contains("unique_sources=2"));
    }

    #[test]
    fn bridge_web_traffic_summary_includes_protocol_counts() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
        r.ws_sessions.push(WebSocketSession::new(
            "wss://example.com/ws",
            "example.com",
            "/ws",
            true,
        ));
        let mut h2 = Http2Session::new("api.example.com", true);
        h2.add_stream(Http2Stream::new(1, "GET", "/data"));
        r.http2_sessions.push(h2);
        let mut grpc = GrpcSession::new("grpc.example.com", true);
        grpc.add_call(GrpcCall::new(
            "/pkg.Svc/Method",
            crate::intercept::protocols::GrpcMethodType::Unary,
        ));
        r.grpc_sessions.push(grpc);

        let mut ctx = CorrelationContext::new();
        ctx.add_reference(CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-1",
            "DB finding",
        ));
        r.correlation = Some(ctx);

        let srd = to_scan_report_data_proxy(&r);
        let summary = srd
            .findings
            .iter()
            .find(|f| f.category == "web-traffic-summary")
            .unwrap();
        assert!(summary.description.contains("ws_sessions=1"));
        assert!(summary.description.contains("http2_sessions=1"));
        assert!(summary.description.contains("grpc_sessions=1"));
        assert!(summary.description.contains("correlation_references=1"));
    }

    #[test]
    fn bridge_all_protocol_findings_together() {
        let mut r = WebProxySessionReport::new("127.0.0.1:8080", false);
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
            protocol: "http1".to_string(),
        });
        r.ws_sessions.push(WebSocketSession::new(
            "wss://example.com/ws",
            "example.com",
            "/ws",
            true,
        ));
        let mut h2 = Http2Session::new("api.example.com", true);
        h2.add_stream(Http2Stream::new(1, "GET", "/data"));
        r.http2_sessions.push(h2);
        let mut grpc = GrpcSession::new("grpc.example.com", true);
        grpc.add_call(GrpcCall::new(
            "/pkg.Svc/Method",
            crate::intercept::protocols::GrpcMethodType::Unary,
        ));
        r.grpc_sessions.push(grpc);

        let srd = to_scan_report_data_proxy(&r);
        // 1 flow + 1 ws + 1 http2 + 1 grpc + 1 summary = 5
        assert_eq!(srd.findings.len(), 5);
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "proxy-intercept-flow"));
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "proxy-websocket-session"));
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "proxy-http2-session"));
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "proxy-grpc-session"));
        assert!(srd
            .findings
            .iter()
            .any(|f| f.category == "web-traffic-summary"));
    }
}
