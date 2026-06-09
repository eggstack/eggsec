mod handlers;
pub mod types;

use crate::tool::registry::ToolRegistry;
use axum::{routing::post, Router};

pub fn router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let state = AppState { registry, api_key };
    Router::new()
        .route("/v1/responses", post(handlers::create_response))
        .with_state(state)
}

#[derive(Clone)]
struct AppState {
    registry: ToolRegistry,
    api_key: Option<String>,
}
