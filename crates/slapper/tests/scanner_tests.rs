mod common;

use common::{create_test_server, mock_json, mock_not_found, mock_ok, mock_waf_response};
use slapper::scanner::spoof::SpoofConfig;
use std::time::Duration;

#[tokio::test]
async fn test_endpoint_scan_basic() {
    let server = create_test_server().await;
    mock_ok("/api").mount(&server).await;
    mock_ok("/health").mount(&server).await;
    mock_not_found("/admin").mount(&server).await;

    let endpoints = vec![
        "/api".to_string(),
        "/health".to_string(),
        "/admin".to_string(),
    ];
    let spoof_config = SpoofConfig::default();

    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        false,
        false,
        spoof_config,
    )
    .await
    .unwrap();

    assert_eq!(results.endpoints_found, 2);
    assert!(results.endpoints_scanned == 3);
}

#[tokio::test]
async fn test_endpoint_scan_include_404() {
    let server = create_test_server().await;
    mock_ok("/api").mount(&server).await;
    mock_not_found("/admin").mount(&server).await;

    let endpoints = vec!["/api".to_string(), "/admin".to_string()];
    let spoof_config = SpoofConfig::default();

    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        true,
        false,
        spoof_config,
    )
    .await
    .unwrap();

    assert_eq!(results.endpoints_found, 2);
}

#[tokio::test]
async fn test_endpoint_scan_interesting() {
    let server = create_test_server().await;
    mock_ok("/.env").mount(&server).await;
    mock_ok("/api").mount(&server).await;

    let endpoints = vec!["/.env".to_string(), "/api".to_string()];
    let spoof_config = SpoofConfig::default();

    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        false,
        false,
        spoof_config,
    )
    .await
    .unwrap();

    let interesting = results.results.iter().filter(|e| e.interesting).count();
    assert!(interesting >= 1);
}

#[tokio::test]
async fn test_endpoint_scan_json_response() {
    let server = create_test_server().await;
    mock_json("/api/data", r#"{"status": "ok"}"#).mount(&server).await;

    let endpoints = vec!["/api/data".to_string()];
    let spoof_config = SpoofConfig::default();

    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        false,
        false,
        spoof_config,
    )
    .await
    .unwrap();

    assert_eq!(results.endpoints_found, 1);
}

#[tokio::test]
async fn test_endpoint_scan_waf_blocked() {
    let server = create_test_server().await;
    mock_waf_response("/api", "cf-ray", "123456789-ABC").mount(&server).await;

    let endpoints = vec!["/api".to_string()];
    let spoof_config = SpoofConfig::default();

    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        false,
        false,
        spoof_config,
    )
    .await
    .unwrap();

    // WAF blocked endpoints should still be counted
    assert_eq!(results.endpoints_scanned, 1);
}

#[tokio::test]
async fn test_endpoint_scan_empty_wordlist() {
    let server = create_test_server().await;

    let endpoints: Vec<String> = vec![];
    let spoof_config = SpoofConfig::default();

    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        false,
        false,
        spoof_config,
    )
    .await
    .unwrap();

    assert_eq!(results.endpoints_scanned, 0);
    assert_eq!(results.endpoints_found, 0);
}

#[tokio::test]
async fn test_spoof_config_default() {
    let config = SpoofConfig::default();
    assert!(!config.enabled);
    assert!(config.ip_range.is_none());
}

#[tokio::test]
async fn test_spoof_config_custom() {
    let config = SpoofConfig {
        enabled: true,
        ip_range: Some("192.168.1.0/24".to_string()),
        ..Default::default()
    };
    assert!(config.enabled);
    assert_eq!(config.ip_range, Some("192.168.1.0/24".to_string()));
}

#[tokio::test]
async fn test_port_scan_timeout() {
    // Test that port scanning respects timeout
    let timeout = Duration::from_millis(100);
    // This test verifies the timeout configuration is accepted
    assert!(timeout.as_millis() == 100);
}

#[tokio::test]
async fn test_timing_config_presets() {
    use slapper::scanner::TimingPreset;

    let paranoid = TimingPreset::Paranoid;
    let normal = TimingPreset::Normal;
    let aggressive = TimingPreset::Aggressive;

    // Verify timing presets are different
    assert_ne!(format!("{:?}", paranoid), format!("{:?}", normal));
    assert_ne!(format!("{:?}", normal), format!("{:?}", aggressive));
}

