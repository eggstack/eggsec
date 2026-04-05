use axum::{routing::get, Router};
use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::tool::registry::ToolRegistry;

#[derive(Debug, Clone, Serialize)]
pub struct Model {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub owned_by: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelList {
    pub object: String,
    pub data: Vec<Model>,
}

fn slapper_models() -> Vec<Model> {
    let created = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    vec![
        Model {
            id: "slapper-recon".to_string(),
            object: "model".to_string(),
            created,
            owned_by: "slapper".to_string(),
        },
        Model {
            id: "slapper-fuzzer".to_string(),
            object: "model".to_string(),
            created,
            owned_by: "slapper".to_string(),
        },
        Model {
            id: "slapper-waf".to_string(),
            object: "model".to_string(),
            created,
            owned_by: "slapper".to_string(),
        },
        Model {
            id: "slapper-scanner".to_string(),
            object: "model".to_string(),
            created,
            owned_by: "slapper".to_string(),
        },
        Model {
            id: "slapper-loadtest".to_string(),
            object: "model".to_string(),
            created,
            owned_by: "slapper".to_string(),
        },
        Model {
            id: "slapper-pipeline".to_string(),
            object: "model".to_string(),
            created,
            owned_by: "slapper".to_string(),
        },
    ]
}

async fn list_models(
    axum::extract::State(_registry): axum::extract::State<ToolRegistry>,
) -> axum::Json<ModelList> {
    axum::Json(ModelList {
        object: "list".to_string(),
        data: slapper_models(),
    })
}

async fn get_model(
    axum::extract::State(_registry): axum::extract::State<ToolRegistry>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> impl axum::response::IntoResponse {
    let models = slapper_models();
    match models.into_iter().find(|m| m.id == model_id) {
        Some(model) => axum::response::Response::builder()
            .status(axum::http::StatusCode::OK)
            .header("content-type", "application/json")
            .body(serde_json::to_string(&model).unwrap())
            .unwrap(),
        None => axum::response::Response::builder()
            .status(axum::http::StatusCode::NOT_FOUND)
            .header("content-type", "application/json")
            .body(
                serde_json::to_string(&serde_json::json!({
                    "error": {
                        "message": format!("Model '{}' not found", model_id),
                        "type": "invalid_request_error",
                        "param": "model",
                        "code": "model_not_found",
                    }
                }))
                .unwrap(),
            )
            .unwrap(),
    }
}

pub fn router(registry: ToolRegistry) -> Router<ToolRegistry> {
    Router::new()
        .route("/v1/models", get(list_models))
        .route("/v1/models/{model_id}", get(get_model))
        .with_state(registry)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_slapper_models_count() {
        let models = slapper_models();
        assert_eq!(models.len(), 6);
    }

    #[test]
    fn test_slapper_models_owned_by() {
        let models = slapper_models();
        for model in models {
            assert_eq!(model.owned_by, "slapper");
        }
    }

    #[test]
    fn test_slapper_models_expected_ids() {
        let models = slapper_models();
        let ids: Vec<&str> = models.iter().map(|m| m.id.as_str()).collect();
        assert!(ids.contains(&"slapper-recon"));
        assert!(ids.contains(&"slapper-fuzzer"));
        assert!(ids.contains(&"slapper-waf"));
        assert!(ids.contains(&"slapper-scanner"));
        assert!(ids.contains(&"slapper-loadtest"));
        assert!(ids.contains(&"slapper-pipeline"));
    }

    #[test]
    fn test_model_list_serialization() {
        let list = ModelList {
            object: "list".to_string(),
            data: slapper_models(),
        };
        let json = serde_json::to_string(&list).unwrap();
        assert!(json.contains("\"object\":\"list\""));
        assert!(json.contains("\"data\":["));
    }
}
