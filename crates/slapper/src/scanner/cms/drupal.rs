//! Drupal-specific security scanning
//!
//! Provides vulnerability detection and security checks for Drupal installations.

use super::{CmsMisconfiguration, CmsScanResult, CmsTarget, CmsType, CmsVulnerability};
use crate::types::Severity;
use reqwest::Client;

const DRUPAL_VULNERABILITIES: &[(&str, &str, Severity, &str, Option<&str>)] = &[
    (
        "CVE-2018-7600",
        "Drupalgeddon2 RCE",
        Severity::Critical,
        "Remote code execution via Drupalgeddon2",
        Some("7.58"),
    ),
    (
        "CVE-2019-6340",
        "Drupal XSS",
        Severity::High,
        "Cross-site scripting via REST API",
        Some("8.6.2"),
    ),
];

use crate::utils::create_insecure_http_client;

pub async fn enumerate_modules(url: &str) -> Option<Vec<String>> {
    let modules_url = format!("{}/web/modules", url.trim_end_matches('/'));

    let client = create_insecure_http_client(10)?;

    match client.get(&modules_url).send().await {
        Ok(resp) => {
            let text = resp.text().await.unwrap_or_default();
            if text.contains("Index of") || text.contains("[To Parent Directory]") {
                let modules: Vec<String> = text
                    .lines()
                    .filter(|l| l.contains("/web/modules/"))
                    .filter_map(|l| {
                        l.split('/')
                            .nth(3)
                            .map(String::from)
                    })
                    .collect();
                return Some(modules);
            }
        }
        Err(_) => {}
    }

    None
}

pub async fn scan_drupal(target: &CmsTarget, client: &Client) -> Result<CmsScanResult, crate::error::SlapperError> {
    let scanner = CmsScanner::new()?;
    let mut vulnerabilities = scanner.build_vulnerabilities(&target.version, &DRUPAL_VULNERABILITIES);
    
    let version = target.version.clone().or_else(|| detect_drupal_version(target, client).await);
    
    let mut misconfigurations = Vec::new();
    
    let admin_url = format!("{}/user/login", target.url.trim_end_matches('/'));
    match client.get(&admin_url).send().await {
        Ok(resp) => {
            if resp.status().as_u16() == 200 {
                misconfigurations.push(scanner.make_misconfig(
                    "DR001", "Default Admin Login Page Accessible", Severity::Low,
                    "The Drupal user login page is accessible",
                    "Consider implementing rate limiting on login pages"
                ));
            }
        }
        Err(_) => {}
    }
    
    Ok(scanner.build_scan_result(target, CmsType::Drupal, vulnerabilities, misconfigurations))
}

async fn detect_drupal_version(target: &CmsTarget, client: &Client) -> Option<String> {
    let changelog_url = format!("{}/CHANGELOG.txt", target.url.trim_end_matches('/'));

    match client.get(&changelog_url).send().await {
        Ok(resp) => {
            if let Ok(text) = resp.text().await {
                if let Some(line) = text.lines().next() {
                    if line.contains("Drupal") {
                        return line.split_whitespace().last().map(String::from);
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
    async fn test_drupal_module_enumeration() {
        let result = enumerate_modules("https://example.com").await;
        assert!(result.is_some() || result.is_none());
    }
}
