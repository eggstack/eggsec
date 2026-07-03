/// Metadata descriptor for rendering a specific result envelope kind.
///
/// This is not a code plugin system. It is metadata that enables consistent
/// frontend rendering across TUI, CLI JSON, web UI, and future desktop/mobile
/// clients. Each known `TaskResultEnvelope.kind` should have a corresponding
/// entry in the registry.
#[derive(Debug, Clone)]
pub struct ResultRendererDescriptor {
    /// The kind discriminator string matching `TaskResultEnvelope.kind`.
    pub kind: &'static str,
    /// Human-readable title for display in UIs.
    pub title: &'static str,
    /// Key field names from the payload for summary display.
    pub summary_fields: &'static [&'static str],
    /// Artifact kinds this renderer expects.
    pub artifact_kinds: &'static [&'static str],
    /// Whether the TUI has rich rendering support for this kind.
    pub supports_rich_tui: bool,
    /// Whether JSON detail output is meaningful for this kind.
    pub supports_json_detail: bool,
}

/// Static registry of known result renderer descriptors.
///
/// Covers all task kinds that produce `TaskResultEnvelope` outcomes.
/// Unknown kinds degrade gracefully (rendered as generic JSON).
pub const RENDERER_REGISTRY: &[ResultRendererDescriptor] = &[
    ResultRendererDescriptor {
        kind: "port-scan",
        title: "Port Scan",
        summary_fields: &["open_ports", "total_scanned"],
        artifact_kinds: &["scan-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "endpoint-scan",
        title: "Endpoint Scan",
        summary_fields: &["endpoints_found"],
        artifact_kinds: &["scan-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "fingerprint",
        title: "Fingerprint",
        summary_fields: &["services"],
        artifact_kinds: &["fingerprint-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "load-test",
        title: "Load Test",
        summary_fields: &["requests_per_second", "latency_p99"],
        artifact_kinds: &["histogram", "latency-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "stress-test",
        title: "Stress Test",
        summary_fields: &["packets_sent", "errors"],
        artifact_kinds: &["stress-report"],
        supports_rich_tui: false,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "fuzz",
        title: "Fuzz",
        summary_fields: &["findings", "payloads_tested"],
        artifact_kinds: &["fuzz-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "waf",
        title: "WAF Detection",
        summary_fields: &["waf_detected", "waf_name"],
        artifact_kinds: &["waf-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "waf-stress",
        title: "WAF Stress",
        summary_fields: &["requests_sent", "blocked"],
        artifact_kinds: &["waf-report"],
        supports_rich_tui: false,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "pipeline",
        title: "Pipeline",
        summary_fields: &["stages_completed", "findings"],
        artifact_kinds: &["pipeline-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "recon",
        title: "Recon",
        summary_fields: &["subdomains", "hosts"],
        artifact_kinds: &["recon-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "packet-capture",
        title: "Packet Capture",
        summary_fields: &["packets_captured"],
        artifact_kinds: &["pcap"],
        supports_rich_tui: false,
        supports_json_detail: false,
    },
    ResultRendererDescriptor {
        kind: "traceroute",
        title: "Traceroute",
        summary_fields: &["hops"],
        artifact_kinds: &["traceroute-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "graphql",
        title: "GraphQL",
        summary_fields: &["schema_found", "endpoints"],
        artifact_kinds: &["graphql-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "oauth",
        title: "OAuth",
        summary_fields: &["flow_tested", "vulnerabilities"],
        artifact_kinds: &["oauth-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "auth-test",
        title: "Auth Test",
        summary_fields: &["credentials_tested", "weaknesses"],
        artifact_kinds: &["auth-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "nse",
        title: "NSE Script",
        summary_fields: &["script", "output_lines"],
        artifact_kinds: &["nse-output"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "hunt",
        title: "Vulnerability Hunt",
        summary_fields: &["vulns_found", "severity"],
        artifact_kinds: &["hunt-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "browser",
        title: "Browser",
        summary_fields: &["pages_loaded", "findings"],
        artifact_kinds: &["browser-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "compliance",
        title: "Compliance",
        summary_fields: &["checks_passed", "checks_failed"],
        artifact_kinds: &["compliance-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "db-pentest",
        title: "DB Pentest",
        summary_fields: &["findings", "db_type"],
        artifact_kinds: &["db-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "wireless",
        title: "Wireless Recon",
        summary_fields: &["networks_found", "clients"],
        artifact_kinds: &["wireless-report"],
        supports_rich_tui: true,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "intercept",
        title: "Intercept Proxy",
        summary_fields: &["requests_captured"],
        artifact_kinds: &["traffic-log"],
        supports_rich_tui: false,
        supports_json_detail: true,
    },
    ResultRendererDescriptor {
        kind: "c2",
        title: "C2 Simulation",
        summary_fields: &["beacons", "commands"],
        artifact_kinds: &["c2-report"],
        supports_rich_tui: false,
        supports_json_detail: true,
    },
];

/// Look up a renderer descriptor by result kind.
///
/// Returns `None` for unknown kinds — callers should degrade gracefully
/// by rendering the payload as generic JSON.
pub fn renderer_for_kind(kind: &str) -> Option<&'static ResultRendererDescriptor> {
    RENDERER_REGISTRY.iter().find(|r| r.kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn renderer_registry_covers_known_kinds() {
        let known = [
            "port-scan",
            "endpoint-scan",
            "fingerprint",
            "load-test",
            "stress-test",
            "fuzz",
            "waf",
            "waf-stress",
            "pipeline",
            "recon",
            "packet-capture",
            "traceroute",
            "graphql",
            "oauth",
            "auth-test",
            "nse",
            "hunt",
            "browser",
            "compliance",
            "db-pentest",
            "wireless",
            "intercept",
            "c2",
        ];
        for kind in &known {
            assert!(
                renderer_for_kind(kind).is_some(),
                "Missing renderer for: {kind}"
            );
        }
    }

    #[test]
    fn renderer_registry_no_duplicates() {
        let mut kinds: Vec<_> = RENDERER_REGISTRY.iter().map(|r| r.kind).collect();
        kinds.sort();
        kinds.dedup();
        assert_eq!(
            kinds.len(),
            RENDERER_REGISTRY.len(),
            "Duplicate renderer kinds found"
        );
    }

    #[test]
    fn unknown_kind_returns_none() {
        assert!(renderer_for_kind("nonexistent").is_none());
    }
}
