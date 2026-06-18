use super::*;

#[test]
fn test_intercept_tab_new() {
    let tab = InterceptTab::new();
    assert!(tab.flows.is_empty());
    assert!(tab.manipulation_history.is_empty());
    assert_eq!(tab.state, AppState::Idle);
    assert!(tab.dry_run);
    assert_eq!(tab.listen_addr, "127.0.0.1:8080");
}

#[test]
fn test_intercept_tab_start_stop() {
    let mut tab = InterceptTab::new();
    tab.start_session();
    assert_eq!(tab.state, AppState::Running);
    assert!(tab.session.is_some());

    tab.stop_session();
    assert_eq!(tab.state, AppState::Idle);
    assert!(tab.session.as_ref().unwrap().ended_at != tab.session.as_ref().unwrap().started_at);
}

#[test]
fn test_intercept_tab_add_flow() {
    let mut tab = InterceptTab::new();
    let flow = ProxyFlow {
        index: 0,
        method: "GET".to_string(),
        url: "https://example.com/".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        request_headers: Default::default(),
        request_body: None,
        response_status: 200,
        response_headers: Default::default(),
        response_body: None,
        is_https: true,
        duration_ms: 100,
        request_body_size: 0,
        response_body_size: 512,
        started_at: chrono::Utc::now().to_rfc3339(),
        completed_at: chrono::Utc::now().to_rfc3339(),
        redaction_applied: None,
        protocol: "http1".to_string(),
    };
    tab.add_flow(flow);
    assert_eq!(tab.flows.len(), 1);
    assert_eq!(tab.selected_flow, Some(0));
}

#[test]
fn test_intercept_tab_focus_navigation() {
    let mut tab = InterceptTab::new();
    assert_eq!(tab.focus_area, InterceptFocusArea::FlowList);
    tab.handle_focus_next();
    assert_eq!(tab.focus_area, InterceptFocusArea::DetailView);
    tab.handle_focus_next();
    assert_eq!(tab.focus_area, InterceptFocusArea::ActionBar);
    tab.handle_focus_next();
    assert_eq!(tab.focus_area, InterceptFocusArea::FlowList);
}

#[test]
fn test_intercept_tab_action_bar_navigation() {
    let mut tab = InterceptTab::new();
    tab.focus_area = InterceptFocusArea::ActionBar;
    assert_eq!(tab.action_bar_index, 0);
    tab.handle_right();
    assert_eq!(tab.action_bar_index, 1);
    tab.handle_right();
    assert_eq!(tab.action_bar_index, 2);
    tab.handle_left();
    assert_eq!(tab.action_bar_index, 1);
}

#[test]
fn test_truncate_str() {
    assert_eq!(truncate_str("hello", 10), "hello");
    assert_eq!(truncate_str("hello world", 8), "hello...");
}

#[test]
fn test_format_bytes() {
    assert_eq!(format_bytes(512), "512B");
    assert_eq!(format_bytes(1536), "1.5K");
    assert_eq!(format_bytes(2097152), "2.0M");
}

