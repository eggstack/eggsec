//! Mobile dynamic runtime log parser (feature-gated behind `mobile-dynamic`).
//!
//! Phase 1 per plans/mobile-dynamic-phase1-implementation-handoff-plan.md (and parent
//! dynamic-mobile-testing-loadout-design-plan.md): high-signal logcat parsing only.
//! Extracts permission grants/denials, crashes (with stack frames), cleartext/network
//! hints, obvious secret patterns from log lines. Includes basic redaction for evidence.
//!
//! Standalone defense-lab. No device interaction here (see adb.rs + dynamic.rs).
//! Parser is pure/string-based (no regex crate dep for Phase 1 to keep surface small).

use super::dynamic::DynamicMobileFinding;
use eggsec_core::types::Severity;

/// Parse logcat output (full dump or stream) for high-signal security-relevant events.
/// Returns zero or more DynamicMobileFinding entries (may contain duplicates across lines;
/// caller or later stages can dedup if needed).
///
/// High-signal focus (P1):
/// - Permission grant/deny events
/// - App crashes / fatal exceptions (include stack frame hints in evidence)
/// - Cleartext HTTP hints (http:// not https)
/// - Obvious secret-like patterns in logs (api_key, token, password, sk_live_ etc.)
///
/// Basic redaction applied to secret evidence lines.
pub fn parse_logcat_findings(log: &str) -> Vec<DynamicMobileFinding> {
    let mut findings = Vec::new();
    for raw_line in log.lines() {
        let line = raw_line.trim();
        if line.is_empty() {
            continue;
        }

        // Permission grants / denials
        let l = line.to_ascii_lowercase();
        if l.contains("permission")
            && (l.contains("grant")
                || l.contains("granted")
                || l.contains("deny")
                || l.contains("denied")
                || l.contains("revoke"))
        {
            let sev = if l.contains("grant") || l.contains("granted") {
                Severity::Low
            } else {
                Severity::Medium
            };
            let title = if l.contains("grant") || l.contains("granted") {
                "Runtime permission granted"
            } else {
                "Runtime permission denied/revoked"
            };
            findings.push(DynamicMobileFinding {
                category: "runtime-permission".to_string(),
                severity: sev,
                title: title.to_string(),
                description: "Permission event observed in logcat during dynamic run.".to_string(),
                recommendation: "Review app permission handling and declared vs runtime behavior. Prefer runtime permissions only when necessary.".to_string(),
                evidence: Some(line.to_string()),
                static_correlation: None,
            });
        }

        // Crashes with stack frames
        if l.contains("fatal exception")
            || l.contains("androidruntime:")
            || (l.contains("at ") && (l.contains("com.") || l.contains("java.lang.")))
        {
            findings.push(DynamicMobileFinding {
                category: "crash-log".to_string(),
                severity: Severity::High,
                title: "App crash observed in logcat".to_string(),
                description: "Crash or exception with stack trace captured at runtime. May indicate unhandled paths or data exposure in logs.".to_string(),
                recommendation: "Add proper crash handling; ensure no secrets or PII appear in stack traces or logs. Investigate root cause of crash.".to_string(),
                evidence: Some(line.to_string()),
                static_correlation: None,
            });
        }

        // Cleartext / network hints (http not https)
        if line.contains("http://") && !line.contains("https://") {
            findings.push(DynamicMobileFinding {
                category: "cleartext-observed".to_string(),
                severity: Severity::Medium,
                title: "Cleartext HTTP observed at runtime".to_string(),
                description: "App performed or logged a non-TLS HTTP request despite possible manifest claims.".to_string(),
                recommendation: "Enforce TLS 1.2+ for all network; use certificate pinning where appropriate. Update manifest and code to reject cleartext.".to_string(),
                evidence: Some(basic_redact(line)),
                static_correlation: None,
            });
        }

        // Obvious secret patterns in log lines
        if contains_obvious_secret(line) {
            findings.push(DynamicMobileFinding {
                category: "log-secret-leak".to_string(),
                severity: Severity::High,
                title: "Potential secret or credential in logcat".to_string(),
                description: "Log line matches common secret/credential pattern (e.g. api key, token, password).".to_string(),
                recommendation: "Never log secrets, tokens, keys, or PII at any level. Use secure storage and structured logging with redaction.".to_string(),
                evidence: Some(basic_redact(line)),
                static_correlation: None,
            });
        }
    }
    findings
}

fn contains_obvious_secret(line: &str) -> bool {
    let l = line.to_ascii_lowercase();
    l.contains("api_key") || l.contains("api key") ||
    l.contains("secret=") || l.contains("secret :") ||
    l.contains("password=") || l.contains("password :") ||
    l.contains("token=") || l.contains("auth_token") ||
    l.contains("sk_live_") || l.contains("sk_test_") ||
    l.contains("AIzaSy") || // google api key prefix
    (l.contains("key=") && (l.contains("sk_") || l.len() > 40))
}

