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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ai_analysis_result_serde() {
        let result = AiAnalysisResult {
            reassessed_severity: "High".to_string(),
            exploitability: "Medium".to_string(),
            impact: "Significant".to_string(),
            remediation: vec!["Fix XSS".to_string(), "Update headers".to_string()],
            confidence: 0.85,
        };
        let json = serde_json::to_string(&result).unwrap();
        let deserialized: AiAnalysisResult = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.reassessed_severity, "High");
        assert_eq!(deserialized.exploitability, "Medium");
        assert_eq!(deserialized.impact, "Significant");
        assert_eq!(deserialized.remediation.len(), 2);
        assert_eq!(deserialized.confidence, 0.85);
    }

    #[test]
    fn test_ai_payload_suggestion_serde() {
        let suggestion = AiPayloadSuggestion {
            payload: "<script>alert(1)</script>".to_string(),
            description: "XSS payload".to_string(),
            expected_result: "Alert popup".to_string(),
        };
        let json = serde_json::to_string(&suggestion).unwrap();
        let deserialized: AiPayloadSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.payload, suggestion.payload);
        assert_eq!(deserialized.description, suggestion.description);
        assert_eq!(deserialized.expected_result, suggestion.expected_result);
    }

    #[test]
    fn test_ai_waf_bypass_suggestion_serde() {
        let suggestion = AiWafBypassSuggestion {
            technique: "Case manipulation".to_string(),
            payload: "<ScRiPt>alert(1)</sCrIpT>".to_string(),
            explanation: "WAFs often miss mixed case payloads".to_string(),
        };
        let json = serde_json::to_string(&suggestion).unwrap();
        let deserialized: AiWafBypassSuggestion = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.technique, suggestion.technique);
        assert_eq!(deserialized.payload, suggestion.payload);
        assert_eq!(deserialized.explanation, suggestion.explanation);
    }

    #[test]
    fn test_scan_finding_serde() {
        let finding = ScanFinding {
            id: "FIND-001".to_string(),
            title: "Reflected XSS".to_string(),
            severity: Severity::High,
            description: "User input reflected without sanitization".to_string(),
        };
        let json = serde_json::to_string(&finding).unwrap();
        let deserialized: ScanFinding = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.id, "FIND-001");
        assert_eq!(deserialized.title, "Reflected XSS");
        assert_eq!(deserialized.severity, Severity::High);
    }

    #[test]
    fn test_scan_finding_all_severity_levels() {
        for severity in &[
            Severity::Critical,
            Severity::High,
            Severity::Medium,
            Severity::Low,
            Severity::Info,
        ] {
            let finding = ScanFinding {
                id: "test".to_string(),
                title: "Test".to_string(),
                severity: *severity,
                description: "Test".to_string(),
            };
            let json = serde_json::to_string(&finding).unwrap();
            let deserialized: ScanFinding = serde_json::from_str(&json).unwrap();
            assert_eq!(deserialized.severity, *severity);
        }
    }
}
