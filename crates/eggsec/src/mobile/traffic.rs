//! Mobile dynamic traffic summary (Phase 2, under `mobile-dynamic`).
//!
//! Provides `TrafficSummary` and a lenient parser for high-level network
//! observation during dynamic runs.
//!
//! Supported inputs (Phase 2a, summary only):
//! - Plain text logs (mitmproxy-style or ad-hoc) containing request lines with URLs.
//! - Minimal HAR JSON (log.entries[*].request.url).
//!
//! The goal is high-signal summary + a few generated `DynamicMobileFinding`
//! entries (cleartext endpoints, suspicious patterns) for the report and bridge.
//! Full body capture, deep inspection, and automatic mitmproxy lifecycle are
//! out of scope for Phase 2a (see plan).

use crate::types::Severity;
use serde::{Deserialize, Serialize};

use super::dynamic::DynamicMobileFinding;

/// High-level traffic summary captured or provided for a dynamic mobile run.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TrafficSummary {
    pub total_requests: u32,
    pub cleartext_requests: u32,
    pub unique_domains: Vec<String>,
    /// Endpoints (full or path+host) flagged as suspicious (e.g. cleartext + sensitive path).
    pub suspicious_endpoints: Vec<String>,
    /// Generated findings derived from the capture (cleartext, suspicious, etc.).
    pub findings: Vec<DynamicMobileFinding>,
}

impl TrafficSummary {
    pub fn new() -> Self {
        Self::default()
    }
}

/// Parse a traffic capture (text log or minimal HAR) into a TrafficSummary.
///
/// Strategy (lenient, no external parser deps):
/// - Input is defensively truncated at 1 MiB to bound memory/CPU.
/// - If input looks like JSON (starts with '{' or contains '"log"' + '"entries"'), attempt minimal HAR walk.
/// - Otherwise treat as text: scan lines for http:// and https:// URLs or host hints.
/// - Count totals, cleartext, unique hosts (casefolded, no port for domain key).
/// - Flag suspicious if scheme is http and path contains login|token|auth|session|key|secret|oauth|api_key patterns.
/// - Emit findings for cleartext observed and suspicious endpoints.
const MAX_CAPTURE_INPUT: usize = 1024 * 1024; // 1 MiB safety cap for parser input

pub fn parse_traffic_capture(input: &str) -> TrafficSummary {
    let input = if input.len() > MAX_CAPTURE_INPUT {
        &input[..MAX_CAPTURE_INPUT]
    } else {
        input
    };

    let mut sum = TrafficSummary::new();

    let trimmed = input.trim();
    if trimmed.starts_with('{') || (trimmed.contains("\"log\"") && trimmed.contains("\"entries\"")) {
        if let Some(har) = try_parse_minimal_har(trimmed) {
            return har;
        }
        // fall through to text if HAR parse didn't yield anything useful
    }

    parse_text_traffic(trimmed, &mut sum);
    post_process(&mut sum);
    sum
}

fn try_parse_minimal_har(json: &str) -> Option<TrafficSummary> {
    // Very small hand-rolled extraction to avoid pulling serde for nested only here.
    // We look for "url" : "..." inside entries.
    // This is best-effort; full HAR consumers can pre-summarize to text if needed.
    // Bounded to avoid pathological inputs even after outer size cap.
    let mut sum = TrafficSummary::new();
    let mut search = json;
    let mut guard = 0usize;
    const MAX_URLS: usize = 10000;
    while let Some(pos) = search.find("\"url\"") {
        if guard >= MAX_URLS {
            break;
        }
        guard += 1;
        // advance past "url" and find the value
        let rest = &search[pos + 5..];
        if let Some(colon) = rest.find(':') {
            let val_start = colon + 1;
            let val = rest[val_start..].trim_start();
            if let Some(stripped) = val.strip_prefix('"') {
                if let Some(endq) = stripped.find('"') {
                    let url = &stripped[..endq];
                    ingest_url(url, &mut sum);
                    // continue search after this url
                    search = &stripped[endq + 1..];
                    continue;
                }
            }
        }
        // advance to avoid infinite
        search = &search[pos + 5..];
    }
    if sum.total_requests == 0 {
        return None;
    }
    post_process(&mut sum);
    Some(sum)
}

