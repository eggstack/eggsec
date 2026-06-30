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
