//! Integration tests for the command registry.
//!
//! Validates that registry entries are consistent with `OperationMetadata`,
//! command IDs are unique, and metadata resolution works correctly.

use eggsec::commands::registry::{
    build_descriptor_for_command, lookup_command, suggest_command, CommandCategory,
    CommandDispatchMode, REGISTERED_COMMANDS,
};
use eggsec::config::metadata_for_tool_id;

#[test]
fn registry_has_entries() {
    assert!(
        !REGISTERED_COMMANDS.is_empty(),
        "Registry should have at least one entry"
    );
}

#[test]
fn all_command_ids_are_unique() {
    let mut seen = rustc_hash::FxHashSet::default();
    for reg in REGISTERED_COMMANDS {
        assert!(
            seen.insert(reg.command_id),
            "duplicate command id: {}",
            reg.command_id
        );
    }
}

#[test]
fn registry_entry_operation_ids_resolve_to_metadata() {
    for reg in REGISTERED_COMMANDS.iter() {
        if let Some(op_id) = reg.operation_id {
            let metadata = metadata_for_tool_id(op_id).unwrap_or_else(|| {
                panic!(
                    "Command '{}' has operation_id '{}' but no matching OperationMetadata found",
                    reg.command_id, op_id
                )
            });
            assert!(
                !metadata.id.is_empty(),
                "Command '{}': resolved metadata for '{}' has empty id",
                reg.command_id,
                op_id
            );
        }
    }
}

#[test]
fn registry_operation_ids_are_canonical_or_aliases() {
    for reg in REGISTERED_COMMANDS.iter() {
        if let Some(op_id) = reg.operation_id {
            assert!(
                metadata_for_tool_id(op_id).is_some(),
                "Command '{}' has operation_id '{}' which is neither a canonical ID nor a known alias",
                reg.command_id,
                op_id
            );
        }
    }
}

#[test]
fn feature_gated_entries_have_nonempty_feature() {
    for reg in REGISTERED_COMMANDS.iter() {
        if let Some(feature) = reg.feature {
            assert!(
                !feature.is_empty(),
                "Command '{}' has an empty feature gate string",
                reg.command_id
            );
        }
    }
}

#[test]
fn registry_backed_side_effecting_commands_build_descriptors() {
    // RegistryBacked dispatch fully relies on `build_descriptor()` to produce the
    // OperationDescriptor passed to `EnforcementContext::evaluate()`. Missing
    // descriptor support here means the dispatch bridge cannot evaluate the
    // command, so this invariant is strict for RegistryBacked entries only.
    for reg in REGISTERED_COMMANDS.iter() {
        if reg.category == CommandCategory::SideEffectingNetwork
            && matches!(reg.dispatch_mode, CommandDispatchMode::RegistryBacked)
        {
            assert!(
                reg.operation_id.is_some(),
                "RegistryBacked side-effecting command '{}' must have an operation_id",
                reg.command_id
            );
            let desc = reg.build_descriptor(Some("test-target".to_string()));
            assert!(
                desc.is_some(),
                "RegistryBacked side-effecting command '{}' must build a descriptor",
                reg.command_id
            );
        }
    }
}

#[test]
fn legacy_wrapped_operation_metadata_is_optional_but_valid_when_present() {
    // LegacyWrapped entries route through the legacy `handle_command()` match
    // path. They may carry an `operation_id` for descriptor metadata (so CLI
    // help and preflight can describe them), but descriptor generation is not
    // a dispatch proof — execution still flows through the legacy handler.
    for reg in REGISTERED_COMMANDS.iter() {
        if matches!(reg.dispatch_mode, CommandDispatchMode::LegacyWrapped) {
            if let Some(op_id) = reg.operation_id {
                assert!(
                    metadata_for_tool_id(op_id).is_some(),
                    "LegacyWrapped command '{}' has operation_id '{}' but no matching OperationMetadata",
                    reg.command_id,
                    op_id
                );
            }
            // legacy path does not require build_descriptor to succeed
        }
    }
}

#[test]
fn helper_and_server_commands_do_not_require_descriptors() {
    // HelperOnly and ServerLifecycle entries may have no operation_id. They are
    // not subject to the registry descriptor-builder invariant because they are
    // not routed through `EnforcementContext::evaluate()`.
    for reg in REGISTERED_COMMANDS.iter() {
        if matches!(
            reg.dispatch_mode,
            CommandDispatchMode::HelperOnly | CommandDispatchMode::ServerLifecycle
        ) {
            // operation_id may be None or Some, but if Some must resolve.
            if let Some(op_id) = reg.operation_id {
                assert!(
                    metadata_for_tool_id(op_id).is_some(),
                    "Helper/Server command '{}' has operation_id '{}' but no matching OperationMetadata",
                    reg.command_id,
                    op_id
                );
            }
        }
    }
}

#[test]
fn cli_interactive_only_not_programmatic() {
    for reg in REGISTERED_COMMANDS.iter() {
        if reg.cli_interactive_only {
            assert!(
                !reg.programmatic_visible,
                "Command '{}' is cli_interactive_only but programmatic_visible is true",
                reg.command_id
            );
        }
    }
}

#[test]
fn pilot_commands_have_metadata() {
    let pilot_commands = ["recon", "scan-ports", "scan-endpoints", "fingerprint"];
    for cmd_id in &pilot_commands {
        let reg = lookup_command(cmd_id)
            .unwrap_or_else(|| panic!("Pilot command '{}' should be registered", cmd_id));
        assert!(
            reg.operation_id.is_some(),
            "Pilot command '{}' should have an operation_id",
            cmd_id
        );
        assert!(
            reg.feature.is_none(),
            "Pilot command '{}' should not require a feature gate",
            cmd_id
        );
        assert!(
            reg.tui_visible,
            "Pilot command '{}' should be TUI visible",
            cmd_id
        );
        assert!(
            reg.registry_backed,
            "Pilot command '{}' should have registry_backed = true",
            cmd_id
        );
    }
}

