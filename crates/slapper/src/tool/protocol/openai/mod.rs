use axum::{routing::post, Router};
use crate::tool::registry::ToolRegistry;

mod handlers;
pub mod types;

pub fn router(registry: ToolRegistry) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .with_state(registry)
}
