//! MCP-engine bridge (Phase D WS4).
//!
//! `McpServer` owns transport concerns: JSON-RPC/wire handling, MCP
//! profile/tool visibility policy, session/resource bookkeeping, streaming
//! adaptation, and optional AI/prompts integration.
//!
//! Engine concerns — operation catalog/introspection, authorization
//! (`EnforcementContext`), and checked execution — live behind the injected
//! [`crate::tool::service::EngineServices`] bundle. This module is the narrow
//! bridge between those worlds:
//!
//! - wire/profile/session code calls into [`McpEngineBridge`] for
//!   descriptor construction, policy decisions, approval, and dispatch;
//! - the bridge delegates to shared engine services and never duplicates
//!   `EnforcementContext` logic;
//! - transport/wire modules never hold concrete `ToolRegistry`,
//!   `ToolDispatcher`, or `AiClient` directly for execution.
//!
//! A small bridge remains in `eggsec` intentionally: moving it into a
//! separate protocol crate would invert dependencies (the bridge needs
//! engine policy types) or create a second composition root.

use crate::config::{OperationDescriptor, PolicyDecision};
use crate::error::EggsecError;
use crate::tool::protocol::mcp::policy::McpProfilePolicy;
use crate::tool::service::EngineServices;
use crate::tool::{ToolRequest, ToolResponse};

/// Narrow engine bridge for MCP handlers.
///
/// Constructed once per `McpServer` from an injected [`EngineServices`]
/// bundle plus the MCP profile policy (tool visibility layer on top of
/// shared enforcement).
#[derive(Clone)]
pub struct McpEngineBridge {
    services: EngineServices,
}

impl McpEngineBridge {
    /// Build a bridge from prebuilt engine services.
    pub fn new(services: EngineServices) -> Self {
        Self { services }
    }

    /// Borrow the underlying engine services (catalog/executor/enforcement).
    pub fn services(&self) -> &EngineServices {
        &self.services
    }

    /// Evaluate `descriptor` through the shared enforcement path.
    ///
    /// MCP profile visibility (`McpProfilePolicy`) is applied by callers on
    /// top of this decision; see
    /// `policy_decision_for_mcp_call_with_enforcement`.
    pub fn evaluate(&self, descriptor: &OperationDescriptor) -> crate::config::EnforcementOutcome {
        self.services.evaluate(descriptor)
    }

    /// Policy decision for an MCP tool call, combining shared enforcement
    /// with the MCP profile layer. Never duplicates `EnforcementContext`
    /// internals; see `policy.rs` for the shared helper.
    pub fn decision_for_mcp_call(
        &self,
        profile_policy: &McpProfilePolicy,
        tool_id: &str,
        capability: Option<&str>,
        arguments: &serde_json::Value,
    ) -> PolicyDecision {
        crate::tool::protocol::mcp::policy::policy_decision_for_mcp_call_with_enforcement(
            profile_policy,
            tool_id,
            capability,
            arguments,
            self.services.enforcement(),
        )
    }

    /// Checked dispatch under a previously issued approval token.
    ///
    /// Fails closed on any tool/target binding mismatch. There is no raw
    /// dispatch path through this bridge.
    pub async fn dispatch_checked(
        &self,
        approved: &crate::config::ApprovedOperation,
        request: ToolRequest,
    ) -> Result<ToolResponse, EggsecError> {
        self.services.dispatch_checked(approved, request).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutionPolicy, LoadedScope};

    #[test]
    fn bridge_delegates_evaluation_to_engine_services() {
        let enforcement = crate::config::EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let services = EngineServices::new(crate::tool::ToolRegistry::new(), enforcement.clone());
        let bridge = McpEngineBridge::new(services);
        let descriptor = crate::config::OperationMetadata::try_descriptor_for_target(
            &crate::config::all_operation_metadata()[0],
            Some("127.0.0.1"),
        )
        .unwrap_or_else(|_| {
            crate::config::all_operation_metadata()[0]
                .descriptor_for_target(Some("127.0.0.1".to_string()))
        });
        let via_bridge = bridge.evaluate(&descriptor);
        let via_context = enforcement.evaluate(&descriptor);
        assert_eq!(
            via_bridge.decision().allowed,
            via_context.decision().allowed
        );
    }
}
