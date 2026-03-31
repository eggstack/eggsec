use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamEvent {
    #[serde(rename = "event")]
    pub event_type: String,
    pub request_id: String,
    pub data: serde_json::Value,
}

impl StreamEvent {
    pub fn to_sse_data(&self) -> String {
        format!(
            "event: {}\ndata: {}\n\n",
            self.event_type,
            serde_json::to_string(&self.data).unwrap_or_default()
        )
    }
}
