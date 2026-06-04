use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResult {
    pub url: String,
    pub connected: bool,
    pub response_headers: Vec<(String, String)>,
    pub subprotocols: Vec<String>,
    pub extensions: Vec<String>,
    pub latency_ms: Option<f64>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_result_creation() {
        let result = ConnectionTestResult {
            url: "ws://example.com".to_string(),
            connected: false,
            response_headers: Vec::new(),
            subprotocols: Vec::new(),
            extensions: Vec::new(),
            latency_ms: None,
            error: Some("Test error".to_string()),
        };
        assert!(!result.connected);
        assert!(result.error.is_some());
    }
}
