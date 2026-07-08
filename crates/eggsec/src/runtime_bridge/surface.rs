use crate::config::ExecutionSurface;
use eggsec_runtime::RuntimeSurface;

/// Error type for runtime bridge conversions.
#[derive(Debug, thiserror::Error)]
pub enum RuntimeBridgeError {
    /// The runtime surface is `Unknown` and cannot be mapped to an execution surface.
    #[error("unknown runtime surface — must be resolved to a concrete surface before execution")]
    UnknownSurface,

    /// The task kind is not yet supported by the bridge.
    #[error("unsupported task kind: {kind}")]
    UnsupportedTaskKind { kind: String },

    /// The task kind is missing a required target field.
    #[error("task kind '{kind}' requires a target but none was provided")]
    MissingTarget { kind: String },

    /// The task kind references an operation ID that has no registered metadata.
    #[error("no operation metadata found for id '{operation_id}'")]
    UnknownOperationId { operation_id: String },

    /// A manual override was supplied for a strict/automated surface.
    #[error("manual override is not permitted for automated surface {surface}")]
    ManualOverrideRejected { surface: String },

    /// Enforcement layer denied the operation.
    #[error("enforcement denied: {reason}")]
    EnforcementDenied { reason: String },
}

/// Convert a [`RuntimeSurface`] to an [`ExecutionSurface`].
///
/// This is the security boundary between the frontend-neutral runtime DTO
/// and the canonical enforcement model. `Unknown` surfaces are rejected
/// rather than silently mapped to a permissive profile.
pub fn runtime_surface_to_execution_surface(
    surface: RuntimeSurface,
) -> Result<ExecutionSurface, RuntimeBridgeError> {
    match surface {
        RuntimeSurface::CliManual => Ok(ExecutionSurface::CliManual),
        RuntimeSurface::CliManualStrict => Ok(ExecutionSurface::CliManualStrict),
        RuntimeSurface::TuiManual => Ok(ExecutionSurface::TuiManual),
        RuntimeSurface::TuiManualStrict => Ok(ExecutionSurface::TuiManualStrict),
        RuntimeSurface::Ci => Ok(ExecutionSurface::Ci),
        RuntimeSurface::McpServer => Ok(ExecutionSurface::McpServer),
        RuntimeSurface::RestApi => Ok(ExecutionSurface::RestApi),
        RuntimeSurface::GrpcApi => Ok(ExecutionSurface::GrpcApi),
        RuntimeSurface::SecurityAgent => Ok(ExecutionSurface::SecurityAgent),
        RuntimeSurface::Unknown => Err(RuntimeBridgeError::UnknownSurface),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_known_surfaces_map_correctly() {
        let cases: &[(RuntimeSurface, ExecutionSurface)] = &[
            (RuntimeSurface::CliManual, ExecutionSurface::CliManual),
            (
                RuntimeSurface::CliManualStrict,
                ExecutionSurface::CliManualStrict,
            ),
            (RuntimeSurface::TuiManual, ExecutionSurface::TuiManual),
            (
                RuntimeSurface::TuiManualStrict,
                ExecutionSurface::TuiManualStrict,
            ),
            (RuntimeSurface::Ci, ExecutionSurface::Ci),
            (RuntimeSurface::McpServer, ExecutionSurface::McpServer),
            (RuntimeSurface::RestApi, ExecutionSurface::RestApi),
            (RuntimeSurface::GrpcApi, ExecutionSurface::GrpcApi),
            (
                RuntimeSurface::SecurityAgent,
                ExecutionSurface::SecurityAgent,
            ),
        ];
        for (runtime, expected) in cases {
            let result = runtime_surface_to_execution_surface(runtime.clone());
            assert_eq!(result.unwrap(), *expected, "mapping for {:?}", runtime);
        }
    }

    #[test]
    fn unknown_surface_errors() {
        let result = runtime_surface_to_execution_surface(RuntimeSurface::Unknown);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            RuntimeBridgeError::UnknownSurface
        ));
    }

    #[test]
    fn strict_surfaces_do_not_honor_manual_override() {
        let strict_surfaces = [
            RuntimeSurface::CliManualStrict,
            RuntimeSurface::TuiManualStrict,
            RuntimeSurface::Ci,
            RuntimeSurface::McpServer,
            RuntimeSurface::RestApi,
            RuntimeSurface::GrpcApi,
            RuntimeSurface::SecurityAgent,
        ];
        for rt in strict_surfaces {
            let exec = runtime_surface_to_execution_surface(rt.clone()).unwrap();
            assert!(
                !exec.honors_manual_override(),
                "{:?} should not honor manual override",
                rt
            );
        }
    }

    #[test]
    fn manual_surfaces_honor_override_only_for_permissive() {
        let permissive = [RuntimeSurface::CliManual, RuntimeSurface::TuiManual];
        for rt in permissive {
            let exec = runtime_surface_to_execution_surface(rt.clone()).unwrap();
            assert!(
                exec.honors_manual_override(),
                "{:?} should honor manual override",
                rt
            );
        }

        let guarded = [
            RuntimeSurface::CliManualStrict,
            RuntimeSurface::TuiManualStrict,
        ];
        for rt in guarded {
            let exec = runtime_surface_to_execution_surface(rt.clone()).unwrap();
            assert!(
                !exec.honors_manual_override(),
                "{:?} should not honor manual override",
                rt
            );
        }
    }
}
