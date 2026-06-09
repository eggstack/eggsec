//! Scope enforcement integration tests.
//!
//! Tests that verify scope checks are enforced before network activity.

use eggsec::config::{Scope, ScopeRule};

#[test]
fn test_scope_bypass_with_url_path() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    let result = scope.is_target_allowed("https://evil.com/fake?redirect=https://example.com");
    let allowed = result.expect("is_target_allowed should not error");
    assert!(!allowed, "Should reject out-of-scope redirects in URL");
}

#[test]
fn test_scope_bypass_with_subdomain() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    let result = scope.is_target_allowed("evil.example.com");
    let allowed = result.expect("is_target_allowed should not error");
    assert!(
        !allowed,
        "Should reject evil.example.com when only example.com is in scope (no wildcard)"
    );
}

#[test]
fn test_scope_bypass_with_at_symbol() {
    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));

    let result = scope.is_target_allowed("user@example.com");
    let allowed = result.expect("is_target_allowed should not error");
    assert!(
        !allowed,
        "Should reject user@example.com when only example.com is in scope"
    );
}

#[test]
fn test_scope_enforcement_via_api() {
    use eggsec::config::{Scope, ScopeRule};

    let mut scope = Scope::default();
    scope
        .allowed_targets
        .push(ScopeRule::new("allowed.example.com".to_string()));
    scope
        .excluded_targets
        .push(ScopeRule::new("evil.example.com".to_string()));

    assert!(
        scope.is_target_allowed("allowed.example.com").unwrap(),
        "allowed.example.com should be in scope"
    );
    assert!(
        !scope.is_target_allowed("evil.example.com").unwrap(),
        "evil.example.com should be excluded"
    );
    assert!(
        !scope.is_target_allowed("other.example.com").unwrap(),
        "other.example.com should be out of scope"
    );
}
