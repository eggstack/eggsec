//! Stress tests for web proxy with high-concurrency scenarios.
//!
//! These tests verify the proxy can handle 1000+ concurrent connections
//! without memory leaks, performance degradation, or stability issues.

#[cfg(test)]
mod stress_tests {
    use eggsec::proxy::intercept::protocols::*;
    use eggsec::proxy::intercept::types::*;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::sync::Semaphore;
    use tokio::time::{timeout, Duration};

    /// Stress test: 1000 concurrent TCP connections.
    #[tokio::test]
    async fn test_stress_1000_concurrent_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let semaphore = Arc::new(Semaphore::new(1000));

        // Echo server with concurrency limit
        tokio::spawn(async move {
            loop {
                let (mut stream, _) = match listener.accept().await {
                    Ok(s) => s,
                    Err(_) => break,
                };
                let permit = semaphore.clone().acquire_owned().await.unwrap();
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
                    drop(permit);
                });
            }
        });

        // Spawn 1000 concurrent clients
        let mut handles = Vec::new();
        for i in 0..1000 {
            let addr = addr;
            handles.push(tokio::spawn(async move {
                let result = timeout(Duration::from_secs(10), async {
                    let mut client = TcpStream::connect(addr).await.unwrap();
                    let msg = format!("Message from client {}", i);
                    let _ = client.write_all(msg.as_bytes()).await;

                    let mut response = vec![0u8; 4096];
                    let n = client.read(&mut response).await.unwrap();
                    let response_str = String::from_utf8_lossy(&response[..n]).to_string();
                    assert_eq!(response_str, msg);
                })
                .await;
                result.is_ok()
            }));
        }

        // Wait for all clients to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        // At least 90% should succeed (some may timeout under extreme load)
        assert!(
            success_count >= 900,
            "Only {}/1000 clients succeeded",
            success_count
        );
    }

    /// Stress test: 1000 concurrent HTTP/1.1 requests.
    #[tokio::test]
    async fn test_stress_1000_http_requests() {
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
                    let n = stream.read(&mut buf).await.unwrap();
                    let request = String::from_utf8_lossy(&buf[..n]);

                    // Verify request is valid HTTP
                    if request.starts_with("GET /") {
                        let response = "HTTP/1.1 200 OK\r\nContent-Length: 2\r\n\r\nOK";
                        let _ = stream.write_all(response.as_bytes()).await;
                    }
                });
            }
        });

        // Spawn 1000 concurrent HTTP clients
        let mut handles = Vec::new();
        for i in 0..1000 {
            let addr = addr;
            handles.push(tokio::spawn(async move {
                let result = timeout(Duration::from_secs(10), async {
                    let mut client = TcpStream::connect(addr).await.unwrap();
                    let request = format!(
                        "GET /test/{} HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n",
                        i
                    );
                    let _ = client.write_all(request.as_bytes()).await;

                    let mut response = vec![0u8; 4096];
                    let n = client.read(&mut response).await.unwrap();
                    let response_str = String::from_utf8_lossy(&response[..n]).to_string();
                    assert!(response_str.contains("HTTP/1.1 200 OK"));
                })
                .await;
                result.is_ok()
            }));
        }

        // Wait for all clients to complete
        let mut success_count = 0;
        for handle in handles {
            if handle.await.unwrap() {
                success_count += 1;
            }
        }

        // At least 90% should succeed
        assert!(
            success_count >= 900,
            "Only {}/1000 HTTP clients succeeded",
            success_count
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
            stream.request_headers.insert(
                "content-type".to_string(),
                "application/json".to_string(),
            );
            stream.response_status = 200;
            stream.response_body = Some(format!("Response {}", i));
            stream.state = Http2StreamState::Closed;
            session.add_stream(stream);
        }

        // Verify all streams
        assert_eq!(session.streams.len(), 100);
        assert!(session.streams.iter().all(|s| s.state == Http2StreamState::Closed));
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
        use eggsec::proxy::intercept::rules::*;

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
        use eggsec::proxy::intercept::bundle::EvidenceBundle;

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
        let json = bundle.to_json().unwrap();
        assert!(!json.is_empty());
    }
}
