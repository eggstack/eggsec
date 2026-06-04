use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InjectionTestResult {
    pub payload: String,
    pub sent: bool,
    pub received_response: bool,
    pub response_content: Option<String>,
    pub vulnerability_detected: bool,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_injection_result_creation() {
        let result = InjectionTestResult {
            payload: "test".to_string(),
            sent: true,
            received_response: true,
            response_content: Some("error".to_string()),
            vulnerability_detected: true,
            details: "Test".to_string(),
        };
        assert!(result.vulnerability_detected);
    }
}
