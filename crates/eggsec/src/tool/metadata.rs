use crate::config::{IntendedUse, OperationMode, OperationRisk};
use serde::{Deserialize, Serialize};

/// Metadata for a tool that describes its capabilities and risk profile
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolMetadata {
    pub name: String,
    pub description: String,
    pub risk_tier: OperationRisk,
    pub requires_target_scope: bool,
    pub requires_explicit_enablement: bool,
    pub can_mutate_state: bool,
    pub can_send_network_traffic: bool,
    pub can_stress_load_test: bool,
    pub can_run_raw_packet_ops: bool,
    pub input_schema: Option<serde_json::Value>,
    pub output_schema: Option<serde_json::Value>,
    pub operation_mode: OperationMode,
    pub intended_uses: Vec<IntendedUse>,
}

impl ToolMetadata {
    /// Check if this tool is allowed by the given policy
    pub fn is_allowed_by(&self, policy: &crate::config::ExecutionPolicy) -> bool {
        if self.requires_explicit_enablement && !self.risk_tier.is_allowed_by(policy) {
            return false;
        }
        true
    }

    /// Get a human-readable summary of restrictions
    pub fn restrictions(&self) -> Vec<String> {
        let mut restrictions = Vec::new();
        if self.requires_target_scope {
            restrictions.push("Requires target scope validation".to_string());
        }
        if self.requires_explicit_enablement {
            restrictions.push("Requires explicit policy enablement".to_string());
        }
        if self.can_stress_load_test {
            restrictions.push("Can perform stress/load testing".to_string());
        }
        if self.can_run_raw_packet_ops {
            restrictions.push("Can run raw packet operations".to_string());
        }
        if self.can_mutate_state {
            restrictions.push("Can mutate state".to_string());
        }
        restrictions
    }

    pub fn lab_classification(&self) -> String {
        let mode = match self.operation_mode {
            crate::config::OperationMode::StandardAssessment => "standard",
            crate::config::OperationMode::DefenseLab => "defense-lab",
            crate::config::OperationMode::HazardousLab => "hazardous-lab",
        };
        let uses: Vec<_> = self.intended_uses.iter().map(|u| u.label()).collect();
        format!("{}: {}", mode, uses.join(", "))
    }

    #[cfg(feature = "rest-api")]
    pub fn is_available_for_profile(&self, profile: &crate::tool::protocol::mcp::profile::McpProfile) -> bool {
        use crate::tool::protocol::mcp::profile::McpProfile;
        match profile {
            McpProfile::OpsAgent => true,
            McpProfile::CodingAgent => {
                matches!(
                    self.operation_mode,
                    crate::config::OperationMode::StandardAssessment
                ) && !self.can_stress_load_test
                    && !self.can_run_raw_packet_ops
            }
        }
    }
}

/// Registry of tool metadata
pub struct ToolMetadataRegistry {
    tools: rustc_hash::FxHashMap<String, ToolMetadata>,
}

impl ToolMetadataRegistry {
    pub fn new() -> Self {
        Self {
            tools: rustc_hash::FxHashMap::default(),
        }
    }

    pub fn register(&mut self, metadata: ToolMetadata) {
        self.tools.insert(metadata.name.clone(), metadata);
    }

    pub fn get(&self, name: &str) -> Option<&ToolMetadata> {
        self.tools.get(name)
    }

    pub fn is_tool_allowed(&self, name: &str, policy: &crate::config::ExecutionPolicy) -> bool {
        self.get(name)
            .map(|m| m.is_allowed_by(policy))
            .unwrap_or(false)
    }

    pub fn list_tools(&self) -> Vec<&ToolMetadata> {
        self.tools.values().collect()
    }

    pub fn list_blocked_tools(
        &self,
        policy: &crate::config::ExecutionPolicy,
    ) -> Vec<&ToolMetadata> {
        self.tools
            .values()
            .filter(|m| !m.is_allowed_by(policy))
            .collect()
    }
}

