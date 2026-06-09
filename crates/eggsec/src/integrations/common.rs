use crate::error::{EggsecError, Result};
use std::time::Duration;

const MAX_ERROR_BODY_LEN: usize = 200;
const MAX_RETRIES: u32 = 3;
const BASE_BACKOFF_MS: u64 = 500;

pub(crate) async fn handle_response_error(
    response: reqwest::Response,
    provider: &str,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let truncated = truncate_utf8(&body, MAX_ERROR_BODY_LEN);
        tracing::warn!("{} API error {}: {}", provider, status, truncated);
        Err(EggsecError::Network(format!(
            "{} API error {}",
            provider, status
        )))
    }
}

fn truncate_utf8(s: &str, max_bytes: usize) -> &str {
    if s.len() <= max_bytes {
        return s;
    }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    if end == 0 {
        ""
    } else {
        &s[..end]
    }
}

pub(crate) async fn send_with_retry(
    req: reqwest::RequestBuilder,
    provider: &str,
) -> Result<reqwest::Response> {
    let mut last_status = None;
    for attempt in 0..MAX_RETRIES {
        let req_clone = req.try_clone().ok_or_else(|| {
            EggsecError::Network(format!("{}: request body not cloneable", provider))
        })?;
        match req_clone.send().await {
            Ok(resp) if resp.status().is_success() => return Ok(resp),
            Ok(resp) if resp.status().as_u16() == 429 || resp.status().is_server_error() => {
                let status = resp.status();
                let retry_after = resp
                    .headers()
                    .get("retry-after")
                    .and_then(|v| v.to_str().ok())
                    .and_then(|v| v.parse::<u64>().ok())
                    .unwrap_or(0);
                let backoff_ms = if retry_after > 0 {
                    retry_after.saturating_mul(1000)
                } else {
                    BASE_BACKOFF_MS * 2u64.pow(attempt)
                };
                tracing::warn!(
                    "{}: got {} on attempt {}/{}, retrying in {}ms",
                    provider,
                    status,
                    attempt + 1,
                    MAX_RETRIES,
                    backoff_ms
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
                last_status = Some(status);
            }
            Ok(resp) => {
                return handle_response_error(resp, provider).await;
            }
            Err(e) => {
                let backoff = BASE_BACKOFF_MS * 2u64.pow(attempt);
                tracing::warn!(
                    "{}: request error on attempt {}/{}, retrying in {}ms: {}",
                    provider,
                    attempt + 1,
                    MAX_RETRIES,
                    backoff,
                    e
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
            }
        }
    }
    match last_status {
        Some(status) => Err(EggsecError::Network(format!(
            "{} API error {} after {} retries",
            provider, status, MAX_RETRIES
        ))),
        None => Err(EggsecError::Network(format!(
            "{}: all {} retries failed",
            provider, MAX_RETRIES
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_utf8_short_string() {
        assert_eq!(truncate_utf8("hello", 10), "hello");
    }

    #[test]
    fn test_truncate_utf8_exact_boundary() {
        assert_eq!(truncate_utf8("hello", 5), "hello");
    }

    #[test]
    fn test_truncate_utf8_ascii() {
        assert_eq!(truncate_utf8("hello world", 5), "hello");
    }

    #[test]
    fn test_truncate_utf8_multibyte_boundary_ok() {
        let s = "héllo";
        let truncated = truncate_utf8(s, 3);
        assert!(!truncated.is_empty());
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn test_truncate_utf8_multibyte_boundary_miss() {
        let s = "héllo";
        let truncated = truncate_utf8(s, 4);
        assert!(!truncated.is_empty());
        assert!(truncated.is_char_boundary(truncated.len()));
    }

    #[test]
    fn test_truncate_utf8_zero_max() {
        assert_eq!(truncate_utf8("hello", 0), "");
    }

    #[test]
    fn test_truncate_utf8_empty_string() {
        assert_eq!(truncate_utf8("", 5), "");
    }

    #[test]
    fn test_truncate_utf8_4byte_chars() {
        let s = "\u{1FA63}";
        assert_eq!(truncate_utf8(s, 1), "");
        assert_eq!(truncate_utf8(s, 2), "");
        assert_eq!(truncate_utf8(s, 3), "");
        assert_eq!(truncate_utf8(s, 4), s);
        assert_eq!(truncate_utf8(s, 0), "");
    }
}
