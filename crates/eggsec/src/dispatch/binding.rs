//! Binding checks for manual-surface `RunRequest` dispatch.
//!
//! The frontend supplies the effective target because `TaskKind::Resume` has
//! no canonical target of its own; the selected checkpoint target is instead
//! the target that was approved by the manual surface.

use crate::config::{normalize_target, ApprovedOperation, OperationTarget};
use eggsec_runtime::request::RunRequest;

/// Request/approval binding failures for the TUI manual dispatch boundary.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum RunRequestBindingError {
    /// Request operation differs from the approved canonical operation.
    #[error(
        "request operation '{request_operation}' does not match approved operation '{approved_operation}'"
    )]
    OperationMismatch {
        request_operation: String,
        approved_operation: String,
    },
    /// The approved operation requires a target, but the caller supplied none.
    #[error("approved operation '{approved_operation}' requires target {approved_target:?}")]
    MissingTarget {
        approved_operation: String,
        approved_target: OperationTarget,
    },
    /// A target was supplied for an operation whose approval is targetless.
    #[error(
        "approved operation '{approved_operation}' is targetless, but request target is '{request_target}'"
    )]
    UnexpectedTarget {
        approved_operation: String,
        request_target: String,
    },
    /// Request target does not normalize to the approved target.
    #[error(
        "request target '{request_target}' normalizes to {request_normalized:?}, not approved target {approved_target:?}"
    )]
    TargetMismatch {
        request_target: String,
        request_normalized: OperationTarget,
        approved_target: OperationTarget,
    },
}

/// Validate that a TUI [`RunRequest`] is bound to its [`ApprovedOperation`].
///
/// `effective_target` is the same target identity used to build the approved
/// descriptor: normally `request.task_kind.canonical_target()`, or the
/// selected checkpoint's target for `TaskKind::Resume`.
pub fn validate_run_request_binding(
    approval: &ApprovedOperation,
    request: &RunRequest,
    effective_target: Option<&str>,
) -> Result<(), RunRequestBindingError> {
    let descriptor = approval.descriptor();
    let request_operation = request.task_kind.operation_id();

    if descriptor.operation != request_operation {
        return Err(RunRequestBindingError::OperationMismatch {
            request_operation: request_operation.to_string(),
            approved_operation: descriptor.operation.clone(),
        });
    }

    match &descriptor.normalized_target {
        OperationTarget::None => {
            if let Some(request_target) = effective_target {
                return Err(RunRequestBindingError::UnexpectedTarget {
                    approved_operation: descriptor.operation.clone(),
                    request_target: request_target.to_string(),
                });
            }
        }
        approved_target => {
            let request_target =
                effective_target.ok_or_else(|| RunRequestBindingError::MissingTarget {
                    approved_operation: descriptor.operation.clone(),
                    approved_target: approved_target.clone(),
                })?;
            let request_normalized = normalize_target(request_target, None);
            if request_normalized != *approved_target {
                return Err(RunRequestBindingError::TargetMismatch {
                    request_target: request_target.to_string(),
                    request_normalized,
                    approved_target: approved_target.clone(),
                });
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{
        EnforcementContext, ExecutionPolicy, ExecutionSurface, IntendedUse, LoadedScope,
        ManualOverride, OperationDescriptor, OperationMode, OperationRisk,
    };
    use eggsec_runtime::request::{
        PortScanParams, ReconParams, ResumeParams, RuntimeSurface, TaskKind,
    };

    fn approval(operation: &str, target: Option<&str>) -> ApprovedOperation {
        let surface = ExecutionSurface::TuiManual;
        let descriptor = OperationDescriptor::new(
            operation.to_string(),
            OperationMode::StandardAssessment,
            OperationRisk::SafeActive,
            vec![IntendedUse::WebAssessment],
            target.map(str::to_string),
            Vec::new(),
            Vec::new(),
            false,
            false,
            Vec::new(),
        );
        let enforcement = EnforcementContext::for_surface(
            surface,
            ExecutionPolicy::default(),
            LoadedScope::default_empty(),
        );
        let override_ = ManualOverride {
            assume_yes: true,
            allow_out_of_scope: true,
            ..ManualOverride::default()
        };
        enforcement
            .approve_manual(surface, descriptor, Some(&override_))
            .expect("test approval is manually authorized")
    }

    fn port_scan(target: &str) -> RunRequest {
        RunRequest {
            task_kind: TaskKind::PortScan(PortScanParams {
                target: target.to_string(),
                ..PortScanParams::default()
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: Vec::new(),
        }
    }

    fn resume() -> RunRequest {
        RunRequest {
            task_kind: TaskKind::Resume(ResumeParams {
                session_path: "checkpoint.session.json".to_string(),
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: Vec::new(),
        }
    }

    #[test]
    fn matching_operation_and_target_are_accepted() {
        let request = port_scan("127.0.0.1");
        let approved = approval("scan-ports", Some("127.0.0.1"));
        assert!(validate_run_request_binding(&approved, &request, Some("127.0.0.1")).is_ok());
    }

    #[test]
    fn operation_mismatch_is_rejected() {
        let request = port_scan("127.0.0.1");
        let approved = approval("recon", Some("127.0.0.1"));
        assert!(matches!(
            validate_run_request_binding(&approved, &request, Some("127.0.0.1")),
            Err(RunRequestBindingError::OperationMismatch { .. })
        ));
    }

    #[test]
    fn missing_target_for_target_bearing_approval_is_rejected() {
        let approved = approval("pipeline", Some("127.0.0.1"));
        assert!(matches!(
            validate_run_request_binding(&approved, &resume(), None),
            Err(RunRequestBindingError::MissingTarget { .. })
        ));
    }

    #[test]
    fn target_supplied_for_targetless_approval_is_rejected() {
        let approved = approval("pipeline", None);
        assert!(matches!(
            validate_run_request_binding(&approved, &resume(), Some("127.0.0.1")),
            Err(RunRequestBindingError::UnexpectedTarget { .. })
        ));
    }

    #[test]
    fn url_normalization_equivalence_is_accepted() {
        let approved = approval("scan-ports", Some("https://example.com"));
        let request = port_scan("https://EXAMPLE.COM:443/");
        assert!(validate_run_request_binding(
            &approved,
            &request,
            Some("https://EXAMPLE.COM:443/")
        )
        .is_ok());
    }

    #[test]
    fn host_case_equivalence_is_accepted() {
        let approved = approval("recon", Some("Example.COM"));
        let request = RunRequest {
            task_kind: TaskKind::Recon(ReconParams {
                target: "example.com".to_string(),
                modules: None,
            }),
            requested_by: None,
            surface: RuntimeSurface::TuiManual,
            labels: Vec::new(),
        };
        assert!(validate_run_request_binding(&approved, &request, Some("example.com")).is_ok());
    }

    #[test]
    fn different_target_is_rejected() {
        let approved = approval("scan-ports", Some("example.com"));
        let request = port_scan("example.com:443");
        assert!(matches!(
            validate_run_request_binding(&approved, &request, Some("example.com:443")),
            Err(RunRequestBindingError::TargetMismatch { .. })
        ));
    }

    #[test]
    fn resume_uses_explicit_checkpoint_target_without_special_case_in_validator() {
        let approved = approval("pipeline", Some("example.com"));
        let request = resume();
        assert!(validate_run_request_binding(&approved, &request, Some("example.com")).is_ok());
        assert!(matches!(
            validate_run_request_binding(&approved, &request, Some("other.example")),
            Err(RunRequestBindingError::TargetMismatch { .. })
        ));
    }
}
