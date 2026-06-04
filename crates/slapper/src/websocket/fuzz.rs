use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FuzzTestResult {
    pub test_name: String,
    pub payload_size: usize,
    pub sent: bool,
    pub connection_dropped: bool,
    pub server_response: Option<String>,
    pub vulnerability_detected: bool,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_result_creation() {
        let result = FuzzTestResult {
            test_name: "Large message".to_string(),
            payload_size: 1000,
            sent: true,
            connection_dropped: false,
            server_response: None,
            vulnerability_detected: false,
            details: "Test".to_string(),
        };
        assert_eq!(result.payload_size, 1000);
    }
}