#[test]
fn test_visible_flows() {
    let mut tab = InterceptTab::new();
    for i in 0..10 {
        tab.flows.push(ProxyFlow {
            index: i,
            method: "GET".to_string(),
            url: format!("https://example.com/{}", i),
            host: "example.com".to_string(),
            path: format!("/{}", i),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: true,
            duration_ms: 0,
            request_body_size: 0,
            response_body_size: 0,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
    }

    tab.scroll_offset = 0;
    assert_eq!(tab.visible_flows(5).len(), 5);
    assert_eq!(tab.visible_flows(5)[0].index, 0);

    tab.scroll_offset = 7;
    assert_eq!(tab.visible_flows(5).len(), 3);
    assert_eq!(tab.visible_flows(5)[0].index, 7);

    tab.scroll_offset = 100;
    assert!(tab.visible_flows(5).is_empty());
}

#[test]
fn test_scroll_offset_maintained_on_nav() {
    let mut tab = InterceptTab::new();
    for i in 0..30 {
        tab.flows.push(ProxyFlow {
            index: i,
            method: "GET".to_string(),
            url: format!("https://example.com/{}", i),
            host: "example.com".to_string(),
            path: format!("/{}", i),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: false,
            duration_ms: 0,
            request_body_size: 0,
            response_body_size: 0,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
    }

    tab.selected_flow = Some(0);
    tab.table_state.select(Some(0));

    tab.handle_down();
    assert_eq!(tab.scroll_offset, 0);
    assert_eq!(tab.selected_flow, Some(1));

    tab.handle_up();
    assert_eq!(tab.scroll_offset, 0);
    assert_eq!(tab.selected_flow, Some(0));
}

#[test]
fn test_performance_mode_toggle() {
    let mut tab = InterceptTab::new();
    assert!(!tab.performance_mode);
    tab.toggle_performance_mode();
    assert!(tab.performance_mode);
    tab.toggle_performance_mode();
    assert!(!tab.performance_mode);
}

#[test]
fn test_estimate_memory_usage() {
    let mut tab = InterceptTab::new();
    assert_eq!(tab.estimate_memory_usage(), 0);

    tab.flows.push(ProxyFlow {
        index: 0,
        method: "GET".to_string(),
        url: "https://example.com/".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        request_headers: Default::default(),
        request_body: Some("body".to_string()),
        response_status: 200,
        response_headers: Default::default(),
        response_body: Some("response".to_string()),
        is_https: true,
        duration_ms: 0,
        request_body_size: 0,
        response_body_size: 0,
        started_at: String::new(),
        completed_at: String::new(),
        redaction_applied: None,
        protocol: "http1".to_string(),
    });

    let usage = tab.estimate_memory_usage();
    assert!(usage > 0);
}

#[test]
fn test_ws_messages_page() {
    let mut tab = InterceptTab::new();
    assert!(tab.ws_messages_page(0, 0, 10).is_empty());
    assert_eq!(tab.ws_session_message_count(0), 0);

    let mut session = WebSocketSession::new("ws://example.com", "example.com", "/ws", false);
    for i in 0..25 {
        session.messages.push(WebSocketMessage {
            direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
            opcode: WebSocketOpcode::Text,
            payload: format!("msg{}", i),
            masked: false,
            payload_size: 0,
            timestamp: String::new(),
            manipulation: None,
        });
    }
    tab.ws_sessions.push(session);

    assert_eq!(tab.ws_messages_page(0, 0, 10).len(), 10);
    assert_eq!(tab.ws_messages_page(0, 0, 10)[0].payload, "msg0");
    assert_eq!(tab.ws_messages_page(0, 1, 10).len(), 10);
    assert_eq!(tab.ws_messages_page(0, 1, 10)[0].payload, "msg10");
    assert_eq!(tab.ws_messages_page(0, 2, 10).len(), 5);
    assert_eq!(tab.ws_messages_page(0, 2, 10)[0].payload, "msg20");
    assert!(tab.ws_messages_page(0, 3, 10).is_empty());
    assert!(tab.ws_messages_page(1, 0, 10).is_empty());
    assert_eq!(tab.ws_session_message_count(0), 25);
}

#[test]
fn test_debounce_state() {
    let mut debounce = DebounceState::new();
    assert!(!debounce.should_apply());
    assert!(debounce.take_if_ready().is_none());

    debounce.enqueue("test".to_string());
    assert!(debounce.pending_filter.is_some());
    // Immediately after enqueue, debounce should not be ready
    assert!(!debounce.should_apply());
}

#[test]
fn test_debounce_take_if_ready() {
    let mut debounce = DebounceState::new();
    debounce.enqueue("filter".to_string());
    assert!(debounce.take_if_ready().is_none());
}

#[test]
fn test_performance_mode_detail_pane() {
    let mut tab = InterceptTab::new();
    tab.flows.push(ProxyFlow {
        index: 0,
        method: "GET".to_string(),
        url: "https://example.com/".to_string(),
        host: "example.com".to_string(),
        path: "/".to_string(),
        request_headers: Default::default(),
        request_body: None,
        response_status: 200,
        response_headers: Default::default(),
        response_body: None,
        is_https: true,
        duration_ms: 0,
        request_body_size: 0,
        response_body_size: 0,
        started_at: String::new(),
        completed_at: String::new(),
        redaction_applied: None,
        protocol: "http1".to_string(),
    });
    tab.selected_flow = Some(0);

    tab.toggle_performance_mode();
    assert!(tab.performance_mode);
    assert!(tab.cached_detail.is_none());
}

#[test]
fn test_reset_clears_new_fields() {
    let mut tab = InterceptTab::new();
    tab.scroll_offset = 5;
    tab.performance_mode = true;
    tab.cached_detail = Some((0, DetailPaneContent::Headers(vec![])));
    tab.reset();
    assert_eq!(tab.scroll_offset, 0);
    assert!(!tab.performance_mode);
    assert!(tab.cached_detail.is_none());
    assert!(tab.ws_sessions.is_empty());
}

#[test]
fn test_ensure_selected_visible() {
    let mut tab = InterceptTab::new();
    for i in 0..20 {
        tab.flows.push(ProxyFlow {
            index: i,
            method: "GET".to_string(),
            url: "https://example.com/".to_string(),
            host: "example.com".to_string(),
            path: "/".to_string(),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: false,
            duration_ms: 0,
            request_body_size: 0,
            response_body_size: 0,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
    }

    tab.selected_flow = Some(15);
    tab.scroll_offset = 0;
    tab.ensure_selected_visible(10);
    assert_eq!(tab.scroll_offset, 6);

    tab.selected_flow = Some(0);
    tab.scroll_offset = 10;
    tab.ensure_selected_visible(10);
    assert_eq!(tab.scroll_offset, 0);
}

#[test]
fn test_stress_10000_flows_memory_estimate() {
    let mut tab = InterceptTab::new();

    // Add 10,000 flows with realistic payload sizes
    for i in 0..10_000 {
        tab.flows.push(ProxyFlow {
index: i,
method: if i % 2 == 0 { "GET" } else { "POST" }.to_string(),
url: format!("https://example.com/api/endpoint/{}", i),
host: "example.com".to_string(),
path: format!("/api/endpoint/{}", i),
request_headers: {
let mut h = std::collections::HashMap::new();
h.insert("User-Agent".to_string(), "stress-test/1.0".to_string());
h.insert("Accept".to_string(), "application/json".to_string());
h
},
request_body: if i % 2 == 0 {
None
} else {
// 500 bytes per request body
Some(format!(
    r#"{{"id": {}, "data": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Payload padding: {}"}}"#,
    i, "X".repeat(200)
))
},
response_status: if i % 10 == 0 { 404 } else { 200 },
response_headers: {
let mut h = std::collections::HashMap::new();
h.insert("Content-Type".to_string(), "application/json".to_string());
h
},
// 500 bytes per response body
response_body: Some(format!(
r#"{{"status": "ok", "id": {}, "result": "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua. Ut enim ad minim veniam, quis nostrud exercitation ullamco laboris nisi ut aliquip ex ea commodo consequat. Duis aute irure dolor in reprehenderit in voluptate velit esse cillum dolore eu fugiat nulla pariatur. Excepteur sint occaecat cupidatat non proident, sunt in culpa qui officia deserunt mollit anim id est laborum. Payload: {}"}}"#,
i, "Y".repeat(100)
)),
is_https: true,
duration_ms: (i as u64) % 1000,
request_body_size: if i % 2 == 0 { 0 } else { 500 },
response_body_size: 500,
started_at: chrono::Utc::now().to_rfc3339(),
completed_at: chrono::Utc::now().to_rfc3339(),
redaction_applied: None,
protocol: "http1".to_string(),
});
    }

    // Verify memory estimate is reasonable (>10MB for 10k flows with 500B payloads)
    // Each flow has: url (~50B) + method (~4B) + host (11B) + path (~18B)
    //   + request_headers (~35B) + response_headers (~30B)
    //   + request_body (500B for odd) + response_body (500B)
    // Per flow avg: ~650B, 10k flows = ~6.5MB, plus overhead
    let usage = tab.estimate_memory_usage();
    assert!(
        usage > 5 * 1024 * 1024,
        "Memory estimate should be >5MB for 10k flows with 500B payloads, got {} bytes",
        usage
    );

    // Verify virtual scrolling works with large flow count
    let visible = tab.visible_flows(20);
    assert_eq!(visible.len(), 20);
    assert_eq!(visible[0].index, 0);

    tab.scroll_offset = 9980;
    let visible = tab.visible_flows(20);
    assert_eq!(visible.len(), 20);
    assert_eq!(visible[0].index, 9980);

    // Verify navigation works
    tab.selected_flow = Some(5000);
    tab.ensure_selected_visible(20);
    assert!(tab.scroll_offset <= 5000);
    assert!(tab.scroll_offset + 20 > 5000);
}

#[test]
fn test_stress_10000_flows_performance_mode_auto_enables() {
    let mut tab = InterceptTab::new();

    // Add flows up to the threshold using add_flow() which triggers auto-enable
    for i in 0..5001 {
        tab.add_flow(ProxyFlow {
            index: i,
            method: "GET".to_string(),
            url: format!("http://example.com/{}", i),
            host: "example.com".to_string(),
            path: format!("/{}", i),
            request_headers: Default::default(),
            request_body: None,
            response_status: 200,
            response_headers: Default::default(),
            response_body: None,
            is_https: false,
            duration_ms: 0,
            request_body_size: 0,
            response_body_size: 0,
            started_at: String::new(),
            completed_at: String::new(),
            redaction_applied: None,
            protocol: "http1".to_string(),
        });
    }

    // Performance mode should auto-enable after 5000 flows
    assert!(
        tab.performance_mode,
        "Performance mode should auto-enable when flows exceed 5000"
    );
}

// ==================== Stream Multiplexing Visualization Tests ====================

#[test]
fn test_http2_streams_for_session_sorted() {
    let mut tab = InterceptTab::new();
    let mut session = Http2Session::new("api.example.com", true);

    // Add streams in non-sorted order
    session.add_stream(Http2Stream::new(5, "GET", "/b"));
    session.add_stream(Http2Stream::new(1, "GET", "/a"));
    session.add_stream(Http2Stream::new(3, "POST", "/c"));

    tab.add_http2_session(session);

    let streams = tab.http2_streams_for_session(0);
    assert_eq!(streams.len(), 3);
    assert_eq!(streams[0].stream_id, 1);
    assert_eq!(streams[1].stream_id, 3);
    assert_eq!(streams[2].stream_id, 5);
}

#[test]
fn test_http2_stream_state_counts() {
    let mut tab = InterceptTab::new();
    let mut session = Http2Session::new("api.example.com", true);

    let mut s1 = Http2Stream::new(1, "GET", "/a");
    s1.state = eggsec::proxy::intercept::protocols::Http2StreamState::Open;

    let mut s2 = Http2Stream::new(3, "GET", "/b");
    s2.state = eggsec::proxy::intercept::protocols::Http2StreamState::Closed;

    let mut s3 = Http2Stream::new(5, "GET", "/c");
    s3.state = eggsec::proxy::intercept::protocols::Http2StreamState::HalfClosedRemote;

    let mut s4 = Http2Stream::new(7, "GET", "/d");
    s4.state = eggsec::proxy::intercept::protocols::Http2StreamState::Open;

    let mut s5 = Http2Stream::new(9, "GET", "/e");
    s5.state = eggsec::proxy::intercept::protocols::Http2StreamState::Idle;

    session.add_stream(s1);
    session.add_stream(s2);
    session.add_stream(s3);
    session.add_stream(s4);
    session.add_stream(s5);

    tab.add_http2_session(session);

    let (open, half_closed, closed, idle) = tab.http2_stream_state_counts(0);
    assert_eq!(open, 2);
    assert_eq!(half_closed, 1);
    assert_eq!(closed, 1);
    assert_eq!(idle, 1);
}

#[test]
fn test_http2_session_count() {
    let mut tab = InterceptTab::new();
    assert_eq!(tab.http2_session_count(), 0);

    tab.add_http2_session(Http2Session::new("a.example.com", true));
    tab.add_http2_session(Http2Session::new("b.example.com", true));
    assert_eq!(tab.http2_session_count(), 2);
}

#[test]
fn test_total_http2_streams() {
    let mut tab = InterceptTab::new();
    let mut s1 = Http2Session::new("a.example.com", true);
    s1.add_stream(Http2Stream::new(1, "GET", "/"));
    s1.add_stream(Http2Stream::new(3, "GET", "/"));

    let mut s2 = Http2Session::new("b.example.com", true);
    s2.add_stream(Http2Stream::new(1, "POST", "/"));
    s2.add_stream(Http2Stream::new(3, "GET", "/"));
    s2.add_stream(Http2Stream::new(5, "GET", "/"));

    tab.add_http2_session(s1);
    tab.add_http2_session(s2);

    assert_eq!(tab.total_http2_streams(), 5);
}

#[test]
fn test_next_prev_http2_session() {
    let mut tab = InterceptTab::new();
    tab.add_http2_session(Http2Session::new("a.example.com", true));
    tab.add_http2_session(Http2Session::new("b.example.com", true));
    tab.add_http2_session(Http2Session::new("c.example.com", true));

    assert_eq!(tab.selected_http2_session, 0);
    tab.next_http2_session();
    assert_eq!(tab.selected_http2_session, 1);
    tab.next_http2_session();
    assert_eq!(tab.selected_http2_session, 2);
    tab.next_http2_session(); // wrap around
    assert_eq!(tab.selected_http2_session, 0);

    tab.prev_http2_session(); // wrap around backward
    assert_eq!(tab.selected_http2_session, 2);
    tab.prev_http2_session();
    assert_eq!(tab.selected_http2_session, 1);
}

// ==================== gRPC Streaming Visualization Tests ====================

#[test]
fn test_grpc_calls_for_session() {
    let mut tab = InterceptTab::new();
    let mut session = GrpcSession::new("api.example.com", true);

    let call = GrpcCall::new(
        "/svc/Method",
        eggsec::proxy::intercept::protocols::GrpcMethodType::Unary,
    );
    session.add_call(call);

    tab.add_grpc_session(session);
    let calls = tab.grpc_calls_for_session(0);
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].path, "/svc/Method");
}

#[test]
fn test_grpc_streaming_summary() {
    let mut tab = InterceptTab::new();
    let mut state =
        GrpcStreamingState::new(eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional);
    state.flow_control_window = 65535;
    state.bytes_in_flight = 0;

    let frame = GrpcStreamFrame {
        stream_id: 1,
        direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
        payload: None,
        size: 100,
        end_stream: false,
        timestamp: String::new(),
        compressed: false,
    };
    state.add_frame(frame);

    tab.add_grpc_streaming_state(state);

    let summary = tab.grpc_streaming_summary(0);
    assert!(summary.is_some());
    let s = summary.unwrap();
    assert!(s.contains("Bidirectional"));
    assert!(s.contains("1 client / 0 server"));
}

#[test]
fn test_total_grpc_stream_frames() {
    let mut tab = InterceptTab::new();
    let mut state1 = GrpcStreamingState::new(
        eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming,
    );
    let mut state2 =
        GrpcStreamingState::new(eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional);

    // 2 client frames to state1
    for _ in 0..2 {
        state1.add_frame(GrpcStreamFrame {
            stream_id: 1,
            direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
            payload: None,
            size: 10,
            end_stream: false,
            timestamp: String::new(),
            compressed: false,
        });
    }

    // 3 server frames to state2
    for _ in 0..3 {
        state2.add_frame(GrpcStreamFrame {
            stream_id: 1,
            direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Response,
            payload: None,
            size: 20,
            end_stream: false,
            timestamp: String::new(),
            compressed: false,
        });
    }

    tab.add_grpc_streaming_state(state1);
    tab.add_grpc_streaming_state(state2);

    assert_eq!(tab.total_grpc_stream_frames(), 5);
}

#[test]
fn test_grpc_stream_frames_page() {
    let mut tab = InterceptTab::new();
    let mut state = GrpcStreamingState::new(
        eggsec::proxy::intercept::protocols::GrpcMethodType::ServerStreaming,
    );

    for i in 0..15 {
        state.add_frame(GrpcStreamFrame {
            stream_id: 1,
            direction: if i % 2 == 0 {
                eggsec::proxy::intercept::types::ProxyFlowDirection::Request
            } else {
                eggsec::proxy::intercept::types::ProxyFlowDirection::Response
            },
            payload: None,
            size: 10,
            end_stream: false,
            timestamp: format!("2026-01-01T00:00:{:02}Z", i),
            compressed: false,
        });
    }

    tab.add_grpc_streaming_state(state);

    let page0 = tab.grpc_stream_frames_page(0, 0, 5);
    assert_eq!(page0.len(), 5);

    let page2 = tab.grpc_stream_frames_page(0, 2, 5);
    assert_eq!(page2.len(), 5);

    let page3 = tab.grpc_stream_frames_page(0, 3, 5);
    assert_eq!(page3.len(), 0); // Out of range

    let page_overflow = tab.grpc_stream_frames_page(0, 0, 20);
    assert_eq!(page_overflow.len(), 15);
}

#[test]
fn test_next_prev_grpc_session() {
    let mut tab = InterceptTab::new();
    tab.add_grpc_session(GrpcSession::new("a.example.com", true));
    tab.add_grpc_session(GrpcSession::new("b.example.com", true));

    assert_eq!(tab.selected_grpc_session, 0);
    tab.next_grpc_session();
    assert_eq!(tab.selected_grpc_session, 1);
    tab.next_grpc_session(); // wrap
    assert_eq!(tab.selected_grpc_session, 0);

    tab.prev_grpc_session(); // wrap
    assert_eq!(tab.selected_grpc_session, 1);
}

#[test]
fn test_grpc_session_count() {
    let mut tab = InterceptTab::new();
    assert_eq!(tab.grpc_session_count(), 0);
    tab.add_grpc_session(GrpcSession::new("a.example.com", true));
    assert_eq!(tab.grpc_session_count(), 1);
}

// ==================== Correlation Visualization Tests ====================

#[test]
fn test_correlation_summary_str_empty() {
    let tab = InterceptTab::new();
    assert_eq!(tab.correlation_summary_str(), "No correlations");
}

#[test]
fn test_correlation_summary_str_with_references() {
    let mut tab = InterceptTab::new();
    let ref1 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
        CorrelationSource::DbPentest,
        "db-001",
        "SQL injection vulnerability",
    );
    let ref2 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
        CorrelationSource::AuthTest,
        "auth-001",
        "Weak JWT secret",
    );
    tab.add_correlation_reference(0, ref1);
    tab.add_correlation_reference(1, ref2);

    let s = tab.correlation_summary_str();
    assert!(s.contains("2 refs"));
    assert!(s.contains("2 unique sources"));
    assert!(s.contains("2 correlated flows"));
}

#[test]
fn test_correlation_source_counts() {
    let mut tab = InterceptTab::new();
    tab.add_correlation_reference(
        0,
        eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-001",
            "test1",
        ),
    );
    tab.add_correlation_reference(
        1,
        eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-002",
            "test2",
        ),
    );
    tab.add_correlation_reference(
        2,
        eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-001",
            "test3",
        ),
    );

    let counts = tab.correlation_source_counts();
    // Should be sorted by count descending
    assert_eq!(counts.len(), 2);
    assert_eq!(counts[0].0, CorrelationSource::DbPentest);
    assert_eq!(counts[0].1, 2);
    assert_eq!(counts[1].0, CorrelationSource::AuthTest);
    assert_eq!(counts[1].1, 1);
}

