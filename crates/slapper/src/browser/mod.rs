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
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BrowserReport {
    pub target: String,
    pub dom_xss: Vec<xss_dom::DomXssFinding>,
    pub spa_routes: Vec<spa_discovery::SpaRoute>,
    pub client_issues: Vec<client_checks::ClientIssue>,
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

    Ok(report)
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
}

impl Default for BrowserConfig {
    fn default() -> Self {
        Self {
            check_dom_xss: true,
            discover_spa_routes: true,
            check_client_security: true,
            crawl_depth: 3,
            timeout_ms: 60000,
        }
    }
}
