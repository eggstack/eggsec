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

pub async fn scan_dom_xss(
    tab: &headless_chrome::Tab,
    config: &BrowserConfig,
) -> Result<Vec<DomXssFinding>> {
    let target = tab.get_url();

    let js_script = r#"
        (function() {
            const sources = [
                { name: 'location.hash', get: () => location.hash },
                { name: 'location.search', get: () => location.search },
                { name: 'document.cookie', get: () => document.cookie },
                { name: 'document.referrer', get: () => document.referrer },
                { name: 'localStorage', get: () => localStorage.getItem('test') },
                { name: 'sessionStorage', get: () => sessionStorage.getItem('test') }
            ];

            const sinks = [
                { name: 'innerHTML', check: (val) => { try { let d = document.createElement('div'); d.innerHTML = val; return true; } catch(e) { return false; } } },
                { name: 'outerHTML', check: (val) => { try { let d = document.createElement('div'); d.outerHTML = val; return true; } catch(e) { return false; } } },
                { name: 'document.write', check: () => true },
                { name: 'eval', check: (val) => { try { eval(val); return false; } catch(e) { return true; } } },
                { name: 'setTimeout', check: () => true },
                { name: 'setInterval', check: () => true },
                { name: 'Function', check: (val) => { try { new Function(val); return true; } catch(e) { return false; } } }
            ];

            const findings = [];
            const testPayload = $payload$;

            for (const source of sources) {
                try {
                    const sourceValue = source.get();
                    if (!sourceValue || sourceValue.length === 0) continue;

                    for (const sink of sinks) {
                        try {
                            if (sink.check && sink.check(testPayload)) {
                                findings.push({
                                    source: source.name,
                                    sink: sink.name,
                                    evidence: `Source: ${source.name}, Sink: ${sink.name}`
                                });
                            }
                        } catch(e) {}
                    }
                } catch(e) {}
            }

            return findings;
        })()
    "#.replace("$payload$", &config.xss_payload);

    let result = tab.evaluate(&js_script, true)?;

    let findings_list: Vec<HashMap<String, String>> = result
        .value
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut findings = Vec::new();

    for item in findings_list {
        let source = item.get("source").cloned().unwrap_or_default();
        let sink = item.get("sink").cloned().unwrap_or_default();
        let evidence = item.get("evidence").cloned().unwrap_or_default();

        let (severity, cvss_score) = calculate_severity(&source, &sink);

        findings.push(DomXssFinding {
            id: format!("xss-{}", &uuid::Uuid::new_v4().to_string()[..8]),
            source: source.clone(),
            sink: sink.clone(),
            location: format!("{} (via browser)", target),
            severity,
            description: format!("DOM XSS: {} to {}", source, sink),
            evidence,
            remediation: get_remediation(&source, &sink),
            cvss_score: Some(cvss_score),
        });
    }

    Ok(findings)
}

fn calculate_severity(source: &str, sink: &str) -> (Severity, f32) {
    let base_score: f32 = match sink {
        "eval" => 9.0,
        "innerHTML" | "outerHTML" => 7.5,
        "document.write" => 8.0,
        "setTimeout" | "setInterval" => 6.5,
        "Function" => 8.5,
        _ => 5.0,
    };

    let modifier: f32 = match source {
        "location.hash" | "location.search" => 1.0,
        "document.cookie" => 1.2,
        "localStorage" | "sessionStorage" => 0.8,
        _ => 1.0,
    };

    let cvss_score: f32 = (base_score * modifier).min(10.0);

    let severity = match cvss_score as u32 {
        9..=10 => Severity::Critical,
        7..=8 => Severity::High,
        4..=6 => Severity::Medium,
        _ => Severity::Low,
    };

    (severity, cvss_score)
}

fn get_remediation(_source: &str, sink: &str) -> String {
    match sink {
        "innerHTML" | "outerHTML" => {
            "Use textContent instead of innerHTML/outerHTML; sanitize HTML with DOMPurify if needed".to_string()
        },
        "eval" | "Function" => {
            "Avoid eval() and Function constructor; use JSON.parse() for data and DOMPurify for HTML".to_string()
        },
        "document.write" => {
            "Replace document.write() with DOM manipulation methods (createElement, appendChild)".to_string()
        },
        "setTimeout" | "setInterval" => {
            "Avoid passing user input to setTimeout/setInterval; use safe string concatenation".to_string()
        },
        _ => {
            "Implement proper input validation and output encoding".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use headless_chrome::Browser;

    #[tokio::test]
    async fn test_scan_dom_xss() {
        let browser = Browser::default().unwrap();
        let tab = browser.new_tab().unwrap();
        tab.set_default_timeout(std::time::Duration::from_millis(30000));
        tab.navigate_to("http://example.com")
            .unwrap()
            .wait_until_navigated()
            .unwrap();
        let config = BrowserConfig::default();
        let findings = scan_dom_xss(&tab, &config).await.unwrap();
        assert!(findings.is_empty());
    }
}
