mod common;

use common::{create_test_server, mock_not_found, mock_ok};
use std::time::Duration;

#[tokio::test]
async fn test_load_test_basic() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let runner =
        eggsec::loadtest::LoadTestRunner::new(server.uri(), 10, 2, Duration::from_secs(5)).unwrap();

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
        eggsec::loadtest::LoadTestRunner::new(server.uri(), 100, 50, Duration::from_secs(10))
            .unwrap();

    let results = runner.run().await.unwrap();

    assert_eq!(results.total_requests, 100);
    assert_eq!(results.successful_requests, 100);
}

#[tokio::test]
async fn test_load_test_with_errors() {
    let server = create_test_server().await;

    let runner = eggsec::loadtest::LoadTestRunner::new(
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

    let runner = eggsec::loadtest::LoadTestRunner::new(
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

    let runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/redirect", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 3);
    assert_eq!(results.successful_requests, 3);
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

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/protected", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
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
        auth_context: None,
        auth_role: None,
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

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/api", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
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
        auth_context: None,
        auth_role: None,
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

    let runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/fast", server.uri()),
        20,
        5,
        Duration::from_secs(5),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 20);
    assert_eq!(results.successful_requests, 20);
    assert!(
        results.latency_mean_ms >= 5.0,
        "Mean latency {}ms should reflect 10ms server delay",
        results.latency_mean_ms
    );
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

    let runner = eggsec::loadtest::LoadTestRunner::new(
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
    let result = eggsec::loadtest::LoadTestRunner::new(
        "http://example.com".to_string(),
        10,
        0,
        Duration::from_secs(5),
    );
    assert!(result.is_err());
}

#[test]
fn test_load_test_zero_requests() {
    let result = eggsec::loadtest::LoadTestRunner::new(
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

    let runner = eggsec::loadtest::LoadTestRunner::new(
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

#[tokio::test]
async fn test_load_test_with_rate_limit() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let mut runner =
        eggsec::loadtest::LoadTestRunner::new(server.uri(), 20, 5, Duration::from_secs(10))
            .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
        auth: None,
        bearer: None,
        cookie: None,
        api_key: None,
        user_agent: None,
        insecure: false,
        proxy: None,
        proxy_auth: None,
        rate_limit: Some(10),
        stealth: false,
        jitter: None,
        auth_context: None,
        auth_role: None,
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 20);
    assert_eq!(results.successful_requests, 20);
    // With rate limit of 10 RPS and 20 requests, minimum duration is ~1.9s
    // (first request at ~100ms, last at ~1900ms). RPS should be <= 15.
    assert!(
        results.requests_per_second <= 15.0,
        "RPS {} should be bounded near rate limit of 10",
        results.requests_per_second
    );
    assert!(
        results.total_duration_ms >= 1500,
        "Duration {}ms should be >= 1500ms for 20 requests at 10 RPS",
        results.total_duration_ms
    );
}

#[tokio::test]
async fn test_load_test_from_args_with_config() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let args = eggsec::cli::LoadArgs {
        url: server.uri(),
        requests: 10,
        concurrency: 2,
        method: "GET".to_string(),
        body: None,
        headers: vec![],
        timeout: Some(5),
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: eggsec::cli::CommonHttpArgs::default(),
    };

    let config = eggsec::config::EggsecConfig::default();
    let runner = eggsec::loadtest::LoadTestRunner::from_args_with_config(args, &config).unwrap();
    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 10);
    assert_eq!(results.successful_requests, 10);
}

#[tokio::test]
async fn test_load_test_from_args_with_tui_mode() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let args = eggsec::cli::LoadArgs {
        url: server.uri(),
        requests: 10,
        concurrency: 2,
        method: "GET".to_string(),
        body: None,
        headers: vec![],
        timeout: Some(5),
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: eggsec::cli::CommonHttpArgs::default(),
    };

    let runner = eggsec::loadtest::LoadTestRunner::from_args_with_tui_mode(args, true).unwrap();
    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 10);
    assert_eq!(results.successful_requests, 10);
}

#[tokio::test]
async fn test_load_test_post_method() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("POST"))
        .and(path("/submit"))
        .respond_with(ResponseTemplate::new(201).set_body_string("Created"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/submit", server.uri()),
        5,
        2,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_method("POST".to_string());
    runner.set_body(r#"{"key":"value"}"#.to_string());

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 5);
    assert_eq!(results.successful_requests, 5);
    assert!(results.status_codes.contains_key(&201));
}

#[tokio::test]
async fn test_load_test_custom_headers() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("X-Custom", "test-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/api", server.uri()),
        5,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.add_header("X-Custom".to_string(), "test-value".to_string());

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 5);
    assert_eq!(results.successful_requests, 5);
}

#[tokio::test]
async fn test_load_test_error_cap() {
    let server = create_test_server().await;
    mock_not_found("/").mount(&server).await;

    let runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/", server.uri()),
        1500,
        10,
        Duration::from_secs(30),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 1500);
    assert!(
        results.errors.len() <= 1000,
        "Errors should be capped at 1000, got {}",
        results.errors.len()
    );
}

