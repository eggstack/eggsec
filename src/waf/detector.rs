use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::utils::create_insecure_client_with_options;
use super::waf_patterns::{get_waf_signatures, get_common_waf_response_patterns};

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

pub struct WafDetector {
    client: reqwest::Client,
    signatures: std::collections::HashMap<String, crate::waf::waf_patterns::WafSignature>,
}

impl WafDetector {
    pub fn new() -> Result<Self> {
        let client = create_insecure_client_with_options(15, |builder| {
            builder
                .redirect(reqwest::redirect::Policy::limited(5))
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        })?;

        Ok(Self {
            client,
            signatures: get_waf_signatures(),
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

        let server_header = headers.get("server")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        for (_key, signature) in &self.signatures {
            let mut score = 0u8;
            let mut sig_matched_headers = Vec::new();
            let mut sig_matched_cookies = Vec::new();
            let mut sig_matched_patterns = Vec::new();

            for header_pattern in &signature.headers {
                let header_pattern_lower = header_pattern.to_lowercase();
                for (header_name, header_value) in headers.iter() {
                    let name_lower = header_name.as_str().to_lowercase();
                    let value_lower = header_value.to_str().unwrap_or("").to_lowercase();
                    
                    if name_lower.contains(&header_pattern_lower) || value_lower.contains(&header_pattern_lower) {
                        score += 25;
                        sig_matched_headers.push(format!("{}: {}", header_name, header_value.to_str().unwrap_or("")));
                    }
                }
            }

            for cookie_pattern in &signature.cookies {
                if let Some(cookie_header) = headers.get("set-cookie") {
                    if let Ok(cookie_str) = cookie_header.to_str() {
                        let cookie_lower = cookie_str.to_lowercase();
                        if cookie_lower.contains(&cookie_pattern.to_lowercase()) {
                            score += 20;
                            sig_matched_cookies.push(cookie_pattern.clone());
                        }
                    }
                }
            }

            for body_pattern in &signature.body_patterns {
                let pattern_lower = body_pattern.to_lowercase();
                if body_lower.contains(&pattern_lower) {
                    score += 15;
                    sig_matched_patterns.push(body_pattern.clone());
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
            }
        }

        if best_match.is_none() {
            for pattern in get_common_waf_response_patterns() {
                if body_lower.contains(pattern) {
                    matched_patterns.push(pattern.to_string());
                    if best_match.is_none() {
                        best_match = Some(("Unknown WAF".to_string(), 30));
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
        
        let response = self.client
            .get(&test_url)
            .send()
            .await?;

        let status = response.status().as_u16();
        let body = response.text().await.unwrap_or_default().to_lowercase();

        let blocked_codes = [403, 406, 429, 503];
        if blocked_codes.contains(&status) {
            return Ok(true);
        }

        let block_patterns = ["blocked", "denied", "forbidden", "waf", "firewall"];
        for pattern in block_patterns {
            if body.contains(pattern) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    pub async fn compare_responses(&self, url: &str, normal_req: &str, malicious_req: &str) -> Result<ResponseDiff> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
            .build()?;

        let normal_response = client
            .get(url)
            .query(&[("q", normal_req)])
            .send()
            .await?;

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
            .filter(|k| {
                malicious_headers.get(*k) != normal_headers.get(*k)
            })
            .map(|k| {
                format!("{}: {} -> {}", k, normal_headers.get(k).unwrap_or(&"".to_string()), malicious_headers.get(k).unwrap_or(&"".to_string()))
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
            body_diffs: if normal_body != malicious_body { Some(true) } else { None },
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
            && (self.malicious_status == 403 || self.malicious_status == 406 || self.malicious_status == 405);
        
        let length_blocked = self.normal_length.saturating_sub(self.malicious_length) > 100;
        
        let header_blocked = self.header_diffs.iter().any(|h| {
            h.to_lowercase().contains("waf") || 
            h.to_lowercase().contains("firewall") ||
            h.to_lowercase().contains("blocked") ||
            h.to_lowercase().contains("attack")
        });

        status_blocked || length_blocked || header_blocked
    }
}
