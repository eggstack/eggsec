use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use super::waf_patterns::{get_common_waf_response_patterns, get_waf_signatures};
use crate::constants::waf;
use crate::utils::create_insecure_client_with_options;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WafDetectionResult {
    pub waf_name: Option<String>,
    pub confidence: u8,
    pub matched_headers: Vec<String>,
    pub matched_cookies: Vec<String>,
    pub matched_patterns: Vec<String>,
    pub server_header: Option<String>,
    pub status_code: u16,
}

struct WafSignatureLower {
    headers: Vec<String>,
    cookies: Vec<String>,
    body_patterns: Vec<String>,
}

pub struct WafDetector {
    client: reqwest::Client,
    signatures: std::collections::HashMap<String, crate::waf::waf_patterns::WafSignature>,
    signatures_lower: std::collections::HashMap<String, WafSignatureLower>,
}

impl WafDetector {
    pub fn new() -> Result<Self> {
        let ua = crate::waf::bypass::headers::get_random_ua().to_string();
        let client = create_insecure_client_with_options(15, |builder| {
            builder
                .redirect(reqwest::redirect::Policy::limited(5))
                .user_agent(ua)
        })?;

        let signatures = get_waf_signatures();
        let signatures_lower = signatures
            .iter()
            .map(|(key, sig)| {
                (
                    key.clone(),
                    WafSignatureLower {
                        headers: sig.headers.iter().map(|h| h.to_lowercase()).collect(),
                        cookies: sig.cookies.iter().map(|c| c.to_lowercase()).collect(),
                        body_patterns: sig.body_patterns.iter().map(|p| p.to_lowercase()).collect(),
                    },
                )
            })
            .collect();

        Ok(Self {
            client,
            signatures,
            signatures_lower,
        })
    }

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

