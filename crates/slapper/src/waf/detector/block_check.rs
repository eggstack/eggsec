use crate::error::Result;
use crate::constants::waf;

use super::WafDetector;

impl WafDetector {
    pub async fn check_waf_block(&self, url: &str, test_payload: &str) -> Result<bool> {
        let test_url = format!("{}?test={}", url, urlencoding::encode(test_payload));

        let response = match self.client.get(&test_url).send().await {
            Ok(r) => r,
            Err(e) => {
                tracing::warn!("WAF block check request failed for {}: {}", url, e);
                return Ok(false);
            }
        };

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default().to_lowercase();

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
