use axum::http::HeaderMap;
use subtle::ConstantTimeEq;

/// Extracts an API token from request headers.
///
/// Checks `X-API-Key` first, then `Authorization: Bearer <token>` (case-insensitive).
/// Returns `None` if no valid token is found.
pub fn extract_api_token(headers: &HeaderMap) -> Option<String> {
    if let Some(val) = headers.get("x-api-key").and_then(|v| v.to_str().ok()) {
        return Some(val.to_string());
    }
    if let Some(val) = headers.get("authorization").and_then(|v| v.to_str().ok()) {
        let trimmed = val.trim();
        // Case-insensitive check for "bearer " prefix
        let lower = trimmed.to_ascii_lowercase();
        if lower.starts_with("bearer ") {
            let token = trimmed[7..].trim(); // "bearer ".len() == 7
            if !token.is_empty() {
                return Some(token.to_string());
            }
        }
    }
    None
}

/// Validates that the request contains a valid API key matching `expected_key`.
///
/// Uses constant-time comparison to prevent timing attacks.
/// Returns `Ok(())` if auth succeeds or no key is configured,
/// `Err(message)` if the key is missing or invalid.
pub fn validate_api_key(
    expected_key: &Option<String>,
    headers: &HeaderMap,
) -> Result<(), &'static str> {
    if let Some(ref key) = expected_key {
        match extract_api_token(headers) {
            Some(ref token) if bool::from(key.as_bytes().ct_eq(token.as_bytes())) => Ok(()),
            _ => Err("Invalid or missing API key"),
        }
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_x_api_key() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "test-token".parse().unwrap());
        assert_eq!(extract_api_token(&headers), Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_bearer_token() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer test-token".parse().unwrap());
        assert_eq!(extract_api_token(&headers), Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_bearer_case_insensitive() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "bearer test-token".parse().unwrap());
        assert_eq!(extract_api_token(&headers), Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_bearer_uppercase() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "BEARER test-token".parse().unwrap());
        assert_eq!(extract_api_token(&headers), Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_bearer_with_whitespace() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer   test-token  ".parse().unwrap());
        assert_eq!(extract_api_token(&headers), Some("test-token".to_string()));
    }

    #[test]
    fn test_extract_bearer_empty() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer ".parse().unwrap());
        assert_eq!(extract_api_token(&headers), None);
    }

    #[test]
    fn test_extract_no_prefix_fails() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "test-token".parse().unwrap());
        assert_eq!(extract_api_token(&headers), None);
    }

    #[test]
    fn test_extract_no_header() {
        let headers = HeaderMap::new();
        assert_eq!(extract_api_token(&headers), None);
    }

    #[test]
    fn test_x_api_key_takes_precedence() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer auth-token".parse().unwrap());
        headers.insert("x-api-key", "xapi-token".parse().unwrap());
        assert_eq!(extract_api_token(&headers), Some("xapi-token".to_string()));
    }

    #[test]
    fn test_validate_api_key_matches() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "secret".parse().unwrap());
        assert!(validate_api_key(&Some("secret".to_string()), &headers).is_ok());
    }

    #[test]
    fn test_validate_api_key_mismatch() {
        let mut headers = HeaderMap::new();
        headers.insert("x-api-key", "wrong".parse().unwrap());
        assert!(validate_api_key(&Some("secret".to_string()), &headers).is_err());
    }

    #[test]
    fn test_validate_api_key_no_key_configured() {
        let headers = HeaderMap::new();
        assert!(validate_api_key(&None, &headers).is_ok());
    }

    #[test]
    fn test_validate_api_key_with_bearer() {
        let mut headers = HeaderMap::new();
        headers.insert("authorization", "Bearer secret".parse().unwrap());
        assert!(validate_api_key(&Some("secret".to_string()), &headers).is_ok());
    }
}
