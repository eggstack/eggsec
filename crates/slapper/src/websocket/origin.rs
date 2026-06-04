use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginTestResult {
    pub origin: String,
    pub accepted: bool,
    pub status_code: Option<u16>,
    pub details: String,
}

pub struct OriginTester;

impl Default for OriginTester {
    fn default() -> Self {
        Self::new()
    }
}

impl OriginTester {
    pub fn new() -> Self {
        Self
    }

    #[cfg(feature = "websocket")]
    pub async fn test_origins(&self, url: &str) -> Result<Vec<OriginTestResult>> {
        use tokio_tungstenite::connect_async;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;

        let test_origins = [
            "null",
            "https://evil.com",
            "https://attacker.com",
            "https://example.com.evil.com",
            "https://evil-example.com",
            "file://",
            "data://",
        ];

        let mut results = Vec::new();

        for origin in test_origins {
            let mut result = OriginTestResult {
                origin: origin.to_string(),
                accepted: false,
                status_code: None,
                details: String::new(),
            };

            let mut request = match url.into_client_request() {
                Ok(r) => r,
                Err(e) => {
                    result.details = format!("Invalid URL: {}", e);
                    results.push(result);
                    continue;
                }
            };

            let origin_header = match origin.parse() {
                Ok(h) => h,
                Err(e) => {
                    result.details = format!("Invalid origin '{}': {}", origin, e);
                    results.push(result);
                    continue;
                }
            };
            request.headers_mut().insert("Origin", origin_header);

            match connect_async(request).await {
                Ok((mut ws_stream, response)) => {
                    result.accepted = true;
                    result.status_code = Some(response.status().as_u16());
                    result.details = format!("Origin '{}' was accepted", origin);
                    if let Err(e) = ws_stream.close(None).await {
                        tracing::debug!("Failed to close WebSocket stream: {}", e);
                    }
                }
                Err(e) => {
                    result.details = format!("Origin '{}' rejected: {}", origin, e);
                }
            }

            results.push(result);
        }

        Ok(results)
    }

    #[cfg(not(feature = "websocket"))]
    pub async fn test_origins(&self, url: &str) -> Result<Vec<OriginTestResult>> {
        Ok(vec![OriginTestResult {
            origin: "N/A".to_string(),
            accepted: false,
            status_code: None,
            details: format!("WebSocket feature not enabled. Build with --features websocket"),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_tester_creation() {
        let tester = OriginTester::new();
        let _ = tester;
    }

    #[test]
    fn test_origin_result_creation() {
        let result = OriginTestResult {
            origin: "https://evil.com".to_string(),
            accepted: true,
            status_code: Some(101),
            details: "Accepted".to_string(),
        };
        assert!(result.accepted);
    }
}