fn parse_text_traffic(text: &str, sum: &mut TrafficSummary) {
    for line in text.lines() {
        let l = line.trim();
        if l.is_empty() {
            continue;
        }
        // Common mitmproxy / log formats often include the URL plainly or as "-> URL" or "URL "
        // Also support "GET http://..." or just "http://..." on the line.
        for token in l.split_whitespace() {
            if token.starts_with("http://") || token.starts_with("https://") {
                ingest_url(token, sum);
            } else if token.contains("://") {
                // other schemes ignored for our mobile http focus
            }
        }
        // Also catch bare hosts in some logs e.g. "Host: api.foo.com" + later path, but for Phase 2a we focus on full URL lines.
        if l.to_ascii_lowercase().contains("host:") {
            // best effort: if we see "host: foo" and previous context had a scheme-less path, but skip complex reconstruction.
        }
    }
}

fn ingest_url(url: &str, sum: &mut TrafficSummary) {
    sum.total_requests = sum.total_requests.saturating_add(1);

    let lower = url.to_ascii_lowercase();
    let is_clear = lower.starts_with("http://");
    if is_clear {
        sum.cleartext_requests = sum.cleartext_requests.saturating_add(1);
    }

    // Extract host (between :// and next / or :port or end)
    let host = extract_host(url);
    if !host.is_empty() && !sum.unique_domains.iter().any(|d| d.eq_ignore_ascii_case(&host)) {
        sum.unique_domains.push(host.clone());
    }

    // Suspicious if cleartext + sensitive path or obvious key in query
    let suspicious = is_clear && is_suspicious_url(&lower);
    if suspicious {
        // store a compact form: scheme://host + path (no query secrets)
        let compact = sanitize_for_listing(url);
        if !sum.suspicious_endpoints.iter().any(|e| e == &compact) {
            sum.suspicious_endpoints.push(compact.clone());
        }
        // also emit a finding immediately
        sum.findings.push(DynamicMobileFinding {
            category: "traffic-cleartext".to_string(),
            severity: Severity::Medium,
            title: "Cleartext endpoint observed in traffic capture".to_string(),
            description: "App or runtime contacted a non-TLS endpoint during the dynamic session.".to_string(),
            recommendation: "Enforce TLS for all egress. Update network security config and code to reject cleartext.".to_string(),
            evidence: Some(compact),
            static_correlation: None,
        });
    }

    if is_clear {
        // Always emit a lighter cleartext-observed from traffic for visibility (even non-suspicious)
        let compact = sanitize_for_listing(url);
        // Avoid flooding: only add a generic cleartext finding once per unique host or first few
        let already = sum.findings.iter().any(|f| {
            f.category == "traffic-cleartext" && f.evidence.as_ref().is_some_and(|e| e.contains(&host))
        });
        if !already {
            sum.findings.push(DynamicMobileFinding {
                category: "traffic-cleartext".to_string(),
                severity: Severity::Low,
                title: "Cleartext HTTP traffic observed".to_string(),
                description: "Non-TLS network activity captured during run.".to_string(),
                recommendation: "Prefer HTTPS everywhere; pin certificates where appropriate.".to_string(),
                evidence: Some(compact),
                static_correlation: None,
            });
        }
    }
}

fn extract_host(url: &str) -> String {
    // after :// up to / or : or end
    if let Some(after_scheme) = url.split_once("://").map(|(_, r)| r) {
        let host_part = after_scheme.split(['/', '?', '#']).next().unwrap_or(after_scheme);
        // strip port if present
        let h = host_part.split(':').next().unwrap_or(host_part);
        return h.to_string();
    }
    String::new()
}

fn is_suspicious_url(lower_url: &str) -> bool {
    let path_and_query = lower_url.split_once("://").map(|(_, r)| r).unwrap_or(lower_url);
    path_and_query.contains("/login")
        || path_and_query.contains("/auth")
        || path_and_query.contains("/oauth")
        || path_and_query.contains("/token")
        || path_and_query.contains("/session")
        || path_and_query.contains("api_key")
        || path_and_query.contains("secret")
        || path_and_query.contains("password")
        || path_and_query.contains("sk_live")
        || path_and_query.contains("sk_test")
}

