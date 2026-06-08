use crate::constants::waf;
use crate::error::Result;
use std::time::Duration;
use tokio::time::timeout;

use super::WafDetector;

impl WafDetector {
    pub async fn check_waf_block(&self, url: &str, test_payload: &str) -> Result<bool> {
        let test_url = format!("{}?test={}", url, urlencoding::encode(test_payload));

        let response = match timeout(
            Duration::from_secs(waf::SMUGGLING_TIMEOUT_SECS),
            self.client.get(&test_url).send(),
        )
        .await
        {
            Ok(Ok(r)) => r,
            Ok(Err(e)) => {
                tracing::warn!("WAF block check request failed for {}: {}", url, e);
                return Ok(false);
            }
            Err(_) => {
                tracing::warn!("WAF block check request timed out for {}", url);
                return Ok(false);
            }
        };

        let status = response.status().as_u16();
        let body = match response.text().await {
            Ok(text) => text.to_lowercase(),
            Err(e) => {
                tracing::debug!("Failed to read response body in WAF block check: {}", e);
                String::new()
            }
        };

        let blocked_codes = waf::BLOCKED_STATUS_CODES;
        if blocked_codes.contains(&status) {
            return Ok(true);
        }

        let block_patterns = waf::BLOCKED_PATTERNS;
        for pattern in block_patterns {
            if body.contains(pattern) {
                return Ok(true);
            }
        }

        Ok(false)
    }
}
