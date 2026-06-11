use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginTestResult {
    pub origin: String,
    pub accepted: bool,
    pub status_code: Option<u16>,
    pub details: String,
}

#[cfg(feature = "websocket")]
pub async fn test_origins(url: &str, timeout_secs: u64) -> Vec<OriginTestResult> {
    tracing::info!("Testing WebSocket origin validation on {}", url);

    let malicious_origins = vec![
        "https://evil.com",
        "http://localhost",
        "null",
        "https://target.com.evil.com",
    ];

    let mut results = Vec::new();

    for origin in malicious_origins {
        let result = test_single_origin(url, origin, timeout_secs).await;
        results.push(result);
    }

    results
}

#[cfg(feature = "websocket")]
async fn test_single_origin(url: &str, origin: &str, timeout_secs: u64) -> OriginTestResult {
    use futures::SinkExt;

    let request = match tokio_tungstenite::tungstenite::http::Request::builder()
        .uri(url)
        .header("Origin", origin)
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", "dGhlIHNhbXBsZSBub25jZQ==")
        .body(())
    {
        Ok(r) => r,
        Err(e) => {
            return OriginTestResult {
                origin: origin.to_string(),
                accepted: false,
                status_code: None,
                details: format!("Failed to build request: {}", e),
            };
        }
    };

    let result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async_with_config(request, None, false),
    )
    .await;

    match result {
        Ok(Ok((mut ws, response))) => {
            let status = response.status().as_u16();
            let accepted = status == 101;
            let details = if accepted {
                format!("Origin '{}' accepted (101 Switching Protocols)", origin)
            } else {
                format!("Origin '{}' rejected (HTTP {})", origin, status)
            };

            if let Err(e) = ws
                .send(tokio_tungstenite::tungstenite::Message::Close(
                    Some(tokio_tungstenite::tungstenite::protocol::CloseFrame {
                        code: tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                        reason: "test complete".into(),
                    }),
                ))
                .await
            {
                tracing::warn!("Failed to send close frame for origin test: {}", e);
            }

            OriginTestResult {
                origin: origin.to_string(),
                accepted,
                status_code: Some(status),
                details,
            }
        }
        Ok(Err(e)) => {
            let err_str = e.to_string();
            let accepted = false;
            let details = if err_str.contains("403") || err_str.contains("Forbidden") {
                format!("Origin '{}' rejected: Forbidden", origin)
            } else if err_str.contains("400") || err_str.contains("Bad Request") {
                format!("Origin '{}' rejected: Bad Request", origin)
            } else {
                format!("Origin '{}' connection failed: {}", origin, err_str)
            };

            tracing::debug!("Origin test for '{}' failed: {}", origin, details);

            OriginTestResult {
                origin: origin.to_string(),
                accepted,
                status_code: None,
                details,
            }
        }
        Err(_) => OriginTestResult {
            origin: origin.to_string(),
            accepted: false,
            status_code: None,
            details: format!("Origin '{}' test timed out after {}s", origin, timeout_secs),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