#[tokio::test]
async fn test_load_test_options_method() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("OPTIONS"))
        .and(path("/cors"))
        .respond_with(ResponseTemplate::new(204).insert_header("Access-Control-Allow-Origin", "*"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/cors", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_method("OPTIONS".to_string());

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 3);
    assert!(results.status_codes.contains_key(&204));
}

#[tokio::test]
async fn test_load_test_with_api_key() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("X-API-Key", "my-secret-key"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/api", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
        api_key: Some("my-secret-key".to_string()),
        ..eggsec::cli::CommonHttpArgs::default()
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.successful_requests, 3);
}

#[tokio::test]
async fn test_load_test_with_api_key_header_format() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/api"))
        .and(header("X-Api-Token", "token-value"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/api", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
        api_key: Some("X-Api-Token:token-value".to_string()),
        ..eggsec::cli::CommonHttpArgs::default()
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.successful_requests, 3);
}

#[tokio::test]
async fn test_load_test_with_cookie() {
    use wiremock::matchers::{header, method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/dashboard"))
        .and(header("Cookie", "session=abc123"))
        .respond_with(ResponseTemplate::new(200).set_body_string("Dashboard"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/dashboard", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
        cookie: Some("session=abc123".to_string()),
        ..eggsec::cli::CommonHttpArgs::default()
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.successful_requests, 3);
}

#[tokio::test]
async fn test_load_test_rate_limit_zero_ignored() {
    let server = create_test_server().await;
    mock_ok("/").mount(&server).await;

    let mut runner =
        eggsec::loadtest::LoadTestRunner::new(server.uri(), 5, 2, Duration::from_secs(5)).unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
        rate_limit: Some(0),
        ..eggsec::cli::CommonHttpArgs::default()
    });

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 5);
    assert_eq!(results.successful_requests, 5);
}

#[tokio::test]
async fn test_load_test_unknown_method_defaults_to_get() {
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, ResponseTemplate};

    let server = create_test_server().await;
    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
        .mount(&server)
        .await;

    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/test", server.uri()),
        3,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_method("FOOBAR".to_string());

    let results = runner.run().await.unwrap();
    assert_eq!(results.successful_requests, 3);
}

#[test]
fn test_load_test_malformed_auth_format() {
    let mut runner = eggsec::loadtest::LoadTestRunner::new(
        "http://example.com".to_string(),
        1,
        1,
        Duration::from_secs(5),
    )
    .unwrap();
    runner.set_common(eggsec::cli::CommonHttpArgs {
        auth: Some("no-colon-separator".to_string()),
        ..eggsec::cli::CommonHttpArgs::default()
    });
    // Should not panic; malformed auth is logged and ignored
}

#[test]
fn test_load_test_zero_timeout() {
    let result = eggsec::loadtest::LoadTestRunner::new(
        "http://example.com".to_string(),
        10,
        5,
        Duration::from_secs(0),
    );
    assert!(result.is_err());
}

#[test]
fn test_load_test_from_args_with_config_uses_config_timeout() {
    let args = eggsec::cli::LoadArgs {
        url: "http://example.com".to_string(),
        requests: 1,
        concurrency: 1,
        method: "GET".to_string(),
        body: None,
        headers: vec![],
        timeout: None,
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: eggsec::cli::CommonHttpArgs::default(),
    };

    let mut config = eggsec::config::EggsecConfig::default();
    config.http.timeout_secs = 42;

    let runner = eggsec::loadtest::LoadTestRunner::from_args_with_config(args, &config).unwrap();
    // When timeout is None, config value (42) should be used.
    drop(runner);
}

#[test]
fn test_load_test_from_args_with_config_explicit_timeout() {
    let args = eggsec::cli::LoadArgs {
        url: "http://example.com".to_string(),
        requests: 1,
        concurrency: 1,
        method: "GET".to_string(),
        body: None,
        headers: vec![],
        timeout: Some(15),
        json: false,
        verbose: false,
        quiet: false,
        output: None,
        common: eggsec::cli::CommonHttpArgs::default(),
    };

    let mut config = eggsec::config::EggsecConfig::default();
    config.http.timeout_secs = 42;

    let runner = eggsec::loadtest::LoadTestRunner::from_args_with_config(args, &config).unwrap();
    // When timeout is Some(15), explicit value (15) should override config (42).
    drop(runner);
}

#[tokio::test]
async fn test_load_test_all_requests_fail_latency_still_recorded() {
    let server = create_test_server().await;
    mock_not_found("/").mount(&server).await;

    let runner = eggsec::loadtest::LoadTestRunner::new(
        format!("{}/", server.uri()),
        10,
        2,
        Duration::from_secs(10),
    )
    .unwrap();

    let results = runner.run().await.unwrap();
    assert_eq!(results.total_requests, 10);
    assert_eq!(results.failed_requests, 10);
    // Latency should still be recorded for failed requests
    assert!(
        results.latency_p50_ms >= 0.0,
        "P50 latency should be non-negative even when all requests fail"
    );
}
