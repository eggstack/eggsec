use axum::Json;
use super::types::*;

pub async fn chat_completions(
    Json(req): Json<ChatCompletionRequest>,
) -> Json<ChatCompletionResponse> {
    let model = req.model.clone();
    Json(ChatCompletionResponse {
        id: format!("chatcmpl-{}", uuid::Uuid::new_v4()),
        object: "chat.completion".to_string(),
        created: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        model,
        choices: vec![Choice {
            index: 0,
            message: ChatMessage {
                role: "assistant".to_string(),
                content: Some("Slapper security testing toolkit ready.".to_string()),
                tool_calls: None,
            },
            finish_reason: "stop".to_string(),
        }],
        usage: Some(Usage {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }),
    })
}
