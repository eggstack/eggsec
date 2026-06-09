#![cfg(feature = "stress-testing")]

use eggsec::stress::authorization::{create_example_stress_config, StressScope};

#[test]
fn test_stress_scope_defaults() {
    let scope = StressScope::default();
    assert!(!scope.allow_stress_test);
    assert_eq!(scope.max_rate_pps, Some(100000));
    assert_eq!(scope.max_duration_secs, Some(300));
    assert!(scope.allowed_stress_types.is_none());
    assert!(scope.require_confirmation);
    assert!(scope.warning_message.is_none());
}

#[test]
fn test_stress_scope_serde_roundtrip() {
    let scope = StressScope {
        allow_stress_test: true,
        max_rate_pps: Some(50000),
        max_duration_secs: Some(60),
        allowed_stress_types: Some(vec!["http".to_string(), "syn".to_string()]),
        require_confirmation: false,
        warning_message: Some("Warning!".to_string()),
    };

    let toml_str = toml::to_string_pretty(&scope).unwrap();
    let parsed: StressScope = toml::from_str(&toml_str).unwrap();

    assert!(parsed.allow_stress_test);
    assert_eq!(parsed.max_rate_pps, Some(50000));
    assert_eq!(parsed.max_duration_secs, Some(60));
    assert_eq!(parsed.allowed_stress_types.as_ref().unwrap().len(), 2);
    assert!(!parsed.require_confirmation);
    assert_eq!(parsed.warning_message.as_deref(), Some("Warning!"));
}

#[test]
fn test_stress_scope_from_toml() {
    let toml_str = r#"
        allow_stress_test = true
        max_rate_pps = 25000
        max_duration_secs = 120
        allowed_stress_types = ["http", "udp"]
        require_confirmation = true
        warning_message = "Authorized testing only"
    "#;

    let scope: StressScope = toml::from_str(toml_str).unwrap();
    assert!(scope.allow_stress_test);
    assert_eq!(scope.max_rate_pps, Some(25000));
    assert_eq!(scope.max_duration_secs, Some(120));
    let types = scope.allowed_stress_types.unwrap();
    assert!(types.contains(&"http".to_string()));
    assert!(types.contains(&"udp".to_string()));
}

#[test]
fn test_stress_scope_optional_fields_missing() {
    let toml_str = r#"
        allow_stress_test = true
    "#;

    let scope: StressScope = toml::from_str(toml_str).unwrap();
    assert!(scope.allow_stress_test);
    assert!(scope.max_rate_pps.is_none());
    assert!(scope.max_duration_secs.is_none());
    assert!(scope.allowed_stress_types.is_none());
}

#[test]
fn test_create_example_stress_config() {
    let config = create_example_stress_config();
    assert!(config.contains("allow_stress_test = true"));
    assert!(config.contains("max_rate_pps = 50000"));
    assert!(config.contains("max_duration_secs = 300"));
    assert!(config.contains("syn"));
    assert!(config.contains("udp"));
    assert!(config.contains("http"));
}

#[test]
fn test_stress_scope_empty_types_list() {
    let toml_str = r#"
        allow_stress_test = true
        allowed_stress_types = []
    "#;

    let scope: StressScope = toml::from_str(toml_str).unwrap();
    assert!(scope.allowed_stress_types.is_some());
    assert!(scope.allowed_stress_types.unwrap().is_empty());
}
