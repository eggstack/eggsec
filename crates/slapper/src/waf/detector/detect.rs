use crate::constants::waf;
use crate::error::Result;
use crate::waf::waf_patterns::get_common_waf_response_patterns;

use super::types::WafDetectionResult;
use super::WafDetector;

impl WafDetector {
    pub async fn detect(&self, url: &str) -> Result<WafDetectionResult> {
        let normalized_url = self.normalize_url(url);

        let response = match self.client.get(&normalized_url).send().await {
            Ok(r) => r,
            Err(e) => {
                return Ok(WafDetectionResult {
                    waf_name: None,
                    confidence: 0,
                    matched_headers: vec![],
                    matched_cookies: vec![],
                    matched_patterns: vec![format!("Request failed: {}", e)],
                    server_header: None,
                    status_code: 0,
                });
            }
        };

        let status = response.status().as_u16();
        let headers = response.headers().clone();
        let body = response.text().await.unwrap_or_default();
        let body_lower = body.to_lowercase();

        let mut matched_headers = Vec::new();
        let mut matched_cookies = Vec::new();
        let mut matched_patterns = Vec::new();
        let mut best_match: Option<(String, u8)> = None;

        let server_header = headers
            .get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let headers_lower: Vec<(String, String)> = headers
            .iter()
            .map(|(name, value)| {
                (
                    name.as_str().to_lowercase(),
                    value.to_str().unwrap_or("").to_lowercase(),
                )
            })
            .collect();

        let cookie_header_lower = headers
            .get("set-cookie")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_lowercase());

        for (sig_key, signature) in self.signatures.iter() {
            let sig_lower = &self.signatures_lower[sig_key];
            let mut score = 0u8;
            let mut sig_matched_headers = Vec::new();
            let mut sig_matched_cookies = Vec::new();
            let mut sig_matched_patterns = Vec::new();

            for header_pattern_lower in &sig_lower.headers {
                for (name_lower, value_lower) in &headers_lower {
                    if name_lower.contains(header_pattern_lower.as_str())
                        || value_lower.contains(header_pattern_lower.as_str())
                    {
                        score += waf::HEADER_MATCH_SCORE;
                        sig_matched_headers.push(format!("{}: {}", name_lower, value_lower));
                    }
                }
            }

            for cookie_pattern_lower in &sig_lower.cookies {
                if let Some(ref cookie_lower) = cookie_header_lower {
                    if cookie_lower.contains(cookie_pattern_lower.as_str()) {
                        score += waf::COOKIE_MATCH_SCORE;
                        sig_matched_cookies.push(
                            signature.cookies[sig_lower
                                .cookies
                                .iter()
                                .position(|c| c == cookie_pattern_lower)
                                .unwrap_or(0)]
                            .clone(),
                        );
                    }
                }
            }

            for (i, body_pattern_lower) in sig_lower.body_patterns.iter().enumerate() {
                if body_lower.contains(body_pattern_lower.as_str()) {
                    score += waf::BODY_MATCH_SCORE;
                    sig_matched_patterns.push(signature.body_patterns[i].clone());
                }
            }

            if score > 0 {
                if let Some((_, best_score)) = &best_match {
                    if score > *best_score {
                        best_match = Some((signature.name.clone(), score));
                        matched_headers = sig_matched_headers;
                        matched_cookies = sig_matched_cookies;
                        matched_patterns = sig_matched_patterns;
                    }
                } else {
                    best_match = Some((signature.name.clone(), score));
                    matched_headers = sig_matched_headers;
                    matched_cookies = sig_matched_cookies;
                    matched_patterns = sig_matched_patterns;
                }

                if score >= waf::HIGH_CONFIDENCE_EXIT {
                    break;
                }
            }
        }

        if best_match.is_none() {
            for pattern in get_common_waf_response_patterns() {
                if body_lower.contains(pattern) {
                    matched_patterns.push(pattern.to_string());
                    if best_match.is_none() {
                        best_match = Some(("Unknown WAF".to_string(), waf::UNKNOWN_WAF_CONFIDENCE));
                    }
                }
            }
        }

        let (waf_name, confidence) = match best_match {
            Some((name, score)) => {
                let conf = score.min(100);
                (Some(name), conf)
            }
            None => (None, 0),
        };

        Ok(WafDetectionResult {
            waf_name,
            confidence,
            matched_headers,
            matched_cookies,
            matched_patterns,
            server_header,
            status_code: status,
        })
    }

    fn normalize_url(&self, url: &str) -> String {
        let url = url.trim();
        if url.starts_with("http://") || url.starts_with("https://") {
            url.to_string()
        } else {
            format!("https://{}", url)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rustc_hash::FxHashMap;

    fn test_detector() -> WafDetector {
        WafDetector {
            client: crate::utils::get_shared_http_client(),
            signatures: FxHashMap::default(),
            signatures_lower: FxHashMap::default(),
        }
    }

    #[test]
    fn test_normalize_url_with_https() {
        let detector = test_detector();
        assert_eq!(
            detector.normalize_url("https://example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_normalize_url_with_http() {
        let detector = test_detector();
        assert_eq!(
            detector.normalize_url("http://example.com"),
            "http://example.com"
        );
    }

    #[test]
    fn test_normalize_url_without_scheme() {
        let detector = test_detector();
        assert_eq!(detector.normalize_url("example.com"), "https://example.com");
    }

    #[test]
    fn test_normalize_url_trims_whitespace() {
        let detector = test_detector();
        assert_eq!(
            detector.normalize_url("  example.com  "),
            "https://example.com"
        );
    }
}
