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

    let escaped_payload = serde_json::to_string(&config.xss_payload)
        .unwrap_or_else(|_| "\"<img src=x onerror=alert(1)>\"".to_string());

    let js_script = format!(
        r#"
        (function() {{
            const sources = [
                {{ name: 'location.hash', get: () => location.hash }},
                {{ name: 'location.search', get: () => location.search }},
                {{ name: 'document.cookie', get: () => document.cookie }},
                {{ name: 'document.referrer', get: () => document.referrer }},
                {{ name: 'localStorage', get: () => localStorage.getItem('test') }},
                {{ name: 'sessionStorage', get: () => sessionStorage.getItem('test') }},
                {{ name: 'WebSocket', get: () => try {{ const ws = new WebSocket('ws://localhost'); return 'ws://localhost'; }} catch(e) {{ return ''; }} }},
                {{ name: 'postMessage', get: () => {{ window.__eggsec_test_msg = ''; window.addEventListener('message', (e) => {{ window.__eggsec_test_msg = e.data; }}); return 'listener_set'; }} }}
            ];

            const sinks = [
                {{ name: 'innerHTML', check: (val) => {{ try {{ let d = document.createElement('div'); d.innerHTML = val; return true; }} catch(e) {{ return false; }} }} }},
                {{ name: 'outerHTML', check: (val) => {{ try {{ let d = document.createElement('div'); d.outerHTML = val; return true; }} catch(e) {{ return false; }} }} }},
                {{ name: 'jQuery.html', check: (val) => {{ try {{ if (typeof jQuery !== 'undefined') {{ jQuery('<div>').html(val); return true; }} return false; }} catch(e) {{ return false; }} }} }},
                {{ name: 'document.write', check: () => true }},
                {{ name: 'eval', check: (val) => {{ try {{ eval(val); return false; }} catch(e) {{ return true; }} }} }},
                {{ name: 'setTimeout', check: () => true }},
                {{ name: 'setInterval', check: () => true }},
                {{ name: 'Function', check: (val) => {{ try {{ new Function(val); return true; }} catch(e) {{ return false; }} }} }},
                {{ name: 'scriptSrc', check: (val) => {{ try {{ let s = document.createElement('script'); s.src = val; return true; }} catch(e) {{ return false; }} }} }},
                {{ name: 'onerror', check: (val) => {{ try {{ let d = document.createElement('img'); d.onerror = val; return true; }} catch(e) {{ return false; }} }} }}
            ];

            const findings = [];
            const testPayload = {payload};

            for (const source of sources) {{
                try {{
                    const sourceValue = source.get();
                    if (!sourceValue || sourceValue.length === 0) continue;

                    for (const sink of sinks) {{
                        try {{
                            if (sink.check && sink.check(testPayload)) {{
                                findings.push({{
                                    source: source.name,
                                    sink: sink.name,
                                    evidence: `Source: ${{source.name}}, Sink: ${{sink.name}}`
                                }});
                            }}
                        }} catch(e) {{}}
                    }}
                }} catch(e) {{}}
            }}

            return findings;
        }})()
    "#,
        payload = escaped_payload
    );

    let result = tab.evaluate(&js_script, true)?;

    let findings_list: Vec<HashMap<String, String>> = match result.value.as_ref() {
        Some(v) => serde_json::from_value(v.clone()).unwrap_or_else(|e| {
            tracing::warn!("Failed to deserialize DOM XSS findings: {}", e);
            Vec::new()
        }),
        None => Vec::new(),
    };

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
        "innerHTML" | "outerHTML" | "jQuery.html" => 7.5,
        "document.write" => 8.0,
        "setTimeout" | "setInterval" => 6.5,
        "Function" => 8.5,
        "scriptSrc" => 7.0,
        "onerror" => 7.0,
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
        "jQuery.html" => {
            "Avoid jQuery .html() with user input; use .text() or sanitize with DOMPurify first".to_string()
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
        "scriptSrc" => {
            "Avoid setting script.src from user input; use a whitelist of allowed script URLs".to_string()
        },
        "onerror" | "onload" | "onclick" => {
            "Avoid assigning user input to on* event handlers; use addEventListener instead".to_string()
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

    #[test]
    fn test_calculate_severity_eval_critical() {
        let (sev, score) = calculate_severity("location.hash", "eval");
        assert_eq!(sev, Severity::Critical);
        assert_eq!(score, 9.0);
    }

    #[test]
    fn test_calculate_severity_innerhtml_high() {
        let (sev, score) = calculate_severity("location.search", "innerHTML");
        assert_eq!(sev, Severity::High);
        assert_eq!(score, 7.5);
    }

    #[test]
    fn test_calculate_severity_jquery_high() {
        let (sev, score) = calculate_severity("location.hash", "jQuery.html");
        assert_eq!(sev, Severity::High);
        assert_eq!(score, 7.5);
    }

    #[test]
    fn test_calculate_severity_cookie_modifier() {
        let (_, score) = calculate_severity("document.cookie", "eval");
        assert_eq!(score, 10.0); // 9.0 * 1.2 = 10.8, capped at 10.0
    }

    #[test]
    fn test_calculate_severity_localstorage_modifier() {
        let (_, score) = calculate_severity("localStorage", "innerHTML");
        assert_eq!(score, 6.0); // 7.5 * 0.8 = 6.0
    }

    #[test]
    fn test_get_remediation_innerhtml() {
        let rem = get_remediation("location.hash", "innerHTML");
        assert!(rem.contains("textContent"));
        assert!(rem.contains("DOMPurify"));
    }

    #[test]
    fn test_get_remediation_jquery() {
        let rem = get_remediation("location.hash", "jQuery.html");
        assert!(rem.contains("jQuery"));
        assert!(rem.contains(".text()"));
    }

    #[test]
    fn test_get_remediation_eval() {
        let rem = get_remediation("location.hash", "eval");
        assert!(rem.contains("eval()"));
        assert!(rem.contains("JSON.parse()"));
    }

    #[test]
    fn test_get_remediation_document_write() {
        let rem = get_remediation("location.hash", "document.write");
        assert!(rem.contains("document.write()"));
    }

    #[test]
    fn test_get_remediation_settimeout() {
        let rem = get_remediation("location.hash", "setTimeout");
        assert!(rem.contains("setTimeout/setInterval"));
    }

    #[test]
    fn test_get_remediation_unknown_sink() {
        let rem = get_remediation("location.hash", "unknown");
        assert!(rem.contains("input validation"));
    }

    #[test]
    fn test_calculate_severity_scriptsrc() {
        let (sev, score) = calculate_severity("location.hash", "scriptSrc");
        assert_eq!(sev, Severity::High);
        assert_eq!(score, 7.0);
    }

    #[test]
    fn test_calculate_severity_onerror() {
        let (sev, score) = calculate_severity("location.search", "onerror");
        assert_eq!(sev, Severity::High);
        assert_eq!(score, 7.0);
    }

    #[test]
    fn test_get_remediation_scriptsrc() {
        let rem = get_remediation("location.hash", "scriptSrc");
        assert!(rem.contains("script.src"));
        assert!(rem.contains("whitelist"));
    }

    #[test]
    fn test_get_remediation_onerror() {
        let rem = get_remediation("location.hash", "onerror");
        assert!(rem.contains("on*"));
        assert!(rem.contains("addEventListener"));
    }
}
