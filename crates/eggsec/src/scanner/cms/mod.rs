//! CMS-specific security scanning
//!
//! Provides security scanning for Content Management Systems including
//! WordPress, Drupal, and Joomla.

pub mod drupal;
pub mod joomla;
pub mod wordpress;

use crate::error::Result;
use crate::types::Severity;
use crate::utils::{create_http_client, create_insecure_http_client};
use regex::RegexBuilder;
use rustc_hash::FxHashMap;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsTarget {
    pub url: String,
    pub detected_cms: Option<CmsType>,
    pub version: Option<String>,
    pub plugins: Vec<String>,
    pub themes: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CmsType {
    WordPress,
    Drupal,
    Joomla,
    Unknown,
}

impl CmsType {
    pub fn as_str(&self) -> &str {
        match self {
            CmsType::WordPress => "WordPress",
            CmsType::Drupal => "Drupal",
            CmsType::Joomla => "Joomla",
            CmsType::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsScanResult {
    pub target: String,
    pub cms_type: CmsType,
    pub version: Option<String>,
    pub vulnerabilities: Vec<CmsVulnerability>,
    pub misconfigurations: Vec<CmsMisconfiguration>,
    pub security_headers: FxHashMap<String, String>,
    pub overall_severity: Severity,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsVulnerability {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub cve_ids: Vec<String>,
    pub fixed_in_version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CmsMisconfiguration {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub recommendation: String,
}

pub struct CmsScanner {
    http_client: reqwest::Client,
}

impl CmsScanner {
    pub fn new() -> Result<Self> {
        Self::new_with_tls_verification(true)
    }

    pub fn new_insecure() -> Result<Self> {
        Self::new_with_tls_verification(false)
    }

    pub fn new_with_tls_verification(verify_tls: bool) -> Result<Self> {
        let client = if verify_tls {
            create_http_client(30)?
        } else {
            create_insecure_http_client(30)?
        };

        Ok(Self {
            http_client: client,
        })
    }

    /// Build vulnerabilities from a list of (cve, title, severity, description, fixed_version)
    pub fn build_vulnerabilities<'a>(
        &self,
        version: &Option<String>,
        vuln_list: &'a [(&'a str, &'a str, Severity, &'a str, Option<&'a str>)],
    ) -> Vec<CmsVulnerability> {
        let mut vulnerabilities = Vec::new();

        if let Some(ref ver) = version {
            for (cve, title, severity, desc, fixed) in vuln_list {
                if let Some(fix_version) = fixed {
                    if version_lt(ver, fix_version) {
                        vulnerabilities.push(CmsVulnerability {
                            id: (*cve).to_string(),
                            title: (*title).to_string(),
                            severity: *severity,
                            description: (*desc).to_string(),
                            cve_ids: vec![(*cve).to_string()],
                            fixed_in_version: Some(fix_version.to_string()),
                        });
                    }
                }
            }
        }

        vulnerabilities
    }

    /// Helper to create a CmsMisconfiguration
    pub fn make_misconfig(
        id: &str,
        title: &str,
        severity: Severity,
        description: &str,
        recommendation: &str,
    ) -> CmsMisconfiguration {
        CmsMisconfiguration {
            id: id.to_string(),
            title: title.to_string(),
            severity,
            description: description.to_string(),
            recommendation: recommendation.to_string(),
        }
    }

    /// Build a CmsScanResult with the given parameters
    pub fn build_scan_result(
        &self,
        target: &CmsTarget,
        cms_type: CmsType,
        vulnerabilities: Vec<CmsVulnerability>,
        misconfigurations: Vec<CmsMisconfiguration>,
    ) -> CmsScanResult {
        let overall_severity = vulnerabilities
            .iter()
            .map(|v| v.severity)
            .max()
            .unwrap_or(Severity::Info);

        CmsScanResult {
            target: target.url.clone(),
            cms_type,
            version: target.version.clone(),
            vulnerabilities,
            misconfigurations,
            security_headers: FxHashMap::default(),
            overall_severity,
        }
    }

    pub async fn detect_cms(&self, url: &str) -> Result<CmsTarget> {
        let response = self.http_client.get(url).send().await
            .map_err(|e| crate::error::EggsecError::Network(format!("Request failed: {}", e)))?;

        let html = response.text().await
            .map_err(|e| crate::error::EggsecError::Network(format!("Failed to read response: {}", e)))?;

        let (cms_type, version) = self.identify_cms(&html, url).await;

        let mut target = CmsTarget {
            url: url.to_string(),
            detected_cms: Some(cms_type),
            version,
            plugins: Vec::new(),
            themes: Vec::new(),
        };

        if cms_type != CmsType::Unknown {
            let (plugins, themes) = self.enumerate_components(url, cms_type).await;
            target.plugins = plugins;
            target.themes = themes;
        }

        Ok(target)
    }

    async fn identify_cms(&self, html: &str, url: &str) -> (CmsType, Option<String>) {
        let html_lower = html.to_lowercase();

        if html_lower.contains("wp-content") || html_lower.contains("wp-includes") {
            if let Some(version) = self.extract_wordpress_version(html) {
                return (CmsType::WordPress, Some(version));
            }
            return (CmsType::WordPress, None);
        }

        if html_lower.contains("drupal") || html_lower.contains("sites/default") {
            return (CmsType::Drupal, None);
        }

        if html_lower.contains("joomla") || html_lower.contains("com_content") {
            return (CmsType::Joomla, None);
        }

        if self.check_xml_rpc(url).await {
            return (CmsType::WordPress, None);
        }

        (CmsType::Unknown, None)
    }

    fn extract_wordpress_version(&self, html: &str) -> Option<String> {
        let version_patterns = [
            r#"<meta name="generator" content="WordPress (\d+\.\d+(?:\.\d+)?)"#,
            r#"wordpressVersion\s*=\s*["'](\d+\.\d+(?:\.\d+)?)["']"#,
        ];

        for pattern in &version_patterns {
            if let Ok(re) = RegexBuilder::new(pattern)
                .size_limit(100_000)
                .build()
            {
                if let Some(caps) = re.captures(html) {
                    if let Some(version) = caps.get(1) {
                        return Some(version.as_str().to_string());
                    }
                }
            }
        }

        None
    }

    async fn check_xml_rpc(&self, url: &str) -> bool {
        let xml_rpc_url = format!("{}/xmlrpc.php", url.trim_end_matches('/'));

        match self.http_client.post(&xml_rpc_url).send().await {
            Ok(resp) => {
                let text = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::debug!("Failed to read response body: {}", e);
                        String::new()
                    }
                };
                text.contains("XML-RPC") || text.contains("blogging")
            }
            Err(_) => false,
        }
    }

    async fn enumerate_components(&self, url: &str, cms_type: CmsType) -> (Vec<String>, Vec<String>) {
        match cms_type {
            CmsType::WordPress => {
                let plugins = wordpress::enumerate_plugins(url, &self.http_client)
                    .await
                    .unwrap_or_default();
                let themes = wordpress::enumerate_themes(url, &self.http_client)
                    .await
                    .unwrap_or_default();
                (plugins, themes)
            }
            CmsType::Drupal => {
                let modules = drupal::enumerate_modules(url, &self.http_client)
                    .await
                    .unwrap_or_default();
                (modules, Vec::new())
            }
            CmsType::Joomla => {
                let extensions = joomla::enumerate_extensions(url, &self.http_client)
                    .await
                    .unwrap_or_default();
                (extensions, Vec::new())
            }
            CmsType::Unknown => (Vec::new(), Vec::new()),
        }
    }

    pub async fn scan(&self, target: &CmsTarget) -> Result<CmsScanResult> {
        match target.detected_cms {
            Some(CmsType::WordPress) => {
                wordpress::scan_wordpress(target, &self.http_client).await
            }
            Some(CmsType::Drupal) => {
                drupal::scan_drupal(target, &self.http_client).await
            }
            Some(CmsType::Joomla) => {
                joomla::scan_joomla(target, &self.http_client).await
            }
            _ => Ok(CmsScanResult {
                target: target.url.clone(),
                cms_type: CmsType::Unknown,
                version: None,
                vulnerabilities: Vec::new(),
                misconfigurations: Vec::new(),
                security_headers: FxHashMap::default(),
                overall_severity: Severity::Info,
            }),
        }
    }
}

fn version_lt(current: &str, fixed: &str) -> bool {
    fn parse_parts(v: &str) -> Vec<u32> {
        v.split('.')
            .map(|p| {
                p.chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect::<String>()
            })
            .map(|p| p.parse::<u32>().unwrap_or(0))
            .collect()
    }

    let current_parts = parse_parts(current);
    let fixed_parts = parse_parts(fixed);
    let max_len = current_parts.len().max(fixed_parts.len());

    for idx in 0..max_len {
        let current_val = *current_parts.get(idx).unwrap_or(&0);
        let fixed_val = *fixed_parts.get(idx).unwrap_or(&0);
        if current_val < fixed_val {
            return true;
        }
        if current_val > fixed_val {
            return false;
        }
    }

    false
}

impl Default for CmsScanner {
    fn default() -> Self {
        Self::new().unwrap_or_else(|_| Self {
            http_client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        })
    }
}

pub async fn check_debug_mode(url: &str, client: &reqwest::Client) -> bool {
    let debug_url = format!("{}/wp-config.php", url.trim_end_matches('/'));

    match client.get(&debug_url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

pub async fn check_directory_listing(url: &str, client: &reqwest::Client) -> bool {
    let test_urls = [
        format!("{}/wp-content/uploads/", url.trim_end_matches('/')),
        format!("{}/wp-admin/", url.trim_end_matches('/')),
        format!("{}/administrator/", url.trim_end_matches('/')),
    ];

    for test_url in &test_urls {
        match client.get(test_url).send().await {
            Ok(resp) => {
                let body = match resp.text().await {
                    Ok(text) => text,
                    Err(e) => {
                        tracing::debug!("Failed to read directory listing response: {}", e);
                        String::new()
                    }
                };
                if body.contains("Index of") || body.contains("[To Parent Directory]") {
                    return true;
                }
            }
            Err(_) => continue,
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cms_type_str() {
        assert_eq!(CmsType::WordPress.as_str(), "WordPress");
        assert_eq!(CmsType::Drupal.as_str(), "Drupal");
        assert_eq!(CmsType::Joomla.as_str(), "Joomla");
    }

    #[tokio::test]
    async fn test_cms_scanner_creation() {
        let scanner = CmsScanner::new();
        assert!(scanner.is_ok());
    }

    #[test]
    fn test_version_lt_semver_like() {
        assert!(version_lt("3.9.15", "3.9.16"));
        assert!(!version_lt("3.9.16", "3.9.16"));
        assert!(!version_lt("3.9.17", "3.9.16"));
        assert!(version_lt("8.10", "8.10.1"));
        assert!(!version_lt("8.10.10", "8.10.2"));
    }
}
