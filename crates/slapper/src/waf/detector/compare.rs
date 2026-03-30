use crate::error::Result;
use std::time::Duration;

use super::WafDetector;
use super::types::ResponseDiff;

impl WafDetector {
    pub async fn compare_responses(
        &self,
        url: &str,
        normal_req: &str,
        malicious_req: &str,
    ) -> Result<ResponseDiff> {
        let ua = crate::waf::bypass::headers::get_random_ua().to_string();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .danger_accept_invalid_certs(true)
            .redirect(reqwest::redirect::Policy::limited(5))
            .user_agent(ua)
            .build()?;

        let normal_response = client.get(url).query(&[("q", normal_req)]).send().await?;

        let malicious_response = client
            .get(url)
            .query(&[("q", malicious_req)])
            .send()
            .await?;

        let normal_status = normal_response.status().as_u16();
        let normal_headers: std::collections::HashMap<String, String> = normal_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let normal_body = normal_response.text().await.unwrap_or_default();
        let normal_length = normal_body.len();

        let malicious_status = malicious_response.status().as_u16();
        let malicious_headers: std::collections::HashMap<String, String> = malicious_response
            .headers()
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
            .collect();
        let malicious_body = malicious_response.text().await.unwrap_or_default();
        let malicious_length = malicious_body.len();

        let header_diffs: Vec<String> = normal_headers
            .keys()
            .filter(|k| malicious_headers.get(*k) != normal_headers.get(*k))
            .map(|k| {
                format!(
                    "{}: {} -> {}",
                    k,
                    normal_headers.get(k).unwrap_or(&"".to_string()),
                    malicious_headers.get(k).unwrap_or(&"".to_string())
                )
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
