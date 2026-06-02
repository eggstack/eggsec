//! Headless browser testing module
//!
//! Provides browser-based security testing including DOM XSS detection, SPA route
//! discovery, and client-side security checks.
//!
//! ## Modules
//!
//! - [`xss_dom`] - DOM XSS detection via source/sink tracing
//! - [`spa_discovery`] - Single Page App route discovery
//! - [`client_checks`] - Client-side security checks

pub mod client_checks;
pub mod corpus;
pub mod spa_discovery;
pub mod xss_dom;

use crate::error::Result;
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserReport {
    pub target: String,
    pub dom_xss: Vec<xss_dom::DomXssFinding>,
    pub spa_routes: Vec<spa_discovery::SpaRoute>,
    pub client_issues: Vec<client_checks::ClientIssue>,
    pub corpus: corpus::RequestCorpus,
    pub total_findings: usize,
}

impl BrowserReport {
    pub fn new(target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..Default::default()
        }
    }
}

#[cfg(feature = "headless-browser")]
pub async fn run_browser_scan(target: &str, config: BrowserConfig) -> Result<BrowserReport> {
    let mut report = BrowserReport::new(target);
    let start_time = Utc::now();

    let browser = headless_chrome::Browser::default()?;
    let tab = browser.new_tab()?;
    tab.set_default_timeout(std::time::Duration::from_millis(config.timeout_ms));
    tab.navigate_to(target)?.wait_until_navigated()?;

    if config.check_dom_xss {
        let findings = xss_dom::scan_dom_xss(&tab, &config).await?;
        report.dom_xss = findings;
        report.total_findings += report.dom_xss.len();
    }

    if config.discover_spa_routes {
        let routes = spa_discovery::discover_routes(&tab, &config).await?;
        report.spa_routes = routes;
    }

    if config.check_client_security {
        let issues = client_checks::check_client_security(&tab, &config).await?;
        report.client_issues = issues;
        report.total_findings += report.client_issues.len();
    }

    let captured = capture_requests(&tab).await?;
    report.corpus = captured;
    report.corpus.crawl_duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;
    report.corpus.pages_visited = 1;

    Ok(report)
}

#[cfg(feature = "headless-browser")]
async fn capture_requests(tab: &headless_chrome::Tab) -> Result<corpus::RequestCorpus> {
    let js_script = r#"
        (function() {
            const forms = [];
            document.querySelectorAll('form').forEach(form => {
                const action = form.getAttribute('action');
                const method = form.getAttribute('method') || 'GET';
                const fields = [];
                form.querySelectorAll('input, textarea, select').forEach(field => {
                    fields.push(field.name || field.id || 'unnamed');
                });
                if (action) {
                    forms.push({
                        action: action,
                        method: method.toUpperCase(),
                        fields: fields
                    });
                }
            });

            const scripts = [];
            document.querySelectorAll('script[src]').forEach(s => {
                scripts.push(s.src);
            });

            const graphqlCandidates = [];
            document.querySelectorAll('script[src]').forEach(s => {
                const text = s.textContent || '';
                if (text.includes('graphql') || text.includes('gql')) {
                    graphqlCandidates.push(s.src);
                }
            });

            return {
                url: window.location.href,
                forms: forms,
                scripts: scripts,
                graphqlCandidates: graphqlCandidates
            };
        })()
    "#;

    let result = tab.evaluate(js_script, true)?;

    let data: serde_json::Value = result
        .value
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let current_url = data
        .get("url")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let forms: Vec<corpus::FormInfo> = data
        .get("forms")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let js_urls: Vec<String> = data
        .get("scripts")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let graphql_candidates: Vec<String> = data
        .get("graphqlCandidates")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut corpus = corpus::RequestCorpus::new();
    if !current_url.is_empty() {
        corpus.urls.push(current_url);
    }
    corpus.forms = forms;
    corpus.javascript_urls = js_urls;
    corpus.graphql_candidates = graphql_candidates;

    Ok(corpus)
}

#[cfg(not(feature = "headless-browser"))]
pub async fn run_browser_scan(_target: &str, _config: BrowserConfig) -> Result<BrowserReport> {
    Err(crate::error::SlapperError::Config(
        "headless-browser feature not enabled".to_string(),
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BrowserConfig {
    pub check_dom_xss: bool,
    pub discover_spa_routes: bool,
    pub check_client_security: bool,
    pub crawl_depth: usize,
    pub timeout_ms: u64,
    pub xss_payload: String,
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            check_dom_xss: true,
            discover_spa_routes: true,
            check_client_security: true,
            crawl_depth: 3,
            timeout_ms: 60000,
            xss_payload: "<img src=x onerror=alert(1)>".to_string(),
        }
    }
}