#[test]
fn pilot_commands_build_descriptors_from_metadata() {
    let test_cases = [
        ("recon", "example.com"),
        ("scan-ports", "192.168.1.1"),
        ("scan-endpoints", "https://example.com"),
        ("fingerprint", "10.0.0.1"),
    ];

    for (cmd_id, target) in &test_cases {
        let desc = build_descriptor_for_command(cmd_id, Some(target.to_string()))
            .unwrap_or_else(|| panic!("Pilot command '{}' should build a descriptor", cmd_id));

        let metadata = metadata_for_tool_id(cmd_id)
            .unwrap_or_else(|| panic!("Pilot command '{}' should have metadata", cmd_id));

        assert_eq!(desc.operation, metadata.id);
        assert_eq!(desc.mode, metadata.mode);
        assert_eq!(desc.risk, metadata.risk);
        assert_eq!(desc.target, Some(target.to_string()));
    }
}

#[test]
fn lookup_returns_correct_entry() {
    let reg = lookup_command("recon").expect("recon should be registered");
    assert_eq!(reg.command_id, "recon");
    assert_eq!(reg.operation_id, Some("recon"));
    assert_eq!(reg.display_name, "Reconnaissance");
    assert_eq!(reg.category, CommandCategory::SideEffectingNetwork);
    assert!(!reg.cli_interactive_only);
    assert!(reg.tui_visible);
}

#[test]
fn lookup_unknown_returns_none() {
    assert!(lookup_command("nonexistent").is_none());
    assert!(lookup_command("").is_none());
}

#[test]
fn suggest_command_finds_close_matches() {
    let suggestions = suggest_command("scan-port");
    assert!(
        suggestions.contains(&"scan-ports"),
        "Should suggest 'scan-ports' for 'scan-port', got {:?}",
        suggestions
    );
}

#[test]
fn suggest_command_returns_empty_for_distant_input() {
    let suggestions = suggest_command("xyzzy12345");
    assert!(suggestions.is_empty());
}

#[test]
fn category_classification_consistent() {
    for reg in REGISTERED_COMMANDS.iter() {
        match reg.category {
            CommandCategory::SideEffectingNetwork => {
                if !reg.cli_interactive_only {
                    assert!(
                        reg.tui_visible,
                        "Non-cli-interactive SideEffectingNetwork command '{}' should be TUI visible",
                        reg.command_id
                    );
                }
            }
            CommandCategory::FrontendServer => {
                assert!(
                    !reg.tui_visible,
                    "FrontendServer command '{}' should not be TUI visible",
                    reg.command_id
                );
            }
            CommandCategory::ConfigOutputHelper => {}
            _ => {}
        }
    }
}

#[test]
fn registry_metadata_alignment_with_all_operation_metadata() {
    for reg in REGISTERED_COMMANDS.iter() {
        if let Some(op_id) = reg.operation_id {
            assert!(
                metadata_for_tool_id(op_id).is_some(),
                "Command '{}' has operation_id '{}' not found in ALL_OPERATION_METADATA (even via alias)",
                reg.command_id,
                op_id
            );
        }
    }
}

#[test]
fn registry_backed_commands_have_metadata() {
    for reg in REGISTERED_COMMANDS.iter() {
        if reg.registry_backed {
            assert!(
                reg.operation_id.is_some(),
                "Registry-backed command '{}' has no operation_id",
                reg.command_id
            );
            assert!(
                matches!(reg.dispatch_mode, CommandDispatchMode::RegistryBacked),
                "Registry-backed command '{}' should have dispatch_mode: RegistryBacked",
                reg.command_id
            );
        }
    }
}

#[test]
fn entries_without_operation_id_not_registry_backed() {
    for reg in REGISTERED_COMMANDS.iter() {
        if reg.operation_id.is_none() {
            assert!(
                !reg.registry_backed,
                "Command '{}' has no operation_id but registry_backed = true",
                reg.command_id
            );
        }
    }
}

#[test]
fn dispatch_mode_consistent_with_fields() {
    for reg in REGISTERED_COMMANDS.iter() {
        match reg.dispatch_mode {
            CommandDispatchMode::RegistryBacked => {
                assert!(
                    reg.registry_backed,
                    "RegistryBacked dispatch for '{}' should have registry_backed = true",
                    reg.command_id
                );
                assert!(
                    reg.operation_id.is_some(),
                    "RegistryBacked dispatch for '{}' should have operation_id",
                    reg.command_id
                );
            }
            CommandDispatchMode::ServerLifecycle => {
                assert!(
                    !reg.tui_visible,
                    "ServerLifecycle command '{}' should not be TUI visible",
                    reg.command_id
                );
                assert!(
                    !reg.cli_interactive_only,
                    "ServerLifecycle command '{}' should not be cli_interactive_only",
                    reg.command_id
                );
            }
            CommandDispatchMode::HelperOnly => {
                assert!(
                    reg.cli_interactive_only,
                    "HelperOnly command '{}' should be cli_interactive_only",
                    reg.command_id
                );
            }
            CommandDispatchMode::LegacyWrapped => {
                assert!(
                    reg.cli_visible,
                    "LegacyWrapped command '{}' should be cli_visible",
                    reg.command_id
                );
            }
            CommandDispatchMode::CatalogOnly => {}
        }
    }
}
