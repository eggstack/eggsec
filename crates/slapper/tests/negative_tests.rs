//! Negative test cases for core modules.
//!
//! Tests error handling and edge cases to ensure the codebase handles
//! invalid inputs gracefully.

use slapper::config::{Scope, ScopeRule};
use slapper::utils::parsing::{parse_ports, resolve_host};
use slapper::utils::target::{extract_domain, normalize_url};

#[test]
fn test_parse_ports_invalid_range() {
    // Invalid range format - "abc-def" cannot be parsed as numbers
    let result = parse_ports("abc-def");
    assert!(result.is_err(), "Should fail for non-numeric range");
    let err = result.unwrap_err();
    let err_msg = err.to_string();
    assert!(
        err_msg.contains("invalid digit")
            || err_msg.contains("parse")
            || err_msg.contains("Invalid"),
        "Error should indicate parsing failure: {}",
        err
    );
}

#[test]
fn test_parse_ports_empty_string() {
    // Empty string fails because "".parse::<u16>() fails
    let result = parse_ports("");
    assert!(result.is_err(), "Should fail for empty string");
}

#[test]
fn test_parse_ports_invalid_characters() {
    // Mixed valid and invalid ports - should fail on "abc"
    let result = parse_ports("80,abc,443");
    assert!(result.is_err(), "Should fail when any port is invalid");
}

#[test]
fn test_parse_ports_overflow() {
    // Port 99999 exceeds u16 max (65535)
    let result = parse_ports("99999");
    assert!(result.is_err(), "Should fail for port > 65535");
}

#[test]
fn test_parse_ports_negative() {
    // Negative ports should fail - "-1" cannot parse as u16
    let result = parse_ports("-1");
    assert!(result.is_err(), "Should fail for negative port");
}

#[test]
fn test_parse_ports_reversed_range() {
    // Reversed range (start > end) is explicitly rejected
    let result = parse_ports("1000-1");
    assert!(result.is_err(), "Should fail when range start > end");
    let err = result.unwrap_err();
    assert!(
        err.to_string().contains("start > end"),
        "Error should mention start > end: {}",
        err
    );
}

#[test]
fn test_parse_ports_whitespace() {
    // Leading/trailing whitespace should be trimmed
    let result = parse_ports(" 80 , 443 ");
    assert!(result.is_ok(), "Should handle whitespace");
    let ports = result.unwrap();
    assert_eq!(ports, vec![80, 443]);
}

#[test]
fn test_parse_ports_large_range() {
    // Very large range - should either work or fail gracefully
    let result = parse_ports("1-65535");
    assert!(result.is_ok(), "Large port range should parse successfully");
    if let Ok(ports) = result {
        assert_eq!(ports.len(), 65535, "Should contain all 65535 ports");
    }
}

#[test]
fn test_parse_ports_duplicate() {
    // Duplicate ports - parse_ports doesn't deduplicate
    let result = parse_ports("80,80,443,443");
    assert!(result.is_ok(), "Should accept duplicates");
    let ports = result.unwrap();
    assert_eq!(ports.len(), 4, "Should keep all 4 entries");
    assert!(ports.contains(&80));
    assert!(ports.contains(&443));
}

#[test]
fn test_port_zero() {
    // Port 0 is valid for parsing (though usually reserved)
    let result = parse_ports("0");
    assert!(result.is_ok(), "Should accept port 0");
    assert_eq!(result.unwrap(), vec![0]);
}

#[test]
fn test_port_max() {
    // Port 65535 is the maximum valid port
    let result = parse_ports("65535");
    assert!(result.is_ok(), "Should accept port 65535");
    assert_eq!(result.unwrap(), vec![65535]);
}

#[test]
fn test_resolve_host_invalid() {
    // Invalid hostname that cannot be resolved
    let result = resolve_host("this-host-does-not-exist-12345.invalid");
    assert!(result.is_err(), "Should fail for unresolvable host");
}

#[test]
fn test_resolve_host_empty() {
    // Empty hostname cannot be resolved
    let result = resolve_host("");
    assert!(result.is_err(), "Should fail for empty hostname");
}

#[test]
fn test_extract_domain_empty_string() {
    // Empty string returns empty string (no validation)
    let result = extract_domain("");
    assert_eq!(result, Some(String::new()));
}

#[test]
fn test_extract_domain_no_scheme() {
    // Without scheme, still extracts domain
    let result = extract_domain("example.com");
    assert_eq!(result, Some("example.com".to_string()));
}

#[test]
fn test_extract_domain_with_path() {
    // URL with path - extracts just domain
    let result = extract_domain("https://example.com/path/to/page");
    assert_eq!(result, Some("example.com".to_string()));
}

#[test]
fn test_extract_domain_with_port() {
    // URL with port - extracts domain without port
    let result = extract_domain("http://example.com:8080/path");
    assert_eq!(result, Some("example.com".to_string()));
}

#[test]
fn test_extract_domain_www_prefix() {
    // www prefix is stripped
    let result = extract_domain("https://www.example.com");
    assert_eq!(result, Some("example.com".to_string()));
}

#[test]
fn test_normalize_url_preserves_scheme() {
    // URL with scheme should be preserved
    let result = normalize_url("http://example.com");
    assert_eq!(result, "http://example.com");

    let result = normalize_url("https://example.com");
    assert_eq!(result, "https://example.com");
}

#[test]
fn test_normalize_url_adds_https() {
    // URL without scheme gets https:// prepended
    let result = normalize_url("example.com");
    assert_eq!(result, "https://example.com");
}

#[test]
fn test_scope_rule_empty_pattern() {
    let rule = ScopeRule::new("".to_string());
    assert!(rule.pattern.is_empty(), "Pattern should be empty");
}

#[test]
fn test_scope_empty_target() {
    let scope = Scope::default();
    let result = scope.is_target_allowed("");
    assert!(result.is_err());
}

#[test]
fn test_scope_invalid_target() {
    let scope = Scope::default();
    let result = scope.is_target_allowed("not a valid target");
    assert!(result.is_err());
}

#[test]
fn test_scope_cidr_edge_cases() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("10.0.0.0/8".to_string()));

    // Valid IP in range
    let result = scope.is_target_allowed("10.255.255.255");
    assert!(result.is_ok());

    // IP outside range
    let result = scope.is_target_allowed("11.0.0.1");
    assert!(result.is_err());
}
