use crate::error::Result;
use rustc_hash::FxHashMap;

use super::types::ResponseDiff;
use super::WafDetector;

impl WafDetector {
    pub async fn compare_responses(
        &self,
        url: &str,
        normal_req: &str,
        malicious_req: &str,
    ) -> Result<ResponseDiff> {
        let normalized_url = Self::normalize_url_static(url);

        let normal_response = self
            .client
            .get(&normalized_url)
            .query(&[("q", normal_req)])
            .send()
            .await?;

        let malicious_response = self
            .client
            .get(&normalized_url)
            .query(&[("q", malicious_req)])
            .send()
            .await?;

        let normal_status = normal_response.status().as_u16();
        let normal_headers: FxHashMap<String, String> = normal_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let normal_body = match normal_response.text().await {
            Ok(text) => text,
            Err(e) => {
                tracing::debug!("Failed to read normal response body in compare: {}", e);
                String::new()
            }
        };
        let normal_length = normal_body.len();

        let malicious_status = malicious_response.status().as_u16();
        let malicious_headers: FxHashMap<String, String> = malicious_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let malicious_body = match malicious_response.text().await {
            Ok(text) => text,
            Err(e) => {
                tracing::debug!("Failed to read malicious response body in compare: {}", e);
                String::new()
            }
        };
        let malicious_length = malicious_body.len();

        let mut all_keys: Vec<&String> = normal_headers
            .keys()
            .chain(malicious_headers.keys())
            .collect();
        all_keys.sort();
        all_keys.dedup();
        let header_diffs: Vec<String> = all_keys
            .iter()
            .filter(|k| malicious_headers.get(**k) != normal_headers.get(**k))
            .map(|k| {
                let normal_val = normal_headers.get(*k).map(String::as_str).unwrap_or("");
                let malicious_val = malicious_headers.get(*k).map(String::as_str).unwrap_or("");
                format!("{}: {} -> {}", k, normal_val, malicious_val)
            })
            .collect();

        Ok(ResponseDiff {
            normal_status,
            normal_length,
            malicious_status,
            malicious_length,
            normal_headers: Some(normal_headers),
            malicious_headers: Some(malicious_headers),
            header_diffs,
            body_diffs: if normal_body != malicious_body {
                Some(true)
            } else {
                None
            },
        })
    }
}
