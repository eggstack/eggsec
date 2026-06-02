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

    if config.check_dom_xss {
        let findings = xss_dom::scan_dom_xss(target, &config).await?;
        report.dom_xss = findings;
        report.total_findings += report.dom_xss.len();
    }

    if config.discover_spa_routes {
        let routes = spa_discovery::discover_routes(target, &config).await?;
        report.spa_routes = routes;
    }

    if config.check_client_security {
        let issues = client_checks::check_client_security(target, &config).await?;
        report.client_issues = issues;
        report.total_findings += report.client_issues.len();
    }

    let captured = capture_requests(target, &config).await?;
    report.corpus = captured;
    report.corpus.crawl_duration_ms = (Utc::now() - start_time).num_milliseconds() as u64;
    report.corpus.pages_visited = 1;

    Ok(report)
}

#[cfg(feature = "headless-browser")]
async fn capture_requests(target: &str, config: &BrowserConfig) -> Result<corpus::RequestCorpus> {
    let browser = headless_chrome::Browser::default()?;
    let tab = browser.new_tab()?;

    tab.set_default_timeout(std::time::Duration::from_millis(config.timeout_ms));

    tab.navigate_to(target)?.wait_until_navigated()?;

    let js_script = r#"
        (function() {
            const requests = [];
            const forms = [];
            const wsUrls = [];

            const originalXhrOpen = XMLHttpRequest.prototype.open;
            XMLHttpRequest.prototype.open = function(method, url) {
                try {
                    const parsed = new URL(url, window.location.origin);
                    requests.push({
                        url: parsed.href,
                        method: method.toUpperCase(),
                        type: 'xhr'
                    });
                } catch(e) {}
                return originalXhrOpen.apply(this, arguments);
            };

            const originalFetch = window.fetch;
            window.fetch = function(url, options) {
                try {
                    const parsed = new URL(url, window.location.origin);
                    requests.push({
                        url: parsed.href,
                        method: (options && options.method) || 'GET',
                        type: 'fetch'
                    });
                } catch(e) {}
                return originalFetch.apply(this, arguments);
            };

            const originalWs = WebSocket;
            window.WebSocket = function(url) {
                wsUrls.push(url);
                return new originalWs(url);
            };

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

            return {
                requests: requests,
                forms: forms,
                wsUrls: wsUrls
            };
        })()
    "#;

    tab.evaluate(js_script, true)?;

    tab.wait_until_navigated()?;

    let js_collect = r#"
        (function() {
            return { urls: [window.location.href] };
        })()
    "#;

    let result = tab.evaluate(js_collect, true)?;

    let data: serde_json::Value = result
        .value
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default();

    let urls: Vec<String> = serde_json::from_value(
        data.get("urls")
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![])),
    )
    .unwrap_or_default();

    let mut corpus = corpus::RequestCorpus::new();
    corpus.urls = urls;
    corpus.websocket_urls = Vec::new();

    corpus.forms = vec![];

    corpus.api_endpoints = Vec::new();
    corpus.javascript_urls = Vec::new();
    corpus.graphql_candidates = Vec::new();
    corpus.openapi_links = Vec::new();

    corpus.crawl_duration_ms = 0;
    corpus.pages_visited = 0;

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