#[test]
fn test_recompute_correlations() {
    let mut tab = InterceptTab::new();
    // Add references with different sources
    let ref1 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
        CorrelationSource::DbPentest,
        "db-001",
        "DB issue",
    );
    let ref2 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
        CorrelationSource::AuthTest,
        "auth-001",
        "Auth issue",
    );
    tab.add_correlation_reference(0, ref1);
    tab.add_correlation_reference(1, ref2);

    tab.recompute_correlations();

    // Should find at least one temporal correlation (both refs present)
    assert!(!tab.temporal_correlations.is_empty());
}

#[test]
fn test_correlations_for_flow() {
    let mut tab = InterceptTab::new();
    let ref1 = eggsec::proxy::intercept::correlation::CorrelationReference::new(
        CorrelationSource::DbPentest,
        "db-001",
        "Issue for flow 5",
    );
    tab.add_correlation_reference(5, ref1);

    let corrs = tab.correlations_for_flow(5);
    assert_eq!(corrs.len(), 1);
    assert_eq!(corrs[0].finding_id, "db-001");

    // No correlations for flow 10
    let empty = tab.correlations_for_flow(10);
    assert_eq!(empty.len(), 0);
}

// ==================== Detail Pane Variant Tests ====================

#[test]
fn test_new_detail_pane_variants() {
    // Ensure all new variants exist
    let _mux = DetailPane::StreamMux;
    let _corr = DetailPane::Correlation;

    // Ensure from_index handles all new variants
    assert_eq!(DetailPane::from_index(8), DetailPane::StreamMux);
    assert_eq!(DetailPane::from_index(9), DetailPane::Correlation);
    assert_eq!(DetailPane::from_index(10), DetailPane::Headers); // overflow
}

