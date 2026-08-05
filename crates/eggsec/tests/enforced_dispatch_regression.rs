use std::fs;
use std::path::Path;

/// A narrow allow entry: raw dispatch is permitted only when BOTH the file
/// path suffix AND the line content match. This prevents broad allowlists
/// from masking production fallback regressions.
#[allow(dead_code)]
struct RawDispatchAllow {
    path_suffix: &'static str,
    line_contains: &'static str,
    reason: &'static str,
}

/// Scans source files for raw `.dispatch(` calls and ensures strict surfaces
/// don't bypass enforcement by using `EnforcedDispatcher::dispatch_checked()`.
#[test]
fn strict_surfaces_do_not_call_raw_dispatch_directly() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    // Strict surfaces that must use EnforcedDispatcher
    let strict_dirs = [
        "eggsec/src/tool/protocol/rest.rs",
        "eggsec/src/tool/protocol/grpc.rs",
        "eggsec/src/tool/protocol/mcp",
        "eggsec/src/agent",
    ];

    // Narrow allowlist: both path AND line must match for a raw dispatch to be permitted.
    let allowlist: &[RawDispatchAllow] = &[
        RawDispatchAllow {
            path_suffix: "src/tool/dispatcher.rs",
            line_contains: "self.inner.dispatch(request).await",
            reason: "EnforcedDispatcher internal terminal call",
        },
        RawDispatchAllow {
            path_suffix: "src/tool/orchestrator",
            line_contains: ".dispatch(",
            reason: "Internal pipeline helper; callers must enforce",
        },
        RawDispatchAllow {
            path_suffix: "src/agent/mod.rs",
            line_contains: "Box::pin(self.dispatch(request))",
            reason: "ScanDispatcherTrait adapter; production execution must use EnforcedDispatcher",
        },
        RawDispatchAllow {
            path_suffix: "src/agent/mod.rs",
            line_contains: ".dispatch(request)",
            reason: "Test-only fallback path; guarded by enforced_dispatcher.is_none() which only occurs via new_for_test()",
        },
        RawDispatchAllow {
            path_suffix: "src/notify",
            line_contains: ".dispatch(",
            reason: "Alert/notification dispatch, not tool dispatch",
        },
        RawDispatchAllow {
            path_suffix: "tests/",
            line_contains: ".dispatch(",
            reason: "Test helpers",
        },
    ];

    let mut violations = Vec::new();

    for rel_path in &strict_dirs {
        let full_path = workspace_root.join(rel_path);
        if full_path.is_dir() {
            // Scan all .rs files in directory
            for entry in fs::read_dir(&full_path).unwrap() {
                let entry = entry.unwrap();
                if entry.path().extension().map_or(false, |e| e == "rs") {
                    check_file(&entry.path(), workspace_root, allowlist, &mut violations);
                }
            }
        } else if full_path.exists() {
            check_file(&full_path, workspace_root, allowlist, &mut violations);
        }
    }

    if !violations.is_empty() {
        let msg = violations.join("\n");
        panic!(
            "Strict surfaces contain raw .dispatch() calls that may bypass enforcement:\n\n{}",
            msg
        );
    }
}

/// CI handler is a passive quality gate with no dispatch path.
/// It must not import or use ToolDispatcher, EnforcedDispatcher,
/// or any tool execution API. (Architecture Invariant #19)
#[test]
fn ci_handler_has_no_dispatch_path() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let ci_handler = workspace_root.join("eggsec/src/commands/handlers/ci.rs");
    assert!(
        ci_handler.exists(),
        "CI handler file not found at expected path"
    );

    let content = fs::read_to_string(&ci_handler).unwrap();
    let forbidden = [
        "ToolDispatcher",
        "EnforcedDispatcher",
        "dispatch_checked",
        "SecurityTool",
        "ToolRegistry",
    ];

    let mut violations = Vec::new();
    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }
        for term in &forbidden {
            if line.contains(term) {
                violations.push(format!(
                    "ci.rs:{}: forbidden term '{}' found: {}",
                    line_num + 1,
                    term,
                    trimmed.chars().take(80).collect::<String>()
                ));
            }
        }
    }

    if !violations.is_empty() {
        let msg = violations.join("\n");
        panic!(
            "CI handler must not contain tool dispatch APIs (Architecture Invariant #19):\n\n{}",
            msg
        );
    }
}

