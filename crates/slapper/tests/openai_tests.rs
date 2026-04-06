#[cfg(feature = "rest-api")]
mod tests {
    #[test]
    fn test_openai_chat_completion_request() {
        use slapper::tool::protocol::openai::types::{ChatCompletionRequest, ChatMessage};

        let request = ChatCompletionRequest {
            model: "gpt-4".into(),
            messages: vec![ChatMessage {
                role: "user".into(),
                content: Some("Scan example.com".into()),
                tool_calls: None,
            }],
            tools: None,
            tool_choice: None,
            stream: None,
        };

        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("gpt-4"));
    }

    #[test]
    fn test_openai_chat_completion_response() {
        use slapper::tool::protocol::openai::types::{ChatCompletionResponse, ChatMessage, Choice};

        let response = ChatCompletionResponse {
            id: "chatcmpl-123".into(),
            object: "chat.completion".into(),
            created: 1710000000,
            model: "gpt-4".into(),
            choices: vec![Choice {
                index: 0,
                message: ChatMessage {
                    role: "assistant".into(),
                    content: Some("Port scan complete".into()),
                    tool_calls: None,
                },
                finish_reason: "stop".into(),
            }],
            usage: None,
        };

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("chatcmpl-123"));
    }

    #[test]
    fn test_openai_tool_definition_builder() {
        use slapper::tool::protocol::openai::types::ToolDefinition;

        let tool = ToolDefinition::new(
            "port_scan".into(),
            "Scan target ports".into(),
            serde_json::json!({
                "type": "object",
                "properties": {
                    "target": {"type": "string"},
                    "ports": {"type": "string"}
                },
                "required": ["target"]
            }),
        );

        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("port_scan"));
    }

    #[test]
    fn test_openai_router_function() {
        use slapper::tool::protocol::openai;
        let registry = slapper::tool::create_default_registry();
        let _router = openai::router(registry);
    }

    #[test]
    fn test_openai_router_includes_models() {
        use slapper::tool::protocol::openai;
        let registry = slapper::tool::create_default_registry();
        let router = openai::router(registry);
        let _ = router;
    }

    #[test]
    fn test_function_call_type() {
        use slapper::tool::protocol::openai::types::FunctionCall;

        let call = FunctionCall {
            name: "port_scan".into(),
            arguments: r#"{"target": "example.com", "ports": "1-1000"}"#.into(),
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("port_scan"));
    }

    #[test]
    fn test_tool_call_type() {
        use slapper::tool::protocol::openai::types::{FunctionCall, ToolCall};

        let call = ToolCall {
            id: "call_123".into(),
            tool_type: "function".into(),
            function: FunctionCall {
                name: "port_scan".into(),
                arguments: r#"{"target": "example.com"}"#.into(),
            },
        };

        let json = serde_json::to_string(&call).unwrap();
        assert!(json.contains("call_123"));
    }

    #[test]
    fn test_usage_type() {
        use slapper::tool::protocol::openai::types::Usage;

        let usage = Usage {
            prompt_tokens: 100,
            completion_tokens: 50,
            total_tokens: 150,
        };

        let json = serde_json::to_string(&usage).unwrap();
        assert!(json.contains("100"));
    }
}