#[test]
fn test_reset_clears_stream_mux_and_correlation() {
    let mut tab = InterceptTab::new();
    tab.add_http2_session(Http2Session::new("a.com", true));
    tab.add_grpc_session(GrpcSession::new("a.com", true));
    tab.add_grpc_streaming_state(GrpcStreamingState::new(
        eggsec::proxy::intercept::protocols::GrpcMethodType::Unary,
    ));
    tab.add_correlation_reference(
        0,
        eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-001",
            "test",
        ),
    );
    tab.selected_http2_session = 2;
    tab.stream_mux_scroll = 5;

    tab.reset();

    assert!(tab.http2_sessions.is_empty());
    assert!(tab.grpc_sessions.is_empty());
    assert!(tab.grpc_streaming_states.is_empty());
    assert!(tab.temporal_correlations.is_empty());
    assert_eq!(tab.correlation_context.references.len(), 0);
    assert_eq!(tab.selected_http2_session, 0);
    assert_eq!(tab.stream_mux_scroll, 0);
}

// ==================== Render Tests ====================

#[test]
fn test_render_stream_multiplexing_with_data() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut tab = InterceptTab::new();
    let mut session = Http2Session::new("api.example.com", true);
    session.add_stream(Http2Stream::new(1, "GET", "/api/data"));
    session.add_stream(Http2Stream::new(3, "POST", "/api/submit"));
    tab.add_http2_session(session);

    let mut grpc_state =
        GrpcStreamingState::new(eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional);
    grpc_state.add_frame(GrpcStreamFrame {
        stream_id: 1,
        direction: eggsec::proxy::intercept::types::ProxyFlowDirection::Request,
        payload: None,
        size: 100,
        end_stream: false,
        timestamp: "2026-01-01T00:00:00Z".to_string(),
        compressed: false,
    });
    tab.add_grpc_streaming_state(grpc_state);

    tab.detail_pane = DetailPane::StreamMux;

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 120, 40);
            tab.render_stream_multiplexing(f, area);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    // Verify stream multiplexing content was rendered
    let content: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect();
    assert!(content.contains("Stream Multiplexing"));
    assert!(content.contains("HTTP/2 Streams"));
    assert!(content.contains("gRPC Streaming"));
    assert!(content.contains("api.example.com"));
}