fn check_file(
    path: &Path,
    workspace_root: &Path,
    allowlist: &[RawDispatchAllow],
    violations: &mut Vec<String>,
) {
    let rel = path.strip_prefix(workspace_root).unwrap_or(path);
    let content = fs::read_to_string(path).unwrap();
    let rel_str = rel.to_string_lossy();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }

        // Check for raw .dispatch( calls (not dispatch_checked)
        if line.contains(".dispatch(") && !line.contains("dispatch_checked") {
            // Check if BOTH path suffix AND line content match an allow entry
            let allowed = allowlist.iter().any(|entry| {
                rel_str.ends_with(entry.path_suffix) && line.contains(entry.line_contains)
            });

            if !allowed {
                violations.push(format!(
                    "{}:{}: raw .dispatch() call found in strict surface: {}",
                    rel.display(),
                    line_num + 1,
                    trimmed.chars().take(80).collect::<String>()
                ));
            }
        }
    }
}

// ========================================================================
// Phase A: Authorization Token and Target-Binding Correction
// Regression tests for dispatch binding.
// ========================================================================

use eggsec::config::{
    EnforcementContext, ExecutionPolicy, ExecutionProfile, OperationDescriptor, OperationMode,
    OperationRisk, PolicyDecision, TargetPolicyKind,
};
#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
use eggsec::tool::dispatcher::{validate_request_binding, DispatchBindingError};
#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
use eggsec::tool::{Target, ToolRequest};

#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
fn make_approved(descriptor: OperationDescriptor) -> eggsec::config::ApprovedOperation {
    let decision = PolicyDecision::allowed(
        &descriptor.operation,
        descriptor.mode,
        descriptor.risk,
        descriptor.intended_uses.clone(),
    );
    eggsec::config::ApprovedOperation::for_test(
        descriptor,
        decision,
        eggsec::config::ExecutionSurface::McpServer,
        ExecutionProfile::McpStrict,
    )
}

#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
fn scan_request(target: &str) -> ToolRequest {
    ToolRequest {
        id: "test".to_string(),
        tool: "scan-ports".to_string(),
        target: Target::ip(target),
        params: serde_json::json!({}),
        options: Default::default(),
        cancellation_token: None,
    }
}

