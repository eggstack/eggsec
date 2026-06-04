use crate::browser::BrowserConfig;
use crate::error::Result;
use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientIssue {
    pub id: String,
    pub issue_type: ClientIssueType,
    pub severity: Severity,
    pub location: String,
    pub description: String,
    pub evidence: String,
    pub remediation: String,
    pub cvss_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ClientIssueType {
    LocalStorageSensitive,
    CorsMisconfiguration,
    CSPSourceMap,
    DebugMode,
    SourceMapsExposed,
    CORSWildcard,
    WeakCiphers,
    CertificateIssues,
}

impl std::fmt::Display for ClientIssueType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientIssueType::LocalStorageSensitive => write!(f, "Local Storage Sensitive"),
            ClientIssueType::CorsMisconfiguration => write!(f, "CORS Misconfiguration"),
            ClientIssueType::CSPSourceMap => write!(f, "CSP Source Map"),
            ClientIssueType::DebugMode => write!(f, "Debug Mode"),
            ClientIssueType::SourceMapsExposed => write!(f, "Source Maps Exposed"),
            ClientIssueType::CORSWildcard => write!(f, "CORS Wildcard"),
            ClientIssueType::WeakCiphers => write!(f, "Weak Ciphers"),
            ClientIssueType::CertificateIssues => write!(f, "Certificate Issues"),
        }
    }
}

pub async fn check_client_security(
    tab: &headless_chrome::Tab,
    _config: &BrowserConfig,
) -> Result<Vec<ClientIssue>> {

    let js_script = r#"
        (function() {
            const issues = [];

            if (localStorage.length > 0) {
                const sensitivePatterns = [
                    /token/i, /auth/i, /key/i, /secret/i, /password/i,
                    /credential/i, /session/i, /jwt/i, /bearer/i
                ];

                for (let i = 0; i < localStorage.length; i++) {
                    const key = localStorage.key(i);
                    const value = localStorage.getItem(key);

                    if (sensitivePatterns.some(p => p.test(key) || p.test(value))) {
                        issues.push({
                            type: 'LocalStorageSensitive',
                            location: 'localStorage.' + key,
                            description: 'Sensitive data stored in localStorage',
                            evidence: `Key: ${key}`,
                            severity: 'Medium',
                            cvss: 5.3
                        });
                    }
                }
            }

            const scripts = document.querySelectorAll('script[src]');
            scripts.forEach(script => {
                if (script.src.endsWith('.map')) {
                    issues.push({
                        type: 'SourceMapsExposed',
                        location: script.src,
                        description: 'Source map exposed in production',
                        evidence: `Source map file: ${script.src}`,
                        severity: 'Low',
                        cvss: 2.5
                    });
                }
            });

            const metaTags = document.querySelectorAll('meta');
            metaTags.forEach(meta => {
                if (meta.name === 'debug' || meta.name === 'application-debug') {
                    issues.push({
                        type: 'DebugMode',
                        location: `meta[name="${meta.name}"]`,
                        description: 'Debug mode enabled in production',
                        evidence: `meta name="debug" content="${meta.content}"`,
                        severity: 'Low',
                        cvss: 3.0
                    });
                }
            });

            const inlineScripts = document.querySelectorAll('script:not([src])');
            inlineScripts.forEach(script => {
                const text = script.textContent || '';
                if (text.includes('console.log') && text.includes('debug')) {
                    issues.push({
                        type: 'DebugMode',
                        location: 'Inline script',
                        description: 'Debug code in production',
                        evidence: 'console.log statements found in inline script',
                        severity: 'Low',
                        cvss: 3.0
                    });
                }
            });

            const cspMeta = document.querySelector('meta[http-equiv="Content-Security-Policy"]');
            if (cspMeta) {
                const cspContent = cspMeta.getAttribute('content') || '';
                if (cspContent.includes('unsafe-eval')) {
                    issues.push({
                        type: 'CSPSourceMap',
                        location: 'meta[csp]',
                        description: 'CSP allows unsafe-eval which can enable source map exploitation',
                        evidence: `CSP contains 'unsafe-eval': ${cspContent.substring(0, 100)}`,
                        severity: 'Medium',
                        cvss: 5.3
                    });
                }
            }

            try {
                const testOrigin = 'https://evil attacker.com';
                const corsXhr = new XMLHttpRequest();
                corsXhr.open('GET', window.location.href, false);
                corsXhr.setRequestHeader('Origin', testOrigin);
                try {
                    corsXhr.send();
                    const acao = corsXhr.getResponseHeader('Access-Control-Allow-Origin');
                    if (acao === '*') {
                        issues.push({
                            type: 'CORSWildcard',
                            location: window.location.origin,
                            description: 'CORS policy allows wildcard origins (Access-Control-Allow-Origin: *)',
                            evidence: 'Server responded with Access-Control-Allow-Origin: *',
                            severity: 'Medium',
                            cvss: 5.3
                        });
                    } else if (acao === testOrigin) {
                        issues.push({
                            type: 'CorsMisconfiguration',
                            location: window.location.origin,
                            description: 'CORS policy reflects arbitrary Origin header',
                            evidence: 'Server reflected attacker-controlled Origin in Access-Control-Allow-Origin',
                            severity: 'High',
                            cvss: 7.4
                        });
                    }
                } catch(e) {}
            } catch(e) {}

            return issues;
        })()
    "#;

    let result = tab.evaluate(js_script, true)?;

    let found_issues: Vec<serde_json::Value> = result
        .value
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let mut issues = Vec::new();

    for item in found_issues {
        let issue_type_str = item
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("Unknown");
        let location = item
            .get("location")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let description = item
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let evidence = item
            .get("evidence")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let severity_str = item
            .get("severity")
            .and_then(|v| v.as_str())
            .unwrap_or("Low");
        let cvss = item.get("cvss").and_then(|v| v.as_f64()).unwrap_or(3.0) as f32;

        let issue_type = match issue_type_str {
            "LocalStorageSensitive" => ClientIssueType::LocalStorageSensitive,
            "SourceMapsExposed" => ClientIssueType::SourceMapsExposed,
            "DebugMode" => ClientIssueType::DebugMode,
            "CSPSourceMap" => ClientIssueType::CSPSourceMap,
            "CORSWildcard" => ClientIssueType::CORSWildcard,
            "WeakCiphers" => ClientIssueType::WeakCiphers,
            "CertificateIssues" => ClientIssueType::CertificateIssues,
            _ => continue,
        };

        let severity = match severity_str {
            "Critical" => Severity::Critical,
            "High" => Severity::High,
            "Medium" => Severity::Medium,
            _ => Severity::Low,
        };

        issues.push(ClientIssue {
            id: format!("cs-{}", &uuid::Uuid::new_v4().to_string()[..8]),
            issue_type,
            severity,
            location,
            description,
            evidence,
            remediation: get_remediation(issue_type_str),
            cvss_score: Some(cvss),
        });
    }

    Ok(issues)
}

