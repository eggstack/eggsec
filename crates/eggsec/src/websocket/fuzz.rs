use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzTestResult {
    pub test_name: String,
    pub payload_size: usize,
    pub sent: bool,
    pub connection_dropped: bool,
    pub server_response: Option<String>,
    pub vulnerability_detected: bool,
    pub details: String,
}

#[cfg(feature = "websocket")]
pub async fn test_dos(url: &str, timeout_secs: u64) -> Vec<FuzzTestResult> {
    tracing::info!("Running WebSocket DoS tests on {}", url);
    let mut results = Vec::new();

    results.push(test_large_message(url, timeout_secs).await);
    results.push(test_ping_flood(url, timeout_secs).await);
    results.push(test_rapid_close(url, timeout_secs).await);

    results
}

#[cfg(feature = "websocket")]
pub async fn test_message_fuzz(url: &str, timeout_secs: u64) -> Vec<FuzzTestResult> {
    tracing::info!("Running WebSocket message fuzzing on {}", url);
    let mut results = Vec::new();

    let fuzz_cases = vec![
        ("", "Empty message"),
        ("\0\0\0", "Null bytes"),
        ("\x01\x02\x03", "Control chars"),
        ("{{{\"a\":1}}", "Template-like"),
        ("<script>alert(1)</script>", "XSS-like"),
        ("' OR 1=1--", "SQLi-like"),
        ("{}", "Empty object"),
    ];

    for (payload, name) in fuzz_cases {
        results.push(test_single_message_fuzz(url, payload, name, timeout_secs).await);
    }

    results
}

#[cfg(feature = "websocket")]
async fn test_large_message(url: &str, timeout_secs: u64) -> FuzzTestResult {
    use futures::{SinkExt, StreamExt};

    let payload_size = 65536;
    let payload = "a".repeat(payload_size);

    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async(url),
    )
    .await;

    let mut ws = match connect_result {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => {
            return FuzzTestResult {
                test_name: "Large message".to_string(),
                payload_size,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection failed: {}", e),
            };
        }
        Err(_) => {
            return FuzzTestResult {
                test_name: "Large message".to_string(),
                payload_size,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection timed out after {}s", timeout_secs),
            };
        }
    };

    let send_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        ws.send(tokio_tungstenite::tungstenite::Message::Text(
            payload.into(),
        )),
    )
    .await;

    let sent = matches!(send_result, Ok(Ok(())));

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let recv_result = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;

    let (server_response, connection_dropped) = match recv_result {
        Ok(Some(Ok(msg))) => {
            let resp = match &msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => Some(t.to_string()),
                tokio_tungstenite::tungstenite::Message::Close(_) => None,
                _ => None,
            };
            (resp, false)
        }
        Ok(Some(Err(_))) => (None, true),
        Ok(None) => (None, true),
        Err(_) => (None, true),
    };

    FuzzTestResult {
        test_name: "Large message".to_string(),
        payload_size,
        sent,
        connection_dropped,
        server_response,
        vulnerability_detected: false,
        details: if connection_dropped {
            "Server closed connection after large message (expected behavior)".to_string()
        } else {
            "Server handled large message".to_string()
        },
    }
}

#[cfg(feature = "websocket")]
async fn test_ping_flood(url: &str, timeout_secs: u64) -> FuzzTestResult {
    use futures::SinkExt;

    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async(url),
    )
    .await;

    let mut ws = match connect_result {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => {
            return FuzzTestResult {
                test_name: "Ping flood".to_string(),
                payload_size: 0,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection failed: {}", e),
            };
        }
        Err(_) => {
            return FuzzTestResult {
                test_name: "Ping flood".to_string(),
                payload_size: 0,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection timed out after {}s", timeout_secs),
            };
        }
    };

    let mut sent_count = 0u32;
    let mut connection_dropped = false;

    for _ in 0..50 {
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            ws.send(tokio_tungstenite::tungstenite::Message::Ping(
                vec![0u8; 64].into(),
            )),
        )
        .await;

        match result {
            Ok(Ok(())) => sent_count += 1,
            _ => {
                connection_dropped = true;
                break;
            }
        }
    }

    FuzzTestResult {
        test_name: "Ping flood".to_string(),
        payload_size: (sent_count * 64) as usize,
        sent: sent_count > 0,
        connection_dropped,
        server_response: None,
        vulnerability_detected: false,
        details: format!(
            "Sent {} pings, connection {}",
            sent_count,
            if connection_dropped {
                "closed (expected rate-limiting)"
            } else {
                "survived"
            }
        ),
    }
}

