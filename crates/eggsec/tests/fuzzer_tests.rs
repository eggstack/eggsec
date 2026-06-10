mod common;

use common::*;
use eggsec::fuzzer::{get_all_payloads_cached, get_payloads, PayloadType, Severity};

#[test]
fn test_fuzzer_mutations() {
    let payload = "test";
    let mutations = eggsec::fuzzer::generate_mutations(payload, 1);
    assert!(!mutations.is_empty());
}

#[test]
fn test_fuzzer_redos_detector() {
    let detector = eggsec::fuzzer::ReDosDetector::new();
    let result = detector.detect(r"(.+)+");
    assert!(result.is_vulnerable);
}

#[test]
fn test_fuzzer_redos_executor() {
    let executor = eggsec::fuzzer::RegexExecutor::new();
    let result = executor.check_pattern(r"a+");
    assert!(!result.is_vulnerable);
    assert!(result.is_match);
}

#[test]
fn test_payload_generation_sqli() {
    let payloads = get_payloads(PayloadType::Sqli);
    assert!(!payloads.is_empty(), "SQLi payloads should not be empty");
    assert!(
        payloads.len() >= 10,
        "Should have at least 10 SQLi payloads"
    );

    // Verify payload structure
    for payload in &payloads {
        assert!(
            !payload.payload.is_empty(),
            "Payload string should not be empty"
        );
        assert!(
            !payload.description.is_empty(),
            "Description should not be empty"
        );
    }

    // Verify critical payloads exist
    let critical_count = payloads
        .iter()
        .filter(|p| p.severity == Severity::Critical)
        .count();
    assert!(
        critical_count > 0,
        "Should have critical severity SQLi payloads"
    );
}

#[test]
fn test_payload_generation_xss() {
    let payloads = get_payloads(PayloadType::Xss);
    assert!(!payloads.is_empty(), "XSS payloads should not be empty");
    assert!(payloads.len() >= 10, "Should have at least 10 XSS payloads");

    // Verify script-based payloads exist
    let has_script = payloads.iter().any(|p| p.payload.contains("<script>"));
    assert!(has_script, "Should have script-based XSS payloads");
}

#[test]
fn test_payload_generation_traversal() {
    let payloads = get_payloads(PayloadType::Traversal);
    assert!(
        !payloads.is_empty(),
        "Traversal payloads should not be empty"
    );

    // Verify directory traversal payloads exist
    let has_traversal = payloads.iter().any(|p| p.payload.contains("../"));
    assert!(has_traversal, "Should have directory traversal payloads");

    // Verify etc/passwd payload exists
    let has_passwd = payloads.iter().any(|p| p.payload.contains("etc/passwd"));
    assert!(has_passwd, "Should have /etc/passwd payload");
}

#[test]
fn test_payload_generation_ssrf() {
    let payloads = get_payloads(PayloadType::Ssrf);
    assert!(!payloads.is_empty(), "SSRF payloads should not be empty");

    // Verify localhost/internal payloads exist
    let has_localhost = payloads
        .iter()
        .any(|p| p.payload.contains("127.0.0.1") || p.payload.contains("localhost"));
    assert!(has_localhost, "Should have localhost SSRF payloads");
}

#[test]
fn test_all_payload_types() {
    let all_payloads = get_all_payloads_cached();
    assert!(!all_payloads.is_empty(), "All payloads should not be empty");

    // Verify all payload types are represented
    let mut types_seen = std::collections::HashSet::new();
    for payload in all_payloads {
        types_seen.insert(payload.payload_type);
    }
    // There are 21 payload types (WebSocket is not included in get_all_payloads)
    assert!(
        types_seen.len() >= 20,
        "Should have at least 20 payload types"
    );
}

