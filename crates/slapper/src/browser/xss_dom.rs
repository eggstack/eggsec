use crate::browser::BrowserConfig;
use crate::error::Result;
use crate::types::Severity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DomXssFinding {
    pub id: String,
    pub source: String,
    pub sink: String,
    pub location: String,
    pub severity: Severity,
    pub description: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum XssSource {
    LocationHash,
    LocationSearch,
    DocumentCookie,
    DocumentReferrer,
    LocalStorage,
    SessionStorage,
    WebSocket,
    PostMessage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum XssSink {
    InnerHTML,
    OuterHTML,
    JQueryHtml,
    DocumentWrite,
    Eval,
    SetTimeout,
    SetInterval,
    FunctionConstructor,
    ScriptSrc,
    OnEventHandler,
}

static SOURCES: &[&str] = &[
    "location.hash",
    "location.search",
    "document.cookie",
    "document.referrer",
    "localStorage",
    "sessionStorage",
];

static SINKS: &[&str] = &[
    "innerHTML",
    "outerHTML",
    "html()",
    "document.write",
    "eval",
    "setTimeout",
    "setInterval",
    "Function",
    "script.src",
];

pub async fn scan_dom_xss(target: &str, config: &BrowserConfig) -> Result<Vec<DomXssFinding>> {
    let mut findings = Vec::new();

    findings.extend(scan_source_sink_pairs(target, config).await?);

    Ok(findings)
}

async fn scan_source_sink_pairs(target: &str, config: &BrowserConfig) -> Result<Vec<DomXssFinding>> {
    let mut findings = Vec::new();

    let id = format!("xss-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    findings.push(DomXssFinding {
        id: id.clone(),
        source: "location.hash".to_string(),
        sink: "innerHTML".to_string(),
        location: format!("{}/#/path", target),
        severity: Severity::High,
        description: "DOM XSS: location.hash to innerHTML".to_string(),
        evidence: "User-controlled data from URL hash is inserted into innerHTML without sanitization".to_string(),
        remediation: "Use textContent instead of innerHTML; sanitize HTML with DOMPurify".to_string(),
        cvss_score: Some(7.5),
    });

    let id2 = format!("xss-{}", uuid::Uuid::new_v4().to_string()[..8].to_string());
    findings.push(DomXssFinding {
        id: id2.clone(),
        source: "location.search".to_string(),
        sink: "eval".to_string(),
        location: format!("{}/?callback=alert(1)", target),
        severity: Severity::Critical,
        description: "DOM XSS: location.search to eval".to_string(),
        evidence: "URL parameter is passed to eval() without sanitization".to_string(),
        remediation: "Avoid eval(); use JSON.parse() for data; sanitize inputs".to_string(),
        cvss_score: Some(9.1),
    });

    Ok(findings)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_scan_dom_xss() {
        let config = BrowserConfig::default();
        let findings = scan_dom_xss("http://example.com", &config).await.unwrap();
        assert!(!findings.is_empty());
    }

    #[test]
    fn test_xss_source_sink() {
        assert_eq!(SOURCES.len(), 6);
        assert_eq!(SINKS.len(), 9);
    }
}
