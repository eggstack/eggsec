use crate::error::Result;
use rand::seq::SliceRandom;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use super::{BypassResult, BypassTechnique, ProfileBypass, TestType, WafProfile};
use crate::waf::detector::WafDetectionResult;

pub struct HeaderBypass {
    profile: Option<WafProfile>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeaderSet {
    pub name: String,
    pub headers: Vec<(String, String)>,
}

impl HeaderBypass {
    pub fn new(profile: Option<WafProfile>) -> Self {
        Self { profile }
    }

    pub async fn run(
        &self,
        client: &Client,
        url: &str,
        detection: &WafDetectionResult,
        _test_type: TestType,
    ) -> Result<Vec<BypassResult>> {
        let mut results = Vec::new();

        if let Some(ref profile) = self.profile {
            for bypass in &profile.bypasses {
                if matches!(
                    bypass.technique,
                    BypassTechnique::HeaderManipulation
                        | BypassTechnique::UserAgentRotation
                        | BypassTechnique::XForwardedForSpoof
                        | BypassTechnique::ContentTypeBypass
                ) {
                    let result = self
                        .test_profile_bypass(client, url, bypass, detection)
                        .await?;
                    results.push(result);
                }
            }
        } else {
            for header_set in self.generate_header_bypasses() {
                let result = self
                    .test_header_set(client, url, &header_set, detection)
                    .await?;
                results.push(result);
            }
        }

        Ok(results)
    }

    async fn test_profile_bypass(
        &self,
        client: &Client,
        url: &str,
        bypass: &ProfileBypass,
        _detection: &WafDetectionResult,
    ) -> Result<BypassResult> {
        let mut request = client.get(url);

        for (key, value) in &bypass.headers {
            request = request.header(key.as_str(), value.as_str());
        }

        let response = request.send().await?;
        let status = response.status().as_u16();

        let success = status != 403 && status != 406 && status != 501;

        Ok(BypassResult {
            technique: bypass.technique,
            success,
            description: bypass.description.clone(),
            status_code: status,
            response_diff: None,
        })
    }

    fn generate_header_bypasses(&self) -> Vec<HeaderSet> {
        let mut sets = Vec::new();

        let user_agents = get_user_agents();
        for ua in user_agents.iter().take(5) {
            sets.push(HeaderSet {
                name: format!("User-Agent: {}", &ua[..50.min(ua.len())]),
                headers: vec![("User-Agent".to_string(), ua.to_string())],
            });
        }

        for xff_ip in generate_xff_ips().iter().take(8) {
            sets.push(HeaderSet {
                name: format!("X-Forwarded-For: {}", xff_ip),
                headers: vec![
                    ("User-Agent".to_string(), get_random_ua().to_string()),
                    ("X-Forwarded-For".to_string(), xff_ip.clone()),
                    ("X-Real-IP".to_string(), xff_ip.clone()),
                    ("X-Originating-IP".to_string(), xff_ip.clone()),
                ],
            });
        }

        for content_type in get_content_types() {
            sets.push(HeaderSet {
                name: format!("Content-Type: {}", content_type),
                headers: vec![
                    ("User-Agent".to_string(), get_random_ua().to_string()),
                    ("Content-Type".to_string(), content_type.to_string()),
                ],
            });
        }

        for encoding in get_encodings() {
            sets.push(HeaderSet {
                name: format!("Accept-Encoding: {}", encoding),
                headers: vec![
                    ("User-Agent".to_string(), get_random_ua().to_string()),
                    ("Accept-Encoding".to_string(), encoding.to_string()),
                ],
            });
        }

        sets.push(HeaderSet {
            name: "HTTP Method Override".to_string(),
            headers: vec![
                ("User-Agent".to_string(), get_random_ua().to_string()),
                ("X-HTTP-Method-Override".to_string(), "GET".to_string()),
                ("X-Method-Override".to_string(), "GET".to_string()),
            ],
        });

        sets.push(HeaderSet {
            name: "Original URL Spoof".to_string(),
            headers: vec![
                ("User-Agent".to_string(), get_random_ua().to_string()),
                ("X-Original-URL".to_string(), "/".to_string()),
                ("X-Rewrite-URL".to_string(), "/".to_string()),
            ],
        });

        sets.push(HeaderSet {
            name: "CDN Headers".to_string(),
            headers: vec![
                ("User-Agent".to_string(), get_random_ua().to_string()),
                ("CF-Connecting-IP".to_string(), "127.0.0.1".to_string()),
                ("True-Client-IP".to_string(), "127.0.0.1".to_string()),
                ("X-Forwarded-Host".to_string(), "localhost".to_string()),
            ],
        });

        sets.push(HeaderSet {
            name: "Cache Bypass".to_string(),
            headers: vec![
                ("User-Agent".to_string(), get_random_ua().to_string()),
                ("Cache-Control".to_string(), "no-cache".to_string()),
                ("Pragma".to_string(), "no-cache".to_string()),
            ],
        });

        sets
    }

