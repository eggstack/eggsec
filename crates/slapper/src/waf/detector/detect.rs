use crate::constants::waf;
use crate::error::Result;
use crate::waf::waf_patterns::get_common_waf_response_patterns;
use ipnetwork::IpNetwork;
use std::net::IpAddr;

use super::types::WafDetectionResult;
use super::WafDetector;

const HEADER_VALUE_MAX_LEN: usize = 256;

impl WafDetector {
    pub async fn detect(&self, url: &str) -> Result<WafDetectionResult> {
        if !self.circuit_breaker.is_available() {
            return Ok(WafDetectionResult {
                waf_name: None,
                confidence: 0,
                request_error: Some("Circuit breaker open".to_string()),
                matched_headers: vec![],
                matched_cookies: vec![],
                matched_patterns: vec![],
                server_header: None,
                status_code: 0,
            });
        }

        let normalized_url = Self::normalize_url_static(url);

        let response = match self.client.get(&normalized_url).send().await {
            Ok(r) => {
                self.circuit_breaker.record_success();
                r
            }
            Err(e) => {
                self.circuit_breaker.record_failure();
                return Ok(WafDetectionResult {
                    waf_name: None,
                    confidence: 0,
                    request_error: Some(e.to_string()),
                    matched_headers: vec![],
                    matched_cookies: vec![],
                    matched_patterns: vec![format!("Request failed: {}", e)],
                    server_header: None,
                    status_code: 0,
                });
            }
        };

        let status = response.status().as_u16();
        let remote_ip = response.remote_addr().map(|addr| addr.ip());
        let headers = response.headers().clone();
        let body = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                tracing::debug!("Failed to read response body in WAF detection: {}", e);
                String::new()
            }
        };
        let body_lower = body.to_lowercase();

        let mut matched_headers = Vec::new();
        let mut matched_cookies = Vec::new();
        let mut matched_patterns = Vec::new();
        let mut best_match: Option<(String, u16)> = None;

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

        let cookie_headers_lower: Vec<String> = headers
            .get_all("set-cookie")
            .iter()
            .filter_map(|v| v.to_str().ok())
            .map(|s| s.to_lowercase())
            .collect();

        for (sig_key, signature) in self.signatures.iter() {
            let sig_lower = &self.signatures_lower[sig_key];
            let mut score = 0u16;
            let mut sig_matched_headers = Vec::new();
            let mut sig_matched_cookies = Vec::new();
            let mut sig_matched_patterns = Vec::new();

            for header_pattern_lower in &sig_lower.headers {
                for (name_lower, value_lower) in &headers_lower {
                    let header_name_match = name_lower == header_pattern_lower.as_str();
                    let header_value_match = value_lower.contains(header_pattern_lower.as_str())
                        && value_lower.len() <= HEADER_VALUE_MAX_LEN;
                    if header_name_match || header_value_match {
                        score = score.saturating_add(waf::HEADER_MATCH_SCORE);
                        sig_matched_headers.push(format!("{}: {}", name_lower, value_lower));
                        break;
                    }
                }
            }

            for (ci, cookie_pattern_lower) in sig_lower.cookies.iter().enumerate() {
                for cookie_header in &cookie_headers_lower {
                    let cookie_name = cookie_header
                        .split(';')
                        .next()
                        .unwrap_or("")
                        .split('=')
                        .next()
                        .unwrap_or("")
                        .trim();
                    if cookie_name == cookie_pattern_lower.as_str() {
                        score = score.saturating_add(waf::COOKIE_MATCH_SCORE);
                        sig_matched_cookies.push(signature.cookies[ci].clone());
                        break;
                    }
                }
            }

            for (i, body_pattern_lower) in sig_lower.body_patterns.iter().enumerate() {
                if body_lower.contains(body_pattern_lower.as_str()) {
                    score = score.saturating_add(waf::BODY_MATCH_SCORE);
                    sig_matched_patterns.push(signature.body_patterns[i].clone());
                }
            }

            if let Some(ip) = remote_ip {
                if apply_remote_ip_match(ip, &signature.ip_ranges, &mut sig_matched_patterns) {
                    score = score.saturating_add(waf::IP_MATCH_SCORE);
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
            let mut weak_hits = 0usize;

            for pattern in get_common_waf_response_patterns() {
                if body_lower.contains(pattern) {
                    matched_patterns.push(pattern.to_string());
                }
            }

            for pattern in waf::WEAK_BLOCK_INDICATOR_PATTERNS {
                if body_lower.contains(pattern) {
                    weak_hits += 1;
                }
            }

            if !matched_patterns.is_empty() || weak_hits >= waf::UNKNOWN_WAF_WEAK_PATTERN_THRESHOLD
            {
                best_match = Some(("Unknown WAF".to_string(), waf::UNKNOWN_WAF_CONFIDENCE));
                if weak_hits > 0 {
                    matched_patterns.push(format!("weak-indicators:{}", weak_hits));
                }
            }
        }

        let (waf_name, confidence) = match best_match {
            Some((name, score)) => {
                let conf = score.min(100) as u8;
                (Some(name), conf)
            }
            None => (None, 0),
        };

        Ok(WafDetectionResult {
            waf_name,
            confidence,
            request_error: None,
            matched_headers,
            matched_cookies,
            matched_patterns,
            server_header,
            status_code: status,
        })
    }

    pub(crate) fn normalize_url_static(url: &str) -> String {
        let trimmed = url.trim();
        if let Ok(parsed) = url::Url::parse(trimmed) {
            parsed.to_string()
        } else if let Ok(parsed) = url::Url::parse(&format!("https://{}", trimmed)) {
            parsed.to_string()
        } else {
            trimmed.to_string()
        }
    }
}

