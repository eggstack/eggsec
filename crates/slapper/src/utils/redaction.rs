//! Evidence redaction utilities.
//!
//! Provides functions to strip or mask sensitive information from strings
//! before they are stored as evidence in findings. This prevents accidental
//! leakage of credentials, tokens, and private keys in scan reports.

use regex::Regex;
use std::sync::LazyLock;

static RE_BEARER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(bearer\s+)[A-Za-z0-9\-._~+/]+=*").unwrap());

static RE_BASIC_AUTH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(basic\s+)[A-Za-z0-9+/]+=*").unwrap());

static RE_API_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new("(?i)(api[_-]?key\\s*[=:]\\s*['\"]?)[A-Za-z0-9\\-._]{16,}['\"]?").unwrap()
});

static RE_AWS_KEY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(AKIA[0-9A-Z]{16})").unwrap());

static RE_JWT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"eyJ[A-Za-z0-9\-._]+\.eyJ[A-Za-z0-9\-._]+\.[A-Za-z0-9\-._]+").unwrap()
});

static RE_COOKIE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(cookie\s*:\s*)[^\r\n]+").unwrap());

static RE_PRIVATE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"-----BEGIN (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----[\s\S]*?-----END (?:RSA |EC |DSA |OPENSSH )?PRIVATE KEY-----",
    )
    .unwrap()
});

static RE_SECRET_VALUE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        "(?i)(secret|password|passwd|token|access_token|auth_token|client_secret|secret_key)\\s*[=:]\\s*['\"]?[^\\s'\"&]+['\"]?",
    )
    .unwrap()
});

static RE_CONNECTION_STRING: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(mysql|postgres|postgresql|mongodb|redis)://[^\s]+").unwrap()
});

static RE_SENSITIVE_KEY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)\b(password|passwd|secret|token|access_token|auth_token|client_secret|secret_key|api_key|apikey|credential|cookie|auth)\b",
    )
    .unwrap()
});
/// Redact sensitive information from a string.
///
/// Handles:
/// - Bearer tokens
/// - Basic auth credentials
/// - API keys in common `key=value` patterns
/// - JWT tokens (three base64 segments separated by dots)
/// - Cookie header values
/// - Private key PEM blocks
/// - Common secret-like key names (`secret`, `password`, `token`, etc.)
pub fn redact_sensitive(input: &str) -> String {
    let mut result = input.to_string();

    result = RE_BEARER.replace_all(&result, "${1}[REDACTED]").to_string();

    result = RE_BASIC_AUTH
        .replace_all(&result, "${1}[REDACTED]")
        .to_string();

    result = RE_API_KEY
        .replace_all(&result, "${1}[REDACTED]")
        .to_string();

    result = RE_AWS_KEY
        .replace_all(&result, "[REDACTED AWS KEY]")
        .to_string();

    result = RE_JWT.replace_all(&result, "[REDACTED]").to_string();

    result = RE_COOKIE.replace_all(&result, "${1}[REDACTED]").to_string();

    result = RE_PRIVATE_KEY
        .replace_all(&result, "[REDACTED PRIVATE KEY]")
        .to_string();

    result = RE_SECRET_VALUE
        .replace_all(&result, "${1}=[REDACTED]")
        .to_string();

    result = RE_CONNECTION_STRING
        .replace_all(&result, "[REDACTED CONNECTION STRING]")
        .to_string();

    result
}

/// Redact sensitive information from JSON values.
///
/// Recursively walks the JSON value tree, redacting string values using
/// [`redact_sensitive`] and renaming object keys that look like they
/// contain sensitive data.
pub fn redact_json(input: &serde_json::Value) -> serde_json::Value {
    match input {
        serde_json::Value::String(s) => serde_json::Value::String(redact_sensitive(s)),
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(redact_json).collect())
        }
        serde_json::Value::Object(obj) => {
            let mut redacted = serde_json::Map::new();
            for (key, value) in obj {
                let redacted_key = if is_sensitive_key(key) {
                    format!("[REDACTED {}]", key.to_uppercase())
                } else {
                    key.clone()
                };
                redacted.insert(redacted_key, redact_json(value));
            }
            serde_json::Value::Object(redacted)
        }
        other => other.clone(),
    }
}

