//! End-to-end pipeline tests.
//!
//! Tests the complete scan pipeline from configuration through execution to output.

use eggsec::config::{EggsecConfig, FuzzProfile, ScanConfig};
use eggsec::utils::parsing::parse_ports;

#[test]
fn test_parse_ports_pipeline() {
    let ports = parse_ports("80,443,8080").unwrap();
    assert_eq!(ports.len(), 3);
    assert!(ports.contains(&80));
    assert!(ports.contains(&443));
    assert!(ports.contains(&8080));
}

#[test]
fn test_parse_ports_range_pipeline() {
    let ports = parse_ports("8000-8010").unwrap();
    assert_eq!(ports.len(), 11);
    assert_eq!(ports[0], 8000);
    assert_eq!(ports[10], 8010);
}

#[test]
fn test_scan_config_defaults() {
    let config = ScanConfig::default();
    assert_eq!(config.default_concurrency, 10);
    assert_eq!(config.port_timeout_secs, 2);
}

#[test]
fn test_fuzz_profile_defaults() {
    let profile = FuzzProfile::default();
    assert!(profile.payload_types.is_empty());
    assert_eq!(profile.concurrency, None);
    assert_eq!(profile.timeout_ms, None);
}

#[test]
fn test_fuzz_profile_custom() {
    let profile = FuzzProfile {
        payload_types: vec!["sqli".to_string(), "xss".to_string()],
        concurrency: Some(20),
        timeout_ms: Some(5000),
    };
    assert_eq!(profile.payload_types.len(), 2);
    assert_eq!(profile.concurrency, Some(20));
    assert_eq!(profile.timeout_ms, Some(5000));
}

#[test]
fn test_eggsec_config_structure() {
    let config = EggsecConfig::default();
    assert!(config.scan.default_concurrency > 0);
    assert!(config.http.timeout_secs > 0);
}

#[test]
fn test_port_parsing_edge_cases() {
    assert!(parse_ports("0").is_ok());
    assert!(parse_ports("65535").is_ok());
    assert!(parse_ports("0-65535").is_ok());

    assert!(parse_ports("-1").is_err());
    assert!(parse_ports("65536").is_err());
}

#[test]
fn test_scope_rule_basic() {
    use eggsec::config::ScopeRule;

    let rule = ScopeRule::new("example.com".to_string());
    assert!(rule.matches(&eggsec::config::TargetScope {
        host: "example.com".to_string(),
        ip: None,
    }));
}

#[test]
fn test_scope_rule_wildcard() {
    use eggsec::config::ScopeRule;

    let rule = ScopeRule::new("*.example.com".to_string());

    assert!(rule.matches(&eggsec::config::TargetScope {
        host: "sub.example.com".to_string(),
        ip: None,
    }));

    assert!(rule.matches(&eggsec::config::TargetScope {
        host: "example.com".to_string(),
        ip: None,
    }));

    assert!(!rule.matches(&eggsec::config::TargetScope {
        host: "other.com".to_string(),
        ip: None,
    }));
}
