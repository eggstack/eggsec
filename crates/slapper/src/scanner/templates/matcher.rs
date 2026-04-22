//! Template matching engine
//!
//! Executes match conditions from vulnerability templates against
//! HTTP responses, DNS results, and other data sources.

use super::models::{HttpMatcher, Matcher, SearchPattern, VulnerabilityTemplate};
use crate::error::Result;
use crate::types::Severity;
use reqwest::Response;
use rustc_hash::FxHashMap;

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub template_id: String,
    pub matched: bool,
    pub matched_by: String,
    pub extracted_values: Vec<String>,
    pub severity: Severity,
}

pub struct TemplateMatcher {
    interactsh_urls: Vec<String>,
    regex_cache: FxHashMap<String, regex::Regex>,
}

impl TemplateMatcher {
    pub fn new() -> Self {
        Self {
            interactsh_urls: Vec::new(),
            regex_cache: FxHashMap::default(),
        }
    }

    pub fn set_interactsh_urls(&mut self, urls: Vec<String>) {
        self.interactsh_urls = urls;
    }

    pub async fn match_template(
        &self,
        template: &VulnerabilityTemplate,
        response: Option<&Response>,
        dns_data: Option<&DnsResponse>,
    ) -> Result<MatchResult> {
        for matcher in &template.matchers {
            let matched = match matcher {
                Matcher::Http(http) => {
                    if let Some(resp) = response {
                        self.match_http(http, resp).await?
                    } else {
                        false
                    }
                }
                Matcher::Dns(dns) => {
                    if let Some(data) = dns_data {
                        self.match_dns(dns, data)?
                    } else {
                        false
                    }
                }
                Matcher::Other => false,
            };

            if matched {
                return Ok(MatchResult {
                    template_id: template.id.clone(),
                    matched: true,
                    matched_by: format!("{:?}", matcher),
                    extracted_values: Vec::new(),
                    severity: template.severity(),
                });
            }
        }

        Ok(MatchResult {
            template_id: template.id.clone(),
            matched: false,
            matched_by: String::new(),
            extracted_values: Vec::new(),
            severity: template.severity(),
        })
    }

    async fn match_http(&self, matcher: &HttpMatcher, response: &Response) -> Result<bool> {
        if let Some(ref path) = matcher.path {
            let resp_path = response.url().path();
            if path != resp_path && path != "*" {
                return Ok(false);
            }
        }

        if let Some(ref method) = matcher.method {
            if response.request().method().as_str() != method {
                return Ok(false);
            }
        }

        for (header, value) in &matcher.headers {
            let resp_header = response
                .headers()
                .get(header)
                .and_then(|v| v.to_str().ok());

            if resp_header.map(|v| v.contains(value)).unwrap_or(false) {
                if !value.contains("{{interactsh-url}}") {
                    return Ok(true);
                }
            }
        }

        if !matcher.search.is_empty() {
            let body = response.text().await?;
            let status = response.status().as_u16();

            if matcher
                .status_codes
                .contains(&status)
            {
                return Ok(true);
            }

            for search in &matcher.search {
                if self.search_pattern(&body, search) {
                    return Ok(true);
                }
            }
        }

        if let Some(ref interactsh) = matcher.interactsh {
            if interactsh.enabled && !self.interactsh_urls.is_empty() {
                let body = response.text().await?;
                for url in &self.interactsh_urls {
                    if body.contains(url) {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    fn match_dns(&self, matcher: &super::models::DnsMatcher, response: &DnsResponse) -> Result<bool> {
        if let Some(ref query_type) = matcher.query_type {
            if &response.query_type != query_type {
                return Ok(false);
            }
        }

        for search in &matcher.search {
            if self.search_pattern(&response.answer, search) {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn search_pattern(&self, text: &str, search: &SearchPattern) -> bool {
        match search.mode {
            super::models::MatchMode::Word => text.contains(&search.pattern),
            super::models::MatchMode::Regex => {
                self.regex_cache
                    .entry(search.pattern.clone())
                    .or_insert_with(|| regex::Regex::new(&search.pattern).unwrap_or_else(|_| regex::Regex::new("").unwrap()))
                    .is_match(text)
            }
            super::models::MatchMode::Binary => {
                let decoded: Vec<u8> = if search.encoding == "base64" {
                    base64::Engine::decode(&base64::engine::general_purpose::STANDARD, &search.pattern)
                        .unwrap_or_else(|_| search.pattern.as_bytes().to_vec())
                } else {
                    search.pattern.as_bytes().to_vec()
                };
                text.as_bytes().windows(decoded.len()).any(|w| w == decoded)
            }
        }
    }
}

impl Default for TemplateMatcher {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct DnsResponse {
    pub query_type: String,
    pub answer: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_word_matching() {
        let matcher = TemplateMatcher::new();
        let search = SearchPattern {
            pattern: "vulnerable".to_string(),
            mode: super::models::MatchMode::Word,
            encoding: String::new(),
        };

        assert!(matcher.search_pattern("This is vulnerable text", &search));
        assert!(!matcher.search_pattern("This is safe text", &search));
    }

    #[test]
    fn test_regex_matching() {
        let matcher = TemplateMatcher::new();
        let search = SearchPattern {
            pattern: r"v\d+\.\d+\.\d+".to_string(),
            mode: super::models::MatchMode::Regex,
            encoding: String::new(),
        };

        assert!(matcher.search_pattern("Version v1.2.3 detected", &search));
        assert!(!matcher.search_pattern("No version here", &search));
    }

    #[test]
    fn test_dns_response_matching() {
        let matcher = TemplateMatcher::new();
        let dns = DnsMatcher {
            query_type: Some("A".to_string()),
            search: vec![SearchPattern {
                pattern: "evil.com".to_string(),
                mode: super::models::MatchMode::Word,
                encoding: String::new(),
            }],
        };

        let response = DnsResponse {
            query_type: "A".to_string(),
            answer: "192.0.2.1 resolves to evil.com".to_string(),
        };

        assert!(matcher.match_dns(&dns, &response).unwrap());
    }
}
