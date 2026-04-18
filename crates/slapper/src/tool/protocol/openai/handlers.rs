use axum::{
    extract::State,
    http::HeaderMap,
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures::stream::{self};
use std::convert::Infallible;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;

use super::types::*;
use super::OpenAiState;
use crate::tool::registry::ToolRegistry;
use crate::tool::request::{Target, ToolRequest};

fn require_auth(state: &Arc<OpenAiState>, headers: &HeaderMap) -> Result<(), &'static str> {
    if let Some(ref key) = state.api_key {
        let auth = headers
            .get("authorization")
            .or_else(|| headers.get("x-api-key"))
            .and_then(|v| v.to_str().ok());

        match auth {
            Some(v) if key.as_bytes().ct_eq(v.as_bytes()).unwrap_u8() == 1 => Ok(()),
            _ => Err("Invalid or missing API key"),
        }
    } else {
        Ok(())
    }
}

pub async fn chat_completions(
    State(state): State<Arc<OpenAiState>>,
    headers: HeaderMap,
    Json(req): Json<ChatCompletionRequest>,
) -> Result<Response, &'static str> {
    require_auth(&state, &headers)?;
    if req.stream.unwrap_or(false) {
        Ok(streaming_response(state.registry.clone(), req).await.into_response())
    } else {
        Ok(Json(non_streaming_response(state.registry.clone(), req).await).into_response())
    }
}

async fn streaming_response(
    registry: ToolRegistry,
    req: ChatCompletionRequest,
) -> Sse<impl futures::Stream<Item = Result<axum::response::sse::Event, Infallible>>> {
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let model = req.model.clone();
    let user_query = extract_user_query(&req.messages);
    let system_prompt = extract_system_prompt(&req.messages);

    let available_tools = registry.list();
    let matched_tools = find_matching_tools(&user_query, &available_tools);

    let mut events: Vec<Result<axum::response::sse::Event, Infallible>> = Vec::new();

    let response_text = generate_response(&system_prompt, &user_query, &matched_tools);

    for word in response_text.split_whitespace() {
        let delta = ChatMessage {
            role: "assistant".to_string(),
            content: Some(format!("{} ", word)),
            tool_calls: None,
        };
        let chunk = StreamChunk {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta,
                finish_reason: None,
            }],
        };
        if let Ok(json) = serde_json::to_string(&chunk) {
            events.push(Ok(axum::response::sse::Event::default().data(json)));
        }
    }

    if !matched_tools.is_empty() {
        let tool_calls: Vec<ToolCall> = matched_tools.iter().map(|t| ToolCall {
            id: format!("call_{}", uuid::Uuid::new_v4()),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: t.name.clone(),
                arguments: serde_json::to_string(&extract_tool_arguments(t, &user_query)).unwrap_or_else(|_| "{}".to_string()),
            },
        }).collect();

        let delta = ChatMessage {
            role: "assistant".to_string(),
            content: None,
            tool_calls: Some(tool_calls),
        };
        let chunk = StreamChunk {
            id: id.clone(),
            object: "chat.completion.chunk".to_string(),
            created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
            model: model.clone(),
            choices: vec![StreamChoice {
                index: 0,
                delta,
                finish_reason: Some("tool_calls".to_string()),
            }],
        };
        if let Ok(json) = serde_json::to_string(&chunk) {
            events.push(Ok(axum::response::sse::Event::default().data(json)));
        }
    }

    events.push(Ok(axum::response::sse::Event::default().data("[DONE]")));

    Sse::new(stream::iter(events))
}

