mod handlers;
mod models;
pub mod types;

use axum::{routing::post, Router};
use crate::tool::registry::ToolRegistry;

pub fn router(registry: ToolRegistry) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .merge(models::router(registry.clone()))
        .with_state(registry)
}
