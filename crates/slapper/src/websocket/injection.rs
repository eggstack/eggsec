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
    tracing::info!(
        "Testing WebSocket injection with {} payloads on {}",
        payloads.len(),
        url
    );
    let mut results = Vec::new();

    for payload in payloads {
        let result = test_single_injection(url, payload, timeout_secs).await;
        results.push(result);
    }

    results
}

#[cfg(feature = "websocket")]
async fn test_single_injection(url: &str, payload: &str, timeout_secs: u64) -> InjectionTestResult {
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

    let recv_result =
        tokio::time::timeout(std::time::Duration::from_secs(timeout_secs), ws.next()).await;

    match recv_result {
        Ok(Some(Ok(msg))) => {
            let response_content = match &msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => Some(t.to_string()),
                tokio_tungstenite::tungstenite::Message::Binary(b) => {
                    Some(String::from_utf8_lossy(b).to_string())
                }
                _ => None,
            };

            let vulnerability_detected =
                detect_injection_vulnerability(payload, response_content.as_deref());

            if vulnerability_detected {
                tracing::warn!(
                    "WebSocket injection vulnerability detected with payload: {}",
                    payload
                );
            }

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
fn detect_injection_vulnerability(payload: &str, response: Option<&str>) -> bool {
    let response = match response {
        Some(r) => r,
        None => return false,
    };

    let response_lower = response.to_lowercase();

    if payload.contains('\'') {
        let sql_error_indicators = [
            "syntax error",
            "mysql error",
            "postgresql error",
            "sqlite error",
            "unclosed quotation mark",
            "quoted string not properly terminated",
            "sql command not properly ended",
            "you have an error in your sql",
            "odbc driver error",
            "jdbc driver error",
        ];
        if sql_error_indicators
            .iter()
            .any(|&ind| response_lower.contains(ind))
        {
            return true;
        }
    }

    if payload.to_lowercase().contains("<script>") && response_lower.contains("<script>") {
        return true;
    }

    let exception_indicators = [
        "unhandled exception",
        "java.lang.",
        "traceback (most recent",
        "stack trace:",
        "nullpointerexception",
        "typeerror:",
        "referenceerror:",
        "syntaxerror:",
    ];
    if exception_indicators
        .iter()
        .any(|&ind| response_lower.contains(ind))
    {
        return true;
    }

    if payload.contains("../") {
        let path_traversal_indicators = [
            "root:",
            "bin/bash",
            "bin/sh",
            "/etc/passwd",
            "/etc/shadow",
            "boot.ini",
            "win.ini",
        ];
        if path_traversal_indicators
            .iter()
            .any(|&ind| response_lower.contains(ind))
        {
            return true;
        }
    }

    false
}

#[cfg(test)]
#[cfg(feature = "websocket")]
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

    #[test]
    fn test_detect_injection_sql_error() {
        assert!(detect_injection_vulnerability(
            "'",
            Some("You have an error in your SQL syntax")
        ));
    }

    #[test]
    fn test_detect_injection_xss_reflection() {
        assert!(detect_injection_vulnerability(
            "<script>alert(1)</script>",
            Some("Received: <script>alert(1)</script>")
        ));
    }

    #[test]
    fn test_detect_injection_java_exception() {
        assert!(detect_injection_vulnerability(
            "'",
            Some("java.lang.NullPointerException at com.app")
        ));
    }

    #[test]
    fn test_detect_injection_python_traceback() {
        assert!(detect_injection_vulnerability(
            "'",
            Some("Traceback (most recent call last):")
        ));
    }

    #[test]
    fn test_detect_injection_path_traversal() {
        assert!(detect_injection_vulnerability(
            "../../../etc/passwd",
            Some("root:x:0:0:root:/root:/bin/bash")
        ));
    }

    #[test]
    fn test_no_false_positive_normal_error() {
        assert!(!detect_injection_vulnerability(
            "'",
            Some("rate limit exceeded")
        ));
    }

    #[test]
    fn test_no_false_positive_sql_in_name() {
        assert!(!detect_injection_vulnerability(
            "'",
            Some("Using SQL Server driver")
        ));
        assert!(!detect_injection_vulnerability(
            "'",
            Some("Using ODBC driver for connections")
        ));
        assert!(!detect_injection_vulnerability(
            "'",
            Some("JDBC connection pool initialized")
        ));
    }

    #[test]
    fn test_no_false_positive_generic_exception_word() {
        assert!(!detect_injection_vulnerability(
            "'",
            Some("This is an exception to the rule")
        ));
    }

    #[test]
    fn test_no_false_positive_path_in_text() {
        assert!(!detect_injection_vulnerability(
            "../../../etc/passwd",
            Some("The request path is invalid")
        ));
    }

    #[test]
    fn test_no_response() {
        assert!(!detect_injection_vulnerability("'", None));
    }
}
