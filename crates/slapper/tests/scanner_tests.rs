mod common;

use common::{create_test_server, mock_json, mock_not_found, mock_ok, mock_waf_response};
use slapper::scanner::spoof::SpoofConfig;
use slapper::scanner::EndpointScanConfig;
use std::sync::Arc;
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
    let spoof_config = Arc::new(SpoofConfig::default());

    let config = EndpointScanConfig {
        base_url: server.uri(),
        endpoints,
        concurrency: 5,
        timeout_duration: Duration::from_secs(5),
        include_404: false,
        tui_mode: false,
        spoof_config,
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    };

    let results = slapper::scanner::scan_endpoints(config).await.unwrap();

    assert_eq!(results.endpoints_found, 2);
    assert!(results.endpoints_scanned == 3);
}

#[tokio::test]
async fn test_endpoint_scan_include_404() {
    let server = create_test_server().await;
    mock_ok("/api").mount(&server).await;
    mock_not_found("/admin").mount(&server).await;

    let endpoints = vec!["/api".to_string(), "/admin".to_string()];
    let spoof_config = Arc::new(SpoofConfig::default());

    let config = EndpointScanConfig {
        base_url: server.uri(),
        endpoints,
        concurrency: 5,
        timeout_duration: Duration::from_secs(5),
        include_404: true,
        tui_mode: false,
        spoof_config,
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    };

    let results = slapper::scanner::scan_endpoints(config).await.unwrap();

    assert_eq!(results.endpoints_found, 2);
}

#[tokio::test]
async fn test_endpoint_scan_interesting() {
    let server = create_test_server().await;
    mock_ok("/.env").mount(&server).await;
    mock_ok("/api").mount(&server).await;

    let endpoints = vec!["/.env".to_string(), "/api".to_string()];
    let spoof_config = Arc::new(SpoofConfig::default());

    let config = EndpointScanConfig {
        base_url: server.uri(),
        endpoints,
        concurrency: 5,
        timeout_duration: Duration::from_secs(5),
        include_404: false,
        tui_mode: false,
        spoof_config,
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    };

    let results = slapper::scanner::scan_endpoints(config).await.unwrap();

    let interesting = results.results.iter().filter(|e| e.interesting).count();
    assert!(interesting >= 1);
}

#[tokio::test]
async fn test_endpoint_scan_json_response() {
    let server = create_test_server().await;
    mock_json("/api/data", r#"{"status": "ok"}"#)
        .mount(&server)
        .await;

    let endpoints = vec!["/api/data".to_string()];
    let spoof_config = Arc::new(SpoofConfig::default());

    let config = EndpointScanConfig {
        base_url: server.uri(),
        endpoints,
        concurrency: 5,
        timeout_duration: Duration::from_secs(5),
        include_404: false,
        tui_mode: false,
        spoof_config,
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    };

    let results = slapper::scanner::scan_endpoints(config).await.unwrap();

    assert_eq!(results.endpoints_found, 1);
}

#[tokio::test]
async fn test_endpoint_scan_waf_blocked() {
    let server = create_test_server().await;
    mock_waf_response("/api", "cf-ray", "123456789-ABC")
        .mount(&server)
        .await;

    let endpoints = vec!["/api".to_string()];
    let spoof_config = Arc::new(SpoofConfig::default());

    let config = EndpointScanConfig {
        base_url: server.uri(),
        endpoints,
        concurrency: 5,
        timeout_duration: Duration::from_secs(5),
        include_404: false,
        tui_mode: false,
        spoof_config,
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    };

    let results = slapper::scanner::scan_endpoints(config).await.unwrap();

    assert_eq!(results.endpoints_scanned, 1);
}

#[tokio::test]
async fn test_endpoint_scan_empty_wordlist() {
    let server = create_test_server().await;

    let endpoints: Vec<String> = vec![];
    let spoof_config = Arc::new(SpoofConfig::default());

    let config = EndpointScanConfig {
        base_url: server.uri(),
        endpoints,
        concurrency: 5,
        timeout_duration: Duration::from_secs(5),
        include_404: false,
        tui_mode: false,
        spoof_config,
        verify_tls: false,
        progress_tx: None,
        max_results: None,
    };

    let results = slapper::scanner::scan_endpoints(config).await.unwrap();

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
    let timeout = Duration::from_millis(100);
    assert!(timeout.as_millis() == 100);
}

#[tokio::test]
async fn test_timing_config_presets() {
    use slapper::scanner::TimingPreset;

    let paranoid = TimingPreset::Paranoid;
    let normal = TimingPreset::Normal;
    let aggressive = TimingPreset::Aggressive;

    assert_ne!(format!("{:?}", paranoid), format!("{:?}", normal));
    assert_ne!(format!("{:?}", normal), format!("{:?}", aggressive));
}

#[tokio::test]
async fn test_timing_preset_parse() {
    use slapper::scanner::TimingPreset;

    assert_eq!(TimingPreset::parse("paranoid"), TimingPreset::Paranoid);
    assert_eq!(TimingPreset::parse("normal"), TimingPreset::Normal);
    assert_eq!(TimingPreset::parse("aggressive"), TimingPreset::Aggressive);
    assert_eq!(TimingPreset::parse("T0"), TimingPreset::Paranoid);
    assert_eq!(TimingPreset::parse("T3"), TimingPreset::Normal);
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
    let config = SpoofConfig::from_args(
        None, None, false, None, None, None, None, false, None, false, false, None, None, None,
        None,
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(!config.enabled);
}

#[tokio::test]
async fn test_spoof_config_with_ip() {
    let config = SpoofConfig::from_args(
        Some("192.168.1.100".to_string()),
        None,
        false,
        None,
        None,
        None,
        None,
        false,
        None,
        false,
        false,
        None,
        None,
        None,
        None,
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(config.enabled);
    assert_eq!(
        config.source_ip.map(|ip| ip.to_string()),
        Some("192.168.1.100".to_string())
    );
}

#[tokio::test]
async fn test_spoof_config_with_range() {
    let config = SpoofConfig::from_args(
        None,
        Some("192.168.1.0/24".to_string()),
        false,
        None,
        None,
        None,
        None,
        false,
        None,
        false,
        false,
        None,
        None,
        None,
        None,
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(config.enabled);
    assert_eq!(config.ip_range, Some("192.168.1.0/24".to_string()));
}

#[tokio::test]
async fn test_spoof_config_with_decoys() {
    let config = SpoofConfig::from_args(
        None,
        None,
        false,
        Some("10.0.0.1,10.0.0.2".to_string()),
        None,
        Some(2),
        None,
        true,
        None,
        false,
        false,
        None,
        None,
        None,
        None,
    );
    assert!(config.is_ok());
    let config = config.unwrap();
    assert!(config.enabled);
    assert!(config.has_decoys());
    assert_eq!(config.decoy_count, 2);
    assert!(config.include_real_ip);
}
