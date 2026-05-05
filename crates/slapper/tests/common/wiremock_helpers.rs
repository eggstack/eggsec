//! Wiremock test server helpers.
//!
//! Provides common mock server setup and response builders for integration tests.

use wiremock::matchers::{header, method, path as path_matcher};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Create a new mock test server.
pub async fn create_test_server() -> MockServer {
    MockServer::start().await
}

/// Create a mock that responds with 200 OK and the given body.
pub fn mock_ok(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
}

/// Create a mock that responds with 200 OK and a custom body.
pub fn mock_ok_with_body(path: &str, body: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(200).set_body_string(body))
}

/// Create a mock that responds with 404 Not Found.
pub fn mock_not_found(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(404))
}

/// Create a mock that responds with a custom status code.
pub fn mock_status(path: &str, status: u16) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(status))
}

/// Create a mock that simulates a WAF response with specific headers.
pub fn mock_waf_response(path: &str, waf_header_name: &str, waf_header_value: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header(waf_header_name, waf_header_value)
                .set_body_string("Blocked by WAF"),
        )
}

/// Create a mock that simulates a Cloudflare WAF response.
pub fn mock_cloudflare_response(path: &str) -> Mock {
    mock_waf_response(path, "cf-ray", "123456789-ABC")
}

/// Create a mock that simulates an AWS WAF response.
pub fn mock_aws_waf_response(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(403)
                .insert_header("x-amzn-waf-action", "challenge")
                .set_body_string("AWS WAF Challenge"),
        )
}

/// Create a mock with a specific response delay.
pub fn mock_slow_response(path: &str, delay_ms: u64) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(200).set_delay(std::time::Duration::from_millis(delay_ms)),
        )
}

/// Create a mock that returns JSON.
pub fn mock_json(path: &str, json: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("content-type", "application/json")
                .set_body_string(json),
        )
}

/// Create a mock that matches a specific header value.
pub fn mock_with_header(path: &str, header_name: &str, header_value: &str, status: u16) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .and(header(header_name, header_value))
        .respond_with(ResponseTemplate::new(status))
}

/// Create a mock that responds to POST requests.
pub fn mock_post_ok(path: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(200).set_body_string("OK"))
}

/// Create a mock that simulates SQL injection vulnerability.
pub fn mock_sqli_vulnerable(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"error": "You have an error in your SQL syntax near '1' at line 1"}"#,
        ))
}

/// Create a mock that simulates XSS vulnerability (reflects input).
pub fn mock_xss_vulnerable(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"<html><body>User input: FUZZ</body></html>"#),
        )
}

/// Create a mock that simulates path traversal vulnerability.
pub fn mock_traversal_vulnerable(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(ResponseTemplate::new(200).set_body_string("root:x:0:0:root:/root:/bin/bash"))
}

/// Create a mock that simulates SSRF vulnerability.
pub fn mock_ssrf_vulnerable(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"internal_ip": "127.0.0.1", "status": "connected"}"#),
        )
}

/// Create a mock that simulates SSTI vulnerability.
pub fn mock_ssti_vulnerable(path: &str) -> Mock {
    Mock::given(method("GET"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"Hello 49"#), // 7*7 = 49
        )
}

/// Create a mock that returns a GraphQL introspection response.
pub fn mock_graphql_introspection(path: &str) -> Mock {
    Mock::given(method("POST"))
        .and(path_matcher(path))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_string(r#"{"data": {"__schema": {"types": [{"name": "Query"}]}}}"#)
                .insert_header("content-type", "application/json"),
        )
}
