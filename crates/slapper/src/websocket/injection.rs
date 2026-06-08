use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionTestResult {
    pub payload: String,
    pub sent: bool,
    pub received_response: bool,
    pub response_content: Option<String>,
    pub vulnerability_detected: bool,
    pub details: String,
}

#[cfg(feature = "websocket")]
pub async fn test_injection(
    url: &str,
    payloads: &[String],
    timeout_secs: u64,
) -> Vec<InjectionTestResult> {
    let mut results = Vec::new();

    for payload in payloads {
        let result = test_single_injection(url, payload, timeout_secs).await;
        results.push(result);
    }

    results
}

#[cfg(feature = "websocket")]
async fn test_single_injection(
    url: &str,
    payload: &str,
    timeout_secs: u64,
) -> InjectionTestResult {
    use futures::{SinkExt, StreamExt};

    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async(url),
    )
    .await;

    let mut ws = match connect_result {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => {
            return InjectionTestResult {
                payload: payload.to_string(),
                sent: false,
                received_response: false,
                response_content: None,
                vulnerability_detected: false,
                details: format!("Connection failed: {}", e),
            };
        }
        Err(_) => {
            return InjectionTestResult {
                payload: payload.to_string(),
                sent: false,
                received_response: false,
                response_content: None,
                vulnerability_detected: false,
                details: format!("Connection timed out after {}s", timeout_secs),
            };
        }
    };

    let send_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            payload.to_string().into(),
        )),
    )
    .await;

    let sent = match send_result {
        Ok(Ok(())) => true,
        Ok(Err(e)) => {
            return InjectionTestResult {
                payload: payload.to_string(),
                sent: false,
                received_response: false,
                response_content: None,
                vulnerability_detected: false,
                details: format!("Send failed: {}", e),
            };
        }
        Err(_) => {
            return InjectionTestResult {
                payload: payload.to_string(),
                sent: false,
                received_response: false,
                response_content: None,
                vulnerability_detected: false,
                details: format!("Send timed out after {}s", timeout_secs),
            };
        }
    };

    let recv_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        ws.next(),
    )
    .await;

    match recv_result {
        Ok(Some(Ok(msg))) => {
            let response_content = match &msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => Some(t.to_string()),
                tokio_tungstenite::tungstenite::Message::Binary(b) => {
                    Some(String::from_utf8_lossy(b).to_string())
                }
                _ => None,
            };

            let vulnerability_detected = detect_injection_vulnerability(payload, &response_content);

            InjectionTestResult {
                payload: payload.to_string(),
                sent,
                received_response: true,
                response_content,
                vulnerability_detected,
                details: if vulnerability_detected {
                    "Potential vulnerability: server responded to injection payload".to_string()
                } else {
                    "No vulnerability detected".to_string()
                },
            }
        }
        Ok(Some(Err(e))) => InjectionTestResult {
            payload: payload.to_string(),
            sent,
            received_response: false,
            response_content: None,
            vulnerability_detected: false,
            details: format!("Receive error: {}", e),
        },
        Ok(None) => InjectionTestResult {
            payload: payload.to_string(),
            sent,
            received_response: false,
            response_content: None,
            vulnerability_detected: false,
            details: "Connection closed by server".to_string(),
        },
        Err(_) => InjectionTestResult {
            payload: payload.to_string(),
            sent,
            received_response: false,
            response_content: None,
            vulnerability_detected: false,
            details: format!("Receive timed out after {}s", timeout_secs),
        },
    }
}

#[cfg(feature = "websocket")]
fn detect_injection_vulnerability(payload: &str, response: &Option<String>) -> bool {
    let response = match response {
        Some(r) => r,
        None => return false,
    };

    let response_lower = response.to_lowercase();

    if response_lower.contains("sql") && payload.contains('\'') {
        return true;
    }

    if response_lower.contains("<script>") && payload.contains("<script>") {
        return true;
    }

    if response_lower.contains("error") && payload.contains('\'') {
        return true;
    }

    if response_lower.contains("exception") || response_lower.contains("stack trace") {
        return true;
    }

    if response_lower.contains("path") && payload.contains("../") {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_result_creation() {
        let result = InjectionTestResult {
            payload: "test".to_string(),
            sent: true,
            received_response: true,
            response_content: Some("error".to_string()),
            vulnerability_detected: true,
            details: "Test".to_string(),
        };
        assert!(result.vulnerability_detected);
    }
}
