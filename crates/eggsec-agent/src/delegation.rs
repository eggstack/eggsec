use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationRequest {
    pub id: Uuid,
    pub task_type: String,
    pub target: String,
    pub parameters: serde_json::Value,
    pub callback_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DelegationResponse {
    pub delegation_id: Uuid,
    pub agent_id: Uuid,
    pub status: String,
    pub result_url: Option<String>,
}