async fn non_streaming_response(
    registry: ToolRegistry,
    req: ChatCompletionRequest,
) -> ChatCompletionResponse {
    let model = req.model.clone();
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let system_prompt = extract_system_prompt(&req.messages);
    let user_query = extract_user_query(&req.messages);

    let mut tool_calls = None;
    let available_tools = registry.list();
    let matched_tools = find_matching_tools(&user_query, &available_tools);

    let content = if !matched_tools.is_empty() && req.tools.is_some() {
        let target = extract_target_from_query(&user_query);
        let mut results = Vec::with_capacity(matched_tools.len().min(3));

        for tool_info in matched_tools.iter().take(3) {
            if let Some(tool) = registry.get(&tool_info.id) {
                let request = ToolRequest::new(tool_info.id.clone(), target.clone());
                match tool.execute(request).await {
                    Ok(response) => {
                        results.push(format!(
                            "{}: {} - found {} findings",
                            tool_info.name,
                            response.status,
                            response.findings.len()
                        ));
                    }
                    Err(e) => {
                        results.push(format!("{}: Error - {}", tool_info.name, e));
                    }
                }
            }
        }

        if results.is_empty() {
            generate_response(&system_prompt, &user_query, &matched_tools)
        } else {
            format!(
                "Executed {} tool(s):\n\n{}",
                results.len(),
                results.join("\n")
            )
        }
    } else {
        generate_response(&system_prompt, &user_query, &matched_tools)
    };

    if !matched_tools.is_empty() && req.tools.is_some() {
        let calls: Vec<ToolCall> = matched_tools.iter().take(3).map(|t| ToolCall {
            id: format!("call_{}", uuid::Uuid::new_v4()),
            tool_type: "function".to_string(),
            function: FunctionCall {
                name: t.name.clone(),
                arguments: serde_json::to_string(&extract_tool_arguments(t, &user_query)).unwrap_or_else(|_| "{}".to_string()),
            },
        }).collect();
        tool_calls = Some(calls);
    }

    let prompt_tokens = user_query.len() / 4;
    let completion_tokens = content.len() / 4;

    ChatCompletionResponse {
        id,
        object: "chat.completion".to_string(),
        created,
        model,
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some(content),
                tool_calls,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Some(Usage {
            prompt_tokens,
            completion_tokens,
            total_tokens: prompt_tokens + completion_tokens,
        }),
    }
}

fn extract_user_query(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .filter(|m| m.role == "user")
        .filter_map(|m| m.content.clone())
        .collect::<Vec<_>>()
        .join("\n")
}

fn extract_system_prompt(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .find(|m| m.role == "system")
        .and_then(|m| m.content.as_ref())
        .cloned()
        .unwrap_or_else(|| "You are Slapper, a security testing toolkit assistant.".to_string())
}

fn extract_target_from_query(query: &str) -> Target {
    let query_lower = query.to_lowercase();

    let url_patterns = ["http://", "https://", "www."];
    for pattern in url_patterns {
        if let Some(idx) = query_lower.find(pattern) {
            let end = query[idx..]
                .find(|c: char| c.is_whitespace())
                .map(|i| idx + i)
                .unwrap_or(query.len());
            let value = query[idx..end].trim_end_matches(|c: char| c.is_ascii_punctuation() && c != '/' && c != ':').to_string();
            if !value.is_empty() {
                return Target::url(value);
            }
        }
    }

    let words: Vec<&str> = query.split_whitespace().collect();
    for word in &words {
        if word.contains('.') && word.split('.').count() == 4 {
            if let Ok(_) = word.parse::<std::net::IpAddr>() {
                return Target::ip(*word);
            }
        }
        if word.parse::<u16>().is_ok() && words.iter().any(|w| w.to_lowercase().contains("port")) {
            continue;
        }
        if word.contains('/') {
            if let Ok(_) = word.parse::<std::net::IpAddr>() {
                return Target::cidr(*word);
            }
        }
    }

    for word in &words {
        if word.len() > 3 && !words.iter().take(3).any(|w| *w == *word) {
            if word.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
                if !word.starts_with("http") && word.contains('.') {
                    return Target::domain(*word);
                }
            }
        }
    }

    Target::url("http://localhost")
}