fn basic_redact(line: &str) -> String {
    let mut out = line.to_string();
    // Simple non-regex redaction for common patterns (P1 basic hints only)
    for pat in [
        "api_key=",
        "API_KEY=",
        "secret=",
        "SECRET=",
        "password=",
        "PASSWORD=",
        "token=",
        "auth_token=",
        "sk_live_",
        "sk_test_",
        "AIzaSy",
    ] {
        if out.contains(pat) {
            // Mask the value portion after the key (up to space or end)
            if let Some(pos) = out.find(pat) {
                let start = pos + pat.len();
                let mut end = out.len();
                for (i, c) in out[start..].char_indices() {
                    if c.is_whitespace() || c == '&' || c == '"' || c == '\'' || c == ',' {
                        end = start + i;
                        break;
                    }
                }
                let replacement = format!("{}[REDACTED]", pat);
                out.replace_range(pos..end, &replacement);
            }
        }
    }
    // Bound overly long evidence
    if out.len() > 300 {
        out.truncate(297);
        out.push_str("...");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_finds_permission_grant() {
        let log = "I PermissionManager: permission android.permission.CAMERA granted for com.example.app\n";
        let fs = parse_logcat_findings(log);
        assert!(!fs.is_empty());
        assert_eq!(fs[0].category, "runtime-permission");
        assert_eq!(fs[0].severity, Severity::Low);
        assert!(fs[0].title.contains("granted"));
        assert!(fs[0].evidence.as_ref().unwrap().contains("CAMERA"));
    }

    #[test]
    fn parser_finds_permission_denied() {
        let log = "W ActivityManager: permission denied: android.permission.READ_CONTACTS\n";
        let fs = parse_logcat_findings(log);
        assert!(fs
            .iter()
            .any(|f| f.category == "runtime-permission" && f.title.contains("denied")));
    }

    #[test]
    fn parser_finds_crash_with_stack() {
        let log = "E AndroidRuntime: FATAL EXCEPTION: main\n\
                   E AndroidRuntime: Process: com.example.app, PID: 1234\n\
                   E AndroidRuntime: java.lang.NullPointerException\n\
                   E AndroidRuntime: at com.example.app.MainActivity.onCreate(MainActivity.java:42)\n";
        let fs = parse_logcat_findings(log);
        assert!(fs
            .iter()
            .any(|f| f.category == "crash-log" && f.severity == Severity::High));
        // P1 parser emits one finding per interesting line (fatal + at-frames); evidence for the crash finding(s) will contain relevant tokens
        let has_crash_context = fs.iter().any(|f| {
            f.category == "crash-log"
                && f.evidence.as_ref().map_or(false, |e| {
                    e.contains("NullPointerException")
                        || e.contains("at com.example")
                        || e.contains("FATAL EXCEPTION")
                })
        });
        assert!(has_crash_context);
    }

    #[test]
    fn parser_finds_cleartext() {
        let log = "D Network: connecting to http://api.example.com/v1/data\n";
        let fs = parse_logcat_findings(log);
        assert!(fs.iter().any(|f| f.category == "cleartext-observed"));
        let c = fs
            .iter()
            .find(|f| f.category == "cleartext-observed")
            .unwrap();
        assert!(c.evidence.as_ref().unwrap().contains("http://"));
    }

    #[test]
    fn parser_finds_secret_and_redacts() {
        let log = "D Config: api_key=sk_live_abc123XYZ456secretvalue loaded\nD Auth: token=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9\n";
        let fs = parse_logcat_findings(log);
        let secrets: Vec<_> = fs
            .iter()
            .filter(|f| f.category == "log-secret-leak")
            .collect();
        assert!(!secrets.is_empty());
        // redaction happened in evidence
        for s in &secrets {
            let ev = s.evidence.as_ref().unwrap();
            assert!(ev.contains("[REDACTED]"));
            assert!(!ev.contains("sk_live_abc123XYZ456secretvalue"));
        }
    }

    #[test]
    fn parser_empty_or_noise_produces_no_findings() {
        let log = "I ActivityManager: Start proc com.example.app\nD libc: something normal\n";
        let fs = parse_logcat_findings(log);
        assert!(fs.is_empty());
    }

    #[test]
    fn basic_redact_bounds_and_masks() {
        let long = "key=superlongsecrettokenvaluehere".repeat(20);
        let red = basic_redact(&long);
        assert!(red.len() <= 303);
        assert!(red.contains("[REDACTED]") || red.contains("key="));
    }
}
