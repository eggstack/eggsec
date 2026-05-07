//! Template matching engine
//!
//! Executes match conditions from vulnerability templates against
//! HTTP responses, DNS results, and other data sources.

use super::models::{HttpMatcher, Matcher, SearchPattern, VulnerabilityTemplate};
use crate::error::Result;
use crate::types::Severity;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct MatchResult {
    pub template_id: String,
    pub matched: bool,
    pub matched_by: String,
    pub extracted_values: Vec<String>,
    pub severity: Severity,
}

#[derive(Debug)]
pub struct HttpResponseData {
    pub path: String,
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

pub struct TemplateMatcher {
    interactsh_urls: Vec<String>,
}

impl TemplateMatcher {
    pub fn new() -> Self {
        Self {
            interactsh_urls: Vec::new(),
        }
    }

    pub fn set_interactsh_urls(&mut self, urls: Vec<String>) {
        self.interactsh_urls = urls;
    }

    pub fn first_interactsh_url(&self) -> Option<&str> {
        self.interactsh_urls.first().map(String::as_str)
    }

    pub async fn match_template(
        &self,
        template: &VulnerabilityTemplate,
        response: Option<&HttpResponseData>,
        dns_data: Option<&DnsResponse>,
    ) -> Result<MatchResult> {
        for matcher in &template.matchers {
            let matched = match matcher {
                Matcher::Http(http) => {
                    if let Some(resp) = response {
                        self.match_http(http, resp)?
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

    fn match_http(&self, matcher: &HttpMatcher, response: &HttpResponseData) -> Result<bool> {
        if let Some(ref path) = matcher.path {
            if path != &response.path && path != "*" {
                return Ok(false);
            }
        }

        let _ = &matcher.method;

        for (header, value) in &matcher.headers {
            let header_key = header.to_ascii_lowercase();
            let resp_header = response.headers.get(&header_key).map(String::as_str);

            if resp_header
                .map(|v| v.contains(value.as_str()))
                .unwrap_or(false)
            {
                if !value.contains("{{interactsh-url}}") {
                    return Ok(true);
                }
            }
        }

        let status = response.status;
        let body = &response.body;

        if matcher.status_codes.contains(&status) {
            return Ok(true);
        }

        if !matcher.search.is_empty() {
            for search in &matcher.search {
                if self.search_pattern(body, search) {
                    return Ok(true);
                }
            }
        }

        if let Some(ref interactsh) = matcher.interactsh {
            if interactsh.enabled && !self.interactsh_urls.is_empty() {
                for url in &self.interactsh_urls {
                    if body.contains(url) {
                        return Ok(true);
                    }
                }
            }
        }

        Ok(false)
    }

    pub(crate) fn match_dns(
        &self,
        matcher: &super::models::DnsMatcher,
        response: &DnsResponse,
    ) -> Result<bool> {
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
            super::models::MatchMode::Regex => regex::RegexBuilder::new(&search.pattern)
                .size_limit(100_000)
                .build()
                .map(|re| re.is_match(text))
                .unwrap_or(false),
            super::models::MatchMode::Binary => {
                let decoded: Vec<u8> = if search.encoding == "base64" {
                    base64::Engine::decode(
                        &base64::engine::general_purpose::STANDARD,
                        &search.pattern,
                    )
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
    use crate::scanner::templates::models::{
        DnsMatcher, HttpMatcher, InteractshConfig, MatchMode, SearchPattern, TemplateInfo,
        VulnerabilityTemplate,
    };
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;

    async fn make_test_response(body: &str) -> HttpResponseData {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let body_owned = body.to_string();

        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await;
            let response = format!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body_owned.len(),
                body_owned
            );
            let _ = socket.write_all(response.as_bytes()).await;
        });

        let url = format!("http://{}", addr);
        let response = reqwest::get(url).await.unwrap();
        let status = response.status().as_u16();
        let path = response.url().path().to_string();
        let mut headers = HashMap::new();
        for (key, value) in response.headers() {
            headers.insert(
                key.as_str().to_ascii_lowercase(),
                value.to_str().unwrap_or_default().to_string(),
            );
        }
        let body = response.text().await.unwrap_or_default();

        HttpResponseData {
            path,
            status,
            headers,
            body,
        }
    }

    #[test]
    fn test_word_matching() {
        let matcher = TemplateMatcher::new();
        let search = SearchPattern {
            pattern: "vulnerable".to_string(),
            mode: MatchMode::Word,
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
            mode: MatchMode::Regex,
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
                mode: MatchMode::Word,
                encoding: String::new(),
            }],
        };

        let response = DnsResponse {
            query_type: "A".to_string(),
            answer: "192.0.2.1 resolves to evil.com".to_string(),
        };

        assert!(matcher.match_dns(&dns, &response).unwrap());
    }

    #[tokio::test]
    async fn test_match_http_search_and_interactsh_reads_body_once() {
        let mut matcher = TemplateMatcher::new();
        matcher.set_interactsh_urls(vec!["callback.example".to_string()]);

        let response = make_test_response("body contains callback.example marker").await;

        let template = VulnerabilityTemplate {
            id: "test-template".to_string(),
            info: TemplateInfo {
                name: "Test".to_string(),
                author: "tester".to_string(),
                severity: "medium".to_string(),
                description: String::new(),
                tags: vec![],
                references: vec![],
                remediation: String::new(),
            },
            matchers: vec![Matcher::Http(HttpMatcher {
                path: Some("/".to_string()),
                method: Some("GET".to_string()),
                headers: HashMap::new(),
                body: None,
                search: vec![SearchPattern {
                    pattern: "not-present".to_string(),
                    mode: MatchMode::Word,
                    encoding: String::new(),
                }],
                status_codes: vec![],
                interactsh: Some(InteractshConfig {
                    enabled: true,
                    authorization: None,
                }),
            })],
            requests: vec![],
        };

        let result = matcher
            .match_template(&template, Some(&response), None)
            .await
            .unwrap();
        assert!(result.matched);
    }
}
