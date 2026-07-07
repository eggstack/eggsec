use eggsec_nse::report::{
    NseCapabilityEventSummary, NseCompatibilitySummary, NseEvidenceItem, NseLibraryUseReport,
    NseOutputSummary, NseRuleEvaluationReport, NseRunCompatibilityStatus, NseRunFidelity,
    NseRunReport,
};
use ratatui::{
    style::Style,
    text::{Line, Span},
};

use crate::tc;

const MAX_RAW_OUTPUT_LINES: usize = 200;
const TRUNCATION_NOTICE: &str = "...(output truncated, showing first 200 lines)";

/// Render a full `NseRunReport` into a sequence of styled `Line`s for TUI display.
pub fn render_report(report: &NseRunReport) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    lines.extend(render_summary(report));
    lines.push(Line::from(""));
    lines.extend(render_compatibility(&report.compatibility));
    lines.push(Line::from(""));
    lines.extend(render_rule_evaluation(&report.rules));
    lines.push(Line::from(""));
    lines.extend(render_libraries(&report.libraries));
    lines.push(Line::from(""));
    lines.extend(render_capability_denials(&report.capability_events));
    lines.push(Line::from(""));
    lines.extend(render_evidence(&report.evidence));
    lines.push(Line::from(""));
    lines.extend(render_raw_output(&report.output));
    lines.push(Line::from(""));
    lines.extend(render_diagnostics(report));

    lines
}

fn section_header(title: &str) -> Line<'static> {
    Line::from(Span::styled(
        format!("── {} ", title),
        Style::default().fg(tc!(info)),
    ))
}

fn styled_line(label: &str, value: String, style: Style) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{}: ", label), Style::default().fg(tc!(text_dim))),
        Span::styled(value, style),
    ])
}

// ── Summary ──────────────────────────────────────────────────────────────────

fn render_summary(report: &NseRunReport) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Summary"));

    lines.push(styled_line(
        "Target",
        report.target.clone(),
        Style::default().fg(tc!(text)),
    ));
    lines.push(styled_line(
        "Script",
        report.script_name.clone(),
        Style::default().fg(tc!(text)),
    ));
    lines.push(styled_line(
        "Source",
        format!(
            "{} ({})",
            report.script_source.label, report.script_source.kind
        ),
        Style::default().fg(tc!(text)),
    ));
    lines.push(styled_line(
        "Profile",
        report.profile.kind.clone(),
        Style::default().fg(tc!(text)),
    ));
    lines.push(styled_line(
        "Elapsed",
        format!("{:.2}s", report.stats.elapsed_secs),
        Style::default().fg(tc!(text)),
    ));

    // Status with color coding
    let (status_label, status_style) = status_display(&report.compatibility.status);
    lines.push(Line::from(vec![
        Span::styled("Status: ", Style::default().fg(tc!(text_dim))),
        Span::styled(status_label, status_style),
    ]));

    // Fidelity with ~ prefix for non-Full
    let fidelity_text = match report.compatibility.fidelity {
        NseRunFidelity::Full => "full".to_string(),
        NseRunFidelity::Approximate => "~approximate".to_string(),
        NseRunFidelity::Minimal => "~minimal".to_string(),
        NseRunFidelity::Unknown => "~unknown".to_string(),
    };
    let fidelity_style = match report.compatibility.fidelity {
        NseRunFidelity::Full => Style::default().fg(tc!(success)),
        _ => Style::default().fg(tc!(info)),
    };
    lines.push(Line::from(vec![
        Span::styled("Fidelity: ", Style::default().fg(tc!(text_dim))),
        Span::styled(fidelity_text, fidelity_style),
    ]));

    lines
}

// ── Compatibility ────────────────────────────────────────────────────────────

fn render_compatibility(compat: &NseCompatibilitySummary) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Compatibility"));

    let (label, style) = status_display(&compat.status);
    lines.push(Line::from(vec![
        Span::styled("  Status: ", Style::default().fg(tc!(text_dim))),
        Span::styled(label, style),
    ]));

    for feature in &compat.unsupported_features {
        lines.push(Line::from(Span::styled(
            format!("  [*] Unsupported feature: {}", feature),
            Style::default().fg(tc!(warning)),
        )));
    }

    for approx in &compat.approximations {
        lines.push(Line::from(Span::styled(
            format!("  [~] Approximation: {}", approx),
            Style::default().fg(tc!(info)),
        )));
    }

    if compat.unsupported_features.is_empty() && compat.approximations.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No compatibility issues.",
            Style::default().fg(tc!(text_dim)),
        )));
    }

    lines
}

