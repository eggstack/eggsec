//! Transport-neutral header handling.
//!
//! Canonical header/cookie application for the workspace. Domain code must
//! use these pure transformation functions instead of concrete-client builder
//! mutation.
//!
//! Semantics (explicit, matching the documented intent):
//! - Auth-context headers **overwrite** any existing header with the same name.
//! - Auth-context cookies are **merged** with any existing `Cookie` header:
//!   auth-context values win on name collision, unrelated existing cookies
//!   are preserved. This is a true merge (the pre-Phase-B concrete-client
//!   helper replaced the header entirely; that behavior is retained only by
//!   the compatibility wrapper in the engine crate).

use crate::TransportError;
use http::{HeaderMap, HeaderName, HeaderValue};
use std::collections::HashMap;

/// Header names whose values are secrets and must never appear in
/// `Debug`/`Display` output, logs, or error strings.
const SENSITIVE_HEADERS: &[&str] = &[
    "authorization",
    "proxy-authorization",
    "cookie",
    "set-cookie",
    "x-api-key",
    "api-key",
    "x-auth-token",
];

/// Returns `true` if `name` carries secret material (case-insensitive).
pub fn is_sensitive_header(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    if SENSITIVE_HEADERS.contains(&lower.as_str()) {
        return true;
    }
    // Bearer-style `X-*-key/token/secret` headers are secrets too.
    lower.contains("secret")
        || (lower.starts_with("x-") && (lower.contains("key") || lower.contains("token")))
}

/// Redacted placeholder used in `Debug` output for secret header values.
pub const REDACTED: &str = "***REDACTED***";

/// Format a [`HeaderMap`] with secret values redacted.
///
/// Used by the custom `Debug` impls on request/response/recorded types so a
/// forgotten `{:?}` in logs can never leak `Authorization`, `Cookie`, or
/// `Proxy-Authorization` material.
pub fn redacted_headers_debug(map: &HeaderMap) -> String {
    let mut parts = Vec::with_capacity(map.len());
    for (name, value) in map.iter() {
        let display = if is_sensitive_header(name.as_str()) {
            REDACTED.to_string()
        } else {
            value.to_str().unwrap_or("<non-utf8>").to_string()
        };
        parts.push(format!("{}: {}", name.as_str(), display));
    }
    format!("{{{}}}", parts.join(", "))
}

/// Parse `name`/`value` pairs into a [`HeaderMap`].
///
/// Invalid names or non-visible-ASCII values are rejected with
/// [`TransportError::InvalidRequest`] (fail-closed, no silent truncation).
pub fn header_map_from_pairs(pairs: &HashMap<String, String>) -> Result<HeaderMap, TransportError> {
    let mut map = HeaderMap::with_capacity(pairs.len());
    for (k, v) in pairs {
        let name: HeaderName = k.parse().map_err(|e| {
            TransportError::InvalidRequest(format!("invalid header name '{k}': {e}"))
        })?;
        let value: HeaderValue = v.parse().map_err(|e| {
            TransportError::InvalidRequest(format!("invalid header value for '{k}': {e}"))
        })?;
        map.insert(name, value);
    }
    Ok(map)
}

