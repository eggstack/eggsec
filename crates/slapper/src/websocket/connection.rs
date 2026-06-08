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

#[cfg(feature = "websocket")]
pub async fn test_connection(url: &str, timeout_secs: u64) -> ConnectionTestResult {
    use futures::SinkExt;
    use std::time::Instant;

    tracing::info!("Testing WebSocket connection to {}", url);

    let start = Instant::now();
    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async(url),
    )
    .await;

    match result {
        Ok(Ok((mut ws, response))) => {
            let latency_ms = start.elapsed().as_secs_f64() * 1000.0;
            tracing::info!("WebSocket connection to {} established (latency: {:.1}ms)", url, latency_ms);
            let headers: Vec<(String, String)> = response
                .headers()
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
                .collect();

            let subprotocols = response
                .headers()
                .get("sec-websocket-protocol")
                .and_then(|v| v.to_str().ok())
                .map(|v| {
                    v.split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            let extensions = response
                .headers()
                .get("sec-websocket-extensions")
                .and_then(|v| v.to_str().ok())
                .map(|v| {
                    v.split(';')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                })
                .unwrap_or_default();

            if let Err(e) = ws
                .send(tokio_tungstenite::tungstenite::Message::Close(
                    Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                        reason: "test complete".into(),
                    }),
                ))
                .await
            {
                tracing::warn!("Failed to send WebSocket close frame: {}", e);
            }

            ConnectionTestResult {
                url: url.to_string(),
                connected: true,
                response_headers: headers,
                subprotocols,
                extensions,
                latency_ms: Some(latency_ms),
                error: None,
            }
        }
        Ok(Err(e)) => {
            tracing::warn!("WebSocket connection to {} failed: {}", url, e);
            ConnectionTestResult {
                url: url.to_string(),
                connected: false,
                response_headers: Vec::new(),
                subprotocols: Vec::new(),
                extensions: Vec::new(),
                latency_ms: None,
                error: Some(e.to_string()),
            }
        }
        Err(_) => {
            tracing::warn!("WebSocket connection to {} timed out after {}s", url, timeout_secs);
            ConnectionTestResult {
                url: url.to_string(),
                connected: false,
                response_headers: Vec::new(),
                subprotocols: Vec::new(),
                extensions: Vec::new(),
                latency_ms: None,
                error: Some(format!("Connection timed out after {}s", timeout_secs)),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
