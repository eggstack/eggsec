use crate::browser::BrowserConfig;
use crate::error::Result;
use headless_chrome::Browser;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct SpaRoute {
    pub path: String,
    pub method: String,
    pub parameters: Vec<String>,
    pub discovered_via: DiscoveryMethod,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub enum DiscoveryMethod {
    Crawl,
    XhrInterception,
    FetchInterception,
    RouteParsing,
}

impl std::fmt::Display for DiscoveryMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DiscoveryMethod::Crawl => write!(f, "Crawl"),
            DiscoveryMethod::XhrInterception => write!(f, "XHR Interception"),
            DiscoveryMethod::FetchInterception => write!(f, "Fetch Interception"),
            DiscoveryMethod::RouteParsing => write!(f, "Route Parsing"),
        }
    }
}

pub async fn discover_routes(target: &str, config: &BrowserConfig) -> Result<Vec<SpaRoute>> {
    let browser = Browser::default()?;
    let tab = browser.new_tab()?;

    tab.set_default_timeout(std::time::Duration::from_millis(config.timeout_ms));

    tab.navigate_to(target)?.wait_until_navigated()?;

    let js_script = r#"
        (function() {
            const routes = new Set();
            const apiEndpoints = new Set();

            const interceptXhr = () => {
                const originalXhrOpen = XMLHttpRequest.prototype.open;
                XMLHttpRequest.prototype.open = function(method, url) {
                    try {
                        const parsed = new URL(url, window.location.origin);
                        if (parsed.pathname.startsWith('/api/') || parsed.pathname.startsWith('/rest/')) {
                            apiEndpoints.add(parsed.pathname);
                        }
                    } catch(e) {}
                    return originalXhrOpen.apply(this, arguments);
                };
            };

            const interceptFetch = () => {
                const originalFetch = window.fetch;
                window.fetch = function(url, options) {
                    try {
                        const parsed = new URL(url, window.location.origin);
                        if (parsed.pathname.startsWith('/api/') || parsed.pathname.startsWith('/rest/')) {
                            apiEndpoints.add(parsed.pathname);
                        }
                    } catch(e) {}
                    return originalFetch.apply(this, arguments);
                };
            };

            const extractRoutesFromDom = () => {
                const links = document.querySelectorAll('a[href]');
                links.forEach(link => {
                    const href = link.getAttribute('href');
                    if (href && (href.startsWith('/') || href.startsWith('#'))) {
                        const path = href.split('?')[0].split('#')[0];
                        if (path && path !== '/' && path !== '#') {
                            routes.add(path);
                        }
                    }
                });

                const forms = document.querySelectorAll('form[action]');
                forms.forEach(form => {
                    const action = form.getAttribute('action');
                    if (action && action.startsWith('/')) {
                        routes.add(action.split('?')[0]);
                    }
                });
            };

            const extractRoutesFromJs = () => {
                const scripts = document.querySelectorAll('script');
                scripts.forEach(script => {
                    const text = script.textContent || '';
                    const routePatterns = [
                        /router(?:\.push|\.replace|\.navigate)?\(['"]([^'")]+)['"]/g,
                        /path:\s*['"]([^'")]+)['"]/g,
                        /url:\s*['"]([^'")]+)['"]/g,
                        /route:\s*['"]([^'")]+)['"]/g,
                    ];

                    routePatterns.forEach(pattern => {
                        let match;
                        while ((match = pattern.exec(text)) !== null) {
                            if (match[1] && match[1].startsWith('/')) {
                                routes.add(match[1].split('?')[0]);
                            }
                        }
                    });
                });
            };

            interceptXhr();
            interceptFetch();
            extractRoutesFromDom();
            extractRoutesFromJs();

            return {
                routes: Array.from(routes),
                apiEndpoints: Array.from(apiEndpoints)
            };
        })()
    "#;

    let result = tab.evaluate(js_script, true)?;

    let data: HashSet<String> = result.value
        .as_ref()
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();

    let discovered_routes: Vec<SpaRoute> = data.iter()
        .filter(|path| path.starts_with('/') && !path.starts_with("/api/") && !path.starts_with("/rest/"))
        .map(|path| SpaRoute {
            path: path.clone(),
            method: "GET".to_string(),
            parameters: extract_parameters(path),
            discovered_via: DiscoveryMethod::Crawl,
        })
        .collect();

    let api_routes: Vec<SpaRoute> = data.iter()
        .filter(|path| path.starts_with("/api/") || path.starts_with("/rest/"))
        .map(|path| SpaRoute {
            path: path.clone(),
            method: "GET".to_string(),
            parameters: extract_parameters(path),
            discovered_via: DiscoveryMethod::XhrInterception,
        })
        .collect();

    let mut all_routes = discovered_routes;
    all_routes.extend(api_routes);

    Ok(all_routes)
}

fn extract_parameters(path: &str) -> Vec<String> {
    let mut params = Vec::new();
    let segments: Vec<&str> = path.split('/').collect();

    for segment in segments {
        if segment.starts_with('{') && segment.ends_with('}') {
            params.push(segment[1..segment.len()-1].to_string());
        } else if segment.starts_with(':') {
            params.push(segment[1..].to_string());
        }
    }

    params
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_discover_routes() {
        let config = BrowserConfig::default();
        let routes = discover_routes("http://example.com", &config).await.unwrap();
        assert!(!routes.is_empty());
    }

    #[test]
    fn test_discovery_methods() {
        assert_eq!(DiscoveryMethod::Crawl, DiscoveryMethod::Crawl);
        assert_eq!(DiscoveryMethod::XhrInterception, DiscoveryMethod::XhrInterception);
    }
}