fn generate_response(
    system_prompt: &str,
    user_query: &str,
    matched_tools: &[crate::tool::registry::ToolInfo],
) -> String {
    let query_lower = user_query.to_lowercase();

    if !matched_tools.is_empty() {
        let tool_names: Vec<&str> = matched_tools.iter().map(|t| t.name.as_str()).collect();
        return format!(
            "I found {} matching tool(s): {}.\n\n\
            These tools can help with your request. Use the tool_calls to execute them.",
            matched_tools.len(),
            tool_names.join(", ")
        );
    }

    if query_lower.contains("scan") || query_lower.contains("vulnerability") {
        format!(
            "Based on your request: \"{}\"\n\n\
            I can help with security scanning. Available capabilities include:\n\
            - Port scanning and service fingerprinting\n\
            - Endpoint discovery\n\
            - Security fuzzing (SQLi, XSS, SSRF, etc.)\n\
            - WAF detection and bypass\n\
            - Reconnaissance (DNS, WHOIS, SSL, subdomains)\n\
            - Load testing\n\n\
            Use the available tools to execute specific security tests.",
            user_query
        )
    } else if query_lower.contains("fuzz") || query_lower.contains("payload") {
        "I can generate security testing payloads for various vulnerability types including SQL injection, XSS, SSRF, path traversal, and more. Use the fuzz tool to test your target.".to_string()
    } else if query_lower.contains("waf") {
        "I can detect and attempt to bypass Web Application Firewalls. I support detection of 30+ WAF products and various bypass techniques including header manipulation and HTTP smuggling.".to_string()
    } else if query_lower.contains("recon") || query_lower.contains("discover") {
        "I can perform passive reconnaissance including DNS enumeration, WHOIS lookup, SSL/TLS analysis, subdomain discovery, technology detection, and CVE mapping.".to_string()
    } else {
        format!(
            "{}\n\nHow can I help you with security testing today?",
            system_prompt
        )
    }
}

fn find_matching_tools(
    query: &str,
    tools: &[crate::tool::registry::ToolInfo],
) -> Vec<crate::tool::registry::ToolInfo> {
    let query_lower = query.to_lowercase();
    tools
        .iter()
        .filter(|t| {
            let name_lower = t.name.to_lowercase();
            let desc_lower = t.description.to_lowercase();
            query_lower.contains(&name_lower)
                || desc_lower.split_whitespace().any(|w| query_lower.contains(w))
                || t.capabilities.iter().any(|c| {
                    c.name.to_lowercase().contains(&query_lower)
                        || c.description.to_lowercase().contains(&query_lower)
                })
        })
        .take(3)
        .cloned()
        .collect()
}

fn extract_tool_arguments(
    tool: &crate::tool::registry::ToolInfo,
    query: &str,
) -> serde_json::Value {
    use crate::tool::request::TargetType;

    let mut args = serde_json::Map::new();
    let target = extract_target_from_query(query);

    for cap in &tool.capabilities {
        for param in &cap.parameters {
            match param.name.to_lowercase().as_str() {
                "target" | "host" | "url" | "domain" => {
                    args.insert(param.name.clone(), serde_json::json!(target.value));
                }
                "port" | "ports" => {
                    if let Some(port_str) = extract_port_from_query(query) {
                        if let Ok(port) = port_str.parse::<u16>() {
                            args.insert(param.name.clone(), serde_json::json!(port));
                        }
                    }
                }
                "concurrency" | "threads" => {
                    if let Some(val) = extract_number_from_query(query, &["concurrency", "threads", "parallel"]) {
                        args.insert(param.name.clone(), serde_json::json!(val));
                    } else if let Some(default) = &param.default {
                        args.insert(param.name.clone(), default.clone());
                    }
                }
                "timeout" | "duration" => {
                    if let Some(val) = extract_number_from_query(query, &["timeout", "duration", "seconds"]) {
                        args.insert(param.name.clone(), serde_json::json!(val));
                    } else if let Some(default) = &param.default {
                        args.insert(param.name.clone(), default.clone());
                    }
                }
                "verbose" | "debug" => {
                    let query_lower = query.to_lowercase();
                    let val = query_lower.contains("verbose") || query_lower.contains("debug") || query_lower.contains("-v");
                    args.insert(param.name.clone(), serde_json::json!(val));
                }
                _ => {
                    if let Some(default) = &param.default {
                        args.insert(param.name.clone(), default.clone());
                    }
                }
            }
        }
        if !args.is_empty() {
            break;
        }
    }

    if args.is_empty() {
        args.insert("target".to_string(), serde_json::json!(target.value));
    }

    serde_json::Value::Object(args)
}