#[test]
fn test_payload_severity_ordering() {
    let payloads = get_payloads(PayloadType::Sqli);

    // Verify severity levels are valid
    for payload in &payloads {
        match payload.severity {
            Severity::Critical
            | Severity::High
            | Severity::Medium
            | Severity::Low
            | Severity::Info => {}
        }
    }

    // Critical payloads should exist for dangerous types
    let critical_sqli = payloads
        .iter()
        .filter(|p| p.severity == Severity::Critical)
        .count();
    assert!(
        critical_sqli > 0,
        "SQLi should have critical severity payloads"
    );
}

#[test]
fn test_payload_tags() {
    let payloads = get_payloads(PayloadType::Sqli);

    // Verify tags are populated
    let has_tags = payloads.iter().any(|p| !p.tags.is_empty());
    assert!(has_tags, "Some payloads should have tags");
}

#[test]
fn test_graphql_payloads() {
    let payloads = get_payloads(PayloadType::GraphQL);
    assert!(!payloads.is_empty(), "GraphQL payloads should not be empty");

    // Verify introspection-related payloads exist
    let has_introspection = payloads
        .iter()
        .any(|p| p.payload.contains("__schema") || p.payload.contains("__type"));
    assert!(has_introspection, "Should have introspection payloads");
}

#[test]
fn test_jwt_payloads() {
    let payloads = get_payloads(PayloadType::Jwt);
    assert!(!payloads.is_empty(), "JWT payloads should not be empty");

    // Verify JWT-related payloads exist
    let has_jwt = payloads.iter().any(|p| {
        p.payload.contains("eyJ") || p.payload.contains("alg") || p.payload.contains("none")
    });
    assert!(has_jwt, "Should have JWT-related payloads");
}

#[test]
fn test_ssti_payloads() {
    let payloads = get_payloads(PayloadType::Ssti);
    assert!(!payloads.is_empty(), "SSTI payloads should not be empty");

    // Verify template syntax payloads exist
    let has_template = payloads
        .iter()
        .any(|p| p.payload.contains("{{") || p.payload.contains("${"));
    assert!(has_template, "Should have template syntax payloads");
}

#[test]
fn test_cmd_injection_payloads() {
    let payloads = get_payloads(PayloadType::Cmd);
    assert!(
        !payloads.is_empty(),
        "Command injection payloads should not be empty"
    );

    // Verify OS command payloads exist
    let has_cmd = payloads
        .iter()
        .any(|p| p.payload.contains(";") || p.payload.contains("|") || p.payload.contains("`"));
    assert!(has_cmd, "Should have command injection payloads");
}

#[test]
fn test_ldap_payloads() {
    let payloads = get_payloads(PayloadType::Ldap);
    assert!(!payloads.is_empty(), "LDAP payloads should not be empty");

    // Verify LDAP filter payloads exist
    let has_filter = payloads
        .iter()
        .any(|p| p.payload.contains("*") || p.payload.contains("(|"));
    assert!(has_filter, "Should have LDAP filter payloads");
}

#[test]
fn test_xxe_payloads() {
    let payloads = get_payloads(PayloadType::Xxe);
    assert!(!payloads.is_empty(), "XXE payloads should not be empty");

    // Verify entity payloads exist
    let has_entity = payloads
        .iter()
        .any(|p| p.payload.contains("ENTITY") || p.payload.contains("DOCTYPE"));
    assert!(has_entity, "Should have XXE entity payloads");
}

#[test]
fn test_idor_payloads() {
    let payloads = get_payloads(PayloadType::Idor);
    assert!(!payloads.is_empty(), "IDOR payloads should not be empty");

    // Verify ID manipulation payloads exist
    let has_id = payloads.iter().any(|p| {
        p.payload.contains("../") || p.payload.contains("admin") || p.payload.contains("1")
    });
    assert!(has_id, "Should have IDOR manipulation payloads");
}

