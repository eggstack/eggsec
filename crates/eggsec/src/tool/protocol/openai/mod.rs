mod handlers;
mod models;
pub mod types;

use crate::config::Scope;
use crate::tool::registry::ToolRegistry;
use axum::{routing::post, Router};
use std::sync::Arc;

#[derive(Clone)]
pub struct OpenAiState {
    pub registry: Arc<ToolRegistry>,
    pub api_key: Option<String>,
    /// Deprecated compat: authorization comes from `services.enforcement()`,
    /// not from this DTO. Retained so the `router` signature stays stable;
    /// handlers ignore it. See Phase D WS3.
    pub scope: Option<Scope>,
    /// Injected engine services (authoritative for approve/dispatch).
    pub services: crate::tool::service::EngineServices,
}

/// Compatibility constructor: builds an inert fail-closed [`EngineServices`]
/// bundle (strict profile + empty scope) and delegates to
/// [`router_with_services`].
///
/// The `scope` DTO is ignored for authorization; it remains in the signature
/// only for backward compatibility. New composition roots must use
/// `router_with_services` with an explicit `EnforcementContext`.
pub fn router(
    registry: Arc<ToolRegistry>,
    api_key: Option<String>,
    scope: Option<Scope>,
) -> Router {
    let _ = scope.as_ref();
    let enforcement = crate::config::EnforcementContext::for_surface(
        crate::config::ExecutionSurface::RestApi,
        crate::config::ExecutionPolicy::default(),
        crate::config::LoadedScope::default_empty(),
    );
    // Rebuild an owned registry for the services bundle; the adapter keeps
    // the original `Arc<ToolRegistry>` for introspection (`list`).
    let services = crate::tool::service::EngineServices::new((*registry).clone(), enforcement);
    router_with_services(registry, api_key, services)
}

/// Injected constructor (Phase D WS3): the caller supplies a prebuilt
/// [`EngineServices`](crate::tool::service::EngineServices) bundle.
/// Transport-only concerns (`api_key`) stay adapter-owned; authorization and
/// execution stay in the engine service path.
pub fn router_with_services(
    registry: Arc<ToolRegistry>,
    api_key: Option<String>,
    services: crate::tool::service::EngineServices,
) -> Router {
    let state: Arc<OpenAiState> = Arc::new(OpenAiState {
        registry,
        api_key,
        scope: None,
        services,
    });

    Router::new()
        .route("/v1/chat/completions", post(handlers::chat_completions))
        .route("/v1/models", axum::routing::get(models::list_models))
        .route(
            "/v1/models/{model_id}",
            axum::routing::get(models::get_model),
        )
        .with_state(state)
}