fn extract_port_from_query(query: &str) -> Option<String> {
    let words: Vec<&str> = query.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        if word.to_lowercase() == "port" && i + 1 < words.len() {
            let next = words[i + 1].trim_end_matches(|c: char| !c.is_ascii_digit());
            if next.parse::<u16>().is_ok() {
                return Some(next.to_string());
            }
        }
        if let Some(stripped) = word.strip_prefix("--port=") {
            if stripped.parse::<u16>().is_ok() {
                return Some(stripped.to_string());
            }
        }
    }
    None
}

fn extract_number_from_query(query: &str, keywords: &[&str]) -> Option<u64> {
    let words: Vec<&str> = query.split_whitespace().collect();
    for (i, word) in words.iter().enumerate() {
        for keyword in keywords {
            if word.to_lowercase().contains(keyword) && i + 1 < words.len() {
                if let Ok(val) = words[i + 1].parse::<u64>() {
                    return Some(val);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_user_query_from_messages() {
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: Some("You are helpful".to_string()), tool_calls: None },
            ChatMessage { role: "user".to_string(), content: Some("Scan example.com".to_string()), tool_calls: None },
        ];
        let query = extract_user_query(&messages);
        assert_eq!(query, "Scan example.com");
    }

    #[test]
    fn test_extract_user_query_multiple_user_messages() {
        let messages = vec![
            ChatMessage { role: "user".to_string(), content: Some("First".to_string()), tool_calls: None },
            ChatMessage { role: "assistant".to_string(), content: Some("OK".to_string()), tool_calls: None },
            ChatMessage { role: "user".to_string(), content: Some("Second".to_string()), tool_calls: None },
        ];
        let query = extract_user_query(&messages);
        assert_eq!(query, "First\nSecond");
    }

    #[test]
    fn test_extract_system_prompt_default() {
        let messages = vec![
            ChatMessage { role: "user".to_string(), content: Some("Hello".to_string()), tool_calls: None },
        ];
        let prompt = extract_system_prompt(&messages);
        assert!(prompt.contains("Slapper"));
    }

    #[test]
    fn test_extract_system_prompt_custom() {
        let messages = vec![
            ChatMessage { role: "system".to_string(), content: Some("Custom prompt".to_string()), tool_calls: None },
        ];
        let prompt = extract_system_prompt(&messages);
        assert_eq!(prompt, "Custom prompt");
    }

    #[test]
    fn test_extract_target_url() {
        let target = extract_target_from_query("Please scan https://example.com/api");
        assert_eq!(target.value, "https://example.com/api");
    }

    #[test]
    fn test_extract_target_domain() {
        let target = extract_target_from_query("Check vulnerabilities on test-server.example.com please");
        assert_eq!(target.value, "test-server.example.com");
    }

    #[test]
    fn test_extract_target_ip() {
        let target = extract_target_from_query("Scan port 80 on 192.168.1.1");
        assert_eq!(target.value, "192.168.1.1");
    }

    #[test]
    fn test_extract_port_from_query() {
        assert_eq!(extract_port_from_query("Scan port 8080"), Some("8080".to_string()));
        assert_eq!(extract_port_from_query("--port=443"), Some("443".to_string()));
        assert_eq!(extract_port_from_query("no port mentioned"), None);
    }

    #[test]
    fn test_extract_number_from_query() {
        assert_eq!(extract_number_from_query("use concurrency 10", &["concurrency"]), Some(10));
        assert_eq!(extract_number_from_query("timeout 30 seconds", &["timeout"]), Some(30));
        assert_eq!(extract_number_from_query("no numbers here", &["concurrency"]), None);
    }

    #[test]
    fn test_generate_response_with_matched_tools() {
        let tools = vec![crate::tool::registry::ToolInfo {
            id: "scan".to_string(),
            name: "scan".to_string(),
            category: crate::tool::traits::ToolCategory::Scanning,
            description: "Port scanner".to_string(),
            capabilities: vec![],
            protocols: vec![],
        }];
        let response = generate_response("system", "scan example.com", &tools);
        assert!(response.contains("1 matching"));
        assert!(response.contains("scan"));
    }

    #[test]
    fn test_generate_response_no_tools() {
        let response = generate_response("system", "hello", &[]);
        assert!(response.contains("system"));
    }
}