fn get_remediation(issue_type: &str) -> String {
    match issue_type {
        "LocalStorageSensitive" => {
            "Store sensitive data in sessionStorage or httpOnly cookies; encrypt if needed"
                .to_string()
        }
        "SourceMapsExposed" => "Remove source maps from production build".to_string(),
        "DebugMode" => "Disable debug mode in production".to_string(),
        "CSPSourceMap" => "Remove 'unsafe-eval' and 'unsafe-inline' from CSP; use nonces".to_string(),
        "CORSWildcard" => "Replace wildcard CORS origin with specific allowed origins".to_string(),
        "CorsMisconfiguration" => "Restrict CORS to specific trusted origins; never reflect arbitrary Origin headers".to_string(),
        "WeakCiphers" => "Disable TLS 1.0/1.1 and weak cipher suites; use TLS 1.2+ with strong ciphers".to_string(),
        "CertificateIssues" => "Fix or replace invalid certificates; ensure proper certificate chain".to_string(),
        _ => "Implement proper security controls".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use headless_chrome::Browser;

    #[tokio::test]
    async fn test_check_client_security() {
        let browser = Browser::default().unwrap();
        let tab = browser.new_tab().unwrap();
        tab.set_default_timeout(std::time::Duration::from_millis(30000));
        tab.navigate_to("http://example.com")
            .unwrap()
            .wait_until_navigated()
            .unwrap();
        let config = BrowserConfig::default();
        let issues = check_client_security(&tab, &config).await.unwrap();
        assert!(!issues.is_empty());
    }

    #[test]
    fn test_client_issue_types() {
        assert_eq!(
            ClientIssueType::LocalStorageSensitive,
            ClientIssueType::LocalStorageSensitive
        );
        assert_eq!(ClientIssueType::CORSWildcard, ClientIssueType::CORSWildcard);
    }
}
