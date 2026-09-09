mod handlers;
pub mod types;

use crate::tool::registry::ToolRegistry;
use axum::{routing::post, Router};

/// Compatibility constructor: builds an inert fail-closed [`EngineServices`]
/// bundle and delegates to [`router_with_services`].
pub fn router(registry: ToolRegistry, api_key: Option<String>) -> Router {
    let enforcement = crate::config::EnforcementContext::for_surface(
        crate::config::ExecutionSurface::RestApi,
        crate::config::ExecutionPolicy::default(),
        crate::config::LoadedScope::default_empty(),
    );
    let services = crate::tool::service::EngineServices::new(registry.clone(), enforcement);
    router_with_services(registry, api_key, services)
}

/// Injected constructor (Phase D WS3): the caller supplies a prebuilt
/// [`EngineServices`](crate::tool::service::EngineServices) bundle.
pub fn router_with_services(
    registry: ToolRegistry,
    api_key: Option<String>,
    services: crate::tool::service::EngineServices,
) -> Router {
    let state = AppState {
        registry,
        api_key,
        services,
    };
    Router::new()
        .route("/v1/responses", post(handlers::create_response))
        .with_state(state)
}

#[derive(Clone)]
pub(crate) struct AppState {
    pub(crate) registry: ToolRegistry,
    pub(crate) api_key: Option<String>,
    pub(crate) services: crate::tool::service::EngineServices,
}
