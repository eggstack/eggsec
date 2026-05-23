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
    assert_eq!(results.successful_requests, 0);
    assert_eq!(results.failed_requests, 5);
    assert!(results.status_codes.contains_key(&404));
}

#[tokio::test]
async fn test_load_test_error_body_consumption() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/error"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Internal Server Error"))
        .mount(&server)
        .await;

    let runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/error", server.uri()),
        5,
        1,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();

    assert_eq!(results.total_requests, 5);
    assert_eq!(results.successful_requests, 0);
    assert!(results.status_codes.contains_key(&500));
}

#[tokio::test]
async fn test_load_test_redirect_following() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/redirect"))
        .respond_with(
            ResponseTemplate::new(302)
                .append_header("Location", "/final")
                .set_body_string("Redirecting..."),
        )
        .mount(&server)
        .await;

    Mock::given(method("GET"))
        .and(path("/final"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Final"))
        .mount(&server)
        .await;

    let runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/redirect", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 3);
}

#[tokio::test]
async fn test_load_test_with_basic_auth() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/protected"))
        .and(header("Authorization", "Basic dXNlcjpwYXNz"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Authenticated"))
        .mount(&server)
        .await;

    let mut runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/protected", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(slapper::cli::CommonHttpArgs {
        auth: Some("user:pass".to_string()),
        bearer: None,
        cookie: None,
        api_key: None,
        user_agent: None,
        insecure: false,
        proxy: None,
        proxy_auth: None,
        rate_limit: None,
        stealth: false,
        jitter: None,
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.successful_requests, 3);
}

#[tokio::test]
async fn test_load_test_with_bearer_token() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("Authorization", "Bearer test-token-123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("{\"status\":\"ok\"}"))
        .mount(&server)
        .await;

    let mut runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/api", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(slapper::cli::CommonHttpArgs {
        auth: None,
        bearer: Some("test-token-123".to_string()),
        cookie: None,
        api_key: None,
        user_agent: None,
        insecure: false,
        proxy: None,
        proxy_auth: None,
        rate_limit: None,
        stealth: false,
        jitter: None,
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.successful_requests, 3);
}

#[tokio::test]
async fn test_load_test_metrics_latency_tracking() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/fast"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(10)))
        .mount(&server)
        .await;

    let runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/fast", server.uri()),
        20,
        5,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 20);
    assert_eq!(results.successful_requests, 20);
    assert!(results.requests_per_second > 0.0);
}

#[tokio::test]
async fn test_load_test_with_slow_response() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/slow"))
        .respond_with(ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(100)))
        .mount(&server)
        .await;

    let runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/slow", server.uri()),
        10,
        2,
        Duration::from_secs(10),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 10);
    assert_eq!(results.successful_requests, 10);
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

#[tokio::test]
async fn test_load_test_4xx_client_errors() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/bad-request"))
        .respond_with(ResponseTemplate::new(400).set_body_string("Bad Request"))
        .mount(&server)
        .await;

    let runner = slapper::loadtest::LoadTestRunner::new(
        format!("{}/bad-request", server.uri()),
        5,
        1,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 5);
    assert!(results.status_codes.contains_key(&400));
}
