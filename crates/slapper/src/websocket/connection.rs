use crate::error::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub url: String,
    pub connected: bool,
    pub response_headers: Vec<(String, String)>,
    pub subprotocols: Vec<String>,
    pub extensions: Vec<String>,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

pub struct ConnectionTester;

impl Default for ConnectionTester {
    fn default() -> Self {
        Self::new()
    }
}

impl ConnectionTester {
    pub fn new() -> Self {
        Self
    }

    #[cfg(feature = "websocket")]
    pub async fn test_connection(&self, url: &str) -> Result<ConnectionTestResult> {
        use std::time::Instant;
        use tokio_tungstenite::connect_async;

        let start = Instant::now();
        match connect_async(url).await {
            Ok((ws_stream, response)) => {
                let latency = start.elapsed().as_secs_f64() * 1000.0;

                let headers: Vec<(String, String)> = response
                    .headers()
                    .iter()
                    .filter_map(|(k, v)| v.to_str().ok().map(|vs| (k.to_string(), vs.to_string())))
                    .collect();

                let subprotocols: Vec<String> = response
                    .headers()
                    .get("sec-websocket-protocol")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                let extensions: Vec<String> = response
                    .headers()
                    .get("sec-websocket-extensions")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.split(',').map(|s| s.trim().to_string()).collect())
                    .unwrap_or_default();

                drop(ws_stream);

                Ok(ConnectionTestResult {
                    url: url.to_string(),
                    connected: true,
                    response_headers: headers,
                    subprotocols,
                    extensions,
                    latency_ms: Some(latency),
                    error: None,
                })
            }
            Err(e) => Ok(ConnectionTestResult {
                url: url.to_string(),
                connected: false,
                response_headers: Vec::new(),
                subprotocols: Vec::new(),
                extensions: Vec::new(),
                latency_ms: None,
                error: Some(e.to_string()),
            }),
        }
    }

    #[cfg(not(feature = "websocket"))]
    pub async fn test_connection(&self, url: &str) -> Result<ConnectionTestResult> {
        Ok(ConnectionTestResult {
            url: url.to_string(),
            connected: false,
            response_headers: Vec::new(),
            subprotocols: Vec::new(),
            extensions: Vec::new(),
            latency_ms: None,
            error: Some(
                "WebSocket feature not enabled. Build with --features websocket".to_string(),
            ),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_tester_creation() {
        let tester = ConnectionTester::new();
        let _ = tester;
    }

    #[test]
    fn test_connection_result_creation() {
        let result = ConnectionTestResult {
            url: "ws://example.com".to_string(),
            connected: false,
            response_headers: Vec::new(),
            subprotocols: Vec::new(),
            extensions: Vec::new(),
            latency_ms: None,
            error: Some("Test error".to_string()),
        };
        assert!(!result.connected);
        assert!(result.error.is_some());
    }
}
