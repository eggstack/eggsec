mod handlers;
mod models;
pub mod types;

use axum::{routing::post, Router};
use std::sync::Arc;
use crate::config::Scope;
use crate::tool::registry::ToolRegistry;

#[derive(Clone)]
pub struct OpenAiState {
    pub registry: Arc<ToolRegistry>,
    pub api_key: Option<String>,
    pub scope: Option<Scope>,
}

pub fn router(registry: Arc<ToolRegistry>, api_key: Option<String>, scope: Option<Scope>) -> Router {
    let state: Arc<OpenAiState> = Arc::new(OpenAiState { registry, api_key, scope });

    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/models", axum::routing::get(models::list_models))
        .route("/v1/models/{model_id}", axum::routing::get(models::get_model))
        .with_state(state)
}
