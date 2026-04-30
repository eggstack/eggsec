use serde::Serialize;
use std::time::{SystemTime, UNIX_EPOCH};
use std::sync::Arc;

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

pub async fn list_models(
    axum::extract::State(_state): axum::extract::State<Arc<crate::tool::protocol::openai::OpenAiState>>,
) -> axum::Json<ModelList> {
    axum::Json(ModelList {
        object: "list".to_string(),
        data: slapper_models(),
    })
}

pub async fn get_model(
    axum::extract::State(_state): axum::extract::State<Arc<crate::tool::protocol::openai::OpenAiState>>,
    axum::extract::Path(model_id): axum::extract::Path<String>,
) -> Result<impl axum::response::IntoResponse, impl axum::response::IntoResponse> {
    let models = slapper_models();
    match models.into_iter().find(|m| m.id == model_id) {
        Some(model) => Ok(axum::Json(model)),
        None => Err(axum::http::StatusCode::NOT_FOUND),
    }
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