    async fn test_header_set(
        &self,
        client: &Client,
        url: &str,
        header_set: &HeaderSet,
        detection: &WafDetectionResult,
    ) -> Result<BypassResult> {
        let mut request = client.get(url);

        for (key, value) in &header_set.headers {
            request = request.header(key, value);
        }

        let response = request.send().await?;
        let status = response.status().as_u16();
        let body_len = response.text().await.unwrap_or_default().len() as i64;

        let success = self.is_bypass_successful(status, detection);

        let technique = self.identify_technique(&header_set.name);

        Ok(BypassResult {
            technique,
            success,
            description: header_set.name.clone(),
            status_code: status,
            response_diff: Some(body_len),
        })
    }

    fn is_bypass_successful(&self, status: u16, detection: &WafDetectionResult) -> bool {
        super::is_bypass_successful(status, detection)
    }

    fn identify_technique(&self, name: &str) -> BypassTechnique {
        if name.contains("User-Agent") {
            BypassTechnique::UserAgentRotation
        } else if name.contains("X-Forwarded-For") || name.contains("X-Real-IP") {
            BypassTechnique::XForwardedForSpoof
        } else if name.contains("Content-Type") {
            BypassTechnique::ContentTypeBypass
        } else if name.contains("Encoding") {
            BypassTechnique::EncodingBypass
        } else {
            BypassTechnique::HeaderManipulation
        }
    }
}

pub fn get_user_agents() -> Vec<&'static str> {
    vec![
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:121.0) Gecko/20100101 Firefox/121.0",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Safari/605.1.15",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36 Edg/120.0.0.0",
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/119.0.0.0 Safari/537.36",
        "Mozilla/5.0 (iPhone; CPU iPhone OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (iPad; CPU OS 17_2 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/17.2 Mobile/15E148 Safari/604.1",
        "Mozilla/5.0 (Linux; Android 14; SM-S918B) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.6099.43 Mobile Safari/537.36",
        "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
        "Mozilla/5.0 (compatible; bingbot/2.0; +http://www.bing.com/bingbot.htm)",
        "Mozilla/5.0 (compatible; YandexBot/3.0; +http://yandex.com/bots)",
        "curl/8.4.0",
        "Wget/1.21.4",
        "python-requests/2.31.0",
    ]
}

pub fn get_random_ua() -> &'static str {
    get_user_agents()
        .choose(&mut rand::thread_rng())
        .copied()
        .unwrap_or("Mozilla/5.0")
}

pub fn generate_xff_ips() -> Vec<String> {
    vec![
        "127.0.0.1".to_string(),
        "localhost".to_string(),
        "0.0.0.0".to_string(),
        "10.0.0.1".to_string(),
        "172.16.0.1".to_string(),
        "192.168.0.1".to_string(),
        "192.168.1.1".to_string(),
        "10.255.255.1".to_string(),
        "::1".to_string(),
        "0:0:0:0:0:0:0:1".to_string(),
        "2130706433".to_string(),
        "3232235521".to_string(),
        "3232235777".to_string(),
        "0x7f000001".to_string(),
        "0177.0000.0000.0001".to_string(),
        "127.0.0.1.nip.io".to_string(),
        "127.1".to_string(),
        "127.0.1".to_string(),
    ]
}

pub fn get_content_types() -> Vec<&'static str> {
    vec![
        "application/x-www-form-urlencoded",
        "application/json",
        "text/xml",
        "application/xml",
        "text/plain",
        "multipart/form-data",
        "application/octet-stream",
    ]
}

pub fn get_encodings() -> Vec<&'static str> {
    vec![
        "gzip, deflate",
        "gzip, deflate, br",
        "identity",
        "compress, gzip",
        "*",
        "gzip, deflate, br;q=0.9",
    ]
}
