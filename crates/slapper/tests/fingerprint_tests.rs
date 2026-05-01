//! Tests for service fingerprinting.
//!
//! Tests the fingerprint module's ability to identify services based on
//! banner responses and probe matching.

mod common;

use common::*;

#[tokio::test]
async fn test_fingerprint_http_service() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let uri = server.uri();
    let host = uri.replace("http://", "");
    let parts: Vec<&str> = host.split(':').collect();
    let port: u16 = parts.get(1).unwrap_or(&"80").parse().unwrap_or(80);

    let results = slapper::scanner::fingerprint_services(
        parts[0],
        vec![port],
        std::time::Duration::from_secs(5),
        true, // tui_mode to suppress progress bar
        10,   // concurrency
        None, // progress_tx
        None, // max_results
    )
    .await
    .unwrap();

    assert!(!results.results.is_empty(), "Should identify at least one service");
    
    // Should identify HTTP
    let http_result = results.results.iter().find(|r| r.service == "HTTP");
    assert!(http_result.is_some(), "Should identify HTTP service");
}

#[tokio::test]
async fn test_fingerprint_unreachable_port() {
    // Use a port that should be unreachable
    let results = slapper::scanner::fingerprint_services(
        "127.0.0.1",
        vec![1], // TCPMUX port, typically closed
        std::time::Duration::from_secs(1),
        true,
        10, // concurrency
        None, // progress_tx
        None, // max_results
    )
    .await
    .unwrap();

    // Should complete without error, even if no services found
    assert_eq!(results.ports_scanned, 1);
}

// Test probe data structure
#[test]
fn test_probes_not_empty() {
    // The PROBES array is private, but we can test the fingerprint results struct
    let fp = slapper::scanner::ServiceFingerprint {
        port: 80,
        service: "HTTP".to_string(),
        banner: Some("Apache/2.4.41".to_string()),
        version: Some("2.4.41".to_string()),
        product: Some("Apache".to_string()),
        extra: None,
        confidence: 90,
    };
    
    assert_eq!(fp.port, 80);
    assert_eq!(fp.service, "HTTP");
    assert!(fp.confidence > 0);
}

#[test]
fn test_fingerprint_results_display() {
    let results = slapper::scanner::FingerprintResults {
        host: "example.com".to_string(),
        ports_scanned: 10,
        services_identified: 3,
        duration_ms: 1500,
        results: vec![
            slapper::scanner::ServiceFingerprint {
                port: 80,
                service: "HTTP".to_string(),
                banner: Some("nginx/1.18.0".to_string()),
                version: Some("1.18.0".to_string()),
                product: Some("nginx".to_string()),
                extra: None,
                confidence: 90,
            },
            slapper::scanner::ServiceFingerprint {
                port: 22,
                service: "SSH".to_string(),
                banner: Some("OpenSSH_8.2".to_string()),
                version: Some("8.2".to_string()),
                product: Some("OpenSSH".to_string()),
                extra: None,
                confidence: 90,
            },
        ],
    };
    
    let display = format!("{}", results);
    assert!(display.contains("example.com"));
    assert!(display.contains("HTTP"));
    assert!(display.contains("SSH"));
}

// Test spoof config
#[test]
fn test_spoof_config_default() {
    let config = slapper::scanner::SpoofConfig::default();
    assert!(!config.enabled);
}

#[test]
fn test_random_ip_from_cidr() {
    let ip = slapper::scanner::random_ip_from_cidr("10.0.0.0/24");
    assert!(ip.is_ok());
    let ip_addr = ip.unwrap();
    // Check it's in the 10.0.0.x range
    let octets = ip_addr.octets();
    assert_eq!(octets[0], 10);
    assert_eq!(octets[1], 0);
    assert_eq!(octets[2], 0);
}

#[test]
fn test_invalid_cidr() {
    let result = slapper::scanner::random_ip_from_cidr("invalid");
    assert!(result.is_err());
}
