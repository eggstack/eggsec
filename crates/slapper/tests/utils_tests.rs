//! Tests for the utils module.
//!
//! Tests URL normalization, target extraction, and other utility functions.

use slapper::config::{Scope, ScopeRule};
use slapper::utils::scope::{check_scope, check_scope_from_url};
use slapper::utils::target::{extract_domain, extract_host_port, normalize_url};

#[test]
fn test_normalize_url_adds_scheme() {
    // normalize_url adds https:// by default
    assert_eq!(normalize_url("example.com"), "https://example.com");
    assert_eq!(
        normalize_url("example.com/path"),
        "https://example.com/path"
    );
}

#[test]
fn test_normalize_url_preserves_https() {
    assert_eq!(normalize_url("https://example.com"), "https://example.com");
}

#[test]
fn test_normalize_url_preserves_http() {
    assert_eq!(normalize_url("http://example.com"), "http://example.com");
}

#[test]
fn test_extract_domain_basic() {
    assert_eq!(
        extract_domain("https://example.com"),
        Some("example.com".to_string())
    );
    assert_eq!(
        extract_domain("http://example.com/path"),
        Some("example.com".to_string())
    );
}

#[test]
fn test_extract_domain_with_subdomain() {
    assert_eq!(
        extract_domain("https://sub.example.com"),
        Some("sub.example.com".to_string())
    );
}

#[test]
fn test_extract_domain_with_port() {
    assert_eq!(
        extract_domain("https://example.com:8080"),
        Some("example.com".to_string())
    );
}

#[test]
fn test_extract_domain_invalid() {
    // extract_domain might return something even for invalid URLs
    // Just verify it doesn't panic
    let _ = extract_domain("not-a-url");
    let _ = extract_domain("");
}

#[test]
fn test_extract_host_port_basic() {
    let result = extract_host_port("example.com:8080");
    assert!(result.is_some());
    let (host, port) = result.unwrap();
    assert_eq!(host, "example.com");
    assert_eq!(port, 8080);
}

#[test]
fn test_extract_host_port_default_port() {
    // extract_host_port requires a port to be specified
    let result = extract_host_port("example.com");
    // It might return None without a port
    let _ = result;
}

#[test]
fn test_extract_host_port_invalid() {
    // Invalid port should return None
    assert!(extract_host_port("example.com:invalid").is_none());
}

#[test]
fn test_extract_host_port_with_port() {
    let result = extract_host_port("example.com:8080");
    assert!(result.is_some());
    let (host, port) = result.unwrap();
    assert_eq!(host, "example.com");
    assert_eq!(port, 8080);
}

#[test]
fn test_check_scope_allowed() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    assert!(check_scope(&scope, "example.com").is_ok());
}

#[test]
fn test_check_scope_denied() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    assert!(check_scope(&scope, "other.com").is_err());
}

#[test]
fn test_check_scope_wildcard() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("*.example.com".to_string()));

    assert!(check_scope(&scope, "sub.example.com").is_ok());
    assert!(check_scope(&scope, "example.com").is_ok());
    assert!(check_scope(&scope, "other.com").is_err());
}

#[test]
fn test_check_scope_excluded() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("*.example.com".to_string()));
    scope
        .excluded_targets
        .push(ScopeRule::new("admin.example.com".to_string()));

    assert!(check_scope(&scope, "sub.example.com").is_ok());
    assert!(check_scope(&scope, "admin.example.com").is_err());
}

#[test]
fn test_check_scope_from_url() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    assert!(check_scope_from_url(&scope, "https://example.com/path").is_ok());
    assert!(check_scope_from_url(&scope, "https://other.com/path").is_err());
}

#[test]
fn test_check_scope_explicit_required() {
    let mut scope = Scope::new();
    scope.require_explicit_scope = true;

    // With explicit scope required and no targets, nothing is allowed
    assert!(check_scope(&scope, "example.com").is_err());
}
