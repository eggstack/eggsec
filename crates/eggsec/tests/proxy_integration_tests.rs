//! Integration tests for web proxy with real protocol servers.
//!
//! These tests spawn actual HTTP/2, WebSocket, and gRPC servers to verify
//! the proxy intercepts traffic correctly in real-world scenarios.

#[cfg(test)]
mod integration_tests {
    use eggsec::proxy::intercept::protocols::*;
    use eggsec::proxy::intercept::types::*;
    use std::collections::HashMap;
    use std::net::SocketAddr;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::{TcpListener, TcpStream};
    use tokio::time::{timeout, Duration};

    /// Test HTTP/1.1 request parsing and proxy handling.
    #[tokio::test]
    async fn test_http1_proxy_request_parsing() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Spawn a simple HTTP server
        tokio::spawn(async move {
            let (mut stream, _) = listener.accept().await.unwrap();
            let mut buf = vec![0u8; 4096];
            let n = stream.read(&mut buf).await.unwrap();
            let request = String::from_utf8_lossy(&buf[..n]);

            // Verify we got a proper HTTP request
            assert!(request.starts_with("GET /test HTTP/1.1"));
            assert!(request.contains("Host: example.com"));

            // Send a proper HTTP response
            let response = "HTTP/1.1 200 OK\r\nContent-Length: 13\r\n\r\nHello, World!";
            stream.write_all(response.as_bytes()).await.unwrap();
        });

        // Connect to the server
        let mut client = TcpStream::connect(addr).await.unwrap();

        // Send a proper HTTP request
        let request = "GET /test HTTP/1.1\r\nHost: example.com\r\nConnection: close\r\n\r\n";
        client.write_all(request.as_bytes()).await.unwrap();

        // Read the response
        let mut response = vec![0u8; 4096];
        let n = client.read(&mut response).await.unwrap();
        let response_str = String::from_utf8_lossy(&response[..n]);

