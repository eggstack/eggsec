use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiAnalysisResult {
    pub reassessed_severity: String,
    pub exploitability: String,
    pub impact: String,
    pub remediation: Vec<String>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPayloadSuggestion {
    pub payload: String,
    pub description: String,
    pub expected_result: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiWafBypassSuggestion {
    pub technique: String,
    pub payload: String,
    pub explanation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanFinding {
    pub id: String,
    pub title: String,
    pub severity: Severity,
    pub description: String,
}
