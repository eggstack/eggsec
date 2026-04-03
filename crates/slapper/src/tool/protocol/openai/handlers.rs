use axum::{
    response::{IntoResponse, Response, Sse},
    Json,
};
use futures::stream::{self};
use std::convert::Infallible;
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::*;
use crate::tool::registry::ToolRegistry;

pub async fn chat_completions(
    axum::extract::State(registry): axum::extract::State<ToolRegistry>,
    Json(req): Json<ChatCompletionRequest>,
) -> Response {
    if req.stream.unwrap_or(false) {
        let id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
        let model = req.model.clone();
        let user_messages: Vec<String> = req.messages.iter()
            .filter(|m| m.role == "user")
            .filter_map(|m| m.content.clone())
            .collect();
        let user_query = user_messages.join("\n");
        let system_prompt = req.messages.iter()
            .find(|m| m.role == "system")
            .and_then(|m| m.content.as_ref())
            .cloned()
            .unwrap_or_else(|| "You are Slapper, a security testing toolkit assistant.".to_string());

        let response_text = generate_response(&system_prompt, &user_query);
        let available_tools = registry.list();
        let matched_tools = find_matching_tools(&user_query, &available_tools);

        let mut events: Vec<Result<axum::response::sse::Event, Infallible>> = Vec::new();

        for word in response_text.split_whitespace() {
            let delta = ChatMessage {
                role: "assistant".to_string(),
                content: Some(format!("{} ", word)),
                tool_calls: None,
            };
            let chunk = StreamChunk {
                id: id.clone(),
                object: "chat.completion.chunk".to_string(),
                created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
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
                    arguments: serde_json::json!({}).to_string(),
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
                created: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
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

        Sse::new(stream::iter(events)).into_response()
    } else {
        let response = process_request(registry, req).await;
        Json(response).into_response()
    }
}

async fn process_request(registry: ToolRegistry, req: ChatCompletionRequest) -> ChatCompletionResponse {
    let model = req.model.clone();
    let id = format!("chatcmpl-{}", uuid::Uuid::new_v4());
    let created = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();

    let system_prompt = req.messages.iter()
        .find(|m| m.role == "system")
        .and_then(|m| m.content.as_ref())
        .cloned()
        .unwrap_or_else(|| "You are Slapper, a security testing toolkit assistant.".to_string());

    let user_messages: Vec<String> = req.messages.iter()
        .filter(|m| m.role == "user")
        .filter_map(|m| m.content.clone())
        .collect();

    let user_query = user_messages.join("\n");

    let mut tool_calls = None;
    let content = generate_response(&system_prompt, &user_query);

    if req.tools.is_some() && !user_query.is_empty() {
        let available_tools = registry.list();
        let matched = find_matching_tools(&user_query, &available_tools);

        if !matched.is_empty() {
            let calls: Vec<ToolCall> = matched.iter().map(|t| ToolCall {
                id: format!("call_{}", uuid::Uuid::new_v4()),
                tool_type: "function".to_string(),
                function: FunctionCall {
                    name: t.name.clone(),
                    arguments: serde_json::json!({}).to_string(),
                },
            }).collect();
            tool_calls = Some(calls);
        }
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

fn generate_response(system_prompt: &str, user_query: &str) -> String {
    let query_lower = user_query.to_lowercase();

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

fn find_matching_tools(query: &str, tools: &[crate::tool::registry::ToolInfo]) -> Vec<crate::tool::registry::ToolInfo> {
    let query_lower = query.to_lowercase();
    tools.iter()
        .filter(|t| {
            let name_lower = t.name.to_lowercase();
            let desc_lower = t.description.to_lowercase();
            query_lower.contains(&name_lower) || desc_lower.split_whitespace().any(|w| query_lower.contains(w))
        })
        .take(3).cloned()
        .collect()
}
