//! Tests for the proxy module.
//!
//! Tests proxy configuration, pool operations, and rotation strategies.

use slapper::proxy::ProxyHealth;
use slapper::proxy::{ProxyConfig, ProxyEntry, ProxyPool, ProxyType};
use std::str::FromStr;

#[test]
fn test_proxy_type_from_str() {
    assert_eq!(ProxyType::from_str("socks5").unwrap(), ProxyType::Socks5);
    assert_eq!(ProxyType::from_str("socks4").unwrap(), ProxyType::Socks4);
    assert_eq!(ProxyType::from_str("http").unwrap(), ProxyType::Http);
    assert_eq!(ProxyType::from_str("https").unwrap(), ProxyType::Https);
    assert_eq!(ProxyType::from_str("tor").unwrap(), ProxyType::Tor);
}

#[test]
fn test_proxy_type_case_insensitive() {
    assert_eq!(ProxyType::from_str("SOCKS5").unwrap(), ProxyType::Socks5);
    assert_eq!(ProxyType::from_str("HTTP").unwrap(), ProxyType::Http);
}

#[test]
fn test_proxy_type_invalid() {
    assert!(ProxyType::from_str("invalid").is_err());
}

#[test]
fn test_proxy_entry_new() {
    let entry = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080);
    assert_eq!(entry.proxy_type, ProxyType::Socks5);
    assert_eq!(entry.address, "127.0.0.1");
    assert_eq!(entry.port, 1080);
    assert!(entry.enabled);
    assert_eq!(entry.priority, 0);
}

#[test]
fn test_proxy_entry_with_weight() {
    let entry =
        ProxyEntry::new(ProxyType::Http, "proxy.example.com".to_string(), 8080).with_weight(5);
    assert_eq!(entry.weight, 5);
}

#[test]
fn test_proxy_entry_with_auth() {
    let entry = ProxyEntry::new(ProxyType::Http, "proxy.example.com".to_string(), 8080)
        .with_auth("user".to_string(), "pass".to_string());
    assert_eq!(entry.username, Some("user".to_string()));
    assert_eq!(entry.password, Some("pass".to_string()));
}

#[test]
fn test_proxy_config_default() {
    let config = ProxyConfig::default();
    assert_eq!(config.health_check_interval_secs, 60);
    assert_eq!(config.health_check_timeout_ms, 5000);
    assert_eq!(config.max_failures_before_disable, 3);
}

#[test]
fn test_proxy_pool_add_and_get() {
    let config = ProxyConfig::default();
    let mut pool = ProxyPool::new(config);

    let entry = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080);
    pool.add(entry);

    assert_eq!(pool.size(), 1);
    assert_eq!(pool.get_all().len(), 1);
}

#[test]
fn test_proxy_pool_get_healthy() {
    let config = ProxyConfig::default();
    let mut pool = ProxyPool::new(config);

    let mut healthy = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080);
    healthy.enabled = true;

    let mut unhealthy = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1081);
    unhealthy.enabled = false;

    pool.add(healthy);
    pool.add(unhealthy);

    assert_eq!(pool.size(), 2);
    assert_eq!(pool.get_healthy().len(), 1);
}

#[test]
fn test_proxy_pool_size_empty() {
    let config = ProxyConfig::default();
    let pool = ProxyPool::new(config);
    assert_eq!(pool.size(), 0);
    assert!(pool.get_all().is_empty());
}

#[test]
fn test_proxy_type_display() {
    assert_eq!(format!("{}", ProxyType::Socks5), "socks5");
    assert_eq!(format!("{}", ProxyType::Socks4), "socks4");
    assert_eq!(format!("{}", ProxyType::Http), "http");
    assert_eq!(format!("{}", ProxyType::Https), "https");
    assert_eq!(format!("{}", ProxyType::Tor), "tor");
}

#[test]
fn test_proxy_type_default() {
    assert_eq!(ProxyType::default(), ProxyType::Socks5);
}

#[test]
fn test_proxy_health_percentage_all_healthy() {
    let health = ProxyHealth {
        total: 10,
        healthy: 10,
        unhealthy: 0,
        results: vec![],
    };
    assert_eq!(health.healthy_percentage(), 100.0);
}

#[test]
fn test_proxy_health_percentage_none_healthy() {
    let health = ProxyHealth {
        total: 10,
        healthy: 0,
        unhealthy: 10,
        results: vec![],
    };
    assert_eq!(health.healthy_percentage(), 0.0);
}

#[test]
fn test_proxy_health_percentage_partial() {
    let health = ProxyHealth {
        total: 4,
        healthy: 3,
        unhealthy: 1,
        results: vec![],
    };
    assert_eq!(health.healthy_percentage(), 75.0);
}

#[test]
fn test_proxy_health_percentage_empty() {
    let health = ProxyHealth {
        total: 0,
        healthy: 0,
        unhealthy: 0,
        results: vec![],
    };
    assert_eq!(health.healthy_percentage(), 0.0);
}

#[test]
fn test_proxy_entry_to_url_no_auth() {
    let proxy = ProxyEntry::new(ProxyType::Socks5, "127.0.0.1".to_string(), 1080);
    assert_eq!(proxy.to_url(), "socks5://127.0.0.1:1080");
}

#[test]
fn test_proxy_entry_to_url_with_auth() {
    let proxy = ProxyEntry::new(ProxyType::Http, "proxy.example.com".to_string(), 8080)
        .with_auth("user".to_string(), "pass".to_string());
    assert_eq!(proxy.to_url(), "http://user:pass@proxy.example.com:8080");
}

#[test]
fn test_proxy_entry_serde_json() {
    let proxy = ProxyEntry::new(ProxyType::Http, "proxy.example.com".to_string(), 8080);
    let json = serde_json::to_string(&proxy).unwrap();
    let parsed: ProxyEntry = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.proxy_type, ProxyType::Http);
    assert_eq!(parsed.address, "proxy.example.com");
    assert_eq!(parsed.port, 8080);
}
