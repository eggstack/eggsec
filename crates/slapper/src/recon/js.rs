use crate::error::Result;
use regex::Regex;
use rustc_hash::FxHashSet;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

use crate::utils::create_http_client_with_options;

static ENDPOINT_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r#"(?:api|endpoint|path|route|url)["']?\s*[:=]\s*["']([^"'<>\s]+)["']"#)
            .expect("valid endpoint pattern"),
        Regex::new(r#"fetch\s*\(\s*["']([^"'<>\s]+)["']"#).expect("valid fetch pattern"),
        Regex::new(r#"axios\.[\w]+\(\s*["']([^"'<>\s]+)["']"#).expect("valid axios pattern"),
        Regex::new(r#"\$\.ajax\s*\(\s*\{[^}]*url\s*:\s*["']([^"'<>\s]+)["']"#).expect("valid jQuery ajax pattern"),
        Regex::new(r#"window\.open\s*\(\s*["']([^"'<>\s]+)["']"#).expect("valid window.open pattern"),
        Regex::new(r#"(?:href|src|action)\s*=\s*["']([^"'<>\s]+)["']"#).expect("valid href/src/action pattern"),
    ]
});

static SECRET_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        (
            r#"(?i)(api[_-]?key|apikey|secret[_-]?key)["']?\s*[:=]\s*["']([^"']{8,})["']"#,
            Regex::new(
                r#"(?i)(api[_-]?key|apikey|secret[_-]?key)["']?\s*[:=]\s*["']([^"']{8,})["']"#,
            )
            .expect("valid API key secret pattern"),
        ),
        (
            r#"(?i)(aws[_-]?access[_-]?key|aws[_-]?secret)["']?\s*[:=]\s*["']([^"']{10,})["']"#,
            Regex::new(
                r#"(?i)(aws[_-]?access[_-]?key|aws[_-]?secret)["']?\s*[:=]\s*["']([^"']{10,})["']"#,
            )
            .expect("valid AWS secret pattern"),
        ),
        (
            r#"(?i)(private[_-]?key|password|passwd|pwd)["']?\s*[:=]\s*["']([^"']{6,})["']"#,
            Regex::new(
                r#"(?i)(private[_-]?key|password|passwd|pwd)["']?\s*[:=]\s*["']([^"']{6,})["']"#,
            )
            .expect("valid password/private key pattern"),
        ),
        (
            r#"(?i)bearer\s+[a-zA-Z0-9\-_\.]+"#,
            Regex::new(r#"(?i)bearer\s+[a-zA-Z0-9\-_\.]+"#).expect("valid bearer token pattern"),
        ),
        (
            r#"(?i)basic\s+[a-zA-Z0-9+/=]+"#,
            Regex::new(r#"(?i)basic\s+[a-zA-Z0-9+/=]+"#).expect("valid basic auth pattern"),
        ),
        (
            r#"(?i)(jwt|token)["']?\s*[:=]\s*["'](eyJ[a-zA-Z0-9\-_\.]+)["']"#,
            Regex::new(r#"(?i)(jwt|token)["']?\s*[:=]\s*["'](eyJ[a-zA-Z0-9\-_\.]+)["']"#).expect("valid JWT pattern"),
        ),
    ]
});

static API_KEY_PATTERNS: LazyLock<Vec<(&'static str, Regex)>> = LazyLock::new(|| {
    vec![
        (
            r#"(?i)sk-[a-zA-Z0-9]{20,}"#,
            Regex::new(r#"(?i)sk-[a-zA-Z0-9]{20,}"#).expect("valid OpenAI API key pattern"),
        ),
        (
            r#"(?i)AIza[0-9A-Za-z\-_]{35}"#,
            Regex::new(r#"(?i)AIza[0-9A-Za-z\-_]{35}"#).expect("valid Google API key pattern"),
        ),
        (
            r#"(?i)ya29\.[0-9A-Za-z\-_]+"#,
            Regex::new(r#"(?i)ya29\.[0-9A-Za-z\-_]+"#).expect("valid Google OAuth pattern"),
        ),
        (
            r#"(?i)github_pat_[a-zA-Z0-9_]{22,}"#,
            Regex::new(r#"(?i)github_pat_[a-zA-Z0-9_]{22,}"#).expect("valid GitHub PAT pattern"),
        ),
        (
            r#"(?i)glpat-[a-zA-Z0-9\-_]{20,}"#,
            Regex::new(r#"(?i)glpat-[a-zA-Z0-9\-_]{20,}"#).expect("valid GitLab PAT pattern"),
        ),
        (
            r#"(?i)AKIA[0-9A-Z]{16}"#,
            Regex::new(r#"(?i)AKIA[0-9A-Z]{16}"#).expect("valid AWS access key pattern"),
        ),
    ]
});

static URL_PATTERN: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"https?://[^\s"'<>]+"#).expect("valid URL pattern"));

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct JsAnalysis {
    pub url: String,
    pub javascript_files: Vec<JsFile>,
    pub extracted_endpoints: Vec<String>,
    pub potential_secrets: Vec<Secret>,
    pub api_keys: Vec<ApiKey>,
    pub urls_found: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsFile {
    pub url: String,
    pub content_preview: Option<String>,
    pub size: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Secret {
    pub secret_type: String,
    pub value: String,
    pub context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKey {
    pub key_type: String,
    pub value: String,
    pub confidence: String,
}

pub struct JsAnalyzer {
    client: reqwest::Client,
}

impl JsAnalyzer {
    pub fn new() -> Result<Self> {
        let client = create_http_client_with_options(30, |builder| {
            builder.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        })?;

        Ok(Self { client })
    }

    pub async fn analyze(&self, url: &str) -> Result<JsAnalysis> {
        let mut analysis = JsAnalysis {
            url: url.to_string(),
            ..Default::default()
        };

        let response = self.client.get(url).send().await?;
        let html = response.text().await?;

        let (javascript_files, inline_scripts) = {
            let document = Html::parse_document(&html);

            let script_selector =
                Selector::parse("script[src]").expect("valid CSS selector: script[src]");
            let mut javascript_files = Vec::new();
            for element in document.select(&script_selector) {
                if let Some(src) = element.value().attr("src") {
                    let full_url = self.resolve_url(url, src);
                    javascript_files.push(JsFile {
                        url: full_url.clone(),
                        content_preview: None,
                        size: None,
                    });
                }
            }

            let script_selector = Selector::parse("script").expect("valid CSS selector: script");
            let mut inline_scripts = Vec::new();
            for element in document.select(&script_selector) {
                if let Some(inner_html) = element.text().next() {
                    if !inner_html.trim().is_empty() {
                        inline_scripts.push(inner_html.to_string());
                    }
                }
            }

            (javascript_files, inline_scripts)
        };

        analysis.javascript_files = javascript_files;

        for js_file in &analysis.javascript_files {
            if let Ok(js_content) = self.client.get(&js_file.url).send().await {
                if let Ok(text) = js_content.text().await {
                    let endpoints = self.extract_endpoints(&text);
                    analysis.extracted_endpoints.extend(endpoints);

                    let secrets = self.extract_secrets(&text);
                    analysis.potential_secrets.extend(secrets);

                    let api_keys = self.extract_api_keys(&text);
                    analysis.api_keys.extend(api_keys);

                    let urls = self.extract_urls(&text);
                    analysis.urls_found.extend(urls);
                }
            }
        }

        for inline_script in inline_scripts {
            let endpoints = self.extract_endpoints(&inline_script);
            analysis.extracted_endpoints.extend(endpoints);

            let secrets = self.extract_secrets(&inline_script);
            analysis.potential_secrets.extend(secrets);

            let api_keys = self.extract_api_keys(&inline_script);
            analysis.api_keys.extend(api_keys);
        }

        analysis.extracted_endpoints.sort();
        analysis.extracted_endpoints.dedup();
        analysis.urls_found.sort();
        analysis.urls_found.dedup();

        Ok(analysis)
    }

    fn resolve_url(&self, base: &str, relative: &str) -> String {
        if relative.starts_with("http://") || relative.starts_with("https://") {
            return relative.to_string();
        }

        if let Ok(base_url) = url::Url::parse(base) {
            if let Ok(resolved) = base_url.join(relative) {
                return resolved.to_string();
            }
        }

        relative.to_string()
    }

    fn extract_endpoints(&self, content: &str) -> Vec<String> {
        let mut endpoints = FxHashSet::default();

        for re in ENDPOINT_PATTERNS.iter() {
            for cap in re.captures_iter(content) {
                if let Some(endpoint) = cap.get(1) {
                    let endpoint_str = endpoint.as_str();
                    if !endpoint_str.starts_with('#')
                        && !endpoint_str.starts_with("javascript:")
                        && endpoint_str.len() > 2
                    {
                        endpoints.insert(endpoint_str.to_string());
                    }
                }
            }
        }

        endpoints.into_iter().collect()
    }

    fn extract_secrets(&self, content: &str) -> Vec<Secret> {
        let mut secrets = Vec::new();

        for (secret_type, re) in SECRET_PATTERNS.iter() {
            for cap in re.captures_iter(content) {
                let full_match = cap
                    .get(0)
                    .map(|m| m.as_str().to_string())
                    .unwrap_or_default();
                secrets.push(Secret {
                    secret_type: secret_type.to_string(),
                    value: full_match.chars().skip(4).take(20).collect::<String>() + "...",
                    context: full_match.chars().take(50).collect(),
                });
            }
        }

        secrets
    }

    fn extract_api_keys(&self, content: &str) -> Vec<ApiKey> {
        let mut api_keys = Vec::new();

        for (key_type, re) in API_KEY_PATTERNS.iter() {
            for cap in re.captures_iter(content) {
                if let Some(key) = cap.get(0) {
                    api_keys.push(ApiKey {
                        key_type: key_type.to_string(),
                        value: key.as_str().chars().take(15).collect::<String>() + "...",
                        confidence: "high".to_string(),
                    });
                }
            }
        }

        api_keys
    }

    fn extract_urls(&self, content: &str) -> Vec<String> {
        let mut urls = FxHashSet::default();

        for cap in URL_PATTERN.find_iter(content) {
            let url = cap.as_str().to_string();
            if url.len() > 10 && !url.contains(".js") && !url.contains(".css") {
                urls.insert(url);
            }
        }

        urls.into_iter().collect()
    }
}

pub async fn analyze_js(url: &str) -> Result<JsAnalysis> {
    let analyzer = JsAnalyzer::new()?;
    analyzer.analyze(url).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_url_absolute() {
        let analyzer = JsAnalyzer::new().unwrap();
        let result = analyzer.resolve_url("https://example.com", "https://other.com/path");
        assert_eq!(result, "https://other.com/path");
    }

    #[test]
    fn test_resolve_url_relative() {
        let analyzer = JsAnalyzer::new().unwrap();
        let result = analyzer.resolve_url("https://example.com", "/api/v1/users");
        assert_eq!(result, "https://example.com/api/v1/users");
    }

    #[test]
    fn test_extract_api_keys_openai() {
        let analyzer = JsAnalyzer::new().unwrap();
        let content = r#"apiKey: "sk-abcdefghijklmnopqrstuvwx""#;
        let keys = analyzer.extract_api_keys(content);
        assert!(!keys.is_empty(), "Should detect OpenAI API key pattern");
    }

    #[test]
    fn test_extract_api_keys_google() {
        let analyzer = JsAnalyzer::new().unwrap();
        let content = r#"key: "AIzaSyA1B2C3D4E5F6G7H8I9J0K1L2M3N4O5P6Q""#;
        let keys = analyzer.extract_api_keys(content);
        assert!(!keys.is_empty(), "Should detect Google API key pattern");
    }

    #[test]
    fn test_extract_urls_from_content() {
        let analyzer = JsAnalyzer::new().unwrap();
        let content = r#"fetch("https://api.example.com/v1/data")"#;
        let urls = analyzer.extract_urls(content);
        assert!(!urls.is_empty());
        assert!(urls.iter().any(|u| u.contains("api.example.com")));
    }

    #[test]
    fn test_extract_urls_filters_short() {
        let analyzer = JsAnalyzer::new().unwrap();
        let content = r#"url: "http://x""#;
        let urls = analyzer.extract_urls(content);
        assert!(urls.is_empty());
    }
}
