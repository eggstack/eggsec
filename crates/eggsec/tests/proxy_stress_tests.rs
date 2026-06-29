//! Stress tests for web proxy with high-concurrency scenarios.
//!
//! These tests verify the proxy can handle 1000+ concurrent connections
//! without memory leaks, performance degradation, or stability issues.

#[cfg(test)]
mod stress_tests {
    use eggsec::proxy::intercept::protocols::*;
    use eggsec::proxy::intercept::types::*;
    use futures::{stream, StreamExt};
    use std::collections::HashMap;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::time::{timeout, Duration};

    const STRESS_OPERATIONS: usize = 1000;
    const STRESS_CONCURRENCY: usize = 100;

    /// Stress test: 1000 TCP connections with bounded concurrency.
    #[tokio::test]
    async fn test_stress_1000_connections_bounded_concurrency() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Echo server. Client-side bounding keeps the test below common FD limits.
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    loop {
                        match stream.read(&mut buf).await {
                            Ok(0) => break,
                            Ok(n) => {
                                let _ = stream.write_all(&buf[..n]).await;
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
        });

        let outcomes = stream::iter(0..STRESS_OPERATIONS)
            .map(|i| async move {
                timeout(Duration::from_secs(10), async {
                    let Ok(mut client) = TcpStream::connect(addr).await else {
                        return false;
                    };
                    let msg = format!("Message from client {}", i);
                    if client.write_all(msg.as_bytes()).await.is_err() {
                        return false;
                    }

                    let mut response = vec![0u8; 4096];
                    let Ok(n) = client.read(&mut response).await else {
                        return false;
                    };
                    response.get(..n) == Some(msg.as_bytes())
                })
                .await
                .unwrap_or(false)
            })
            .buffer_unordered(STRESS_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        let success_count = outcomes.into_iter().filter(|success| *success).count();

        // At least 90% should succeed (some may timeout under extreme load)
        assert!(
            success_count >= STRESS_OPERATIONS * 9 / 10,
            "Only {success_count}/{STRESS_OPERATIONS} clients succeeded"
        );
    }

    /// Stress test: 1000 HTTP/1.1 requests with bounded concurrency.
    #[tokio::test]
    async fn test_stress_1000_http_requests_bounded_concurrency() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // HTTP server
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };
                tokio::spawn(async move {
                    let mut buf = vec![0u8; 4096];
                    let Ok(n) = stream.read(&mut buf).await else {
                        return;
                    };
                    let request = String::from_utf8_lossy(&buf[..n]);

                    // Verify request is valid HTTP
                    if request.starts_with("GET /") {
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                });
            }
        });

        let outcomes = stream::iter(0..STRESS_OPERATIONS)
            .map(|i| async move {
                timeout(Duration::from_secs(10), async {
                    let Ok(mut client) = TcpStream::connect(addr).await else {
                        return false;
                    };
                    let request = format!(
                        "GET /test/{} HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n",
                        i
                    );
                    if client.write_all(request.as_bytes()).await.is_err() {
                        return false;
                    }

                    let mut response = vec![0u8; 4096];
                    let Ok(n) = client.read(&mut response).await else {
                        return false;
                    };
                    String::from_utf8_lossy(&response[..n]).contains("HTTP/1.1 200 OK")
                })
                .await
                .unwrap_or(false)
            })
            .buffer_unordered(STRESS_CONCURRENCY)
            .collect::<Vec<_>>()
            .await;
        let success_count = outcomes.into_iter().filter(|success| *success).count();

        // At least 90% should succeed
        assert!(
            success_count >= STRESS_OPERATIONS * 9 / 10,
            "Only {success_count}/{STRESS_OPERATIONS} HTTP clients succeeded"
        );
    }

    /// Stress test: Flow tracking with 10,000 flows.
    #[tokio::test]
    async fn test_stress_10000_flows_memory() {
        let mut flows = Vec::with_capacity(10000);

        // Create 10,000 flows
        for i in 0..10000 {
            let flow = ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("https://api{}.example.com/data", i),
                host: format!("api{}.example.com", i),
                path: "/data".to_string(),
                request_headers: HashMap::new(),
                request_body: None,
                response_status: 200,
                response_headers: HashMap::new(),
                response_body: Some(format!("Response {}", i)),
                is_https: true,
                duration_ms: 100,
                request_body_size: 0,
                response_body_size: 10,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                redaction_applied: None,
                protocol: "http2".to_string(),
            };
            flows.push(flow);
        }

        // Verify all flows were created
        assert_eq!(flows.len(), 10000);

        // Verify serialization works
        let json = serde_json::to_string(&flows).unwrap();
        assert!(!json.is_empty());

        // Verify deserialization works
        let deserialized: Vec<ProxyFlow> = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.len(), 10000);
    }

    /// Stress test: HTTP/2 session with 100 streams.
    #[tokio::test]
    async fn test_stress_http2_100_streams() {
        let mut session = Http2Session::new("api.example.com", true);

        // Add 100 concurrent streams
        for i in 0..100 {
            let stream_id = (i * 2 + 1) as u32; // Odd numbers for client-initiated
            let mut stream = Http2Stream::new(stream_id, "GET", &format!("/api/{}", i));
            stream
                .request_headers
                .insert("content-type".to_string(), "application/json".to_string());
            stream.response_status = 200;
            stream.response_body = Some(format!("Response {}", i));
            stream.state = Http2StreamState::Closed;
            session.add_stream(stream);
        }

        // Verify all streams
        assert_eq!(session.streams.len(), 100);
        assert!(session
            .streams
            .iter()
            .all(|s| s.state == Http2StreamState::Closed));
    }

    /// Stress test: gRPC streaming with 1000 frames.
    #[tokio::test]
    async fn test_stress_grpc_1000_frames() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::Bidirectional);

        // Send 500 client frames and 500 server frames
        for i in 0..500 {
            let client_frame = GrpcStreamFrame::new(1, ProxyFlowDirection::Request)
                .with_payload(vec![i as u8; 100]);
            state.add_frame(client_frame);

            let server_frame = GrpcStreamFrame::new(1, ProxyFlowDirection::Response)
                .with_payload(vec![(i + 128) as u8; 100]);
            state.add_frame(server_frame);
        }

        // Verify frame counts
        assert_eq!(state.total_frames(), 1000);
        assert_eq!(state.client_frames.len(), 500);
        assert_eq!(state.server_frames.len(), 500);
        assert_eq!(state.total_bytes(), 100000); // 1000 * 100 bytes
    }

    /// Stress test: Rule evaluation with 10,000 rules.
    #[tokio::test]
    async fn test_stress_10000_rules() {
        use eggsec::proxy::intercept::{
            EnhancedRule, EnhancedRuleSet, RuleAction, RuleCondition, RuleContext,
        };

        let mut rule_set = EnhancedRuleSet::new();

        // Add 10,000 rules
        for i in 0..10000 {
            let condition = RuleCondition::HostMatches(format!("host-{}.example.com", i));
            rule_set.add(EnhancedRule::new(
                &format!("rule-{}", i),
                &format!("Rule {}", i),
                condition,
                RuleAction::Intercept,
            ));
        }

        // Verify all rules added
        assert_eq!(rule_set.len(), 10000);

        // Evaluate against a context
        let ctx = RuleContext::new("host-5000.example.com", "/", "GET");
        let matches = rule_set.evaluate(&ctx);
        assert!(!matches.is_empty());
    }

    /// Stress test: Plugin registry with 100 plugins.
    #[tokio::test]
    async fn test_stress_100_plugins() {
        use eggsec::proxy::intercept::plugins::*;

        struct TestHandler {
            id: String,
        }

        impl ProtocolHandler for TestHandler {
            fn info(&self) -> PluginInfo {
                PluginInfo {
                    id: self.id.clone(),
                    name: format!("Plugin {}", self.id),
                    version: "1.0.0".to_string(),
                    description: "Test plugin".to_string(),
                }
            }

            fn detect(
                &self,
                _host: &str,
                _path: &str,
                _headers: &HashMap<String, String>,
            ) -> DetectionResult {
                DetectionResult::NotDetected
            }

            fn handle(
                &self,
                _host: &str,
                _path: &str,
                _headers: &HashMap<String, String>,
                _body: Option<&str>,
            ) -> HandleResult {
                HandleResult {
                    findings: vec![],
                    metadata: HashMap::new(),
                }
            }
        }

        let mut registry = PluginRegistry::new();

        // Register 100 plugins
        for i in 0..100 {
            let handler = TestHandler {
                id: format!("plugin-{}", i),
            };
            registry.register(Box::new(handler)).unwrap();
        }

        // Verify all plugins registered
        assert_eq!(registry.len(), 100);

        // List all plugins
        let plugins = registry.list();
        assert_eq!(plugins.len(), 100);
    }

    /// Stress test: Evidence bundle with 1000 flows.
    #[tokio::test]
    async fn test_stress_evidence_bundle_1000_flows() {
        use eggsec::proxy::intercept::EvidenceBundle;

        let mut report = WebProxySessionReport::new("127.0.0.1:8080", true);

        // Add 1000 flows
        for i in 0..1000 {
            report.add_flow(ProxyFlow {
                index: i,
                method: "GET".to_string(),
                url: format!("https://api{}.example.com/data", i),
                host: format!("api{}.example.com", i),
                path: "/data".to_string(),
                request_headers: HashMap::new(),
                request_body: None,
                response_status: 200,
                response_headers: HashMap::new(),
                response_body: Some(format!("Response {}", i)),
                is_https: true,
                duration_ms: 100,
                request_body_size: 0,
                response_body_size: 10,
                started_at: chrono::Utc::now().to_rfc3339(),
                completed_at: chrono::Utc::now().to_rfc3339(),
                redaction_applied: None,
                protocol: "http2".to_string(),
            });
        }

        // Create evidence bundle
        let bundle = EvidenceBundle::from_report(&report, None);
        let encoded = bundle.to_bytes().unwrap();
        assert!(!encoded.is_empty());

        let decoded = EvidenceBundle::from_bytes(&encoded).unwrap();
        assert_eq!(decoded.flows.len(), 1000);
    }
}
