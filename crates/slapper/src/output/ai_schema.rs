use crate::types::Severity;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiOutput {
    pub findings: Vec<AiFinding>,
    pub summary: AiSummary,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiFinding {
    pub title: String,
    pub severity: Severity,
    pub description: String,
    pub evidence: Vec<AiEvidence>,
    pub remediation: Vec<AiRemediation>,
    pub confidence: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiEvidence {
    pub source: String,
    pub content: String,
    pub relevance: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiRemediation {
    pub priority: u8,
    pub action: String,
    pub effort: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiSummary {
    pub total_findings: usize,
    pub critical_count: usize,
    pub high_count: usize,
    pub risk_score: f32,
    pub executive_summary: String,
}
