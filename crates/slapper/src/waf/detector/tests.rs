use super::types::{ResponseDiff, WafDetectionResult};

#[test]
fn test_response_diff_is_waf_blocked_by_status() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 403,
        malicious_length: 4900,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec![],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_is_waf_blocked_by_406() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 406,
        malicious_length: 4900,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec![],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_is_waf_blocked_by_405() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 405,
        malicious_length: 4900,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec![],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_not_blocked_same_status() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 4900,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec![],
        body_diffs: None,
    };
    assert!(!diff.is_waf_blocked());
}

#[test]
fn test_response_diff_blocked_by_length() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 4800,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec![],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_not_blocked_small_length_diff() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 4950,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec![],
        body_diffs: None,
    };
    assert!(!diff.is_waf_blocked());
}

#[test]
fn test_response_diff_blocked_by_waf_header() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 5000,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["x-waf-blocked: true".to_string()],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_blocked_by_firewall_header() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 5000,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["X-Firewall-Action: deny".to_string()],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_blocked_by_blocked_header() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 5000,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["x-blocked-request: yes".to_string()],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_blocked_by_attack_header() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 5000,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["x-attack-detected: sql-injection".to_string()],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_response_diff_not_blocked_irrelevant_header() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 5000,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["x-request-id: abc123".to_string()],
        body_diffs: None,
    };
    assert!(!diff.is_waf_blocked());
}

#[test]
fn test_response_diff_header_case_insensitive() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 200,
        malicious_length: 5000,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["X-WAF-Status: Active".to_string()],
        body_diffs: None,
    };
    assert!(diff.is_waf_blocked());
}

#[test]
fn test_waf_detection_result_serialization() {
    let result = WafDetectionResult {
        waf_name: Some("Cloudflare".to_string()),
        confidence: 75,
        request_error: None,
        matched_headers: vec!["cf-ray: abc123".to_string()],
        matched_cookies: vec!["__cfduid".to_string()],
        matched_patterns: vec![],
        server_header: Some("cloudflare".to_string()),
        status_code: 403,
    };
    let json = serde_json::to_string(&result).unwrap();
    let deserialized: WafDetectionResult = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.waf_name, Some("Cloudflare".to_string()));
    assert_eq!(deserialized.confidence, 75);
    assert_eq!(deserialized.status_code, 403);
}

#[test]
fn test_response_diff_serialization() {
    let diff = ResponseDiff {
        normal_status: 200,
        normal_length: 5000,
        malicious_status: 403,
        malicious_length: 100,
        normal_headers: None,
        malicious_headers: None,
        header_diffs: vec!["x-waf: blocked".to_string()],
        body_diffs: Some(true),
    };
    let json = serde_json::to_string(&diff).unwrap();
    let deserialized: ResponseDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.normal_status, 200);
    assert_eq!(deserialized.malicious_status, 403);
    assert!(deserialized.body_diffs.is_some());
}
