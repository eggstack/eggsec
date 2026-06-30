use std::fs;
use std::path::Path;

/// Scans source files for raw `.dispatch(` calls and ensures strict surfaces
/// don't bypass enforcement by using `EnforcedDispatcher::dispatch_checked()`.
#[test]
fn strict_surfaces_do_not_call_raw_dispatch_directly() {
    let workspace_root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();

    // Strict surfaces that must use EnforcedDispatcher
    let strict_dirs = [
        "src/tool/protocol/rest.rs",
        "src/tool/protocol/grpc.rs",
        "src/tool/protocol/mcp",
        "src/agent",
        "src/commands/handlers/ci.rs",
    ];

    // Files where raw dispatch is allowed (with reasons)
    let allowlist = [
        // EnforcedDispatcher wrapper internals
        (
            "src/tool/dispatcher.rs",
            "EnforcedDispatcher::dispatch_checked inner call",
        ),
        // Internal pipeline helper (callers must enforce)
        (
            "src/tool/orchestrator",
            "Internal pipeline helper; callers must enforce",
        ),
        // Test-only paths
        ("src/agent", "Test-only new_for_test() fallback"),
        // Notification dispatch (not tool dispatch)
        (
            "src/notify",
            "Alert/notification dispatch, not tool dispatch",
        ),
        // Test files
        ("tests/", "Test helpers"),
    ];

    let mut violations = Vec::new();

    for rel_path in &strict_dirs {
        let full_path = workspace_root.join(rel_path);
        if full_path.is_dir() {
            // Scan all .rs files in directory
            for entry in fs::read_dir(&full_path).unwrap() {
                let entry = entry.unwrap();
                if entry.path().extension().map_or(false, |e| e == "rs") {
                    check_file(&entry.path(), workspace_root, &allowlist, &mut violations);
                }
            }
        } else if full_path.exists() {
            check_file(&full_path, workspace_root, &allowlist, &mut violations);
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

fn check_file(
    path: &Path,
    workspace_root: &Path,
    allowlist: &[(&str, &str)],
    violations: &mut Vec<String>,
) {
    let rel = path.strip_prefix(workspace_root).unwrap_or(path);
    let content = fs::read_to_string(path).unwrap();

    for (line_num, line) in content.lines().enumerate() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }

        // Check for raw .dispatch( calls (not dispatch_checked)
        if line.contains(".dispatch(") && !line.contains("dispatch_checked") {
            let rel_str = rel.to_string_lossy();

            // Check if this file is in the allowlist
            let allowed = allowlist
                .iter()
                .any(|(pattern, _reason)| rel_str.starts_with(pattern));

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