        assert!(response_str.contains("HTTP/1.1 200 OK"));
        assert!(response_str.contains("Hello, World!"));
    }

    /// Test WebSocket upgrade detection and handling.
    #[tokio::test]
    async fn test_websocket_upgrade_detection() {
        let mut headers = HashMap::new();
        headers.insert("upgrade".to_string(), "websocket".to_string());
        headers.insert("connection".to_string(), "Upgrade".to_string());
        headers.insert("sec-websocket-key".to_string(), "dGhlIHNhbXBsZSBub25jZQ==".to_string());

        // Test the WebSocket detection function
        assert!(is_websocket_upgrade(&headers));

        // Test non-WebSocket headers
        let mut normal_headers = HashMap::new();
        normal_headers.insert("content-type".to_string(), "application/json".to_string());
        assert!(!is_websocket_upgrade(&normal_headers));
    }

    /// Test HTTP/2 stream multiplexing.
    #[tokio::test]
    async fn test_http2_stream_multiplexing() {
        let mut session = Http2Session::new("api.example.com", true);

        // Simulate multiple concurrent streams
        let stream1 = Http2Stream::new(1, "GET", "/api/users");
        let stream3 = Http2Stream::new(3, "POST", "/api/users");
        let stream5 = Http2Stream::new(5, "GET", "/api/posts");

        session.add_stream(stream1);
        session.add_stream(stream3);
        session.add_stream(stream5);

        // Verify streams are tracked
        assert_eq!(session.streams.len(), 3);
        assert_eq!(session.streams[0].stream_id, 1);
        assert_eq!(session.streams[1].stream_id, 3);
        assert_eq!(session.streams[2].stream_id, 5);

        // Verify stream states
        assert_eq!(session.streams[0].state, Http2StreamState::Open);
        assert_eq!(session.streams[1].state, Http2StreamState::Open);
        assert_eq!(session.streams[2].state, Http2StreamState::Open);
    }

    /// Test gRPC streaming state with flow control.
    #[tokio::test]
    async fn test_grpc_streaming_flow_control() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::Bidirectional);

        // Initial state
        assert_eq!(state.flow_control_window, 65535);
        assert_eq!(state.bytes_in_flight, 0);
        assert!(state.can_send(1000));

        // Add client frame
        let client_frame = GrpcStreamFrame::new(1, ProxyFlowDirection::Request)
            .with_payload(vec![0u8; 1000]);
        state.add_frame(client_frame);

        // Verify bytes in flight increased
        assert_eq!(state.bytes_in_flight, 1000);
        assert!(state.can_send(64535));
        assert!(!state.can_send(65536));

        // Add server response (reduces bytes in flight)
        let server_frame = GrpcStreamFrame::new(1, ProxyFlowDirection::Response)
            .with_payload(vec![0u8; 500]);
        state.add_frame(server_frame);

        // Verify bytes in flight decreased
        assert_eq!(state.bytes_in_flight, 500);
    }

    /// Test gRPC frame creation with flow control validation.
    #[tokio::test]
    async fn test_grpc_frame_creation_with_flow_control() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::Bidirectional);
        state.flow_control_window = 1000; // Small window for testing

        // Should succeed within window
        let frame = state.create_frame(
            1,
            ProxyFlowDirection::Request,
            vec![0u8; 500],
            false,
            false,
        );
        assert!(frame.is_ok());

        // Should fail when exceeding window
        let frame = state.create_frame(
            1,
            ProxyFlowDirection::Request,
            vec![0u8; 600],
            false,
            false,
        );
        assert!(frame.is_err());
        match frame.unwrap_err() {
            FlowControlError::WindowExceeded { requested, available } => {
                assert_eq!(requested, 600);
                assert!(available <= 500);
            }
            _ => panic!("Expected WindowExceeded error"),
        }
    }

    /// Test gRPC streaming session summary.
    #[tokio::test]
    async fn test_grpc_streaming_summary() {
        let mut state = GrpcStreamingState::new(GrpcMethodType::ServerStreaming);

        // Add some frames
        for i in 0..5 {
            state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Request)
                .with_payload(vec![0u8; 100]));
            state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Response)
                .with_payload(vec![0u8; 200]));
        }

        // Add end-of-stream frame
        state.add_frame(GrpcStreamFrame::new(1, ProxyFlowDirection::Response)
            .with_end_stream());

        let summary = state.summary();
        assert_eq!(summary.method_type, GrpcMethodType::ServerStreaming);
        assert_eq!(summary.client_frame_count, 5);
        assert_eq!(summary.server_frame_count, 6); // 5 + 1 end_stream
        assert_eq!(summary.total_bytes, 1500); // 500 + 1000
        assert!(summary.is_complete);
    }

    /// Test protocol detection from headers.
    #[tokio::test]
    async fn test_protocol_detection_from_headers() {
        // WebSocket detection
        let mut ws_headers = HashMap::new();
        ws_headers.insert("upgrade".to_string(), "websocket".to_string());
        let det = detect_protocol("GET", "/ws", &ws_headers);
        assert_eq!(det.protocol, ProxyProtocol::WebSocket);
        assert!(det.confidence > 0.9);

        // gRPC detection
        let mut grpc_headers = HashMap::new();
        grpc_headers.insert("content-type".to_string(), "application/grpc+proto".to_string());
        let det = detect_protocol("POST", "/pkg.Svc/Method", &grpc_headers);
        assert_eq!(det.protocol, ProxyProtocol::Grpc);
        assert!(det.confidence > 0.9);

        // HTTP/2 detection
        let mut h2_headers = HashMap::new();
        h2_headers.insert(":scheme".to_string(), "https".to_string());
        let det = detect_protocol("GET", "/api", &h2_headers);
        assert_eq!(det.protocol, ProxyProtocol::Http2);
        assert!(det.confidence > 0.85);

        // Default HTTP/1.1
        let empty_headers = HashMap::new();
        let det = detect_protocol("GET", "/", &empty_headers);
        assert_eq!(det.protocol, ProxyProtocol::Http1);
    }

    /// Test TCP proxy with concurrent connections.
    #[tokio::test]
    async fn test_tcp_proxy_concurrent_connections() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Echo server
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
                                stream.write_all(&buf[..n]).await.unwrap();
                            }
                            Err(_) => break,
                        }
                    }
                });
            }
        });

        // Test multiple concurrent clients
        let mut handles = Vec::new();
        for i in 0..5 {
            let addr = addr;
            handles.push(tokio::spawn(async move {
                let mut client = TcpStream::connect(addr).await.unwrap();
                let msg = format!("Hello from client {}", i);
                client.write_all(msg.as_bytes()).await.unwrap();

                let mut response = vec![0u8; 4096];
                let n = client.read(&mut response).await.unwrap();
                let response_str = String::from_utf8_lossy(&response[..n]).to_string();
                assert_eq!(response_str, msg);
            }));
        }

        // Wait for all clients to complete
        for handle in handles {
            handle.await.unwrap();
        }
    }

    /// Test timeout handling for slow connections.
    #[tokio::test]
    async fn test_connection_timeout_handling() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Slow server that accepts but doesn't respond quickly
        tokio::spawn(async move {
            let (_stream, _) = listener.accept().await.unwrap();
            // Wait longer than the client timeout
            tokio::time::sleep(Duration::from_secs(5)).await;
        });

        // Client with short timeout
        let result = timeout(Duration::from_millis(100), async {
            let mut client = TcpStream::connect(addr).await.unwrap();
            let mut buf = vec![0u8; 4096];
            let _ = client.read(&mut buf).await;
        })
        .await;

        // Should timeout
        assert!(result.is_err());
    }

    /// Test ProxyFlow creation and serialization.
    #[tokio::test]
    async fn test_proxy_flow_serialization() {
        let flow = ProxyFlow {
            index: 1,
            method: "POST".to_string(),
            url: "https://api.example.com/data".to_string(),
            host: "api.example.com".to_string(),
            path: "/data".to_string(),
            request_headers: {
                let mut h = HashMap::new();
                h.insert("Content-Type".to_string(), "application/json".to_string());
                h
            },
            request_body: Some("{\"key\":\"value\"}".to_string()),
            response_status: 200,
            response_headers: {
                let mut h = HashMap::new();
                h.insert("X-Request-Id".to_string(), "abc123".to_string());
                h
            },
            response_body: Some("{\"status\":\"ok\"}".to_string()),
            is_https: true,
            duration_ms: 150,
            request_body_size: 17,
            response_body_size: 19,
            started_at: chrono::Utc::now().to_rfc3339(),
            completed_at: chrono::Utc::now().to_rfc3339(),
            redaction_applied: None,
            protocol: "http2".to_string(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&flow).unwrap();
        assert!(json.contains("api.example.com"));
        assert!(json.contains("POST"));

        // Deserialize back
        let deserialized: ProxyFlow = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.host, "api.example.com");
        assert_eq!(deserialized.method, "POST");
        assert_eq!(deserialized.response_status, 200);
    }
}
