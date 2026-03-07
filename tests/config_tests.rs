use slapper::config::{Scope, ScopeRule, SlapperConfig};

#[test]
fn test_config_default() {
    let config = SlapperConfig::default();
    assert_eq!(config.http.timeout_secs, 30);
    assert!(config.http.verify_tls);
}

#[test]
fn test_scope_rule_wildcard() {
    let rule = ScopeRule::new("*.example.com".to_string());

    assert!(rule.matches(&parse_target("sub.example.com")));
    assert!(rule.matches(&parse_target("example.com")));
    assert!(!rule.matches(&parse_target("other.com")));
}

#[test]
fn test_scope_rule_exact() {
    let rule = ScopeRule::new("example.com".to_string());

    assert!(rule.matches(&parse_target("example.com")));
    assert!(!rule.matches(&parse_target("sub.example.com")));
}

#[test]
fn test_scope_allow_deny() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));
    scope
        .excluded_targets
        .push(ScopeRule::new("admin.example.com".to_string()));

    assert!(scope.is_target_allowed("example.com").unwrap());
    assert!(!scope.is_target_allowed("admin.example.com").unwrap());
    assert!(!scope.is_target_allowed("other.com").unwrap());
}

#[test]
fn test_scope_require_explicit() {
    let mut scope = Scope::new();
    scope.require_explicit_scope = true;

    assert!(!scope.is_target_allowed("example.com").unwrap());
}

#[test]
fn test_scope_port_restrictions() {
    let mut scope = Scope::new();
    scope.allowed_ports = Some(vec![80, 443]);
    scope.excluded_ports = vec![22];

    assert!(scope.is_port_allowed(80));
    assert!(scope.is_port_allowed(443));
    assert!(!scope.is_port_allowed(8080));
    assert!(!scope.is_port_allowed(22));
}

fn parse_target(host: &str) -> slapper::config::TargetScope {
    slapper::config::TargetScope::parse(host).unwrap()
}