fn sanitize_for_listing(url: &str) -> String {
    // Keep scheme://host/path , truncate query, redact obvious secrets in evidence form
    let mut s = url.to_string();
    if let Some((pre, _q)) = s.split_once('?') {
        s = pre.to_string();
    }
    // basic redact in the path portion too (expanded set for Phase 2 polish)
    for pat in [
        "api_key=",
        "secret=",
        "password=",
        "token=",
        "sk_live_",
        "sk_test_",
        "auth=",
        "apikey=",
        "api-key=",
        "access_token=",
        "refresh_token=",
        "session=",
        "cookie=",
        "key=",
        "private_key=",
        "bearer ",
    ] {
        if let Some(pos) = s.to_ascii_lowercase().find(&pat.to_ascii_lowercase()) {
            let start = pos + pat.len();
            let mut end = s.len();
            for (i, c) in s[start..].char_indices() {
                if c.is_whitespace() || c == '&' || c == '"' || c == '\'' || c == '\0' {
                    end = start + i;
                    break;
                }
            }
            s.replace_range(pos..end, &format!("{}[REDACTED]", pat));
        }
    }
    if s.len() > 200 {
        s.truncate(197);
        s.push_str("...");
    }
    s
}

fn post_process(sum: &mut TrafficSummary) {
    // Dedup domains case-insensitively (already done in ingest), sort for determinism
    sum.unique_domains.sort_by_key(|a| a.to_ascii_lowercase());
    sum.suspicious_endpoints.sort();
    // findings are emitted during ingest; keep order of appearance
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_text_with_cleartext_and_suspicious() {
        let log = r#"
127.0.0.1:5555 -> GET http://api.example.com/v1/profile 200
POST https://auth.example.com/oauth/token HTTP/1.1
GET http://api.example.com/login?user=foo&api_key=sk_live_SECRET 401
"#;
        let s = parse_traffic_capture(log);
        assert!(s.total_requests >= 2);
        assert!(s.cleartext_requests >= 2);
        assert!(s.unique_domains.iter().any(|d| d.contains("api.example.com")));
        assert!(s.unique_domains.iter().any(|d| d.contains("auth.example.com")));
        assert!(!s.suspicious_endpoints.is_empty());
        assert!(s.findings.iter().any(|f| f.category == "traffic-cleartext"));
        // at least one finding should mention redaction or suspicious path
        let has_susp = s.findings.iter().any(|f| f.evidence.as_ref().map_or(false, |e| e.contains("[REDACTED]") || e.contains("/login")));
        assert!(has_susp);
    }

    #[test]
    fn parses_minimal_har() {
        let har = r#"{
  "log": {
    "entries": [
      { "request": { "url": "http://insecure.test/v1/data?token=abc" } },
      { "request": { "url": "https://secure.test/api" } }
    ]
  }
}"#;
        let s = parse_traffic_capture(har);
        assert!(s.total_requests >= 2);
        assert!(s.cleartext_requests >= 1);
        assert!(s.unique_domains.iter().any(|d| d == "insecure.test"));
    }

    #[test]
    fn empty_or_noise_yields_zero() {
        let s = parse_traffic_capture("just some noise\nno urls here\n");
        assert_eq!(s.total_requests, 0);
        assert!(s.findings.is_empty());
    }

    #[test]
    fn unique_domains_dedup_and_sorted() {
        let log = "http://B.example.com/x\nhttp://a.example.com/y\nhttp://B.example.com/z";
        let s = parse_traffic_capture(log);
        // Case-insensitive alpha sort on lowered keys; first-seen casing is preserved for the kept entry.
        // Lower cmp yields "a" before "b" => a.example before B.example in the final vec.
        assert_eq!(s.unique_domains, vec!["a.example.com".to_string(), "B.example.com".to_string()]);
    }

    #[test]
    fn suspicious_endpoint_generation_for_sensitive_paths() {
        let log = r#"
GET http://api.test/login?user=1
POST http://auth.test/oauth/token
http://leak.test/session?id=secret123
https://good.test/api?key=sk_live_xxx
"#;
        let s = parse_traffic_capture(log);
        assert!(s.suspicious_endpoints.iter().any(|e| e.contains("/login")));
        assert!(s.suspicious_endpoints.iter().any(|e| e.contains("/oauth/token")));
        assert!(s.suspicious_endpoints.iter().any(|e| e.contains("/session")));
        // findings emitted for suspicious (medium) + generic cleartext (low)
        assert!(s.findings.iter().any(|f| f.category == "traffic-cleartext"
            && f.severity == crate::types::Severity::Medium
            && f.evidence.as_ref().map_or(false, |e| e.contains("/login"))));
    }

    #[test]
    fn proxy_simulation_path_via_synthetic_traffic_and_bridge_roundtrips() {
        // --proxy simulation feeds traffic (user runs mitm externally); here we parse a mitmproxy-style log
        // and exercise DynamicMobileReport.traffic_summary + format + to_scan_report_data_dynamic bridge
        let proxy_log = "127.0.0.1:8080 -> GET http://insecure.proxy.test/v1/profile 200\nPOST https://secure.proxy.test/auth";
        let sum = parse_traffic_capture(proxy_log);
        assert!(sum.total_requests >= 2);
        assert!(sum.cleartext_requests >= 1);
        assert!(sum.unique_domains.iter().any(|d| d.contains("insecure.proxy.test")));

        let mut r = crate::mobile::DynamicMobileReport::new("app.apk");
        r.traffic_summary = Some(sum.clone());
        let pretty = crate::mobile::format_dynamic_report(&r);
        assert!(pretty.contains("Phase 2 extensions present:"));
        assert!(pretty.contains("traffic: requests="));
        // format_dynamic_report surfaces a count-based traffic summary line (domains=N etc.); the actual domain strings
        // live in the native traffic_summary.unique_domains (and in the bridged "mobile-dynamic-android-traffic-summary" info finding description).
        assert!(pretty.contains("domains="));

        let data = crate::mobile::to_scan_report_data_dynamic(&r);
        assert!(data.findings.iter().any(|f| f.category == "mobile-dynamic-android-traffic-summary"));
        // the bridge description includes the domain count/details
        assert!(data.findings.iter().any(|f| f.category == "mobile-dynamic-android-traffic-summary" && f.description.contains("domains=")));
        // roundtrip via report carrying the summary
        let j = serde_json::to_string(&r).unwrap();
        let back: crate::mobile::DynamicMobileReport = serde_json::from_str(&j).unwrap();
        assert!(back.traffic_summary.is_some());
        assert_eq!(back.traffic_summary.as_ref().unwrap().suspicious_endpoints.len(), sum.suspicious_endpoints.len());
        assert!(back.traffic_summary.as_ref().unwrap().unique_domains.iter().any(|d| d.contains("insecure.proxy.test")));
    }

    #[test]
    fn large_input_is_truncated_defensively() {
        // Outer size guard in parser input (1 MiB cap) + early truncation behavior.
        let big = "http://big.test/x\n".repeat(200_000); // ~3+ MiB raw
        let s = parse_traffic_capture(&big);
        // Should still parse first entries and not OOM or hang; total bounded.
        assert!(s.total_requests > 0);
        assert!(s.total_requests <= 100_000); // generous but bounded
    }

    #[test]
    fn malformed_har_falls_back_gracefully() {
        let bad = r#"{"log":{"entries":[{"request":{"url":"http://bad.test/a"}}, {"request":{}} ]}}"#;
        let s = parse_traffic_capture(bad);
        // Minimal valid url should still be ingested; malformed entry ignored.
        assert!(s.total_requests >= 1);
        assert!(s.unique_domains.iter().any(|d| d.contains("bad.test")));
    }

    #[test]
    fn very_long_lines_and_mixed_schemes_are_tolerated() {
        let long = format!("http://long.test/{} \n", "x".repeat(300));
        let mixed = format!(
            "{}\nftp://ignore.test/x\nhttps://good.test/y\nws://skip.test/z\n",
            long
        );
        let s = parse_traffic_capture(&mixed);
        assert!(s.total_requests >= 2);
        assert!(s.unique_domains.iter().any(|d| d.contains("long.test")));
        assert!(s.unique_domains.iter().any(|d| d.contains("good.test")));
        // non-http(s) ignored
        assert!(!s.unique_domains.iter().any(|d| d.contains("ignore.test") || d.contains("skip.test")));
    }

    #[test]
    fn sanitize_redacts_expanded_secret_patterns() {
        // Secrets embedded in *path* (query is stripped before redaction; path-embedded secrets are redacted).
        let log = "GET http://leak.test/login/api-keySECRET123?user=1\nhttp://x.test/p/private_key=XYZ\nhttp://b.test/bearer TOKEN123";
        let s = parse_traffic_capture(log);
        let evs: Vec<_> = s.findings.iter().filter_map(|f| f.evidence.as_ref()).collect();
        let joined: String = evs.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(" ");
        assert!(joined.contains("api-key=[REDACTED]") || joined.contains("private_key=[REDACTED]") || joined.contains("bearer [REDACTED]"));
    }
}
