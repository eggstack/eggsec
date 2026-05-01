use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    Json,
};
use std::time::{SystemTime, UNIX_EPOCH};

use super::types::*;
use super::AppState;
use crate::tool::request::ToolRequest;
use crate::tool::request::Target;
use crate::tool::response::ResponseSeverity;

pub async fn create_response(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(req): Json<ResponsesRequest>,
) -> impl IntoResponse {
    if let Err(e) = validate_auth(&state.api_key, &headers) {
        return e.into_response();
    }

    if req.stream.unwrap_or(false) {
        return (StatusCode::NOT_IMPLEMENTED, Json(serde_json::json!({
            "error": { "code": "not_implemented", "message": "Streaming not yet supported for /v1/responses" }
        }))).into_response();
    }

    let target = extract_target(&req.input);
    let user_query = input_to_string(&req.input);

    let available_tools = state.registry.list();
    let matched_tools = find_matching_tools(&user_query, &available_tools);

    let mut output_items: Vec<OutputItem> = Vec::new();
    let mut findings: Vec<AiFinding> = Vec::new();

    for tool_info in matched_tools.iter().take(5) {
        if let Some(tool) = state.registry.get(&tool_info.id) {
            let request = ToolRequest::new(tool_info.id.clone(), target.clone());
            match tool.execute(request).await {
                Ok(response) => {
                    for finding in &response.findings {
                        findings.push(AiFinding {
                            severity: response_severity_to_string(&finding.severity),
                            title: finding.title.clone(),
                            description: finding.description.clone(),
                            location: Some(finding.location.clone()),
                            evidence: vec![AiEvidence {
                                description: "Tool execution result".to_string(),
                                raw_data: Some(serde_json::to_string(&finding.metadata).unwrap_or_default()),
                                source: Some(tool_info.name.clone()),
                            }],
                            remediation: finding.remediation.as_ref().map(|r| AiRemediation {
                                summary: r.clone(),
                                steps: vec![r.clone()],
                                priority: Some(response_severity_to_string(&finding.severity)),
                            }),
                            cwe_id: None,
                            confidence: Some(0.8),
                        });
                    }
                }
                Err(e) => {
                    tracing::warn!("Tool execution failed: {} - {}", tool_info.name, e);
                }
            }
        }
    }

    if findings.is_empty() {
        let response_text = generate_text_response(&user_query, &matched_tools);
        output_items.push(OutputItem::Message {
            item_type: "message".to_string(),
            id: format!("msg_{}", uuid::Uuid::new_v4()),
            role: "assistant".to_string(),
            content: vec![MessageContent::Text {
                content_type: "input_text".to_string(),
                text: response_text,
            }],
            status: Some("completed".to_string()),
        });
    } else {
        for finding in findings {
            output_items.push(OutputItem::Finding {
                item_type: "slapper:security_finding".to_string(),
                id: format!("finding_{}", uuid::Uuid::new_v4()),
                finding,
            });
        }
    }

    let response = ResponsesResponse {
        id: format!("resp_{}", uuid::Uuid::new_v4()),
        object: "response".to_string(),
        created_at: SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_secs(),
        status: "completed".to_string(),
        output: output_items,
        model: req.model,
        incomplete_details: None,
        metadata: req.metadata,
        usage: Some(Usage {
            input_tokens: (user_query.len() / 4) as u32,
            output_tokens: 0,
            total_tokens: (user_query.len() / 4) as u32,
        }),
        error: None,
    };

    Json(response).into_response()
}

fn validate_auth(api_key: &Option<String>, headers: &HeaderMap) -> Result<(), (StatusCode, Json<serde_json::Value>)> {
    if let Some(expected) = api_key {
        let provided = headers
            .get("authorization")
            .and_then(|v| v.to_str().ok())
            .and_then(|v| v.strip_prefix("Bearer "))
            .or_else(|| headers.get("x-api-key").and_then(|v| v.to_str().ok()));

        match provided {
            Some(key) if crate::utils::constant_time_eq(key, expected) => Ok(()),
            _ => Err((
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({
                    "error": { "code": "unauthorized", "message": "Invalid or missing API key" }
                })),
            )),
        }
    } else {
        Ok(())
    }
}

