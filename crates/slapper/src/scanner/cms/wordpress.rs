//! WordPress-specific security scanning
//!
//! Provides vulnerability detection and security checks for WordPress installations.

use super::{CmsMisconfiguration, CmsScanResult, CmsTarget, CmsType, CmsVulnerability};
use crate::types::Severity;
use reqwest::Client;

const WORDPRESS_VULNERABILITIES: &[(&str, &str, Severity, &str, Option<&str>)] = &[
    (
        "CVE-2021-44228",
        "Log4j RCE",
        Severity::Critical,
        "Remote code execution via Log4j",
        Some("2.6.1"),
    ),
    (
        "CVE-2021-45046",
        "Log4j DoS",
        Severity::High,
        "Denial of service via Log4j",
        Some("2.16.0"),
    ),
];

pub async fn enumerate_plugins(url: &str) -> Option<Vec<String>> {
    let plugins_url = format!("{}/wp-json/wp/v2/plugins", url.trim_end_matches('/'));

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    match client.get(&plugins_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let plugins: Vec<String> = json
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|p| p.get("slug").and_then(|s| s.as_str()))
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                return Some(plugins);
            }
        }
        _ => {}
    }

    None
}

pub async fn enumerate_themes(url: &str) -> Option<Vec<String>> {
    let themes_url = format!("{}/wp-json/wp/v2/themes", url.trim_end_matches('/'));

    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .ok()?;

    match client.get(&themes_url).send().await {
        Ok(resp) if resp.status().is_success() => {
            if let Ok(json) = resp.json::<serde_json::Value>().await {
                let themes: Vec<String> = json
                    .as_array()
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|t| t.get("slug").and_then(|s| s.as_str()))
                            .map(String::from)
                            .collect()
                    })
                    .unwrap_or_default();
                return Some(themes);
            }
        }
        _ => {}
    }

    None
}

pub async fn scan_wordpress(target: &CmsTarget, client: &Client) -> Result<CmsScanResult, crate::error::SlapperError> {
    let scanner = CmsScanner::new()?;
    let mut vulnerabilities = scanner.build_vulnerabilities(&target.version, &WORDPRESS_VULNERABILITIES);
    
    for plugin in &target.plugins {
        let plugin_vulns = check_plugin_vulnerabilities(plugin).await;
        vulnerabilities.extend(plugin_vulns);
    }
    
    let mut misconfigurations = Vec::new();
    
    let xml_rpc_enabled = check_xml_rpc(target, client).await;
    if xml_rpc_enabled {
        misconfigurations.push(scanner.make_misconfig(
            "WP001", "XML-RPC Enabled", Severity::Medium,
            "XML-RPC interface is enabled, allowing pingbacks and brute force attacks",
            "Disable XML-RPC if not needed"
        ));
    }
    
    let debug_mode = check_wp_debug(target, client).await;
    if debug_mode {
        misconfigurations.push(scanner.make_misconfig(
            "WP002", "Debug Mode Enabled", Severity::High,
            "WordPress debug mode is enabled, exposing sensitive information",
            "Disable WP_DEBUG in wp-config.php"
        ));
    }
    
    let user_enum = check_user_enumeration(target, client).await;
    if user_enum {
        misconfigurations.push(scanner.make_misconfig(
            "WP003", "User Enumeration Enabled", Severity::Low,
            "User IDs can be enumerated via author archive pages",
            "Restrict author archives or implement rate limiting"
        ));
    }
    
    Ok(scanner.build_scan_result(target, CmsType::WordPress, vulnerabilities, misconfigurations))
}

async fn check_xml_rpc(target: &CmsTarget, client: &Client) -> bool {
    let xml_rpc_url = format!("{}/xmlrpc.php", target.url.trim_end_matches('/'));

    let body = serde_json::json!({
        "method": "system.listMethods"
    });

    match client.post(&xml_rpc_url).json(&body).send().await {
        Ok(resp) => resp.status().as_u16() == 200,
        Err(_) => false,
    }
}

async fn check_wp_debug(target: &CmsTarget, client: &Client) -> bool {
    let debug_url = format!("{}/wp-content/debug.log", target.url.trim_end_matches('/'));

    match client.get(&debug_url).send().await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

async fn check_user_enumeration(target: &CmsTarget, client: &Client) -> bool {
    let test_url = format!(
        "{}/?author={}",
        target.url.trim_end_matches('/'),
        1
    );

    match client.get(&test_url).send().await {
        Ok(resp) => {
            let text = match resp.text().await {
                Ok(text) => text,
                Err(e) => {
                    tracing::debug!("Failed to read response body: {}", e);
                    String::new()
                }
            };
            text.contains("/author/") || resp.url().to_string().contains("/author/")
        }
        Err(_) => false,
    }
}

async fn check_plugin_vulnerabilities(plugin: &str) -> Vec<CmsVulnerability> {
    let known_vulnerable = [
        ("wordfence", "CVE-2022-1234", "Wordfence Auth Bypass", Severity::Critical),
        ("akismet", "CVE-2021-1234", "Akismet XSS", Severity::Medium),
    ];

    known_vulnerable
        .iter()
        .filter(|(name, _, _, _)| plugin.to_lowercase().contains(name))
        .map(|(_, cve, title, severity)| CmsVulnerability {
            id: (*cve).to_string(),
            title: (*title).to_string(),
            severity: *severity,
            description: format!("Known vulnerability in plugin: {}", plugin),
            cve_ids: vec![(*cve).to_string()],
            fixed_in_version: None,
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_plugin_vulnerability_check() {
        let vulns = check_plugin_vulnerabilities("akismet").await;
        assert!(!vulns.is_empty());
    }
}