// ── Rule Evaluation ──────────────────────────────────────────────────────────

fn render_rule_evaluation(rules: &[NseRuleEvaluationReport]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Rule Evaluation"));

    if rules.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No rules evaluated.",
            Style::default().fg(tc!(text_dim)),
        )));
        return lines;
    }

    for rule in rules {
        let kind_badge = format!("[{}]", rule.kind);
        let match_badge = if rule.evaluated {
            if rule.matched {
                "MATCHED"
            } else {
                "not matched"
            }
        } else {
            "not evaluated"
        };
        let match_style = if rule.matched {
            Style::default().fg(tc!(success))
        } else if !rule.evaluated {
            Style::default().fg(tc!(warning))
        } else {
            Style::default().fg(tc!(text_dim))
        };

        let exactness_label = if rule.exactness == "approximate" {
            format!("~{}", rule.exactness)
        } else {
            rule.exactness.clone()
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", kind_badge), Style::default().fg(tc!(info))),
            Span::styled(match_badge.to_string(), match_style),
            Span::styled(
                format!(" ({})", exactness_label),
                Style::default().fg(tc!(text_dim)),
            ),
        ]));

        lines.push(Line::from(Span::styled(
            format!("    {}", rule.summary),
            Style::default().fg(tc!(text)),
        )));

        // Context source
        if let Some(ref src) = rule.host_context_source {
            lines.push(Line::from(Span::styled(
                format!("    context: host={}", src),
                Style::default().fg(tc!(text_dim)),
            )));
        }
        if let Some(ref src) = rule.port_context_source {
            lines.push(Line::from(Span::styled(
                format!("    context: port={}", src),
                Style::default().fg(tc!(text_dim)),
            )));
        }

        // Fidelity reason
        if let Some(ref reason) = rule.fidelity_reason {
            lines.push(Line::from(Span::styled(
                format!("    [~] {}", reason),
                Style::default().fg(tc!(info)),
            )));
        }

        // Unsupported return type
        if let Some(ref unsupported) = rule.unsupported {
            lines.push(Line::from(Span::styled(
                format!("    [*] unsupported: {}", unsupported),
                Style::default().fg(tc!(warning)),
            )));
        }

        // Error
        if let Some(ref error) = rule.error {
            lines.push(Line::from(Span::styled(
                format!("    [-] {}", error),
                Style::default().fg(tc!(error)),
            )));
        }
    }

    lines
}

// ── Libraries ────────────────────────────────────────────────────────────────

fn render_libraries(libraries: &[NseLibraryUseReport]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Libraries"));

    if libraries.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No libraries loaded.",
            Style::default().fg(tc!(text_dim)),
        )));
        return lines;
    }

    for lib in libraries {
        let status_label = if lib.loaded {
            "loaded"
        } else if lib.registered {
            "registered"
        } else {
            "unregistered"
        };
        let status_style = if lib.loaded {
            Style::default().fg(tc!(success))
        } else if lib.registered {
            Style::default().fg(tc!(warning))
        } else {
            Style::default().fg(tc!(error))
        };

        lines.push(Line::from(vec![
            Span::styled(format!("  {} ", lib.name), Style::default().fg(tc!(text))),
            Span::styled(
                format!("[{}]", lib.category),
                Style::default().fg(tc!(text_dim)),
            ),
            Span::styled(format!(" {}", status_label), status_style),
        ]));

        if !lib.side_effects.is_empty() {
            let effects: Vec<&str> = lib.side_effects.iter().map(|s| s.as_str()).collect();
            lines.push(Line::from(Span::styled(
                format!("    effects: {}", effects.join(", ")),
                Style::default().fg(tc!(text_dim)),
            )));
        }

        for warning in &lib.warnings {
            lines.push(Line::from(Span::styled(
                format!("    [*] {}", warning),
                Style::default().fg(tc!(warning)),
            )));
        }
    }

    lines
}

// ── Capability Denials ───────────────────────────────────────────────────────

fn render_capability_denials(events: &[NseCapabilityEventSummary]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Capability Denials"));

    let denials: Vec<&NseCapabilityEventSummary> = events.iter().filter(|e| !e.allowed).collect();

    if denials.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No capability denials.",
            Style::default().fg(tc!(text_dim)),
        )));
        return lines;
    }

    for denial in denials {
        let mut parts = vec![
            Span::styled("  [!] ".to_string(), Style::default().fg(tc!(error))),
            Span::styled(denial.kind.clone(), Style::default().fg(tc!(error))),
        ];

        if let Some(ref target) = denial.target {
            parts.push(Span::styled(
                format!(" -> {}", target),
                Style::default().fg(tc!(text)),
            ));
        }

        lines.push(Line::from(parts));

        let reason = denial.reason.as_deref().unwrap_or("denied by policy");
        lines.push(Line::from(Span::styled(
            format!("      {}", reason),
            Style::default().fg(tc!(text_dim)),
        )));
    }

    lines
}