#[test]
fn test_redos_payloads() {
    let payloads = get_payloads(PayloadType::Redos);
    assert!(!payloads.is_empty(), "ReDoS payloads should not be empty");

    // Verify regex patterns exist
    let has_pattern = payloads
        .iter()
        .any(|p| p.payload.contains("(a+)+") || p.payload.contains(".*.*"));
    assert!(has_pattern, "Should have ReDoS pattern payloads");
}

#[test]
fn test_compression_payloads() {
    let payloads = get_payloads(PayloadType::Compression);
    assert!(
        !payloads.is_empty(),
        "Compression payloads should not be empty"
    );

    // Verify compression-related payloads exist
    let has_compression = payloads.iter().any(|p| {
        p.payload.contains("gzip") || p.payload.contains("deflate") || p.payload.len() > 50
    });
    assert!(has_compression, "Should have compression-related payloads");
}

#[test]
fn test_host_header_payloads() {
    let payloads = get_payloads(PayloadType::Host);
    assert!(
        !payloads.is_empty(),
        "Host header payloads should not be empty"
    );

    // Verify host manipulation payloads exist
    let has_host = payloads
        .iter()
        .any(|p| p.payload.contains("localhost") || p.payload.contains("127.0.0.1"));
    assert!(has_host, "Should have host header manipulation payloads");
}

#[test]
fn test_cache_poisoning_payloads() {
    let payloads = get_payloads(PayloadType::Cache);
    assert!(
        !payloads.is_empty(),
        "Cache poisoning payloads should not be empty"
    );

    // Verify cache-related payloads exist
    let has_cache = payloads
        .iter()
        .any(|p| p.payload.contains("X-Forwarded-Host") || p.payload.contains("X-Forwarded-Proto"));
    assert!(has_cache, "Should have cache poisoning payloads");
}

#[test]
fn test_csv_injection_payloads() {
    let payloads = get_payloads(PayloadType::Csv);
    assert!(
        !payloads.is_empty(),
        "CSV injection payloads should not be empty"
    );

    // Verify formula injection payloads exist
    let has_formula = payloads.iter().any(|p| {
        p.payload.starts_with('=') || p.payload.starts_with('+') || p.payload.starts_with('@')
    });
    assert!(has_formula, "Should have CSV formula injection payloads");
}

#[test]
fn test_soap_payloads() {
    let payloads = get_payloads(PayloadType::Soap);
    assert!(!payloads.is_empty(), "SOAP payloads should not be empty");

    // Verify XML payloads exist
    let has_xml = payloads
        .iter()
        .any(|p| p.payload.contains("<") && p.payload.contains(">"));
    assert!(has_xml, "Should have XML/SOAP payloads");
}

#[test]
fn test_redirect_payloads() {
    let payloads = get_payloads(PayloadType::Redirect);
    assert!(
        !payloads.is_empty(),
        "Redirect payloads should not be empty"
    );

    // Verify redirect payloads exist
    let has_redirect = payloads.iter().any(|p| {
        p.payload.contains("http://") || p.payload.contains("https://") || p.payload.contains("//")
    });
    assert!(has_redirect, "Should have redirect payloads");
}

#[test]
fn test_header_expansion_payloads() {
    let payloads = get_payloads(PayloadType::Headers);
    assert!(
        !payloads.is_empty(),
        "Header expansion payloads should not be empty"
    );

    // Verify header payloads exist
    let has_header = payloads
        .iter()
        .any(|p| p.payload.contains("X-") || p.payload.contains("Content-"));
    assert!(has_header, "Should have header expansion payloads");
}

#[test]
fn test_oauth_payloads() {
    let payloads = get_payloads(PayloadType::OAuth);
    assert!(!payloads.is_empty(), "OAuth payloads should not be empty");

    // Verify OAuth-specific payloads exist
    let has_oauth = payloads.iter().any(|p| {
        p.payload.contains("redirect_uri")
            || p.payload.contains("scope")
            || p.payload.contains("state")
    });
    assert!(has_oauth, "Should have OAuth payloads");
}

