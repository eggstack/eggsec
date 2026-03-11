use std::time::Duration;
use wiremock::{Mock, MockServer, ResponseTemplate};
use wiremock::matchers::{method, path as path_matcher};
use slapper::scanner::spoof::SpoofConfig;

async fn create_test_server() -> MockServer {
    MockServer::start().await
}

fn mock_ok(p: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(p))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
}

fn mock_not_found(p: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(p))
        .respond_with(ResponseTemplate::new(404))
}

#[tokio::test]
async fn test_endpoint_scan_basic() {
    let server = create_test_server().await;
    mock_ok("/api").mount(&server).await;
    mock_ok("/health").mount(&server).await;
    mock_not_found("/admin").mount(&server).await;
    
    let endpoints = vec!["/api".to_string(), "/health".to_string(), "/admin".to_string()];
    let spoof_config = SpoofConfig::default();
    
    let results = slapper::scanner::scan_endpoints(
        &server.uri(),
        endpoints,
        5,
        Duration::from_secs(5),
        false,
        false,
        spoof_config,
    ).await.unwrap();
    
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
    ).await.unwrap();
    
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
    ).await.unwrap();
    
    let interesting = results.results.iter().filter(|e| e.interesting).count();
    assert!(interesting >= 1);
}