/// Check if a key name suggests it contains sensitive data.
///
/// Uses word-boundary matching to avoid false positives like "timeout"
/// matching "token".
fn is_sensitive_key(key: &str) -> bool {
    let lower = key.to_lowercase();
    RE_SENSITIVE_KEY.is_match(&lower)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn redacts_bearer_token() {
        let input = "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.dozjgNryP4J3jVmNHl0w5N_XgL0n3I9PlFUP0THsR8U";
        let result = redact_sensitive(input);
        assert!(!result.contains("eyJ"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_basic_auth() {
        let input = "Authorization: Basic dXNlcjpwYXNzd29yZA==";
        let result = redact_sensitive(input);
        assert!(!result.contains("dXNlcjpwYXNzd29yZA=="));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_api_key_equals() {
        let input = "api_key=sk-1234567890abcdef1234567890abcdef";
        let result = redact_sensitive(input);
        assert!(!result.contains("sk-1234567890abcdef"));
        assert!(result.contains("[REDACTED]"));
    }

    #[test]
    fn redacts_api_key_colon() {
        let input = "api-key: abcdef1234567890abcdef12";
        let result = redact_sensitive(input);
        assert!(!result.contains("abcdef1234567890"));
    }

    #[test]
    fn redacts_private_key() {
        let input =
            "-----BEGIN RSA PRIVATE KEY-----\nMIIEpAIBAAKCAQEA...\n-----END RSA PRIVATE KEY-----";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED PRIVATE KEY]"));
        assert!(!result.contains("MIIEpAIBAAKCAQEA"));
    }

    #[test]
    fn redacts_ec_private_key() {
        let input = "-----BEGIN EC PRIVATE KEY-----\nMHQCAQ...\n-----END EC PRIVATE KEY-----";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED PRIVATE KEY]"));
    }

    #[test]
    fn redacts_cookie_header() {
        let input = "Cookie: session=abc123; token=xyz789";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("session=abc123"));
    }

    #[test]
    fn redacts_secret_key_value() {
        let input = "secret=mysupersecretvalue123";
        let result = redact_sensitive(input);
        assert!(result.contains("secret=[REDACTED]"));
        assert!(!result.contains("mysupersecretvalue123"));
    }

    #[test]
    fn redacts_password_key_value() {
        let input = "password: hunter2";
        let result = redact_sensitive(input);
        assert!(result.contains("password=[REDACTED]"));
        assert!(!result.contains("hunter2"));
    }

    #[test]
    fn redacts_token_key_value() {
        let input = "token=abc123def456ghi789";
        let result = redact_sensitive(input);
        assert!(result.contains("token=[REDACTED]"));
    }

    #[test]
    fn preserves_non_sensitive() {
        let input = "GET /api/users HTTP/1.1\nHost: example.com";
        let result = redact_sensitive(input);
        assert_eq!(result, input);
    }

    #[test]
    fn preserves_normal_text() {
        let input = "SQL injection found in parameter 'id'";
        let result = redact_sensitive(input);
        assert_eq!(result, input);
    }

    #[test]
    fn redacts_case_insensitive_bearer() {
        let input = "authorization: bearer abcdefghijklmnopqrstuvwxyz123456";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED]"));
        assert!(!result.contains("abcdefghijklmnopqrstuvwxyz123456"));
    }

    #[test]
    fn handles_multiple_secrets() {
        let input = "Bearer eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxMjM0NTY3ODkwIn0.SflKxwRJSMeKKF2QT4fwpMeJf36POk6yJV_adQssw5c\nAuthorization: Basic dXNlcjpwYXNzd29yZA==";
        let result = redact_sensitive(input);
        assert!(!result.contains("eyJ"));
        assert!(!result.contains("dXNlcjpwYXNzd29yZA=="));
    }

    #[test]
    fn redacts_aws_key() {
        let input = "AKIAIOSFODNN7EXAMPLE";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED AWS KEY]"));
    }

    #[test]
    fn redacts_aws_key_in_context() {
        let input = "aws_access_key_id = AKIAIOSFODNN7EXAMPLE";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED AWS KEY]"));
        assert!(!result.contains("AKIAIOSFODNN7EXAMPLE"));
    }

    #[test]
    fn redacts_connection_string() {
        let input = "postgres://user:password@localhost:5432/db";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED CONNECTION STRING]"));
    }

    #[test]
    fn redacts_mysql_connection_string() {
        let input = "mysql://root:secret@db.example.com:3306/mydb";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED CONNECTION STRING]"));
    }

    #[test]
    fn redacts_mongodb_connection_string() {
        let input = "mongodb://admin:pass123@mongo.host:27017/authdb";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED CONNECTION STRING]"));
    }

    #[test]
    fn redacts_redis_connection_string() {
        let input = "redis://default:mypassword@redis.host:6379";
        let result = redact_sensitive(input);
        assert!(result.contains("[REDACTED CONNECTION STRING]"));
    }

    #[test]
    fn redacts_secret_key_with_extended_pattern() {
        let input = "secret_key=supersecretvalue123456";
        let result = redact_sensitive(input);
        assert!(result.contains("secret_key=[REDACTED]"));
    }

    #[test]
    fn redacts_client_secret() {
        let input = "client_secret='s3cr3t_v4lue_here'";
        let result = redact_sensitive(input);
        assert!(result.contains("client_secret=[REDACTED]"));
    }

    #[test]
    fn redact_json_redacts_sensitive_keys() {
        let input = serde_json::json!({
            "username": "admin",
            "password": "secret123",
            "api_key": "sk-1234567890abcdef"
        });
        let result = redact_json(&input);
        assert_eq!(result["username"], "admin");
        assert!(result
            .as_object()
            .unwrap()
            .contains_key("[REDACTED PASSWORD]"));
    }

    #[test]
    fn redact_json_preserves_non_sensitive() {
        let input = serde_json::json!({
            "name": "Test",
            "count": 42
        });
        let result = redact_json(&input);
        assert_eq!(result, input);
    }

    #[test]
    fn redact_json_recurses_arrays() {
        let input = serde_json::json!({
            "items": [
                {"token": "abc123def456ghi789"},
                {"value": "safe"}
            ]
        });
        let result = redact_json(&input);
        let items = result["items"].as_array().unwrap();
        assert_eq!(items[0]["token=[REDACTED TOKEN]"], serde_json::Value::Null);
        assert_eq!(items[1]["value"], "safe");
    }

    #[test]
    fn redact_json_nested_objects() {
        let input = serde_json::json!({
            "config": {
                "auth": {
                    "secret": "hunter2",
                    "timeout": 30
                }
            }
        });
        let result = redact_json(&input);
        let auth_obj = result["config"]
            .as_object()
            .unwrap()
            .get("[REDACTED AUTH]")
            .unwrap()
            .as_object()
            .unwrap();
        assert_eq!(auth_obj["timeout"], 30);
        assert!(auth_obj.contains_key("[REDACTED SECRET]"));
    }
}
