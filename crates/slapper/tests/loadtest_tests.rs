mod common;

use common::{create_test_server, mock_ok};
use std::time::Duration;

#[tokio::test]
async fn test_load_test_basic() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let runner =
        slapper::loadtest::LoadTestRunner::new(server.uri(), 10, 2, Duration::from_secs(5))
            .unwrap();

    let results = runner.run().await.unwrap();

    assert_eq!(results.total_requests, 10);
    assert_eq!(results.successful_requests, 10);
    assert_eq!(results.failed_requests, 0);
    assert!(results.requests_per_second > 0.0);
}

#[tokio::test]
async fn test_load_test_concurrency() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let runner =
        slapper::loadtest::LoadTestRunner::new(server.uri(), 100, 50, Duration::from_secs(10))
            .unwrap();

    let results = runner.run().await.unwrap();

    assert_eq!(results.total_requests, 100);
    assert_eq!(results.successful_requests, 100);
}

#[tokio::test]
async fn test_load_test_with_errors() {
    let server = create_test_server().await;
    mock_ok("/ok").mount(&server).await;

    let runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/notfound", server.uri()),
        5,
        1,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();

    assert_eq!(results.total_requests, 5);
    assert_eq!(results.successful_requests, 5);
    assert!(results.status_codes.contains_key(&404));
}

#[test]
fn test_load_test_zero_concurrency() {
    let result = slapper::loadtest::LoadTestRunner::new(
        "http://example.com".to_string(),
        10,
        0,
        Duration::from_secs(5),
    );
    assert!(result.is_err());
}

#[test]
fn test_load_test_zero_requests() {
    let result = slapper::loadtest::LoadTestRunner::new(
        "http://example.com".to_string(),
        0,
        10,
        Duration::from_secs(5),
    );
    assert!(result.is_err());
}
