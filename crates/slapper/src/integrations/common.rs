use crate::error::{Result, SlapperError};

const MAX_ERROR_BODY_LEN: usize = 200;

pub(crate) async fn handle_response_error(
    response: reqwest::Response,
    provider: &str,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        let truncated = if body.len() > MAX_ERROR_BODY_LEN {
            format!("{}...", &body[..MAX_ERROR_BODY_LEN])
        } else {
            body.clone()
        };
        tracing::warn!("{} API error {}: {}", provider, status, truncated);
        Err(SlapperError::Network(format!(
            "{} API error {}",
            provider, status
        )))
    }
}
