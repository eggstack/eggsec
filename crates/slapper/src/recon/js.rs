use anyhow::Result;
use regex::Regex;
use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

use crate::utils::create_http_client_with_options;

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

            let script_selector = Selector::parse("script[src]")
                .expect("valid CSS selector: script[src]");
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

            let script_selector = Selector::parse("script")
                .expect("valid CSS selector: script");
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
        let mut endpoints = HashSet::new();

        let patterns = [
            r#"(?:api|endpoint|path|route|url)["']?\s*[:=]\s*["']([^"'<>\s]+)["']"#,
            r#"fetch\s*\(\s*["']([^"'<>\s]+)["']"#,
            r#"axios\.[\w]+\(\s*["']([^"'<>\s]+)["']"#,
            r#"\$\.ajax\s*\(\s*\{[^}]*url\s*:\s*["']([^"'<>\s]+)["']"#,
            r#"window\.open\s*\(\s*["']([^"'<>\s]+)["']"#,
            r#"(?:href|src|action)\s*=\s*["']([^"'<>\s]+)["']"#,
        ];

        for pattern in patterns {
            if let Ok(re) = Regex::new(pattern) {
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
        }

        endpoints.into_iter().collect()
    }

    fn extract_secrets(&self, content: &str) -> Vec<Secret> {
        let mut secrets = Vec::new();

        let patterns = [
            (
                r#"(?i)(api[_-]?key|apikey|secret[_-]?key)["']?\s*[:=]\s*["']([^"']{8,})["']"#,
                "API_KEY",
            ),
            (
                r#"(?i)(aws[_-]?access[_-]?key|aws[_-]?secret)["']?\s*[:=]\s*["']([^"']{10,})["']"#,
                "AWS_KEY",
            ),
            (
                r#"(?i)(private[_-]?key|password|passwd|pwd)["']?\s*[:=]\s*["']([^"']{6,})["']"#,
                "PASSWORD",
            ),
            (r#"(?i)bearer\s+[a-zA-Z0-9\-_\.]+"#, "BEARER_TOKEN"),
            (r#"(?i)basic\s+[a-zA-Z0-9+/=]+"#, "BASIC_AUTH"),
            (
                r#"(?i)(jwt|token)["']?\s*[:=]\s*["'](eyJ[a-zA-Z0-9\-_\.]+)["']"#,
                "JWT",
            ),
        ];

        for (pattern, secret_type) in patterns {
            if let Ok(re) = Regex::new(pattern) {
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
        }

        secrets
    }

    fn extract_api_keys(&self, content: &str) -> Vec<ApiKey> {
        let mut api_keys = Vec::new();

        let patterns = [
            (r#"(?i)sk-[a-zA-Z0-9]{20,}"#, "OpenAI"),
            (r#"(?i)AIza[0-9A-Za-z\-_]{35}"#, "Google API"),
            (r#"(?i)ya29\.[0-9A-Za-z\-_]+"#, "Google OAuth"),
            (r#"(?i)github_pat_[a-zA-Z0-9_]{22,}"#, "GitHub PAT"),
            (r#"(?i)glpat-[a-zA-Z0-9\-_]{20,}"#, "GitLab PAT"),
            (r#"(?i)AKIA[0-9A-Z]{16}"#, "AWS Access Key"),
        ];

        for (pattern, key_type) in patterns {
            if let Ok(re) = Regex::new(pattern) {
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
        }

        api_keys
    }

    fn extract_urls(&self, content: &str) -> Vec<String> {
        let mut urls = HashSet::new();

        let url_pattern = Regex::new(r#"https?://[^\s\"'<>]+"#)
            .expect("valid URL extraction regex");
        for cap in url_pattern.find_iter(content) {
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
