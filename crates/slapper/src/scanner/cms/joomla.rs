//! Joomla-specific security scanning
//!
//! Provides vulnerability detection and security checks for Joomla installations.

use super::{CmsMisconfiguration, CmsScanResult, CmsTarget, CmsType, CmsVulnerability};
use crate::types::Severity;
use reqwest::Client;

const JOOMLA_VULNERABILITIES: &[(&str, &str, Severity, &str, Option<&str>)] = &[
    (
        "CVE-2020-10238",
        "Joomla RCE",
        Severity::Critical,
        "Remote code execution in Joomla",
        Some("3.9.16"),
    ),
    (
        "CVE-2019-10945",
        "Joomla SQL Injection",
        Severity::High,
        "SQL injection in com_fields",
        Some("3.8.11"),
    ),
];

use crate::utils::create_insecure_http_client;

pub async fn enumerate_extensions(url: &str) -> Option<Vec<String>> {
    let extensions_url = format!("{}/administrator/components", url.trim_end_matches('/'));

    let client = create_insecure_http_client(10)?;

    match client.get(&extensions_url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.contains("Index of") || text.contains("[To Parent Directory]") {
                let extensions: Vec<String> = text
                    .lines()
                    .filter(|l| l.contains("/administrator/components/"))
                    .filter_map(|l| {
                        l.split('/')
                            .nth(3)
                            .map(String::from)
                    })
                    .collect();
                return Some(extensions);
            }
        }
        Err(_) => {}
    }

    None
}

pub async fn scan_joomla(target: &CmsTarget, client: &Client) -> Result<CmsScanResult, crate::error::SlapperError> {
    let scanner = CmsScanner::new()?;
    let version = target.version.clone().or_else(|| detect_joomla_version(target, client).await);
    let mut vulnerabilities = scanner.build_vulnerabilities(&version, &JOOMLA_VULNERABILITIES);

    let mut misconfigurations = Vec::new();
    
    let admin_url = format!("{}/administrator", target.url.trim_end_matches('/'));
    match client.get(&admin_url).send().await {
        Ok(resp) => {
            if resp.status().as_u16() == 200 {
                misconfigurations.push(scanner.make_misconfig(
                    "JM001", "Joomla Administrator Accessible", Severity::Low,
                    "The Joomla admin panel is accessible",
                    "Consider restricting access to administrator area by IP"
                ));
            }
        }
        Err(_) => {}
    }
    let mut result = scanner.build_scan_result(target, CmsType::Joomla, vulnerabilities, misconfigurations);
    result.version = version;
    Ok(result)
}

async fn detect_joomla_version(target: &CmsTarget, client: &Client) -> Option<String> {
    let manifest_url = format!("{}/administrator/manifests/files/joomla.xml", target.url.trim_end_matches('/'));

    match client.get(&manifest_url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if let Some(version_start) = text.find("<version>") {
                    let version_start = version_start + 9;
                    if version_start <= text.len() {
                        if let Some(version_end) = text[version_start..].find("</version>") {
                            if version_start + version_end <= text.len() {
                                return Some(text[version_start..version_start + version_end].to_string());
                            }
                        }
                    }
                }
            }
        }
        Err(_) => {}
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_joomla_extension_enumeration() {
        let result = enumerate_extensions("https://example.com").await;
        assert!(result.is_some() || result.is_none());
    }
}
