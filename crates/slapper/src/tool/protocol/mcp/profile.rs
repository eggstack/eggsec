use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum McpProfile {
    OpsAgent,
    CodingAgent,
}

impl Default for McpProfile {
    fn default() -> Self {
        McpProfile::OpsAgent
    }
}

impl McpProfile {
    pub fn server_name(&self) -> &str {
        match self {
            McpProfile::OpsAgent => "slapper-tool-api",
            McpProfile::CodingAgent => "slapper-coding-agent-mcp",
        }
    }

    pub fn server_description(&self) -> &str {
        match self {
            McpProfile::OpsAgent => "High-performance security testing toolkit for AI agents",
            McpProfile::CodingAgent => "Bounded live security validation tools for coding agents",
        }
    }

    pub fn is_coding_agent(&self) -> bool {
        matches!(self, McpProfile::CodingAgent)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_profile() {
        assert_eq!(McpProfile::default(), McpProfile::OpsAgent);
    }

    #[test]
    fn test_server_names() {
        assert_eq!(McpProfile::OpsAgent.server_name(), "slapper-tool-api");
        assert_eq!(
            McpProfile::CodingAgent.server_name(),
            "slapper-coding-agent-mcp"
        );
    }

    #[test]
    fn test_is_coding_agent() {
        assert!(!McpProfile::OpsAgent.is_coding_agent());
        assert!(McpProfile::CodingAgent.is_coding_agent());
    }

    #[test]
    fn test_serde_roundtrip() {
        let ops = McpProfile::OpsAgent;
        let coding = McpProfile::CodingAgent;

        let ops_json = serde_json::to_string(&ops).unwrap();
        let coding_json = serde_json::to_string(&coding).unwrap();

        assert_eq!(ops_json, "\"ops-agent\"");
        assert_eq!(coding_json, "\"coding-agent\"");

        let ops_de: McpProfile = serde_json::from_str(&ops_json).unwrap();
        let coding_de: McpProfile = serde_json::from_str(&coding_json).unwrap();

        assert_eq!(ops_de, McpProfile::OpsAgent);
        assert_eq!(coding_de, McpProfile::CodingAgent);
    }
}