impl Default for ToolMetadataRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Create a default registry with common security tools registered
pub fn default_tool_registry() -> ToolMetadataRegistry {
    let mut registry = ToolMetadataRegistry::new();

    // Passive tools - always allowed
    registry.register(ToolMetadata {
        name: "plan".to_string(),
        description: "Create an AI-driven execution plan".to_string(),
        risk_tier: OperationRisk::Passive,
        requires_target_scope: false,
        requires_explicit_enablement: false,
        can_mutate_state: false,
        can_send_network_traffic: false,
        can_stress_load_test: false,
        can_run_raw_packet_ops: false,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::StandardAssessment,
        intended_uses: vec![IntendedUse::CodingAgentVerification],
    });

    registry.register(ToolMetadata {
        name: "scan_ports".to_string(),
        description: "Scan target ports for open services".to_string(),
        risk_tier: OperationRisk::SafeActive,
        requires_target_scope: true,
        requires_explicit_enablement: false,
        can_mutate_state: false,
        can_send_network_traffic: true,
        can_stress_load_test: false,
        can_run_raw_packet_ops: false,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::StandardAssessment,
        intended_uses: vec![IntendedUse::WebAssessment],
    });

    registry.register(ToolMetadata {
        name: "fuzz".to_string(),
        description: "Fuzz target endpoints with payloads".to_string(),
        risk_tier: OperationRisk::Intrusive,
        requires_target_scope: true,
        requires_explicit_enablement: true,
        can_mutate_state: false,
        can_send_network_traffic: true,
        can_stress_load_test: false,
        can_run_raw_packet_ops: false,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::StandardAssessment,
        intended_uses: vec![IntendedUse::WebAssessment],
    });

    registry.register(ToolMetadata {
        name: "stress".to_string(),
        description: "Stress test target with high-rate traffic".to_string(),
        risk_tier: OperationRisk::StressTest,
        requires_target_scope: true,
        requires_explicit_enablement: true,
        can_mutate_state: false,
        can_send_network_traffic: true,
        can_stress_load_test: true,
        can_run_raw_packet_ops: false,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::HazardousLab,
        intended_uses: vec![IntendedUse::DistributedSystemStress],
    });

    registry.register(ToolMetadata {
        name: "raw_packet_send".to_string(),
        description: "Send raw network packets".to_string(),
        risk_tier: OperationRisk::RawPacket,
        requires_target_scope: true,
        requires_explicit_enablement: true,
        can_mutate_state: false,
        can_send_network_traffic: true,
        can_stress_load_test: false,
        can_run_raw_packet_ops: true,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::HazardousLab,
        intended_uses: vec![IntendedUse::ProtocolEdgeValidation],
    });

    registry.register(ToolMetadata {
        name: "credential_test".to_string(),
        description: "Test credentials against target".to_string(),
        risk_tier: OperationRisk::CredentialTesting,
        requires_target_scope: true,
        requires_explicit_enablement: true,
        can_mutate_state: false,
        can_send_network_traffic: true,
        can_stress_load_test: false,
        can_run_raw_packet_ops: false,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::StandardAssessment,
        intended_uses: vec![IntendedUse::WebAssessment],
    });

    registry.register(ToolMetadata {
        name: "remote_exec".to_string(),
        description: "Execute commands on remote target".to_string(),
        risk_tier: OperationRisk::RemoteExecution,
        requires_target_scope: true,
        requires_explicit_enablement: true,
        can_mutate_state: true,
        can_send_network_traffic: true,
        can_stress_load_test: false,
        can_run_raw_packet_ops: false,
        input_schema: None,
        output_schema: None,
        operation_mode: OperationMode::HazardousLab,
        intended_uses: vec![IntendedUse::DistributedSystemStress],
    });

    registry
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ExecutionPolicy;

    #[test]
    fn default_registry_has_tools() {
        let registry = default_tool_registry();
        assert!(registry.get("plan").is_some());
        assert!(registry.get("fuzz").is_some());
        assert!(registry.get("stress").is_some());
    }

    #[test]
    fn passive_tools_always_allowed() {
        let registry = default_tool_registry();
        let policy = ExecutionPolicy::default();
        assert!(registry.is_tool_allowed("plan", &policy));
    }

    #[test]
    fn intrusive_tools_blocked_by_default() {
        let registry = default_tool_registry();
        let policy = ExecutionPolicy::default();
        assert!(!registry.is_tool_allowed("fuzz", &policy));
        assert!(!registry.is_tool_allowed("stress", &policy));
    }

    #[test]
    fn intrusive_tools_allowed_when_enabled() {
        let mut registry = default_tool_registry();
        let mut policy = ExecutionPolicy::default();
        policy.allow_intrusive_fuzzing = true;
        assert!(registry.is_tool_allowed("fuzz", &policy));
    }

    #[test]
    fn blocked_tools_listed() {
        let registry = default_tool_registry();
        let policy = ExecutionPolicy::default();
        let blocked = registry.list_blocked_tools(&policy);
        assert!(!blocked.is_empty());
        assert!(blocked.iter().any(|m| m.name == "fuzz"));
    }

    #[test]
    fn tool_metadata_restrictions() {
        let meta = ToolMetadata {
            name: "test".to_string(),
            description: "Test".to_string(),
            risk_tier: OperationRisk::StressTest,
            requires_target_scope: true,
            requires_explicit_enablement: true,
            can_mutate_state: false,
            can_send_network_traffic: true,
            can_stress_load_test: true,
            can_run_raw_packet_ops: false,
            input_schema: None,
            output_schema: None,
            operation_mode: OperationMode::HazardousLab,
            intended_uses: vec![IntendedUse::DistributedSystemStress],
        };
        let restrictions = meta.restrictions();
        assert!(restrictions.iter().any(|r| r.contains("scope")));
        assert!(restrictions.iter().any(|r| r.contains("stress")));
    }

    #[test]
    fn lab_classification_standard() {
        let meta = ToolMetadata {
            name: "plan".to_string(),
            description: "Create plan".to_string(),
            risk_tier: OperationRisk::Passive,
            requires_target_scope: false,
            requires_explicit_enablement: false,
            can_mutate_state: false,
            can_send_network_traffic: false,
            can_stress_load_test: false,
            can_run_raw_packet_ops: false,
            operation_mode: crate::config::OperationMode::StandardAssessment,
            intended_uses: vec![crate::config::IntendedUse::WebAssessment],
            input_schema: None,
            output_schema: None,
        };
        let cls = meta.lab_classification();
        assert!(cls.contains("standard"));
    }

    #[test]
    #[cfg(feature = "rest-api")]
    fn coding_agent_blocks_hazardous() {
        let meta = ToolMetadata {
            name: "stress".to_string(),
            description: "Stress test".to_string(),
            risk_tier: OperationRisk::StressTest,
            requires_target_scope: true,
            requires_explicit_enablement: true,
            can_mutate_state: false,
            can_send_network_traffic: true,
            can_stress_load_test: true,
            can_run_raw_packet_ops: false,
            operation_mode: crate::config::OperationMode::HazardousLab,
            intended_uses: vec![crate::config::IntendedUse::DistributedSystemStress],
            input_schema: None,
            output_schema: None,
        };
        assert!(!meta.is_available_for_profile(&crate::tool::protocol::mcp::profile::McpProfile::CodingAgent));
        assert!(meta.is_available_for_profile(&crate::tool::protocol::mcp::profile::McpProfile::OpsAgent));
    }

    #[test]
    #[cfg(feature = "rest-api")]
    fn coding_agent_allows_standard() {
        let meta = ToolMetadata {
            name: "scan_ports".to_string(),
            description: "Scan ports".to_string(),
            risk_tier: OperationRisk::SafeActive,
            requires_target_scope: true,
            requires_explicit_enablement: false,
            can_mutate_state: false,
            can_send_network_traffic: true,
            can_stress_load_test: false,
            can_run_raw_packet_ops: false,
            operation_mode: crate::config::OperationMode::StandardAssessment,
            intended_uses: vec![crate::config::IntendedUse::WebAssessment],
            input_schema: None,
            output_schema: None,
        };
        assert!(meta.is_available_for_profile(&crate::tool::protocol::mcp::profile::McpProfile::CodingAgent));
    }
}