#[test]
fn test_render_correlation_with_data() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut tab = InterceptTab::new();
    tab.add_correlation_reference(
        0,
        eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::DbPentest,
            "db-001",
            "SQL injection in /api/users",
        ),
    );
    tab.add_correlation_reference(
        0,
        eggsec::proxy::intercept::correlation::CorrelationReference::new(
            CorrelationSource::AuthTest,
            "auth-001",
            "Missing authentication on /api/admin",
        ),
    );
    tab.recompute_correlations();
    tab.selected_flow = Some(0);
    tab.detail_pane = DetailPane::Correlation;

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 120, 40);
            tab.render_correlation(f, area);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect();
    assert!(content.contains("Cross-Loadout Correlation"));
    assert!(content.contains("DB-Pentest"));
    assert!(content.contains("Auth-Test"));
    assert!(content.contains("Temporal Correlations"));
}

#[test]
fn test_render_stream_multiplexing_empty() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut tab = InterceptTab::new();
    tab.detail_pane = DetailPane::StreamMux;

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 120, 30);
            tab.render_stream_multiplexing(f, area);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect();
    assert!(content.contains("Stream Multiplexing"));
    assert!(content.contains("No HTTP/2 sessions"));
}

#[test]
fn test_render_correlation_empty() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut tab = InterceptTab::new();
    tab.detail_pane = DetailPane::Correlation;

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 120, 30);
            tab.render_correlation(f, area);
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect();
    assert!(content.contains("Cross-Loadout Correlation"));
    assert!(content.contains("No correlations"));
}

