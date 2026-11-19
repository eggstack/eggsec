//! Integration tests for the self-contained HTML rendering mode.
//!
//! The self-contained variant exists for embedders whose Content Security
//! Policy forbids any remote origin. It must carry no remote reference at
//! all, while the default rendering keeps the CLI's artifact unchanged.

use eggsec_output::convert::{convert_to_html, convert_to_html_self_contained};
use eggsec_output::html::HtmlReport;
use eggsec_output::markdown::{Finding, ScanSummary};
use eggsec_output::{FindingData, PortData, ScanReportData, ServiceData};

/// The remote chart-library reference the default rendering emits, as one
/// whole line (indentation and trailing newline included).
const REMOTE_CHART_SCRIPT_LINE: &str =
    "    <script src=\"https://cdn.jsdelivr.net/npm/chart.js\"></script>\n";

/// A minimal report with one finding, one port, one service, and one
/// wireless network. Its content carries no URL, so the remote-origin
/// assertions measure the renderer rather than the fixture.
fn sample_report() -> ScanReportData {
    ScanReportData {
        target: "example.test".to_string(),
        scan_type: "web".to_string(),
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        findings: vec![FindingData {
            title: "Reflected XSS in search".to_string(),
            severity: "High".to_string(),
            category: "xss".to_string(),
            description: "Unescaped query parameter reflected in the response.".to_string(),
            location: "/search?q=test".to_string(),
            evidence: Some("Response body reflects the query parameter verbatim.".to_string()),
            remediation: Some("Escape the output before echoing it back.".to_string()),
            cwe_ids: vec!["CWE-79".to_string()],
        }],
        open_ports: vec![PortData {
            port: 443,
            status: "open".to_string(),
            protocol: Some("tcp".to_string()),
            state: Some("open".to_string()),
            service: Some("nginx".to_string()),
            version: Some("1.24.0".to_string()),
            banner: None,
        }],
        services: vec![ServiceData {
            service: "nginx".to_string(),
            version: Some("1.24.0".to_string()),
            banner: None,
        }],
        duration_ms: 1234,
        wireless_networks: vec![],
        policy_summary: None,
    }
}

#[test]
fn default_html_keeps_the_remote_chart_library_reference() {
    let report = sample_report();

    let html = convert_to_html(&report);

    assert!(
        html.contains(REMOTE_CHART_SCRIPT_LINE),
        "the default rendering must keep the CLI's remote chart-library reference"
    );
    assert!(html.contains("https://cdn.jsdelivr.net/npm/chart.js"));
}

#[test]
fn self_contained_html_carries_no_remote_origin_reference() {
    let report = sample_report();

    let html = convert_to_html_self_contained(&report);

    assert!(
        !html.contains("http://"),
        "self-contained HTML must not reference an http origin: {html}"
    );
    assert!(
        !html.contains("https://"),
        "self-contained HTML must not reference an https origin: {html}"
    );
    assert!(!html.contains("cdn.jsdelivr.net"));
}

#[test]
fn self_contained_html_fetches_nothing_from_a_remote_origin() {
    // The guarantee that matters for a Content Security Policy is about the
    // document's own references -- the attributes a browser would resolve
    // against a remote origin. Report *content* may legitimately quote a
    // URL (a finding's evidence, an advisory link); such a string is inert
    // text, not a fetch, and stripping it would corrupt the report.
    let mut report = sample_report();
    report.findings[0].description = "Advisory: https://example.test/advisory".to_string();

    let html = convert_to_html_self_contained(&report);

    assert!(!html.contains("src=\"http"));
    assert!(!html.contains("href=\"http"));
    assert!(!html.contains("url(http"));
    assert!(!html.contains("src='http"));
    assert!(!html.contains("href='http"));
    // The URL survives as escaped text, which is correct and inert.
    assert!(html.contains("https://example.test/advisory"));
}

#[test]
fn self_contained_html_keeps_the_report_substantive_content() {
    let report = sample_report();

    let html = convert_to_html_self_contained(&report);

    assert!(
        html.contains("example.test"),
        "the target must survive the self-contained rendering"
    );
    assert!(
        html.contains("Reflected XSS in search"),
        "the finding title must survive the self-contained rendering"
    );
    assert!(
        html.contains("xss"),
        "the finding category must survive the self-contained rendering"
    );
    assert!(
        html.contains("/search?q=test"),
        "the finding location must survive the self-contained rendering"
    );
}

#[test]
fn self_contained_html_differs_from_the_default_only_by_the_remote_reference() {
    let report = sample_report();

    let default_html = convert_to_html(&report);
    let self_contained_html = convert_to_html_self_contained(&report);

    assert!(
        default_html.contains(REMOTE_CHART_SCRIPT_LINE),
        "the default rendering must carry the remote reference to strip"
    );
    assert_eq!(
        default_html.replace(REMOTE_CHART_SCRIPT_LINE, ""),
        self_contained_html,
        "the two renderings must differ only by the remote chart-library line"
    );
}

#[test]
fn html_report_self_contained_renders_no_remote_reference() {
    let report = sample_report();
    let summary = ScanSummary::from(&report);
    let findings: Vec<Finding> = report.findings.iter().map(|f| f.into()).collect();
    let html_report = HtmlReport::new(summary, findings);

    let default_html = html_report.generate();
    let self_contained_html = html_report.generate_self_contained();

    assert!(default_html.contains(REMOTE_CHART_SCRIPT_LINE));
    assert!(!self_contained_html.contains("http://"));
    assert!(!self_contained_html.contains("https://"));
    assert_eq!(
        default_html.replace(REMOTE_CHART_SCRIPT_LINE, ""),
        self_contained_html
    );
}

#[test]
fn html_report_self_contained_keeps_the_report_content() {
    let report = sample_report();
    let summary = ScanSummary::from(&report);
    let findings: Vec<Finding> = report.findings.iter().map(|f| f.into()).collect();
    let html_report = HtmlReport::new(summary, findings);

    let html = html_report.generate_self_contained();

    assert!(html.contains("example.test"));
    assert!(html.contains("Reflected XSS in search"));
}
