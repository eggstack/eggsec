use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OriginTestResult {
    pub origin: String,
    pub accepted: bool,
    pub status_code: Option<u16>,
    pub details: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_origin_result_creation() {
        let result = OriginTestResult {
            origin: "https://evil.com".to_string(),
            accepted: true,
            status_code: Some(101),
            details: "Accepted".to_string(),
        };
        assert!(result.accepted);
    }
}
