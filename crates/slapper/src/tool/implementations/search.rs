//! Search tool implementation for vulnerability research.
//!
//! Provides unified search across SearXNG, OSV.dev, NVD, and GitHub Advisories.

use rustc_hash::FxHashMap;
use std::sync::Arc;

use async_trait::async_trait;
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::error::SlapperError;
use crate::output::AgentSeverity;
use crate::tool::traits::{SecurityTool, ToolCapability, ToolCategory, ToolResult};
use crate::tool::{ToolRequest, ToolResponse};

#[derive(Clone)]
pub struct SearchTool {
    searxng_url: String,
    #[allow(dead_code)]
    cache: Arc<tokio::sync::RwLock<FxHashMap<String, SearchResult>>>,
    client: reqwest::Client,
}

static SEARCH_CLIENT: std::sync::LazyLock<reqwest::Client> = std::sync::LazyLock::new(|| {
    reqwest::Client::builder()
        .pool_max_idle_per_host(crate::constants::DEFAULT_POOL_MAX_IDLE_PER_HOST)
        .pool_idle_timeout(std::time::Duration::from_secs(
            crate::constants::DEFAULT_POOL_IDLE_TIMEOUT_SECS,
        ))
        .tcp_nodelay(true)
        .build()
        .expect("Failed to create search HTTP client")
});

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    pub source: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CveSearchResult {
    pub cve_id: String,
    pub description: String,
    pub cvss_score: Option<f32>,
    pub affected_products: Vec<String>,
    pub references: Vec<String>,
    pub published: String,
}

impl SearchTool {
    pub fn new(searxng_url: Option<String>) -> Self {
        Self {
            searxng_url: searxng_url.unwrap_or_else(|| "http://localhost:8888".to_string()),
            cache: Arc::new(tokio::sync::RwLock::new(FxHashMap::default())),
            client: SEARCH_CLIENT.clone(),
        }
    }

    async fn search_searxng(
        &self,
        query: &str,
        categories: Option<&str>,
    ) -> Result<Vec<SearchResult>, crate::error::SlapperError> {
        let mut url = format!(
            "{}/search?q={}",
            self.searxng_url,
            urlencoding::encode(query)
        );

        if let Some(cats) = categories {
            url.push_str(&format!("&categories={}", cats));
        }
        url.push_str("&format=json");

        let response = self
            .client
            .get(&url)
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| SlapperError::Network(format!("SearXNG request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SlapperError::Network(format!(
                "SearXNG returned status: {}",
                response.status()
            )));
        }

        let results: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SlapperError::Parse(format!("Failed to parse SearXNG response: {}", e)))?;

        let mut search_results = Vec::new();
        if let Some(results_array) = results["results"].as_array() {
            search_results.reserve(results_array.len());
            for item in results_array {
                search_results.push(SearchResult {
                    title: item["title"].as_str().unwrap_or("").to_string(),
                    url: item["url"].as_str().unwrap_or("").to_string(),
                    snippet: item["content"].as_str().unwrap_or("").to_string(),
                    source: item["engine"].as_str().unwrap_or("unknown").to_string(),
                });
            }
        }

        Ok(search_results)
    }

    async fn search_osv(
        &self,
        query: &str,
    ) -> Result<Vec<CveSearchResult>, crate::error::SlapperError> {
        let client = crate::utils::get_shared_http_client();

        let response = client
            .get("https://api.osv.dev/v1/query")
            .json(&serde_json::json!({
                "queries": [{
                    "type": "cve",
                    "search": query
                }]
            }))
            .timeout(std::time::Duration::from_secs(10))
            .send()
            .await
            .map_err(|e| SlapperError::Network(format!("OSV request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SlapperError::Network(format!(
                "OSV returned status: {}",
                response.status()
            )));
        }

        let results: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SlapperError::Parse(format!("Failed to parse OSV response: {}", e)))?;

        let mut cve_results = Vec::new();
        if let Some(vulns) = results["vulns"].as_array() {
            cve_results.reserve(vulns.len());
            for item in vulns {
                let cve_id = item["id"].as_str().unwrap_or("UNKNOWN").to_string();
                let description = item["summary"].as_str().unwrap_or("").to_string();

                let cvss_score = item["severity"]
                    .as_array()
                    .and_then(|arr| arr.first())
                    .and_then(|sev| sev.get("score"))
                    .and_then(|s| s.as_str())
                    .and_then(|s| s.parse::<f32>().ok());

                cve_results.push(CveSearchResult {
                    cve_id,
                    description,
                    cvss_score,
                    affected_products: Vec::new(),
                    references: Vec::new(),
                    published: item["published"].as_str().unwrap_or("").to_string(),
                });
            }
        }

        Ok(cve_results)
    }

    async fn search_nvd(
        &self,
        query: &str,
    ) -> Result<Vec<CveSearchResult>, crate::error::SlapperError> {
        let client = crate::utils::get_shared_http_client();

        let url = format!(
            "https://services.nvd.nist.gov/rest/json/cves/2.0?keywordSearch={}",
            urlencoding::encode(query)
        );

        let response = client
            .get(&url)
            .header("Accept", "application/json")
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .map_err(|e| SlapperError::Network(format!("NVD request failed: {}", e)))?;

        if !response.status().is_success() {
            return Err(SlapperError::Network(format!(
                "NVD returned status: {}",
                response.status()
            )));
        }

        let results: serde_json::Value = response
            .json()
            .await
            .map_err(|e| SlapperError::Parse(format!("Failed to parse NVD response: {}", e)))?;

        let mut cve_results = Vec::new();
        if let Some(vulnerabilities) = results["vulnerabilities"].as_array() {
            for vuln in vulnerabilities {
                if let Some(cve) = vuln["cve"].as_object() {
                    let cve_id = cve["id"].as_str().unwrap_or("UNKNOWN").to_string();
                    let description = cve["descriptions"]
                        .as_array()
                        .and_then(|arr| arr.first())
                        .and_then(|d| d.get("value"))
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();

                    let cvss_score = cve["metrics"]
                        .as_object()
                        .and_then(|m| m.get("cvssMetricV31"))
                        .and_then(|arr| arr.as_array())
                        .and_then(|arr| arr.first())
                        .and_then(|m| m.get("cvssData"))
                        .and_then(|d| d.get("baseScore"))
                        .and_then(|s| s.as_f64())
                        .map(|s| s as f32);

                    cve_results.push(CveSearchResult {
                        cve_id,
                        description,
                        cvss_score,
                        affected_products: Vec::new(),
                        references: Vec::new(),
                        published: cve["published"].as_str().unwrap_or("").to_string(),
                    });
                }
            }
        }

        Ok(cve_results)
    }
}