fn extract_target(input: &Input) -> Target {
    let text = input_to_string(input);
    let text_lower = text.to_lowercase();

    for pattern in &["http://", "https://"] {
        if let Some(idx) = text_lower.find(pattern) {
            let rest = &text[idx..];
            let end = rest.find(|c: char| c.is_whitespace() || c == '\n' || c == '\r')
                .unwrap_or(rest.len());
            let url = rest[..end].trim_end_matches(|c: char| c.is_ascii_punctuation() && c != '/' && c != ':').to_string();
            if !url.is_empty() {
                return Target::url(url);
            }
        }
    }

    for word in text.split_whitespace() {
        if word.parse::<std::net::IpAddr>().is_ok() {
            return Target::ip(word.to_string());
        }
        if word.contains('.') && word.chars().all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-') {
            return Target::domain(word.to_string());
        }
    }

    Target::url("http://localhost".to_string())
}

fn input_to_string(input: &Input) -> String {
    match input {
        Input::Text(s) => s.clone(),
        Input::Items(items) => items
            .iter()
            .filter_map(|item| item.content.as_ref())
            .cloned()
            .collect::<Vec<_>>()
            .join("\n"),
    }
}

fn find_matching_tools(query: &str, tools: &[crate::tool::registry::ToolInfo]) -> Vec<crate::tool::registry::ToolInfo> {
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
        .take(5)
        .cloned()
        .collect()
}

fn generate_text_response(query: &str, matched_tools: &[crate::tool::registry::ToolInfo]) -> String {
    if !matched_tools.is_empty() {
        let tool_names: Vec<&str> = matched_tools.iter().map(|t| t.name.as_str()).collect();
        format!(
            "I found {} matching tool(s): {}. These tools can help with your request.",
            matched_tools.len(),
            tool_names.join(", ")
        )
    } else {
        format!("I can help with security testing. Available capabilities include port scanning, fuzzing, WAF detection, and reconnaissance. Your query: \"{}\"", query)
    }
}

fn response_severity_to_string(severity: &ResponseSeverity) -> String {
    match severity {
        ResponseSeverity::Critical => "critical",
        ResponseSeverity::High => "high",
        ResponseSeverity::Medium => "medium",
        ResponseSeverity::Low => "low",
        ResponseSeverity::Info => "info",
        ResponseSeverity::None => "none",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Severity;

    fn severity_to_string(severity: &Severity) -> String {
        severity.as_str().to_string()
    }

    #[test]
    fn test_input_to_string_text() {
        let input = Input::Text("scan example.com".to_string());
        assert_eq!(input_to_string(&input), "scan example.com");
    }

    #[test]
    fn test_input_to_string_items() {
        let input = Input::Items(vec![
            InputItem { item_type: "input_text".to_string(), content: Some("hello".to_string()), name: None, call_id: None, output: None },
            InputItem { item_type: "input_text".to_string(), content: Some("world".to_string()), name: None, call_id: None, output: None },
        ]);
        assert_eq!(input_to_string(&input), "hello\nworld");
    }

    #[test]
    fn test_extract_target_url() {
        let input = Input::Text("scan https://example.com/path".to_string());
        let target = extract_target(&input);
        assert!(target.value.contains("example.com"));
    }

    #[test]
    fn test_extract_target_domain() {
        let input = Input::Text("scan example.com".to_string());
        let target = extract_target(&input);
        assert_eq!(target.value, "example.com");
    }

    #[test]
    fn test_severity_to_string() {
        assert_eq!(severity_to_string(&Severity::Critical), "critical");
        assert_eq!(severity_to_string(&Severity::High), "high");
        assert_eq!(severity_to_string(&Severity::Medium), "medium");
        assert_eq!(severity_to_string(&Severity::Low), "low");
        assert_eq!(severity_to_string(&Severity::Info), "info");
    }

    #[test]
    fn test_constant_time_eq() {
        assert!(crate::utils::constant_time_eq("test", "test"));
        assert!(!crate::utils::constant_time_eq("test", "other"));
    }
}
