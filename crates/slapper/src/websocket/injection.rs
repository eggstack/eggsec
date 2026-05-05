use crate::error::Result;
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

pub struct InjectionTester;

impl Default for InjectionTester {
    fn default() -> Self {
        Self::new()
    }
}

impl InjectionTester {
    pub fn new() -> Self {
        Self
    }

    #[cfg(feature = "websocket")]
    pub async fn test_injections(&self, url: &str) -> Result<Vec<InjectionTestResult>> {
        use futures::{SinkExt, StreamExt};
        use tokio_tungstenite::connect_async;
        use tokio_tungstenite::tungstenite::Message;

        let payloads = vec![
            ("' OR '1'='1", "SQL injection"),
            ("<script>alert(1)</script>", "XSS injection"),
            ("{{7*7}}", "Template injection"),
            ("${7*7}", "Expression injection"),
            ("../../../../etc/passwd", "Path traversal"),
            ("__proto__", "Prototype pollution"),
            ("'; DROP TABLE users;--", "SQL destructive"),
            ("<img src=x onerror=alert(1)>", "XSS event handler"),
            ("javascript:alert(1)", "JavaScript URI"),
            ("data:text/html,<script>alert(1)</script>", "Data URI XSS"),
            ("\\x3cscript\\x3ealert(1)\\x3c/script\\x3e", "Encoded XSS"),
            ("$(cat /etc/passwd)", "Command injection"),
            ("`id`", "Backtick command injection"),
            ("#{7*7}", "Ruby ERB injection"),
            ("<svg onload=alert(1)>", "SVG XSS"),
        ];

        let mut results = Vec::new();

        for (payload, test_type) in payloads {
            let mut result = InjectionTestResult {
                payload: payload.to_string(),
                sent: false,
                received_response: false,
                response_content: None,
                vulnerability_detected: false,
                details: format!("{} test", test_type),
            };

            match connect_async(url).await {
                Ok((mut ws_stream, _)) => {
                    if ws_stream.send(Message::Text(payload.into())).await.is_ok() {
                        result.sent = true;

                        if let Ok(Some(msg)) = tokio::time::timeout(
                            std::time::Duration::from_secs(5),
                            ws_stream.next(),
                        )
                        .await
                        {
                            if let Ok(msg) = msg {
                                result.received_response = true;
                                if let Message::Text(text) = msg {
                                    result.response_content = Some(text.to_string());
                                    let lower = text.to_lowercase();
                                    if lower.contains("error")
                                        || lower.contains("exception")
                                        || lower.contains("syntax")
                                        || lower.contains("unexpected")
                                        || lower.contains("stack trace")
                                    {
                                        result.vulnerability_detected = true;
                                        result.details = format!(
                                            "{}: Server returned error response",
                                            test_type
                                        );
                                    }
                                }
                            }
                        }
                    }
                    let _ = ws_stream.close(None).await;
                }
                Err(e) => {
                    result.details = format!("Connection failed: {}", e);
                }
            }

            results.push(result);
        }

        Ok(results)
    }

    #[cfg(not(feature = "websocket"))]
    pub async fn test_injections(&self, url: &str) -> Result<Vec<InjectionTestResult>> {
        Ok(vec![InjectionTestResult {
            payload: "N/A".to_string(),
            sent: false,
            received_response: false,
            response_content: None,
            vulnerability_detected: false,
            details: format!(
                "WebSocket feature not enabled. Cannot test {}. Build with --features websocket",
                url
            ),
        }])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_tester_creation() {
        let tester = InjectionTester::new();
        let _ = tester;
    }

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
