#![cfg(all(feature = "rest-api", feature = "ai-integration"))]

use eggsec::tool::protocol::mcp::{validate_auth_internal, validate_auth_params, McpError};

#[test]
fn test_auth_no_key_configured() {
    let result = validate_auth_internal(&None, None);
    assert!(result.is_ok());
}

#[test]
fn test_auth_with_key_matching() {
    let api_key = Some("test-api-key".to_string());
    let result = validate_auth_internal(&api_key, Some("test-api-key"));
    assert!(result.is_ok());
}

#[test]
fn test_auth_with_key_mismatching() {
    let api_key = Some("test-api-key".to_string());
    let result = validate_auth_internal(&api_key, Some("wrong-key"));
    assert!(result.is_err());
}

#[test]
fn test_auth_with_key_empty_input() {
    let api_key = Some("test-api-key".to_string());
    let result = validate_auth_internal(&api_key, None);
    assert!(result.is_err());
}

#[test]
fn test_auth_no_key_with_input() {
    let api_key = None;
    let result = validate_auth_internal(&api_key, Some("any-key"));
    assert!(result.is_ok());
}

#[test]
fn test_auth_params_with_key() {
    let api_key = Some("test-api-key".to_string());
    let params = serde_json::json!({"api_key": "test-api-key"});
    let result = validate_auth_params(&api_key, &Some(params));
    assert!(result.is_ok());
}

#[test]
fn test_auth_params_with_wrong_key() {
    let api_key = Some("test-api-key".to_string());
    let params = serde_json::json!({"api_key": "wrong-key"});
    let result = validate_auth_params(&api_key, &Some(params));
    assert!(result.is_err());
}

#[test]
fn test_auth_params_missing_key() {
    let api_key = Some("test-api-key".to_string());
    let params = serde_json::json!({"target": "http://example.com"});
    let result = validate_auth_params(&api_key, &Some(params));
    assert!(result.is_err());
}

#[test]
fn test_auth_params_no_api_key_in_config() {
    let api_key = None;
    let params = serde_json::json!({"api_key": "any-key"});
    let result = validate_auth_params(&api_key, &Some(params));
    assert!(result.is_ok());
}

#[test]
fn test_auth_params_empty_params() {
    let api_key = Some("test-api-key".to_string());
    let result = validate_auth_params(&api_key, &None);
    assert!(result.is_err());
}

#[test]
fn test_validate_auth_unauthorized_error() {
    let api_key = Some("test-api-key".to_string());
    let result = validate_auth_internal(&api_key, Some("wrong"));
    match result {
        Err(McpError { code: -32001, .. }) => {}
        _ => panic!("Expected Unauthorized error"),
    }
}

#[tokio::test]
async fn test_rate_limit_concurrent_requests() {
    use eggsec::utils::rate_limiter::RateLimiter;

    let mut rate_limiter = RateLimiter::new(10);
    for _ in 0..5 {
        rate_limiter.acquire().await;
    }
    assert!(rate_limiter.available() < 10.0);
}

use eggsec::utils::rate_limiter::{RateLimitStatus, RateLimiter};

#[test]
fn test_rate_limiter_initialization() {
    let rate_limiter = RateLimiter::new(1);

    let status: RateLimitStatus = rate_limiter.get_status("test");
    assert_eq!(status.requests_per_minute, 60);
}

#[test]
fn test_rate_limiter_status_reports_current_limit() {
    let rate_limiter = RateLimiter::new(10);
    let status = rate_limiter.get_status("anonymous");
    assert_eq!(status.requests_per_minute, 600);
    assert_eq!(status.tokens_available, 10.0);
}
