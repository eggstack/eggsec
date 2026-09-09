//! Autonomous-agent dependency injection (Phase D WS5).
//!
//! The autonomous security agent historically constructed its default tool
//! registry/dispatcher internally (`create_default_registry()`). That coupled
//! orchestration to the engine composition root and made tests pay the cost
//! of building every tool.
//!
//! New code must inject execution via [`AgentExecutionService`]:
//!
//! - catalog/introspection comes from [`crate::tool::service::OperationCatalog`];
//! - approved execution/preflight comes from checked-only services;
//! - scheduler/coordination primitives remain engine-independent
//!   (`eggsec-agent` crate, `CronScheduler`);
//! - optional AI (`AiClient`) and persistence/alert channels are explicit
//!   constructor parameters, not ambient globals.
//!
//! The injected executor exposes *only* checked execution, so the agent
//! cannot bypass strict `SecurityAgent` enforcement: every dispatch requires
//! an [`crate::config::ApprovedOperation`] token issued for the
//! `AgentStrict` profile. Downgrade attempts (manual/guided profiles,
//! missing approval) fail closed.
//!
//! [`crate::tool::service::EngineServices`] implements this trait, so
//! production composition roots can pass it directly. Tests may substitute a
//! fake without building the entire tool registry.

use std::future::Future;
use std::pin::Pin;

use crate::config::{ApprovedOperation, EnforcementOutcome, ExecutionSurface, OperationDescriptor};
use crate::error::EggsecError;
use crate::tool::{ToolRequest, ToolResponse};

/// Checked-only execution + policy service for the autonomous agent.
///
/// Object-safe so tests can substitute `Arc<dyn AgentExecutionService>`.
/// Implementations must delegate authorization to
/// [`crate::config::EnforcementContext`]; they must not reimplement
/// scope/CIDR, feature, risk, or manifest-provenance checks.
pub trait AgentExecutionService: Send + Sync {
    /// Evaluate `descriptor` through shared enforcement (no execution).
    fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome;

    /// Strict approval for the `SecurityAgent` surface. Never honors manual
    /// overrides. Implementations must reject non-`AgentStrict` contexts.
    fn approve_security_agent(
        &self,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, crate::config::EnforcementError>;

    /// Checked dispatch under a previously issued approval token.
    /// Fails closed on any binding mismatch. No raw dispatch exists.
    fn dispatch_checked<'a>(
        &'a self,
        approved: &'a ApprovedOperation,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, EggsecError>> + Send + 'a>>;

    /// Borrow the enforcement profile surface for audit correlation.
    fn execution_surface(&self) -> ExecutionSurface {
        ExecutionSurface::SecurityAgent
    }
}

impl AgentExecutionService for crate::tool::service::EngineServices {
    fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
        crate::tool::service::EngineServices::evaluate(self, descriptor)
    }

    fn approve_security_agent(
        &self,
        descriptor: OperationDescriptor,
    ) -> Result<ApprovedOperation, crate::config::EnforcementError> {
        // Fail closed when the injected context is not AgentStrict: the
        // agent must never run under manual/guarded/MCP profiles even when
        // a caller supplies a mismatched bundle.
        if self.enforcement().execution_profile != crate::config::ExecutionProfile::AgentStrict {
            let decision = self.evaluate(&descriptor).decision().clone();
            return Err(crate::config::EnforcementError::Denied { decision });
        }
        self.approve(ExecutionSurface::SecurityAgent, descriptor)
    }

    fn dispatch_checked<'a>(
        &'a self,
        approved: &'a ApprovedOperation,
        request: ToolRequest,
    ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, EggsecError>> + Send + 'a>> {
        Box::pin(self.dispatch_checked(approved, request))
    }
}

#[cfg(test)]
pub(crate) mod test_helpers {
    use super::*;
    use crate::config::{
        EnforcementContext, ExecutionPolicy, LoadedScope, OperationDescriptor, OperationMode,
        OperationRisk,
    };
    use std::sync::Arc;

    /// Fake checked executor for agent tests: no tool registry required.
    pub struct FakeAgentExecutor {
        enforcement: EnforcementContext,
        response: ToolResponse,
    }

    impl FakeAgentExecutor {
        pub fn agent_strict(response: ToolResponse) -> Self {
            Self {
                enforcement: EnforcementContext::agent_strict(
                    ExecutionPolicy::default(),
                    LoadedScope::default_empty(),
                ),
                response,
            }
        }

        pub fn with_enforcement(enforcement: EnforcementContext, response: ToolResponse) -> Self {
            Self {
                enforcement,
                response,
            }
        }
    }

    impl AgentExecutionService for FakeAgentExecutor {
        fn evaluate(&self, descriptor: &OperationDescriptor) -> EnforcementOutcome {
            self.enforcement.evaluate(descriptor)
        }

        fn approve_security_agent(
            &self,
            descriptor: OperationDescriptor,
        ) -> Result<ApprovedOperation, crate::config::EnforcementError> {
            if self.enforcement.execution_profile != crate::config::ExecutionProfile::AgentStrict {
                let decision = self.evaluate(&descriptor).decision().clone();
                return Err(crate::config::EnforcementError::Denied { decision });
            }
            self.enforcement
                .approve(ExecutionSurface::SecurityAgent, descriptor)
        }

        fn dispatch_checked<'a>(
            &'a self,
            approved: &'a ApprovedOperation,
            _request: ToolRequest,
        ) -> Pin<Box<dyn Future<Output = Result<ToolResponse, EggsecError>> + Send + 'a>> {
            let response = self.response.clone();
            let expected_operation = approved.descriptor().operation.clone();
            Box::pin(async move {
                // Prove binding awareness in the fake: reject mismatched
                // operation IDs the same way the real dispatcher would.
                if _request.tool != expected_operation
                    && !crate::config::operation_matches_tool_id(
                        &_request.tool,
                        &expected_operation,
                    )
                {
                    return Err(EggsecError::Config(format!(
                        "fake dispatch binding failed: request '{}' != approved '{}'",
                        _request.tool, expected_operation
                    )));
                }
                Ok(response)
            })
        }
    }

    pub fn fake_tool_response() -> ToolResponse {
        ToolResponse::success("fake-req", "fake-tool", serde_json::json!({}))
    }

    pub fn sample_descriptor() -> OperationDescriptor {
        OperationDescriptor::new(
            "scan-ports".to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            Vec::new(),
            Some("127.0.0.1".to_string()),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        )
    }

    #[test]
    fn fake_agent_executor_rejects_non_agent_strict() {
        let enforcement = EnforcementContext::manual_permissive(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let fake = FakeAgentExecutor::with_enforcement(enforcement, fake_tool_response());
        let descriptor = sample_descriptor();
        assert!(fake.approve_security_agent(descriptor).is_err());
    }

    #[test]
    fn engine_services_rejects_non_agent_strict_for_agent() {
        use crate::tool::service::EngineServices;
        let enforcement = EnforcementContext::mcp_strict(
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let services = EngineServices::new(crate::tool::ToolRegistry::new(), enforcement);
        let descriptor = sample_descriptor();
        assert!(services
            .approve(crate::config::ExecutionSurface::SecurityAgent, descriptor)
            .is_err());
        // And via the agent trait entry point:
        let descriptor = sample_descriptor();
        assert!(AgentExecutionService::approve_security_agent(&services, descriptor).is_err());
    }
}