fn remote_ip_in_cidr(ip: IpAddr, cidr: &str) -> bool {
    cidr.parse::<IpNetwork>()
        .map(|network| network.contains(ip))
        .unwrap_or(false)
}

fn apply_remote_ip_match(
    ip: IpAddr,
    ip_ranges: &[String],
    matched_patterns: &mut Vec<String>,
) -> bool {
    if ip_ranges.iter().any(|cidr| remote_ip_in_cidr(ip, cidr)) {
        matched_patterns.push(format!("remote-ip-match:{}", ip));
        return true;
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    #[test]
    fn test_normalize_url_with_https() {
        assert_eq!(
            WafDetector::normalize_url_static("https://example.com"),
            "https://example.com/"
        );
    }

    #[test]
    fn test_normalize_url_with_http() {
        assert_eq!(
            WafDetector::normalize_url_static("http://example.com"),
            "http://example.com/"
        );
    }

    #[test]
    fn test_normalize_url_without_scheme() {
        assert_eq!(
            WafDetector::normalize_url_static("example.com"),
            "https://example.com/"
        );
    }

    #[test]
    fn test_normalize_url_trims_whitespace() {
        assert_eq!(
            WafDetector::normalize_url_static("  example.com  "),
            "https://example.com/"
        );
    }

    #[test]
    fn test_normalize_url_with_path() {
        assert_eq!(
            WafDetector::normalize_url_static("https://example.com/foo/bar"),
            "https://example.com/foo/bar"
        );
    }

    #[test]
    fn test_normalize_url_with_query() {
        let result = WafDetector::normalize_url_static("https://example.com/page?q=test");
        assert!(result.starts_with("https://example.com/page"));
        assert!(result.contains("q=test"));
    }

    #[test]
    fn test_remote_ip_in_cidr_ipv4_match() {
        let ip = IpAddr::V4(Ipv4Addr::new(104, 16, 1, 10));
        assert!(remote_ip_in_cidr(ip, "104.16.0.0/12"));
    }

    #[test]
    fn test_remote_ip_in_cidr_ipv4_miss() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        assert!(!remote_ip_in_cidr(ip, "104.16.0.0/12"));
    }

    #[test]
    fn test_remote_ip_in_cidr_ipv6_match() {
        let ip = IpAddr::V6(Ipv6Addr::LOCALHOST);
        assert!(remote_ip_in_cidr(ip, "::1/128"));
    }

    #[test]
    fn test_remote_ip_in_cidr_invalid_cidr() {
        let ip = IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1));
        assert!(!remote_ip_in_cidr(ip, "not-a-cidr"));
    }

    #[test]
    fn test_apply_remote_ip_match_adds_pattern_on_match() {
        let ip = IpAddr::V4(Ipv4Addr::new(104, 16, 1, 10));
        let ip_ranges = vec!["104.16.0.0/12".to_string()];
        let mut matched = Vec::new();
        let did_match = apply_remote_ip_match(ip, &ip_ranges, &mut matched);
        assert!(did_match);
        assert!(matched.iter().any(|p| p == "remote-ip-match:104.16.1.10"));
    }

    #[test]
    fn test_apply_remote_ip_match_no_pattern_on_miss() {
        let ip = IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8));
        let ip_ranges = vec!["104.16.0.0/12".to_string()];
        let mut matched = Vec::new();
        let did_match = apply_remote_ip_match(ip, &ip_ranges, &mut matched);
        assert!(!did_match);
        assert!(matched.is_empty());
    }
}