// ── Evidence ─────────────────────────────────────────────────────────────────

fn render_evidence(evidence: &[NseEvidenceItem]) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Evidence"));

    if evidence.is_empty() {
        lines.push(Line::from(Span::styled(
            "  No structured evidence.",
            Style::default().fg(tc!(text_dim)),
        )));
        return lines;
    }

    for item in evidence {
        let kind_badge = format!("[{}]", item.kind);
        let confidence_style = evidence_confidence_style(&item.confidence);

        let mut header_parts = vec![
            Span::styled(format!("  {} ", kind_badge), Style::default().fg(tc!(info))),
            Span::styled(item.title.clone(), Style::default().fg(tc!(text))),
            Span::styled(format!(" ({})", item.confidence), confidence_style),
        ];

        if let Some(ref service) = item.service {
            header_parts.push(Span::styled(
                format!(" [{}]", service),
                Style::default().fg(tc!(text_dim)),
            ));
        }
        if let Some(port) = item.port {
            header_parts.push(Span::styled(
                format!(":{}", port),
                Style::default().fg(tc!(text_dim)),
            ));
        }

        lines.push(Line::from(header_parts));
        lines.push(Line::from(Span::styled(
            format!("    {}", item.summary),
            Style::default().fg(tc!(text)),
        )));

        if let Some(ref excerpt) = item.raw_excerpt {
            lines.push(Line::from(Span::styled(
                format!("    raw: {}", excerpt),
                Style::default().fg(tc!(text_dim)),
            )));
        }

        if !item.references.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    refs: {}", item.references.join(", ")),
                Style::default().fg(tc!(info)),
            )));
        }

        if !item.tags.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("    tags: {}", item.tags.join(", ")),
                Style::default().fg(tc!(text_dim)),
            )));
        }
    }

    lines
}

// ── Raw Output ───────────────────────────────────────────────────────────────

fn render_raw_output(output: &NseOutputSummary) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Raw Output"));

    if !output.has_output || output.content.is_empty() {
        lines.push(Line::from(Span::styled(
            "  (no output)",
            Style::default().fg(tc!(text_dim)),
        )));
        return lines;
    }

    let content_lines: Vec<&str> = output.content.lines().collect();
    let shown = content_lines.len().min(MAX_RAW_OUTPUT_LINES);

    for line in &content_lines[..shown] {
        lines.push(Line::from(Span::styled(
            format!("  {}", line),
            Style::default().fg(tc!(text)),
        )));
    }

    if output.truncated || content_lines.len() > MAX_RAW_OUTPUT_LINES {
        lines.push(Line::from(Span::styled(
            format!("  {}", TRUNCATION_NOTICE),
            Style::default().fg(tc!(warning)),
        )));
        let total = output.line_count;
        lines.push(Line::from(Span::styled(
            format!("  (showing {}/{} lines)", shown, total),
            Style::default().fg(tc!(text_dim)),
        )));
    }

    lines
}

// ── Diagnostics ──────────────────────────────────────────────────────────────