#[test]
fn test_render_protocol_info_http2_with_sessions() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut tab = InterceptTab::new();
    // Add a flow first (required for render_protocol_info)
    let flow = ProxyFlow {
        index: 0,
        method: "GET".to_string(),
        url: "https://api.example.com/data".to_string(),
        host: "api.example.com".to_string(),
        path: "/data".to_string(),
        request_headers: Default::default(),
        request_body: None,
        response_status: 200,
        response_headers: Default::default(),
        response_body: None,
        is_https: true,
        duration_ms: 50,
        request_body_size: 0,
        response_body_size: 1024,
        started_at: String::new(),
        completed_at: String::new(),
        redaction_applied: None,
        protocol: "h2".to_string(),
    };
    tab.add_flow(flow);
    tab.selected_flow = Some(0);

    // Add an HTTP/2 session
    let mut session = Http2Session::new("api.example.com", true);
    session.tune_windows(1024 * 1024, 1024 * 1024);
    session.add_stream(Http2Stream::new(1, "GET", "/data"));
    tab.add_http2_session(session);

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 120, 30);
            tab.render_protocol_info(f, area, "HTTP/2");
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect();
    assert!(content.contains("HTTP/2 Protocol Details"));
    assert!(content.contains("Captured Sessions"));
    assert!(content.contains("api.example.com"));
    assert!(content.contains("Stream Mux"));
}