#[test]
fn test_grpc_payloads() {
    let payloads = get_payloads(PayloadType::Grpc);
    assert!(!payloads.is_empty(), "gRPC payloads should not be empty");
}

#[test]
fn test_deserialization_payloads() {
    let payloads = get_payloads(PayloadType::Deser);
    assert!(
        !payloads.is_empty(),
        "Deserialization payloads should not be empty"
    );

    // Verify serialization payloads exist
    let has_ser = payloads.iter().any(|p| {
        p.payload.contains("rO0") || p.payload.contains("aced") || p.payload.contains("<?xml")
    });
    assert!(has_ser, "Should have deserialization payloads");
}

/// Central audit ensuring every PayloadType has substantial, real (non-empty)
/// payload coverage. Prevents accidental regression to stub-status modules.
#[test]
fn test_payload_audit_all_types_substantial() {
    const MIN_PER_TYPE: usize = 10;
    const MIN_TOTAL: usize = 1000;

    let mut total = 0usize;
    let mut counts: Vec<(PayloadType, usize)> = Vec::new();

    for pt in PayloadType::all_variants() {
        let payloads = get_payloads(*pt);
        let n = payloads.len();
        assert!(
            n >= MIN_PER_TYPE,
            "{:?} has only {} payloads (min {}); likely a stub or regression",
            pt,
            n,
            MIN_PER_TYPE
        );
        for p in &payloads {
            assert!(
                !p.payload.is_empty(),
                "{:?} payload empty: {:?}",
                pt,
                p.description
            );
            assert!(
                !p.description.is_empty(),
                "{:?} description empty for payload {:?}",
                pt,
                p.payload
            );
            assert!(
                !p.tags.is_empty(),
                "{:?} tags empty for payload {:?}",
                pt,
                p.payload
            );
        }
        total += n;
        counts.push((*pt, n));
    }

    assert!(
        total >= MIN_TOTAL,
        "Total payload count {} is below minimum {}",
        total,
        MIN_TOTAL
    );

    // Sanity: every type is represented in the cached view
    let all_cached = get_all_payloads_cached();
    assert_eq!(
        all_cached.len(),
        total,
        "Cached total {} differs from sum-of-types {}",
        all_cached.len(),
        total
    );
}

/// Ensures no payload looks like a placeholder (TODO/FIXME/lorem ipsum/etc.).
/// Real attack strings should never contain dev markers.
#[test]
fn test_payload_audit_no_placeholders() {
    const BANNED: &[&str] = &[
        "TODO",
        "FIXME",
        "XXX",
        "lorem ipsum",
        "PLACEHOLDER",
        "REPLACE_ME",
        "<insert ",
    ];

    for pt in PayloadType::all_variants() {
        for p in get_payloads(*pt) {
            for bad in BANNED {
                assert!(
                    !p.payload.contains(bad),
                    "{:?} payload contains banned placeholder {:?}: {:?}",
                    pt,
                    bad,
                    p.description
                );
            }
        }
    }
}

/// Severity distribution should have at least some Critical/High entries across
/// the high-risk types. Some probe-heavy classes (e.g. GraphQL introspection,
/// Oast placeholders) legitimately carry a mix of Info/Low/Medium probes, so
/// we require an absolute minimum of high-impact payloads rather than a
/// strict ratio.
#[test]
fn test_payload_audit_critical_severity_present() {
    const MIN_HIGH_IMPACT: usize = 3;

    for pt in PayloadType::all_variants() {
        let payloads = get_payloads(*pt);
        if payloads.is_empty() {
            continue;
        }
        let critical_or_high = payloads
            .iter()
            .filter(|p| matches!(p.severity, Severity::Critical | Severity::High))
            .count();
        assert!(
            critical_or_high >= MIN_HIGH_IMPACT,
            "{:?} has only {} Critical/High severity payloads (min {})",
            pt,
            critical_or_high,
            MIN_HIGH_IMPACT
        );
    }
}