/// Apply auth-context headers and cookies to a transport-neutral [`HeaderMap`].
///
/// This is the canonical replacement for the engine's concrete-client
/// compatibility wrapper (`auth_context::apply_auth_context_to_request`).
/// It is a pure transformation: no concrete HTTP client types appear in
/// its signature.
///
/// - `headers`: each entry overwrites any existing header with the same name.
/// - `cookies`: merged with any existing `Cookie` header via
///   [`merge_cookie_header`]; auth-context values win on name collision.
pub fn apply_auth_headers(
    map: &mut HeaderMap,
    headers: &HashMap<String, String>,
    cookies: &HashMap<String, String>,
) -> Result<(), TransportError> {
    for (k, v) in headers {
        let name: HeaderName = k.parse().map_err(|e| {
            TransportError::InvalidRequest(format!("invalid auth header name '{k}': {e}"))
        })?;
        let value: HeaderValue = v.parse().map_err(|e| {
            TransportError::InvalidRequest(format!("invalid auth header value for '{k}': {e}"))
        })?;
        map.insert(name, value);
    }
    if !cookies.is_empty() {
        let existing = map
            .get(http::header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let merged = merge_cookie_header(existing.as_deref(), cookies);
        let value: HeaderValue = merged.parse().map_err(|e| {
            TransportError::InvalidRequest(format!("invalid merged Cookie header: {e}"))
        })?;
        map.insert(http::header::COOKIE, value);
    }
    Ok(())
}

/// Merge auth-context cookies with an existing `Cookie` header value.
///
/// `existing` is parsed as `name=value; ...` pairs. Auth-context entries win
/// on name collision; unrelated existing cookies are preserved. Output order
/// is deterministic (sorted by cookie name) so tests and logs are stable
/// despite `HashMap` iteration randomness.
pub fn merge_cookie_header(
    existing: Option<&str>,
    auth_cookies: &HashMap<String, String>,
) -> String {
    let mut merged: std::collections::BTreeMap<String, String> = std::collections::BTreeMap::new();
    if let Some(cur) = existing {
        for pair in cur.split(';') {
            let pair = pair.trim();
            if pair.is_empty() {
                continue;
            }
            if let Some((k, v)) = pair.split_once('=') {
                merged.insert(k.trim().to_string(), v.trim().to_string());
            } else {
                merged.insert(pair.to_string(), String::new());
            }
        }
    }
    for (k, v) in auth_cookies {
        merged.insert(k.clone(), v.clone());
    }
    merged
        .iter()
        .map(|(k, v)| format!("{k}={v}"))
        .collect::<Vec<_>>()
        .join("; ")
}

/// Build a `Cookie` header value from auth-context cookies only (no existing
/// header). Deterministic (sorted) ordering.
pub fn cookies_to_header_value(cookies: &HashMap<String, String>) -> String {
    merge_cookie_header(None, cookies)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map_with(cookie: &str) -> HeaderMap {
        let mut m = HeaderMap::new();
        m.insert(
            http::header::COOKIE,
            HeaderValue::from_str(cookie).expect("cookie"),
        );
        m
    }

    #[test]
    fn sensitive_headers_detected_case_insensitive() {
        assert!(is_sensitive_header("Authorization"));
        assert!(is_sensitive_header("AUTHORIZATION"));
        assert!(is_sensitive_header("cookie"));
        assert!(is_sensitive_header("Proxy-Authorization"));
        assert!(is_sensitive_header("X-Api-Key"));
        assert!(!is_sensitive_header("Content-Type"));
        assert!(!is_sensitive_header("User-Agent"));
    }

    #[test]
    fn redacted_debug_never_leaks_secrets() {
        let mut m = HeaderMap::new();
        m.insert(
            http::header::AUTHORIZATION,
            HeaderValue::from_static("Bearer s3cr3t"),
        );
        m.insert(
            http::header::COOKIE,
            HeaderValue::from_static("session=abc123"),
        );
        m.insert(
            http::header::USER_AGENT,
            HeaderValue::from_static("Eggsec/1.0"),
        );
        let dbg = redacted_headers_debug(&m);
        assert!(!dbg.contains("s3cr3t"), "auth secret leaked: {dbg}");
        assert!(!dbg.contains("abc123"), "cookie secret leaked: {dbg}");
        assert!(dbg.contains("Eggsec/1.0"), "non-secret lost: {dbg}");
        assert!(dbg.contains(REDACTED));
    }

    #[test]
    fn auth_headers_overwrite_and_cookies_merge() {
        let mut m = map_with("keep=1; session=old");
        let mut headers = HashMap::new();
        headers.insert("Authorization".to_string(), "Bearer new".to_string());
        let mut cookies = HashMap::new();
        cookies.insert("session".to_string(), "new".to_string());
        apply_auth_headers(&mut m, &headers, &cookies).expect("apply");
        assert_eq!(
            m.get(http::header::AUTHORIZATION)
                .expect("auth")
                .to_str()
                .expect("str"),
            "Bearer new"
        );
        let cookie = m
            .get(http::header::COOKIE)
            .expect("cookie")
            .to_str()
            .expect("str");
        assert!(cookie.contains("keep=1"), "unrelated cookie lost: {cookie}");
        assert!(
            cookie.contains("session=new"),
            "auth cookie missing: {cookie}"
        );
        assert!(
            !cookie.contains("session=old"),
            "stale cookie kept: {cookie}"
        );
    }

    #[test]
    fn invalid_header_name_rejected_fail_closed() {
        let mut m = HeaderMap::new();
        let mut headers = HashMap::new();
        headers.insert("Bad\nName".to_string(), "v".to_string());
        let err = apply_auth_headers(&mut m, &headers, &HashMap::new()).unwrap_err();
        assert!(matches!(err, TransportError::InvalidRequest(_)));
    }
}