#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
#[test]
fn phase_a_target_mismatch_rejected() {
    let descriptor = OperationDescriptor {
        operation: "scan-ports".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("10.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    let approved = make_approved(descriptor);

    // Different target should fail
    let request = scan_request("10.0.0.2");
    let result = validate_request_binding(&approved, &request);
    assert!(result.is_err());
    match result.unwrap_err() {
        DispatchBindingError::TargetMismatch {
            request_target,
            approved_target,
        } => {
            assert_eq!(request_target, "10.0.0.2");
            assert_eq!(approved_target, "10.0.0.1");
        }
        other => panic!("expected TargetMismatch, got {:?}", other),
    }
}

#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
#[test]
fn phase_a_target_required_missing_target_rejected() {
    let descriptor = OperationDescriptor {
        operation: "scan-ports".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("10.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    let approved = make_approved(descriptor);

    // Empty target should fail
    let request = ToolRequest {
        id: "test".to_string(),
        tool: "scan-ports".to_string(),
        target: Target {
            target_type: eggsec::tool::TargetType::Ip,
            value: "".to_string(),
            scope: None,
        },
        params: serde_json::json!({}),
        options: Default::default(),
        cancellation_token: None,
    };
    let result = validate_request_binding(&approved, &request);
    assert!(result.is_err());
    match result.unwrap_err() {
        DispatchBindingError::MissingTarget {
            approved_operation,
            expected_target,
        } => {
            assert_eq!(approved_operation, "scan-ports");
            assert_eq!(expected_target, "10.0.0.1");
        }
        other => panic!("expected MissingTarget, got {:?}", other),
    }
}

#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
#[test]
fn phase_a_targetless_rejects_smuggled_target() {
    let descriptor = OperationDescriptor {
        operation: "search".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::Passive,
        intended_uses: vec![],
        target: None,
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    let approved = make_approved(descriptor);

    // Smuggled target should fail
    let request = ToolRequest {
        id: "test".to_string(),
        tool: "search".to_string(),
        target: Target::domain("evil.example.com"),
        params: serde_json::json!({}),
        options: Default::default(),
        cancellation_token: None,
    };
    let result = validate_request_binding(&approved, &request);
    assert!(result.is_err());
    match result.unwrap_err() {
        DispatchBindingError::UnexpectedTarget {
            approved_operation,
            request_target,
        } => {
            assert_eq!(approved_operation, "search");
            assert_eq!(request_target, "evil.example.com");
        }
        other => panic!("expected UnexpectedTarget, got {:?}", other),
    }
}

#[cfg(all(feature = "test-helpers", feature = "tool-api"))]
#[test]
fn phase_a_conflicting_targets_rejected() {
    let descriptor = OperationDescriptor {
        operation: "scan-ports".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("10.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    let approved = make_approved(descriptor);

    // Conflicting typed and param targets
    let request = ToolRequest {
        id: "test".to_string(),
        tool: "scan-ports".to_string(),
        target: Target::ip("10.0.0.1"),
        params: serde_json::json!({"target": "10.0.0.99"}),
        options: Default::default(),
        cancellation_token: None,
    };
    let result = validate_request_binding(&approved, &request);
    assert!(result.is_err());
    match result.unwrap_err() {
        DispatchBindingError::ConflictingTargets {
            typed_target,
            param_target,
        } => {
            assert_eq!(typed_target, "10.0.0.1");
            assert_eq!(param_target, "10.0.0.99");
        }
        other => panic!("expected ConflictingTargets, got {:?}", other),
    }
}

#[test]
fn phase_a_surface_profile_mismatch_rejects_approval() {
    let ctx = EnforcementContext::mcp_strict(ExecutionPolicy::default(), Default::default());
    let descriptor = OperationDescriptor {
        operation: "scan".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![],
        target: Some("127.0.0.1".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    // McpStrict context with CliManual surface = mismatch
    let result = ctx.approve(eggsec::config::ExecutionSurface::CliManual, descriptor);
    assert!(result.is_err());
    match result.unwrap_err() {
        eggsec::config::EnforcementError::SurfaceProfileMismatch {
            surface,
            surface_profile,
            context_profile,
        } => {
            assert_eq!(surface, eggsec::config::ExecutionSurface::CliManual);
            assert_eq!(surface_profile, ExecutionProfile::ManualPermissive);
            assert_eq!(context_profile, ExecutionProfile::McpStrict);
        }
        other => panic!("expected SurfaceProfileMismatch, got {:?}", other),
    }
}

#[test]
fn phase_a_target_required_rejects_none_via_try_descriptor() {
    for meta in eggsec::config::all_operation_metadata() {
        match meta.target_policy {
            TargetPolicyKind::TargetRequired
            | TargetPolicyKind::ExplicitScopeRequired
            | TargetPolicyKind::PrivateOrLocalRequired => {
                let result = meta.try_descriptor_for_target(None);
                assert!(
                    result.is_err(),
                    "operation '{}' with {:?} should reject None",
                    meta.id,
                    meta.target_policy
                );
            }
            _ => {}
        }
    }
}

#[test]
fn phase_a_no_target_rejects_nonempty_via_try_descriptor() {
    for meta in eggsec::config::all_operation_metadata() {
        if meta.target_policy == TargetPolicyKind::NoTarget {
            let result = meta.try_descriptor_for_target(Some("example.com"));
            assert!(
                result.is_err(),
                "operation '{}' with NoTarget should reject non-empty target",
                meta.id
            );
        }
    }
}