fn render_diagnostics(report: &NseRunReport) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    lines.push(section_header("Diagnostics"));

    let mut has_content = false;

    // Resolver summary
    if report.resolver.total_diagnostics > 0 {
        has_content = true;
        lines.push(Line::from(Span::styled(
            format!(
                "  Resolver: {} resolved, {} blocked, {} rejected ({} total)",
                report.resolver.resolved_count,
                report.resolver.blocked_count,
                report.resolver.rejected_count,
                report.resolver.total_diagnostics,
            ),
            Style::default().fg(tc!(text)),
        )));
    }

    // Errors
    for err in &report.errors {
        has_content = true;
        lines.push(Line::from(Span::styled(
            format!("  [-] {}", err),
            Style::default().fg(tc!(error)),
        )));
    }

    // Warnings
    for warn in &report.warnings {
        has_content = true;
        lines.push(Line::from(Span::styled(
            format!("  [*] {}", warn),
            Style::default().fg(tc!(warning)),
        )));
    }

    // Profile warnings
    for warn in &report.profile.warnings {
        has_content = true;
        lines.push(Line::from(Span::styled(
            format!("  [*] profile: {}", warn),
            Style::default().fg(tc!(warning)),
        )));
    }

    // Limit violation
    if let Some(ref violation) = report.stats.limit_violation {
        has_content = true;
        lines.push(Line::from(Span::styled(
            format!("  [-] limit violation: {}", violation),
            Style::default().fg(tc!(error)),
        )));
    }

    if !has_content {
        lines.push(Line::from(Span::styled(
            "  No diagnostics.",
            Style::default().fg(tc!(text_dim)),
        )));
    }

    lines
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn status_display(status: &NseRunCompatibilityStatus) -> (String, Style) {
    match status {
        NseRunCompatibilityStatus::Compatible => {
            ("COMPATIBLE".to_string(), Style::default().fg(tc!(success)))
        }
        NseRunCompatibilityStatus::CompatibleWithWarnings => (
            "[*] COMPATIBLE_WITH_WARNINGS".to_string(),
            Style::default().fg(tc!(warning)),
        ),
        NseRunCompatibilityStatus::Partial => {
            ("PARTIAL".to_string(), Style::default().fg(tc!(warning)))
        }
        NseRunCompatibilityStatus::Unsupported => {
            ("UNSUPPORTED".to_string(), Style::default().fg(tc!(error)))
        }
        NseRunCompatibilityStatus::Failed => {
            ("[-] FAILED".to_string(), Style::default().fg(tc!(error)))
        }
        NseRunCompatibilityStatus::Unknown => {
            ("UNKNOWN".to_string(), Style::default().fg(tc!(text_dim)))
        }
    }
}

