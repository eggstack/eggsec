mod handlers;
mod models;
pub mod types;

use axum::{routing::post, Router};
use std::sync::Arc;
use crate::tool::registry::ToolRegistry;

#[derive(Clone)]
pub struct OpenAiState {
    pub registry: Arc<ToolRegistry>,
    pub api_key: Option<String>,
}

pub fn router(registry: Arc<ToolRegistry>, api_key: Option<String>) -> Router {
    let state: Arc<OpenAiState> = Arc::new(OpenAiState { registry, api_key });

    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/models", axum::routing::get(models::list_models))
        .route("/v1/models/{model_id}", axum::routing::get(models::get_model))
        .with_state(state)
}
