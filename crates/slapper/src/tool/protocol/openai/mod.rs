use axum::{routing::post, Router};
use crate::tool::registry::ToolRegistry;

mod handlers;
mod types;

pub fn router(_registry: ToolRegistry) -> Router {
    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
}
