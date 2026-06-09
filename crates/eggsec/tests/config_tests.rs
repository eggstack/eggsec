use eggsec::config::{Scope, ScopeRule, EggsecConfig, TargetScope};

#[test]
fn test_config_default() {
    let config = EggsecConfig::default();
    assert_eq!(config.http.timeout_secs, 30);
    assert!(config.http.verify_tls);
}

#[test]
fn test_config_http_defaults() {
    let config = EggsecConfig::default();
    assert_eq!(config.http.timeout_secs, 30);
    assert_eq!(config.http.max_retries, 3);
    assert_eq!(config.http.retry_delay_ms, 1000);
    assert!(config.http.verify_tls);
    assert!(config.http.follow_redirects);
    assert_eq!(config.http.max_redirects, 10);
}

#[test]
fn test_config_scan_defaults() {
    let config = EggsecConfig::default();
    assert_eq!(config.scan.default_concurrency, 10);
    assert!(!config.scan.stealth_mode);
    assert!(!config.scan.save_session);
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

#[test]
fn test_scope_multiple_allowed_targets() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("example.com".to_string()));
    scope
        .allowed_targets
        .push(ScopeRule::new("test.com".to_string()));

    assert!(scope.is_target_allowed("example.com").unwrap());
    assert!(scope.is_target_allowed("test.com").unwrap());
    assert!(!scope.is_target_allowed("other.com").unwrap());
}

#[test]
fn test_scope_excluded_overrides_allowed() {
    let mut scope = Scope::new();
    scope
        .allowed_targets
        .push(ScopeRule::new("*.example.com".to_string()));
    scope
        .excluded_targets
        .push(ScopeRule::new("admin.example.com".to_string()));

    assert!(scope.is_target_allowed("api.example.com").unwrap());
    assert!(!scope.is_target_allowed("admin.example.com").unwrap());
}

#[test]
fn test_config_output_defaults() {
    let config = EggsecConfig::default();
    assert!(!config.output.save_results);
    assert!(config.output.color);
    assert!(config.output.progress_bars);
    assert!(config.output.include_timestamp);
}

#[test]
fn test_config_notification_defaults() {
    let config = EggsecConfig::default();
    assert!(!config.notifications.notify_on_complete);
    assert!(!config.notifications.notify_on_findings);
}

#[test]
fn test_scope_new_is_empty() {
    let scope = Scope::new();
    assert!(scope.allowed_targets.is_empty());
    assert!(scope.excluded_targets.is_empty());
    assert!(scope.allowed_ports.is_none());
    assert!(scope.excluded_ports.is_empty());
}

#[test]
fn test_scope_default_allows_all() {
    let scope = Scope::default();
    // Default scope should allow all targets
    assert!(scope.is_target_allowed("example.com").unwrap());
    assert!(scope.is_target_allowed("anything.com").unwrap());
}

fn parse_target(host: &str) -> eggsec::config::TargetScope {
    eggsec::config::TargetScope::parse(host).unwrap()
}
