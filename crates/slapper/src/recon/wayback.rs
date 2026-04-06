
#![allow(dead_code)]

use crate::error::Result;
use crate::types::SensitiveString;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::utils::create_http_client_with_options;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WaybackResult {
    pub domain: String,
    pub snapshots: Vec<WaybackSnapshot>,
    pub total_snapshots: usize,
    pub endpoints_discovered: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WaybackSnapshot {
    pub url: String,
    pub timestamp: String,
    pub original_url: String,
    pub mimetype: Option<String>,
    pub status_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaybackApiResponse {
    #[serde(default)]
    items: Vec<WaybackApiItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WaybackApiItem {
    #[serde(rename = "timestamp")]
    timestamp: Option<String>,
    #[serde(rename = "original")]
    original: Option<String>,
    #[serde(rename = "mimetype", default)]
    mimetype: Option<String>,
    #[serde(rename = "statuscode", default)]
    statuscode: Option<String>,
}

pub struct WaybackClient {
    client: Client,
    api_key: Option<SensitiveString>,
}

impl WaybackClient {
    pub fn new(api_key: Option<SensitiveString>) -> Result<Self> {
        let client = create_http_client_with_options(30, |builder| {
            builder.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        })?;

        Ok(Self { client, api_key })
    }

    pub async fn get_snapshots(&self, domain: &str, limit: usize) -> Result<WaybackResult> {
        let url = if let Some(ref key) = self.api_key {
            format!(
                "https://web.archive.org/cdx/search/cdx?url={}&output=json&fl=timestamp,original,mimetype,statuscode&filter=statuscode:200&limit={}&api_key={}",
                domain, limit, key.expose_secret()
            )
        } else {
            format!(
                "https://web.archive.org/cdx/search/cdx?url={}&output=json&fl=timestamp,original,mimetype,statuscode&filter=statuscode:200&limit={}",
                domain, limit
            )
        };

        let response = self.client.get(&url).send().await?;

        if !response.status().is_success() {
            return Ok(WaybackResult {
                domain: domain.to_string(),
                snapshots: Vec::new(),
                total_snapshots: 0,
                endpoints_discovered: Vec::new(),
            });
        }

        let text = response.text().await?;
        let mut snapshots = Vec::new();
        let mut endpoints = std::collections::HashSet::new();

        let lines: Vec<&str> = text.lines().collect();
        for line in lines.iter().skip(1) {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 2 {
                let timestamp = parts.first().unwrap_or(&"").to_string();
                let original = parts.get(1).unwrap_or(&"").to_string();
                let mimetype = parts.get(2).map(|s| s.to_string());
                let status_code = parts.get(3).and_then(|s| s.parse::<u16>().ok());

                if !original.is_empty() {
                    snapshots.push(WaybackSnapshot {
                        url: format!("https://web.archive.org/web/{}/{}", timestamp, original),
                        timestamp,
                        original_url: original.clone(),
                        mimetype,
                        status_code,
                    });

                    if let Ok(url) = url::Url::parse(&original) {
                        let path = url.path().to_string();
                        if !path.is_empty() && path != "/" {
                            endpoints.insert(path);
                        }
                    }
                }
            }
        }

        let endpoints_discovered: Vec<String> = endpoints.into_iter().collect();
        let total_snapshots = snapshots.len();

        Ok(WaybackResult {
            domain: domain.to_string(),
            snapshots,
            total_snapshots,
            endpoints_discovered,
        })
    }

    pub async fn get_latest_snapshot(&self, url: &str) -> Result<Option<WaybackSnapshot>> {
        let encoded_url = urlencoding::encode(url);
        let api_url = format!("https://web.archive.org/web/timemap/link/{}", encoded_url);

        let response = self.client.get(&api_url).send().await?;

        if !response.status().is_success() {
            return Ok(None);
        }

        let text = response.text().await?;
        let lines: Vec<&str> = text.lines().collect();

        if let Some(first_line) = lines.first() {
            let parts: Vec<&str> = first_line.split(',').collect();
            if parts.len() >= 2 {
                let timestamp = parts.first().unwrap_or(&"").trim_matches('"');
                let original = parts.get(1).unwrap_or(&"").trim_matches('"');

                return Ok(Some(WaybackSnapshot {
                    url: format!("https://web.archive.org/web/{}/{}", timestamp, original),
                    timestamp: timestamp.to_string(),
                    original_url: original.to_string(),
                    mimetype: None,
                    status_code: None,
                }));
            }
        }

        Ok(None)
    }
}

pub async fn get_wayback_snapshots(
    domain: &str,
    api_key: Option<&SensitiveString>,
    limit: usize,
) -> Result<WaybackResult> {
    let client = WaybackClient::new(api_key.cloned())?;
    client.get_snapshots(domain, limit).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wayback_result_default() {
        let result = WaybackResult::default();
        assert!(result.domain.is_empty());
        assert!(result.snapshots.is_empty());
        assert_eq!(result.total_snapshots, 0);
        assert!(result.endpoints_discovered.is_empty());
    }

    #[test]
    fn test_wayback_snapshot_serialization() {
        let snapshot = WaybackSnapshot {
            url: "https://web.archive.org/web/20230101000000/https://example.com/page".to_string(),
            timestamp: "20230101000000".to_string(),
            original_url: "https://example.com/page".to_string(),
            mimetype: Some("text/html".to_string()),
            status_code: Some(200),
        };
        let json = serde_json::to_string(&snapshot).unwrap();
        assert!(json.contains("20230101000000"));
        assert!(json.contains("example.com"));
        let decoded: WaybackSnapshot = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.timestamp, "20230101000000");
        assert_eq!(decoded.status_code, Some(200));
    }

    #[test]
    fn test_wayback_result_serialization() {
        let result = WaybackResult {
            domain: "example.com".to_string(),
            snapshots: vec![
                WaybackSnapshot {
                    url: "https://web.archive.org/web/20230101/https://example.com".to_string(),
                    timestamp: "20230101000000".to_string(),
                    original_url: "https://example.com".to_string(),
                    mimetype: Some("text/html".to_string()),
                    status_code: Some(200),
                },
            ],
            total_snapshots: 1,
            endpoints_discovered: vec!["/".to_string(), "/page".to_string()],
        };
        let json = serde_json::to_string(&result).unwrap();
        let decoded: WaybackResult = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.domain, "example.com");
        assert_eq!(decoded.snapshots.len(), 1);
        assert_eq!(decoded.total_snapshots, 1);
        assert_eq!(decoded.endpoints_discovered.len(), 2);
    }

    #[test]
    fn test_wayback_client_new() {
        let client = WaybackClient::new(None);
        assert!(client.is_ok());
        let client = WaybackClient::new(Some(SensitiveString::new("test-key".to_string())));
        assert!(client.is_ok());
    }

    #[test]
    fn test_wayback_snapshot_clone() {
        let snapshot = WaybackSnapshot {
            url: "https://web.archive.org/web/20230101/test".to_string(),
            timestamp: "20230101".to_string(),
            original_url: "https://example.com".to_string(),
            mimetype: None,
            status_code: None,
        };
        let cloned = snapshot.clone();
        assert_eq!(cloned.timestamp, "20230101");
    }

    #[test]
    fn test_wayback_api_response_deserialization() {
        let json = r#"{"items":[{"timestamp":"20230101","original":"https://example.com","mimetype":"text/html","statuscode":"200"}]}"#;
        let resp: WaybackApiResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.items.len(), 1);
        assert_eq!(resp.items[0].timestamp.as_deref(), Some("20230101"));
        assert_eq!(resp.items[0].statuscode.as_deref(), Some("200"));
    }

    #[test]
    fn test_wayback_api_response_empty() {
        let json = r#"{"items":[]}"#;
        let resp: WaybackApiResponse = serde_json::from_str(json).unwrap();
        assert!(resp.items.is_empty());
    }

    #[test]
    fn test_wayback_api_item_optional_fields() {
        let json = r#"{"timestamp":null,"original":null,"mimetype":null,"statuscode":null}"#;
        let item: WaybackApiItem = serde_json::from_str(json).unwrap();
        assert!(item.timestamp.is_none());
        assert!(item.original.is_none());
        assert!(item.mimetype.is_none());
        assert!(item.statuscode.is_none());
    }

    #[test]
    fn test_wayback_result_empty_snapshots() {
        let result = WaybackResult {
            domain: "nonexistent.example".to_string(),
            snapshots: vec![],
            total_snapshots: 0,
            endpoints_discovered: vec![],
        };
        assert!(result.snapshots.is_empty());
        assert_eq!(result.total_snapshots, 0);
        assert!(result.endpoints_discovered.is_empty());
    }
}