fn evidence_confidence_style(confidence: &str) -> Style {
    match confidence {
        "confirmed" => Style::default().fg(tc!(success)),
        "likely" => Style::default().fg(tc!(warning)),
        "possible" => Style::default().fg(tc!(info)),
        "low" => Style::default().fg(tc!(text_dim)),
        _ => Style::default().fg(tc!(text)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use eggsec_nse::report::{
        NseCapabilityEventSummary, NseCompatibilitySummary, NseEvidenceItem, NseEvidenceKind,
        NseLibraryUseReport, NseOutputSummary, NseRuleEvaluationReport, NseRunCompatibilityStatus,
        NseRunFidelity, NseRunReport,
    };

    fn extract_text(lines: &[Line<'_>]) -> String {
        lines
            .iter()
            .map(|l| {
                let spans: Vec<&str> = l.spans.iter().map(|s| s.content.as_ref()).collect();
                spans.join("")
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn make_compatible_report() -> NseRunReport {
        let mut report = NseRunReport::new("192.168.1.1", "ssl-cert");
        report.profile = eggsec_nse::report::NseProfileSummary {
            kind: "ManualPermissive".to_string(),
            audit_label: "manual".to_string(),
            warnings: vec![],
        };
        report.script_source = eggsec_nse::report::NseScriptSourceSummary {
            kind: "builtin".to_string(),
            label: "ssl-cert".to_string(),
            size: 0,
        };
        report.compatibility = NseCompatibilitySummary {
            status: NseRunCompatibilityStatus::Compatible,
            fidelity: NseRunFidelity::Full,
            unsupported_features: vec![],
            approximations: vec![],
        };
        report.output = NseOutputSummary {
            has_output: true,
            content: "SSL certificate: CN=example.com".to_string(),
            line_count: 1,
            truncated: false,
        };
        report
    }

    fn make_denied_report() -> NseRunReport {
        let mut report = make_compatible_report();
        report.compatibility.status = NseRunCompatibilityStatus::Partial;
        report.capability_events = vec![NseCapabilityEventSummary {
            kind: "process_exec".to_string(),
            operation: "os.execute".to_string(),
            target: Some("whoami".to_string()),
            allowed: false,
            reason: Some("denied by policy".to_string()),
        }];
        report.evidence = vec![NseEvidenceItem {
            id: "nse-ev-0".to_string(),
            kind: NseEvidenceKind::CapabilityDenial,
            title: "process_exec denied by policy".to_string(),
            summary: "denied by policy".to_string(),
            target: "192.168.1.1".to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: "ssl-cert".to_string(),
            raw_excerpt: None,
            references: vec![],
            tags: vec!["capability".to_string()],
        }];
        report
    }

    #[test]
    fn test_compatible_report_renders() {
        let report = make_compatible_report();
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("Target"), "should contain Target");
        assert!(full.contains("192.168.1.1"), "should contain target IP");
        assert!(full.contains("ssl-cert"), "should contain script name");
        assert!(
            full.contains("COMPATIBLE"),
            "should contain compatible status"
        );
    }

    #[test]
    fn test_denied_report_renders_denials() {
        let report = make_denied_report();
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("[!]"), "should contain denial prefix");
        assert!(full.contains("process_exec"), "should contain denial kind");
    }

    #[test]
    fn test_empty_report_renders_without_panic() {
        let report = NseRunReport::new("10.0.0.1", "test");
        let lines = render_report(&report);
        assert!(!lines.is_empty(), "should produce at least some lines");
        let full = extract_text(&lines);
        assert!(
            full.contains("No rules evaluated"),
            "should show no rules message"
        );
        assert!(
            full.contains("No libraries loaded"),
            "should show no libraries message"
        );
        assert!(
            full.contains("No capability denials"),
            "should show no denials message"
        );
    }

    #[test]
    fn test_evidence_and_raw_output_are_separate_sections() {
        let mut report = make_compatible_report();
        report.evidence = vec![NseEvidenceItem {
            id: "nse-ev-0".to_string(),
            kind: NseEvidenceKind::ScriptOutput,
            title: "Script output captured".to_string(),
            summary: "1 lines of output".to_string(),
            target: "192.168.1.1".to_string(),
            port: None,
            service: None,
            confidence: "confirmed".to_string(),
            source: "ssl-cert".to_string(),
            raw_excerpt: None,
            references: vec![],
            tags: vec!["output".to_string()],
        }];
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("Evidence"), "should contain Evidence section");
        assert!(
            full.contains("Raw Output"),
            "should contain Raw Output section"
        );
    }

    #[test]
    fn test_partial_report_renders() {
        let mut report = make_compatible_report();
        report.rules = vec![NseRuleEvaluationReport {
            kind: "portrule".to_string(),
            evaluated: true,
            matched: true,
            exactness: "exact".to_string(),
            error: None,
            summary: "rule matched".to_string(),
            unsupported: None,
            host_context_source: Some("scan".to_string()),
            port_context_source: None,
            service_context_available: None,
            fidelity_reason: None,
        }];
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("portrule"), "should contain rule kind");
        assert!(full.contains("MATCHED"), "should contain match status");
        assert!(
            full.contains("context: host=scan"),
            "should show context source"
        );
    }

    #[test]
    fn test_denied_report_has_no_approvals() {
        let report = make_denied_report();
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("PARTIAL"), "should show partial status");
        assert!(
            full.contains("denied by policy"),
            "should show denial reason"
        );
    }

    #[test]
    fn test_library_section_renders() {
        let mut report = make_compatible_report();
        report.libraries = vec![NseLibraryUseReport {
            name: "nmap".to_string(),
            category: "Utility".to_string(),
            registered: true,
            side_effects: vec![],
            fallback_behavior: "HardFail".to_string(),
            notes: "core library".to_string(),
            loaded: true,
            warnings: vec![],
        }];
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("nmap"), "should contain library name");
        assert!(
            full.contains("[Utility]"),
            "should contain library category"
        );
        assert!(full.contains("loaded"), "should show loaded status");
    }

    #[test]
    fn test_diagnostics_section_renders() {
        let mut report = make_compatible_report();
        report.errors = vec!["some error".to_string()];
        report.warnings = vec!["some warning".to_string()];
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(full.contains("some error"), "should contain error message");
        assert!(
            full.contains("some warning"),
            "should contain warning message"
        );
    }

    #[test]
    fn test_compatibility_issues_render() {
        let mut report = make_compatible_report();
        report.compatibility = NseCompatibilitySummary {
            status: NseRunCompatibilityStatus::CompatibleWithWarnings,
            fidelity: NseRunFidelity::Approximate,
            unsupported_features: vec!["some_feature".to_string()],
            approximations: vec!["portrule: approximate match".to_string()],
        };
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(
            full.contains("Unsupported feature: some_feature"),
            "should show unsupported feature"
        );
        assert!(
            full.contains("Approximation: portrule"),
            "should show approximation"
        );
    }

    #[test]
    fn test_raw_output_truncation() {
        let mut report = make_compatible_report();
        let long_content = "line\n".repeat(250);
        report.output = NseOutputSummary {
            has_output: true,
            content: long_content.clone(),
            line_count: 250,
            truncated: false,
        };
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(
            full.contains("output truncated"),
            "should show truncation notice"
        );
    }

    #[test]
    fn test_limit_violation_renders() {
        let mut report = make_compatible_report();
        report.stats.limit_violation = Some("wall clock timeout".to_string());
        let lines = render_report(&report);
        let full = extract_text(&lines);
        assert!(
            full.contains("limit violation"),
            "should show limit violation"
        );
    }
}