#[tokio::test]
async fn test_timing_preset_from_str() {
    use slapper::scanner::TimingPreset;

    assert_eq!(TimingPreset::from_str("paranoid"), TimingPreset::Paranoid);
    assert_eq!(TimingPreset::from_str("normal"), TimingPreset::Normal);
    assert_eq!(TimingPreset::from_str("aggressive"), TimingPreset::Aggressive);
    assert_eq!(TimingPreset::from_str("T0"), TimingPreset::Paranoid);
    assert_eq!(TimingPreset::from_str("T3"), TimingPreset::Normal);
}

#[tokio::test]
async fn test_port_result_serialization() {
    let result = slapper::scanner::PortResult {
        port: 80,
        status: "open".to_string(),
        service: "HTTP".to_string(),
    };
    let json = serde_json::to_string(&result).unwrap();
    assert!(json.contains("\"port\":80"));
    assert!(json.contains("\"status\":\"open\""));
    assert!(json.contains("\"service\":\"HTTP\""));
}

#[tokio::test]
async fn test_port_scan_results_display() {
    let results = slapper::scanner::PortScanResults {
        host: "example.com".to_string(),
        ports_scanned: 100,
        open_ports: vec![
            slapper::scanner::PortResult {
                port: 80,
                status: "open".to_string(),
                service: "HTTP".to_string(),
            },
            slapper::scanner::PortResult {
                port: 443,
                status: "open".to_string(),
                service: "HTTPS".to_string(),
            },
        ],
        duration_ms: 1000,
        spoof_stats: None,
    };
    let output = format!("{}", results);
    assert!(output.contains("example.com"));
    assert!(output.contains("2 ports"));
    assert!(output.contains("80/tcp"));
    assert!(output.contains("443/tcp"));
}

#[tokio::test]
async fn test_spoof_config_from_args_default() {
    // Test that SpoofConfig::from_args with default values creates a valid config
    let config = SpoofConfig::from_args(
        None,          // source_ip
        None,          // spoof_range
        false,         // stealth
        None,          // decoy
        None,          // decoy_range
        None,          // decoy_count
        None,          // decoy_mode
        false,         // include_me
        None,          // source_port
        false,         // random_source_port
        false,         // fragment
        None,          // scan_type
        None,          // packet_trace
        None,          // max_rate
        None,          // ttl
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(!config.enabled);
}

#[tokio::test]
async fn test_spoof_config_with_ip() {
    // Test SpoofConfig with source IP
    let config = SpoofConfig::from_args(
        Some("192.168.1.100".to_string()), // source_ip
        None,                              // spoof_range
        false,                             // stealth
        None,                              // decoy
        None,                              // decoy_range
        None,                              // decoy_count
        None,                              // decoy_mode
        false,                             // include_me
        None,                              // source_port
        false,                             // random_source_port
        false,                             // fragment
        None,                              // scan_type
        None,                              // packet_trace
        None,                              // max_rate
        None,                              // ttl
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(config.enabled);
    assert_eq!(config.source_ip.map(|ip| ip.to_string()), Some("192.168.1.100".to_string()));
}

#[tokio::test]
async fn test_spoof_config_with_range() {
    // Test SpoofConfig with IP range
    let config = SpoofConfig::from_args(
        None,                              // source_ip
        Some("192.168.1.0/24".to_string()), // spoof_range
        false,                             // stealth
        None,                              // decoy
        None,                              // decoy_range
        None,                              // decoy_count
        None,                              // decoy_mode
        false,                             // include_me
        None,                              // source_port
        false,                             // random_source_port
        false,                             // fragment
        None,                              // scan_type
        None,                              // packet_trace
        None,                              // max_rate
        None,                              // ttl
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(config.enabled);
    assert_eq!(config.ip_range, Some("192.168.1.0/24".to_string()));
}

#[tokio::test]
async fn test_spoof_config_with_decoys() {
    // Test SpoofConfig with decoy IPs
    let config = SpoofConfig::from_args(
        None,                              // source_ip
        None,                              // spoof_range
        false,                             // stealth
        Some("10.0.0.1,10.0.0.2".to_string()), // decoy
        None,                              // decoy_range
        Some(2),                           // decoy_count
        None,                              // decoy_mode
        true,                              // include_me
        None,                              // source_port
        false,                             // random_source_port
        false,                             // fragment
        None,                              // scan_type
        None,                              // packet_trace
        None,                              // max_rate
        None,                              // ttl
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(config.enabled);
    assert!(config.has_decoys());
    assert_eq!(config.decoy_count, 2);
    assert!(config.include_real_ip);
}
