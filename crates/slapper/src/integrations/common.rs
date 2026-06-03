use crate::error::{Result, SlapperError};

pub(crate) async fn handle_response_error(
    response: reqwest::Response,
    provider: &str,
) -> Result<reqwest::Response> {
    if response.status().is_success() {
        Ok(response)
    } else {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        tracing::warn!("{} API error {}: {}", provider, status, body);
        Err(SlapperError::Network(format!(
            "{} API error {}: {}",
            provider, status, body
        )))
    }
}
