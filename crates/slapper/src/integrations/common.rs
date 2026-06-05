use crate::error::{Result, SlapperError};
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
        Err(SlapperError::Network(format!(
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
            SlapperError::Network(format!("{}: request body not cloneable for retry", provider))
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
                let backoff = if retry_after > 0 {
                    retry_after
                } else {
                    BASE_BACKOFF_MS * 2u64.pow(attempt)
                };
                tracing::warn!(
                    "{}: got {} on attempt {}/{}, retrying in {}ms",
                    provider,
                    status,
                    attempt + 1,
                    MAX_RETRIES,
                    backoff
                );
                tokio::time::sleep(Duration::from_millis(backoff)).await;
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
        Some(status) => Err(SlapperError::Network(format!(
            "{} API error {} after {} retries",
            provider, status, MAX_RETRIES
        ))),
        None => Err(SlapperError::Network(format!(
            "{}: all {} retries failed",
            provider, MAX_RETRIES
        ))),
    }
}