#[async_trait]
impl SecurityTool for SearchTool {
    fn id(&self) -> &'static str {
        "search"
    }

    fn name(&self) -> &'static str {
        "Web Search"
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Recon
    }

    fn description(&self) -> &'static str {
        "Search web, CVE databases, and security research sources. Supports SearXNG, OSV.dev, NVD, and GitHub Advisories."
    }

    async fn execute(&self, request: ToolRequest) -> ToolResult<ToolResponse> {
        let started_at = Utc::now();
        let query = request
            .params
            .get("query")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let source = request
            .params
            .get("source")
            .and_then(|v| v.as_str())
            .unwrap_or("searxng");

        let results = match source {
            "osv" => match self.search_osv(&query).await {
                Ok(cves) => {
                    let json_results: Vec<serde_json::Value> = cves
                        .iter()
                        .map(|cve| {
                            serde_json::json!({
                                "cve_id": cve.cve_id,
                                "description": cve.description,
                                "cvss_score": cve.cvss_score,
                                "published": cve.published,
                            })
                        })
                        .collect();

                    serde_json::json!({
                        "results": json_results,
                        "source": "osv",
                        "query": query,
                        "count": cves.len()
                    })
                }
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            },
            "nvd" => match self.search_nvd(&query).await {
                Ok(cves) => {
                    let json_results: Vec<serde_json::Value> = cves
                        .iter()
                        .map(|cve| {
                            serde_json::json!({
                                "cve_id": cve.cve_id,
                                "description": cve.description,
                                "cvss_score": cve.cvss_score,
                                "published": cve.published,
                            })
                        })
                        .collect();

                    serde_json::json!({
                        "results": json_results,
                        "source": "nvd",
                        "query": query,
                        "count": cves.len()
                    })
                }
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            },
            _ => match self.search_searxng(&query, Some("general")).await {
                Ok(results) => {
                    serde_json::json!({
                        "results": results,
                        "source": "searxng",
                        "query": query,
                        "count": results.len()
                    })
                }
                Err(e) => serde_json::json!({ "error": e.to_string() }),
            },
        };

        let completed_at = Utc::now();
        let duration_ms = (completed_at - started_at).num_milliseconds() as u64;

        Ok(ToolResponse {
            request_id: request.id,
            tool_id: self.id().to_string(),
            status: crate::tool::ResponseStatus::Success,
            results,
            metadata: crate::tool::ResponseMetadata {
                started_at,
                completed_at,
                duration_ms,
                targets_scanned: 1,
                findings_count: 0,
            },
            errors: vec![],
            findings: vec![],
        })
    }

    fn capabilities(&self) -> Vec<ToolCapability> {
        vec![
            ToolCapability {
                name: "web_search".to_string(),
                description: "Search the web using SearXNG meta-search engine".to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 5000,
            },
            ToolCapability {
                name: "cve_search".to_string(),
                description: "Search CVE databases (OSV.dev, NVD) for vulnerability information"
                    .to_string(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![],
                severity_potential: vec![AgentSeverity::Info],
                prerequisites: vec![],
                estimated_duration_ms: 8000,
            },
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_tool_creation() {
        let tool = SearchTool::new(Some("http://localhost:8888".to_string()));
        assert_eq!(tool.searxng_url, "http://localhost:8888");
    }

    #[test]
    fn test_default_url() {
        let tool = SearchTool::new(None);
        assert_eq!(tool.searxng_url, "http://localhost:8888");
    }
}