#[cfg(feature = "websocket")]
async fn test_rapid_close(url: &str, timeout_secs: u64) -> FuzzTestResult {
    use futures::SinkExt;

    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async(url),
    )
    .await;

    let mut ws = match connect_result {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => {
            return FuzzTestResult {
                test_name: "Rapid close".to_string(),
                payload_size: 0,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection failed: {}", e),
            };
        }
        Err(_) => {
            return FuzzTestResult {
                test_name: "Rapid close".to_string(),
                payload_size: 0,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection timed out after {}s", timeout_secs),
            };
        }
    };

    let mut sent_count = 0u32;
    let mut connection_dropped = false;

    for _ in 0..10 {
        let result = tokio::time::timeout(
            std::time::Duration::from_millis(100),
            ws.send(tokio_tungstenite::tungstenite::Message::Close(Some(
                tokio_tungstenite::tungstenite::protocol::CloseFrame {
                    code:
                        tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode::Normal,
                    reason: "test".into(),
                },
            ))),
        )
        .await;

        match result {
            Ok(Ok(())) => sent_count += 1,
            Ok(Err(_)) => {
                connection_dropped = true;
                break;
            }
            Err(_) => {
                connection_dropped = true;
                break;
            }
        }
    }

    if !connection_dropped {
        use futures::StreamExt;
        let recv_result =
            tokio::time::timeout(std::time::Duration::from_millis(500), ws.next()).await;
        match recv_result {
            Ok(Some(Err(_))) | Ok(None) => connection_dropped = true,
            _ => {}
        }
    }

    FuzzTestResult {
        test_name: "Rapid close".to_string(),
        payload_size: 0,
        sent: sent_count > 0,
        connection_dropped,
        server_response: None,
        vulnerability_detected: false,
        details: format!(
            "Sent {} close frames, connection {}",
            sent_count,
            if connection_dropped {
                "closed"
            } else {
                "alive"
            }
        ),
    }
}

#[cfg(feature = "websocket")]
async fn test_single_message_fuzz(
    url: &str,
    payload: &str,
    name: &str,
    timeout_secs: u64,
) -> FuzzTestResult {
    use futures::{SinkExt, StreamExt};

    let payload_size = payload.len();

    let connect_result = tokio::time::timeout(
        std::time::Duration::from_secs(timeout_secs),
        tokio_tungstenite::connect_async(url),
    )
    .await;

    let mut ws = match connect_result {
        Ok(Ok((ws, _))) => ws,
        Ok(Err(e)) => {
            return FuzzTestResult {
                test_name: name.to_string(),
                payload_size,
                sent: false,
                connection_dropped: false,
                server_response: None,
                vulnerability_detected: false,
                details: format!("Connection failed: {}", e),
            };
        }
        Err(_) => {
            return FuzzTestResult {
                test_name: name.to_string(),
                payload_size,
                sent: false,
                connection_dropped: false,
                server_response: None,
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

    let sent = matches!(send_result, Ok(Ok(())));

    let recv_result = tokio::time::timeout(std::time::Duration::from_secs(2), ws.next()).await;

    match recv_result {
        Ok(Some(Ok(msg))) => {
            let server_response = match &msg {
                tokio_tungstenite::tungstenite::Message::Text(t) => Some(t.to_string()),
                _ => None,
            };

            let vulnerability_detected = server_response
                .as_ref()
                .map(|r| {
                    let lower = r.to_lowercase();
                    lower.contains("unhandled exception")
                        || lower.contains("internal server error")
                        || lower.contains("server error")
                        || lower.contains("500 ")
                        || lower.contains("exception:")
                        || lower.contains("stacktrace:")
                        || lower.contains("stack trace:")
                        || lower.contains("traceback")
                        || lower.contains("nullpointerexception")
                        || lower.contains("syntaxerror:")
                        || lower.contains("typeerror:")
                        || lower.contains("referenceerror:")
                })
                .unwrap_or(false);

            if vulnerability_detected {
                tracing::warn!(
                    "WebSocket fuzz vulnerability detected with payload '{}': server returned error response",
                    name
                );
            }

            FuzzTestResult {
                test_name: name.to_string(),
                payload_size,
                sent,
                connection_dropped: false,
                server_response,
                vulnerability_detected,
                details: if vulnerability_detected {
                    "Server returned error-like response".to_string()
                } else {
                    "Server responded normally".to_string()
                },
            }
        }
        Ok(Some(Err(_))) | Ok(None) => FuzzTestResult {
            test_name: name.to_string(),
            payload_size,
            sent,
            connection_dropped: true,
            server_response: None,
            vulnerability_detected: false,
            details: "Connection closed after fuzz message".to_string(),
        },
        Err(_) => FuzzTestResult {
            test_name: name.to_string(),
            payload_size,
            sent,
            connection_dropped: false,
            server_response: None,
            vulnerability_detected: false,
            details: "No response within timeout".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_result_creation() {
        let result = FuzzTestResult {
            test_name: "Large message".to_string(),
            payload_size: 1000,
            sent: true,
            connection_dropped: false,
            server_response: None,
            vulnerability_detected: false,
            details: "Test".to_string(),
        };
        assert_eq!(result.payload_size, 1000);
    }
}
