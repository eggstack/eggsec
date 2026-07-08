use crate::config::{
    preflight_operation, ApprovedOperation, EnforcementContext, ExecutionPolicy, LoadedScope,
    ManualOverride, PreflightResult,
};
use eggsec_runtime::request::RuntimeSurface;
use eggsec_runtime::RunRequest;

use super::descriptor::descriptor_for_run_request;
use super::surface::{runtime_surface_to_execution_surface, RuntimeBridgeError};

/// Preflight a runtime request before dispatch.
///
/// Converts the runtime surface and request into canonical enforcement types,
/// then delegates to the existing `preflight_operation()` logic. This is the
/// entry point for daemon-backed frontends to check whether an operation is
/// allowed, requires confirmation, or is denied.
pub fn preflight_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<PreflightResult, RuntimeBridgeError> {
    let exec_surface = runtime_surface_to_execution_surface(surface)?;
    let descriptor = descriptor_for_run_request(request)?;

    if !exec_surface.honors_manual_override() && manual_override.is_some() {
        return Err(RuntimeBridgeError::ManualOverrideRejected {
            surface: exec_surface.to_string(),
        });
    }

    let enforcement = EnforcementContext::for_surface(exec_surface, policy, loaded_scope);
    Ok(preflight_operation(
        exec_surface,
        &enforcement,
        descriptor,
        manual_override,
    ))
}

/// Approve a runtime request for dispatch.
///
/// Returns an [`ApprovedOperation`] only if the enforcement layer permits the
/// operation. Manual surfaces use `approve_manual()`; strict/automated surfaces
/// use `approve()`. Manual overrides are rejected for automated surfaces.
pub fn approve_run_request(
    surface: RuntimeSurface,
    policy: ExecutionPolicy,
    loaded_scope: LoadedScope,
    request: &RunRequest,
    manual_override: Option<&ManualOverride>,
) -> Result<ApprovedOperation, RuntimeBridgeError> {
    let exec_surface = runtime_surface_to_execution_surface(surface)?;
    let descriptor = descriptor_for_run_request(request)?;

    let enforcement = EnforcementContext::for_surface(exec_surface, policy, loaded_scope);

    if exec_surface.honors_manual_override() {
        enforcement
            .approve_manual(exec_surface, descriptor, manual_override)
            .map_err(|e| RuntimeBridgeError::EnforcementDenied {
                reason: e.to_string(),
            })
    } else {
        if manual_override.is_some() {
            return Err(RuntimeBridgeError::ManualOverrideRejected {
                surface: exec_surface.to_string(),
            });
        }
        enforcement.approve(exec_surface, descriptor).map_err(|e| {
            RuntimeBridgeError::EnforcementDenied {
                reason: e.to_string(),
            }
        })
    }
}

/// Error type for runtime bridge conversions.
///
/// This extends the base `RuntimeBridgeError` with enforcement-specific variants.
// Note: The `RuntimeBridgeError` in surface.rs already defines the core variants.
// We add `EnforcementDenied` here for the approval path.

