#[cfg(feature = "rest-api")]
mod tests {
    #[test]
    fn test_responses_request_type() {
        use slapper::tool::protocol::openresponses::types::{Input, ResponsesRequest};

        let _req = ResponsesRequest {
            model: "gpt-4".into(),
            input: Input::Text("scan example.com".into()),
            instructions: None,
            include: None,
            tools: None,
            tool_choice: None,
            stream: None,
            temperature: None,
            max_output_tokens: None,
            previous_response_id: None,
            metadata: None,
        };
    }

    #[test]
    fn test_tool_info_type_exists() {
        use slapper::tool::{ToolCapability, ToolCategory, ToolInfo};

        let _info = ToolInfo {
            id: "scan".into(),
            name: "Port Scan".into(),
            category: ToolCategory::Scanning,
            description: "Scan ports".into(),
            capabilities: vec![ToolCapability {
                name: "port_scan".into(),
                description: "Scan target ports".into(),
                parameters: vec![],
                examples: vec![],
                attack_surface: vec![],
                severity_potential: vec![],
                prerequisites: vec![],
                estimated_duration_ms: 5000,
            }],
            protocols: vec!["tcp".into()],
        };
    }

    #[test]
    fn test_openresponses_router_function() {
        use slapper::tool::protocol::openresponses;
        let registry = slapper::tool::create_default_registry();
        let _router = openresponses::router(registry, Some("test-key".to_string()));
    }

    #[test]
    fn test_ai_finding_serialization() {
        use slapper::tool::protocol::openresponses::types::AiFinding;

        let finding = AiFinding {
            severity: "high".into(),
            title: "SQL Injection".into(),
            description: "Potential SQLi detected".into(),
            location: Some("POST /login".into()),
            evidence: vec![],
            remediation: None,
            cwe_id: None,
            confidence: Some(0.9),
        };

        let json = serde_json::to_string(&finding).unwrap();
        assert!(json.contains("SQL Injection"));
    }

    #[test]
    fn test_output_item_message_variant() {
        use slapper::tool::protocol::openresponses::types::OutputItem;

        let item = OutputItem::Message {
            item_type: "message".into(),
            id: "msg-1".into(),
            role: "assistant".into(),
            content: vec![],
            status: Some("completed".into()),
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("message"));
    }

    #[test]
    fn test_output_item_finding_variant() {
        use slapper::tool::protocol::openresponses::types::{AiFinding, OutputItem};

        let item = OutputItem::Finding {
            item_type: "slapper:security_finding".into(),
            id: "finding-1".into(),
            finding: AiFinding {
                severity: "critical".into(),
                title: "XSS".into(),
                description: "Cross-site scripting".into(),
                location: Some("/search".into()),
                evidence: vec![],
                remediation: None,
                cwe_id: Some("79".into()),
                confidence: Some(1.0),
            },
        };

        let json = serde_json::to_string(&item).unwrap();
        assert!(json.contains("slapper:security_finding"));
    }

    #[test]
    fn test_responses_response_type() {
        use slapper::tool::protocol::openresponses::types::{OutputItem, ResponsesResponse};

        let response = ResponsesResponse {
            id: "resp-123".into(),
            object: "response".into(),
            created_at: 1710000000,
            status: "completed".into(),
            output: vec![OutputItem::Message {
                item_type: "message".into(),
                id: "msg-1".into(),
                role: "assistant".into(),
                content: vec![],
                status: Some("completed".into()),
            }],
            model: "gpt-4".into(),
            incomplete_details: None,
            metadata: None,
            usage: None,
            error: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("resp-123"));
    }

    #[test]
    fn test_input_text_variant() {
        use slapper::tool::protocol::openresponses::types::Input;

        let input = Input::Text("fuzz http://example.com".to_string());
        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("fuzz"));
    }

    #[test]
    fn test_input_items_variant() {
        use slapper::tool::protocol::openresponses::types::{Input, InputItem};

        let input = Input::Items(vec![InputItem {
            item_type: "input_text".into(),
            content: Some("scan example.com".into()),
            name: None,
            call_id: None,
            output: None,
        }]);

        let json = serde_json::to_string(&input).unwrap();
        assert!(json.contains("input_text"));
    }

    #[test]
    fn test_function_tool_type() {
        use slapper::tool::protocol::openresponses::types::{FunctionTool, ToolChoice};

        let tool = FunctionTool {
            tool_type: "function".into(),
            name: "port_scan".into(),
            description: "Scan target ports".into(),
            parameters: serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {"type": "string"},
                    "ports": {"type": "string"}
                }
            }),
        };

        let _choice = ToolChoice::Auto;
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("port_scan"));
    }

    #[test]
    fn test_error_response_type() {
        use slapper::tool::protocol::openresponses::types::ErrorResponse;

        let error = ErrorResponse {
            code: "invalid_request".into(),
            message: "Invalid target provided".into(),
        };

        let json = serde_json::to_string(&error).unwrap();
        assert!(json.contains("invalid_request"));
    }

    #[test]
    fn test_usage_type() {
        use slapper::tool::protocol::openresponses::types::Usage;

        let usage = Usage {
            input_tokens: 100,
            output_tokens: 50,
            total_tokens: 150,
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("100"));
    }
}
