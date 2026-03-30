
#![allow(dead_code)]

use crate::error::Result;
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
    api_key: Option<String>,
}

impl WaybackClient {
    pub fn new(api_key: Option<String>) -> Result<Self> {
        let client = create_http_client_with_options(30, |builder| {
            builder.user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36")
        })?;

        Ok(Self { client, api_key })
    }

    pub async fn get_snapshots(&self, domain: &str, limit: usize) -> Result<WaybackResult> {
        let url = if let Some(ref key) = self.api_key {
            format!(
                "https://web.archive.org/cdx/search/cdx?url={}&output=json&fl=timestamp,original,mimetype,statuscode&filter=statuscode:200&limit={}&api_key={}",
                domain, limit, key
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
    api_key: Option<String>,
    limit: usize,
) -> Result<WaybackResult> {
    let client = WaybackClient::new(api_key)?;
    client.get_snapshots(domain, limit).await
}