// We need to extend RuntimeBridgeError. Since it's defined in surface.rs,
// we re-export and extend it. Actually, let's add the variant there.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{ExecutionPolicy, ExecutionSurface, LoadedScope, OperationRisk};
    use eggsec_runtime::request::*;

    fn default_policy() -> ExecutionPolicy {
        ExecutionPolicy::default()
    }

    fn make_request(task_kind: TaskKind, surface: RuntimeSurface) -> RunRequest {
        RunRequest {
            task_kind,
            requested_by: None,
            surface,
            labels: vec![],
        }
    }

    fn port_scan_request(surface: RuntimeSurface) -> RunRequest {
        make_request(
            TaskKind::PortScan(PortScanParams {
                target: "10.0.0.1".into(),
                ports: Some("80".into()),
                scan_type: None,
                timeout_ms: None,
            }),
            surface,
        )
    }

    fn fuzz_request(surface: RuntimeSurface) -> RunRequest {
        make_request(
            TaskKind::Fuzz(FuzzParams {
                target: "https://example.com".into(),
                payload_type: None,
                threads: None,
            }),
            surface,
        )
    }

    #[test]
    fn cli_manual_preflight_allows_port_scan() {
        use crate::config::PreflightOutcomeKind;
        let req = port_scan_request(RuntimeSurface::CliManual);
        let result = preflight_run_request(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        )
        .unwrap();
        assert!(
            matches!(
                result.outcome_kind,
                PreflightOutcomeKind::Allow | PreflightOutcomeKind::Warn
            ),
            "port scan on CliManual should be allow or warn, got {:?}",
            result.outcome_kind
        );
    }

    #[test]
    fn cli_manual_approve_allows_port_scan() {
        let req = port_scan_request(RuntimeSurface::CliManual);
        let result = approve_run_request(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        );
        assert!(
            result.is_ok(),
            "CliManual should approve port scan: {:?}",
            result.err()
        );
    }

    #[test]
    fn mcp_server_rejects_manual_override() {
        let req = port_scan_request(RuntimeSurface::McpServer);
        let override_ = ManualOverride {
            assume_yes: true,
            allow_out_of_scope: true,
            ..Default::default()
        };
        let result = preflight_run_request(
            RuntimeSurface::McpServer,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::ManualOverrideRejected { .. }
        ));
    }

    #[test]
    fn mcp_server_approve_rejects_manual_override() {
        let req = port_scan_request(RuntimeSurface::McpServer);
        let override_ = ManualOverride {
            assume_yes: true,
            ..Default::default()
        };
        let result = approve_run_request(
            RuntimeSurface::McpServer,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::ManualOverrideRejected { .. }
        ));
    }

    #[test]
    fn unknown_surface_errors_before_policy_evaluation() {
        let req = port_scan_request(RuntimeSurface::Unknown);
        let result = preflight_run_request(
            RuntimeSurface::Unknown,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::UnknownSurface
        ));
    }

    #[test]
    fn cli_manual_strict_does_not_honor_override() {
        let req = port_scan_request(RuntimeSurface::CliManualStrict);
        let override_ = ManualOverride {
            assume_yes: true,
            allow_out_of_scope: true,
            ..Default::default()
        };
        let result = preflight_run_request(
            RuntimeSurface::CliManualStrict,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::ManualOverrideRejected { .. }
        ));
    }

    #[test]
    fn security_agent_rejects_manual_override() {
        let req = port_scan_request(RuntimeSurface::SecurityAgent);
        let override_ = ManualOverride {
            assume_yes: true,
            ..Default::default()
        };
        let result = approve_run_request(
            RuntimeSurface::SecurityAgent,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::ManualOverrideRejected { .. }
        ));
    }

    #[test]
    fn cli_manual_can_approve_warnings() {
        use crate::config::PreflightOutcomeKind;
        let req = port_scan_request(RuntimeSurface::CliManual);
        let mut policy = default_policy();
        policy.max_risk_without_confirm = OperationRisk::Passive;
        let result = preflight_run_request(
            RuntimeSurface::CliManual,
            policy,
            LoadedScope::default_empty(),
            &req,
            None,
        )
        .unwrap();
        assert!(
            matches!(
                result.outcome_kind,
                PreflightOutcomeKind::Allow | PreflightOutcomeKind::Warn
            ),
            "CliManual port scan should be allow or warn, got {:?}",
            result.outcome_kind
        );
        let approved = approve_run_request(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        );
        assert!(
            approved.is_ok(),
            "CliManual should approve port scan: {:?}",
            approved.err()
        );
    }

    #[test]
    fn cli_manual_strict_rejects_high_risk_without_scope() {
        let req = fuzz_request(RuntimeSurface::CliManualStrict);
        let mut policy = default_policy();
        policy.max_risk_without_confirm = OperationRisk::Passive;
        let result = approve_run_request(
            RuntimeSurface::CliManualStrict,
            policy,
            LoadedScope::default_empty(),
            &req,
            None,
        );
        assert!(
            result.is_err(),
            "CliManualStrict should reject high-risk without scope"
        );
    }

    #[test]
    fn tui_manual_preflight_allows_port_scan() {
        use crate::config::PreflightOutcomeKind;
        let req = port_scan_request(RuntimeSurface::TuiManual);
        let result = preflight_run_request(
            RuntimeSurface::TuiManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        )
        .unwrap();
        assert!(
            matches!(
                result.outcome_kind,
                PreflightOutcomeKind::Allow | PreflightOutcomeKind::Warn
            ),
            "port scan on TuiManual should be allow or warn, got {:?}",
            result.outcome_kind
        );
    }

    #[test]
    fn ci_surface_rejects_manual_override() {
        let req = port_scan_request(RuntimeSurface::Ci);
        let override_ = ManualOverride {
            assume_yes: true,
            ..Default::default()
        };
        let result = approve_run_request(
            RuntimeSurface::Ci,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::ManualOverrideRejected { .. }
        ));
    }

    #[test]
    fn rest_api_rejects_manual_override() {
        let req = port_scan_request(RuntimeSurface::RestApi);
        let override_ = ManualOverride {
            assume_yes: true,
            ..Default::default()
        };
        let result = approve_run_request(
            RuntimeSurface::RestApi,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
    }

    #[test]
    fn grpc_api_rejects_manual_override() {
        let req = port_scan_request(RuntimeSurface::GrpcApi);
        let override_ = ManualOverride {
            assume_yes: true,
            ..Default::default()
        };
        let result = approve_run_request(
            RuntimeSurface::GrpcApi,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            Some(&override_),
        );
        assert!(result.is_err());
    }

    #[test]
    fn daemon_backed_cli_manual_remains_manual() {
        let req = port_scan_request(RuntimeSurface::CliManual);
        let result = preflight_run_request(
            RuntimeSurface::CliManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        )
        .unwrap();
        assert_eq!(result.surface, ExecutionSurface::CliManual);
        assert_eq!(
            result.profile,
            crate::config::ExecutionProfile::ManualPermissive
        );
    }

    #[test]
    fn daemon_backed_tui_manual_remains_manual() {
        let req = port_scan_request(RuntimeSurface::TuiManual);
        let result = preflight_run_request(
            RuntimeSurface::TuiManual,
            default_policy(),
            LoadedScope::default_empty(),
            &req,
            None,
        )
        .unwrap();
        assert_eq!(result.surface, ExecutionSurface::TuiManual);
        assert_eq!(
            result.profile,
            crate::config::ExecutionProfile::ManualPermissive
        );
    }
}
