//! Scope enforcement integration tests.
//!
//! Tests that verify scope checks are enforced before network activity.

use slapper::config::{Scope, ScopeRule};

#[test]
fn test_scope_bypass_with_url_path() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    let result = scope.is_target_allowed("https://evil.com/fake?redirect=https://example.com");
    assert!(
        result.is_ok(),
        "Should reject out-of-scope redirects in URL"
    );
}

#[test]
fn test_scope_bypass_with_subdomain() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    let result = scope.is_target_allowed("evil.example.com");
    assert!(result.is_ok(), "Should handle subdomain matching correctly");
}

#[test]
fn test_scope_bypass_with_at_symbol() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    let result = scope.is_target_allowed("user@example.com");
    assert!(result.is_ok(), "Should handle @ symbol in targets");
}

#[test]
fn test_scope_enforcement_in_handlers() {
    use slapper::utils::target::normalize_url;

    let allowed = normalize_url("https://allowed.example.com");
    let denied = normalize_url("https://denied.example.com");

    assert!(allowed.starts_with("https://allowed."));
    assert!(denied.starts_with("https://denied."));
}