#[test]
fn test_render_protocol_info_grpc_with_sessions() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    let mut tab = InterceptTab::new();
    let flow = ProxyFlow {
        index: 0,
        method: "POST".to_string(),
        url: "https://api.example.com/svc/Method".to_string(),
        host: "api.example.com".to_string(),
        path: "/svc/Method".to_string(),
        request_headers: Default::default(),
        request_body: None,
        response_status: 200,
        response_headers: Default::default(),
        response_body: None,
        is_https: true,
        duration_ms: 30,
        request_body_size: 50,
        response_body_size: 100,
        started_at: String::new(),
        completed_at: String::new(),
        redaction_applied: None,
        protocol: "grpc".to_string(),
    };
    tab.add_flow(flow);
    tab.selected_flow = Some(0);

    // Add a gRPC session with a call
    let mut session = GrpcSession::new("api.example.com", true);
    session.add_call(GrpcCall::new(
        "/svc/Method",
        eggsec::proxy::intercept::protocols::GrpcMethodType::Bidirectional,
    ));
    tab.add_grpc_session(session);

    // Add a security finding
    tab.add_grpc_security_finding(eggsec::proxy::intercept::protocols::GrpcSecurityFinding {
        service_path: "/svc/Method".to_string(),
        category: "missing_auth".to_string(),
        severity: 7,
        description: "Method does not require authentication".to_string(),
        remediation: Some("Add auth interceptor".to_string()),
    });

    terminal
        .draw(|f| {
            let area = ratatui::layout::Rect::new(0, 0, 120, 30);
            tab.render_protocol_info(f, area, "gRPC");
        })
        .unwrap();

    let buffer = terminal.backend().buffer().clone();
    let content: String = buffer
        .content
        .iter()
        .map(|cell| cell.symbol().to_string())
        .collect();
    assert!(content.contains("gRPC Protocol Details"));
    assert!(content.contains("BIDI"));
    assert!(content.contains("Security Findings"));
    assert!(content.contains("missing_auth"));
}

#[test]
fn test_render_detail_pane_handles_new_variants_without_flow() {
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    let backend = TestBackend::new(120, 30);
    let mut terminal = Terminal::new(backend).unwrap();

    // Test StreamMux pane without a flow selected
    let mut tab = InterceptTab::new();
    tab.detail_pane = DetailPane::StreamMux;
    let area = ratatui::layout::Rect::new(0, 0, 120, 30);
    terminal.draw(|f| tab.render_detail_pane(f, area)).unwrap();

    // Test Correlation pane without a flow selected
    tab.detail_pane = DetailPane::Correlation;
    terminal.draw(|f| tab.render_detail_pane(f, area)).unwrap();
}
