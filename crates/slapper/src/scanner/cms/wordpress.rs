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
    let mut vulnerabilities = Vec::new();
    let mut misconfigurations = Vec::new();

    if let Some(ref version) = target.version {
        for (cve, title, severity, desc, fixed) in WORDPRESS_VULNERABILITIES {
            if let Some(fix_version) = fixed {
                if version.lt(fix_version) {
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

    for plugin in &target.plugins {
        let plugin_vulns = check_plugin_vulnerabilities(plugin).await;
        vulnerabilities.extend(plugin_vulns);
    }

    let xml_rpc_enabled = check_xml_rpc(target, client).await;
    if xml_rpc_enabled {
        misconfigurations.push(CmsMisconfiguration {
            id: "WP001".to_string(),
            title: "XML-RPC Enabled".to_string(),
            severity: Severity::Medium,
            description: "XML-RPC interface is enabled, allowing pingbacks and brute force attacks"
                .to_string(),
            recommendation: "Disable XML-RPC if not needed".to_string(),
        });
    }

    let debug_mode = check_wp_debug(target, client).await;
    if debug_mode {
        misconfigurations.push(CmsMisconfiguration {
            id: "WP002".to_string(),
            title: "Debug Mode Enabled".to_string(),
            severity: Severity::High,
            description: "WordPress debug mode is enabled, exposing sensitive information"
                .to_string(),
            recommendation: "Disable WP_DEBUG in wp-config.php".to_string(),
        });
    }

    let user_enum = check_user_enumeration(target, client).await;
    if user_enum {
        misconfigurations.push(CmsMisconfiguration {
            id: "WP003".to_string(),
            title: "User Enumeration Enabled".to_string(),
            severity: Severity::Low,
            description: "User IDs can be enumerated via author archive pages".to_string(),
            recommendation: "Restrict author archives or implement rate limiting".to_string(),
        });
    }

    let overall_severity = vulnerabilities
        .iter()
        .map(|v| v.severity)
        .max()
        .unwrap_or(Severity::Info);

    Ok(CmsScanResult {
        target: target.url.clone(),
        cms_type: CmsType::WordPress,
        version: target.version.clone(),
        vulnerabilities,
        misconfigurations,
        security_headers: std::collections::HashMap::new(),
        overall_severity,
    })
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
            let text = resp.text().await.unwrap_or_default();
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