        for (sig_key, signature) in self.signatures.iter() {
            let sig_lower = &self.signatures_lower[sig_key];
            let mut score = 0u8;
            let mut sig_matched_headers = Vec::new();
            let mut sig_matched_cookies = Vec::new();
            let mut sig_matched_patterns = Vec::new();

            for header_pattern_lower in &sig_lower.headers {
                for (header_name, header_value) in headers.iter() {
                    let name_lower = header_name.as_str().to_lowercase();
                    let value_lower = header_value.to_str().unwrap_or("").to_lowercase();

                    if name_lower.contains(header_pattern_lower.as_str())
                        || value_lower.contains(header_pattern_lower.as_str())
                    {
                        score += waf::HEADER_MATCH_SCORE;
                        sig_matched_headers.push(format!(
                            "{}: {}",
                            header_name,
                            header_value.to_str().unwrap_or("")
                        ));
                    }
                }
            }

            for cookie_pattern_lower in &sig_lower.cookies {
                if let Some(cookie_header) = headers.get("set-cookie") {
                    if let Ok(cookie_str) = cookie_header.to_str() {
                        let cookie_lower = cookie_str.to_lowercase();
                        if cookie_lower.contains(cookie_pattern_lower.as_str()) {
                            score += waf::COOKIE_MATCH_SCORE;
                            sig_matched_cookies.push(signature.cookies[sig_lower.cookies.iter().position(|c| c == cookie_pattern_lower).unwrap_or(0)].clone());
                        }
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

    pub async fn compare_responses(
        &self,
        url: &str,
        normal_req: &str,
        malicious_req: &str,
    ) -> Result<ResponseDiff> {
        let ua = crate::waf::bypass::headers::get_random_ua().to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(ua)
            .build()?;

        let normal_response = client.get(url).query(&[("q", normal_req)]).send().await?;

        let malicious_response = client
            .get(url)
            .query(&[("q", malicious_req)])
            .send()
            .await?;

        let normal_status = normal_response.status().as_u16();
        let normal_headers: std::collections::HashMap<String, String> = normal_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let normal_body = normal_response.text().await.unwrap_or_default();
        let normal_length = normal_body.len();

        let malicious_status = malicious_response.status().as_u16();
        let malicious_headers: std::collections::HashMap<String, String> = malicious_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let malicious_body = malicious_response.text().await.unwrap_or_default();
        let malicious_length = malicious_body.len();

        let header_diffs: Vec<String> = normal_headers
            .keys()
            .filter(|k| malicious_headers.get(*k) != normal_headers.get(*k))
            .map(|k| {
                format!(
                    "{}: {} -> {}",
                    k,
                    normal_headers.get(k).unwrap_or(&"".to_string()),
                    malicious_headers.get(k).unwrap_or(&"".to_string())
                )
            })
            .collect();

        Ok(ResponseDiff {
            normal_status,
            normal_length,
            malicious_status,
            malicious_length,
            normal_headers: Some(normal_headers),
            malicious_headers: Some(malicious_headers),
            header_diffs,
            body_diffs: if normal_body != malicious_body {
                Some(true)
            } else {
                None
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDiff {
    pub normal_status: u16,
    pub normal_length: usize,
    pub malicious_status: u16,
    pub malicious_length: usize,
    pub normal_headers: Option<std::collections::HashMap<String, String>>,
    pub malicious_headers: Option<std::collections::HashMap<String, String>>,
    pub header_diffs: Vec<String>,
    pub body_diffs: Option<bool>,
}

impl ResponseDiff {
    pub fn is_waf_blocked(&self) -> bool {
        let status_blocked = self.malicious_status != self.normal_status
            && (self.malicious_status == 403
                || self.malicious_status == 406
                || self.malicious_status == 405);

        let length_blocked = self.normal_length.saturating_sub(self.malicious_length) > waf::LENGTH_DIFF_THRESHOLD;

        let header_blocked = self.header_diffs.iter().any(|h| {
            h.to_lowercase().contains("waf")
                || h.to_lowercase().contains("firewall")
                || h.to_lowercase().contains("blocked")
                || h.to_lowercase().contains("attack")
        });

        status_blocked || length_blocked || header_blocked
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_detector() -> WafDetector {
        WafDetector {
            client: reqwest::Client::new(),
            signatures: std::collections::HashMap::new(),
            signatures_lower: std::collections::HashMap::new(),
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
        assert_eq!(
            detector.normalize_url("example.com"),
            "https://example.com"
        );
    }

    #[test]
    fn test_normalize_url_trims_whitespace() {
        let detector = test_detector();
        assert_eq!(
            detector.normalize_url("  example.com  "),
            "https://example.com"
        );
    }

    #[test]
    fn test_response_diff_is_waf_blocked_by_status() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 403,
            malicious_length: 4900,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec![],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_is_waf_blocked_by_406() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 406,
            malicious_length: 4900,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec![],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_is_waf_blocked_by_405() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 405,
            malicious_length: 4900,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec![],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_not_blocked_same_status() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 4900,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec![],
            body_diffs: None,
        };
        assert!(!diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_blocked_by_length() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 4800,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec![],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_not_blocked_small_length_diff() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 4950,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec![],
            body_diffs: None,
        };
        assert!(!diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_blocked_by_waf_header() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 5000,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["x-waf-blocked: true".to_string()],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_blocked_by_firewall_header() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 5000,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["X-Firewall-Action: deny".to_string()],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_blocked_by_blocked_header() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 5000,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["x-blocked-request: yes".to_string()],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_blocked_by_attack_header() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 5000,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["x-attack-detected: sql-injection".to_string()],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_not_blocked_irrelevant_header() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 5000,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["x-request-id: abc123".to_string()],
            body_diffs: None,
        };
        assert!(!diff.is_waf_blocked());
    }

    #[test]
    fn test_response_diff_header_case_insensitive() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 200,
            malicious_length: 5000,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["X-WAF-Status: Active".to_string()],
            body_diffs: None,
        };
        assert!(diff.is_waf_blocked());
    }

    #[test]
    fn test_waf_detection_result_serialization() {
        let result = WafDetectionResult {
            waf_name: Some("Cloudflare".to_string()),
            confidence: 75,
            matched_headers: vec!["cf-ray: abc123".to_string()],
            matched_cookies: vec!["__cfduid".to_string()],
            matched_patterns: vec![],
            server_header: Some("cloudflare".to_string()),
            status_code: 403,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: WafDetectionResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.waf_name, Some("Cloudflare".to_string()));
        assert_eq!(deserialized.confidence, 75);
        assert_eq!(deserialized.status_code, 403);
    }

    #[test]
    fn test_response_diff_serialization() {
        let diff = ResponseDiff {
            normal_status: 200,
            normal_length: 5000,
            malicious_status: 403,
            malicious_length: 100,
            normal_headers: None,
            malicious_headers: None,
            header_diffs: vec!["x-waf: blocked".to_string()],
            body_diffs: Some(true),
        };
        let json = serde_json::to_string(&diff).unwrap();
        let deserialized: ResponseDiff = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.normal_status, 200);
        assert_eq!(deserialized.malicious_status, 403);
        assert!(deserialized.body_diffs.is_some());
    }
}
