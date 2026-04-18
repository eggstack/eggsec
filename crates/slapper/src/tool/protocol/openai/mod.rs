mod handlers;
mod models;
pub mod types;

use axum::{routing::post, Router};
use std::sync::Arc;
use crate::tool::registry::ToolRegistry;

#[derive(Clone)]
pub struct OpenAiState {
    pub registry: ToolRegistry,
    pub api_key: Option<String>,
}

pub fn router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let state = Arc::new(OpenAiState { registry, api_key });
    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .merge(models::router(state.registry.clone()))
        .with_state(state)
}
