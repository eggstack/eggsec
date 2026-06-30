//! Integration tests for the command registry.
//!
//! Validates that registry entries are consistent with `OperationMetadata`,
//! command IDs are unique, and metadata resolution works correctly.

use eggsec::commands::registry::{
    all_command_ids, build_descriptor_for_command, lookup_command, suggest_command,
    CommandCategory, REGISTERED_COMMANDS,
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
    let ids = all_command_ids();
    let mut sorted = ids.clone();
    sorted.sort();
    sorted.dedup();
    assert_eq!(
        ids.len(),
        sorted.len(),
        "Duplicate command IDs found: {:?}",
        ids.iter()
            .copied()
            .collect::<Vec<_>>()
            .windows(2)
            .find(|w| w[0] == w[1])
    );
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
            // metadata_for_tool_id resolves aliases, so metadata.id may differ from op_id.
            // Verify the resolved metadata is valid (has a non-empty ID).
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
            // The operation_id should be either a canonical ID or resolve via alias
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
fn side_effecting_entries_have_descriptor_builder() {
    for reg in REGISTERED_COMMANDS.iter() {
        if reg.category == CommandCategory::SideEffectingNetwork && reg.operation_id.is_some() {
            // Side-effecting commands with metadata should be able to build a descriptor
            let desc = reg.build_descriptor(Some("test-target".to_string()));
            assert!(
                desc.is_some(),
                "Side-effecting command '{}' with operation_id should build a descriptor",
                reg.command_id
            );
        }
    }
}

#[test]
fn manual_only_not_exposed_programmatically() {
    for reg in REGISTERED_COMMANDS.iter() {
        if reg.manual_only {
            // Manual-only commands should not be TUI-visible
            assert!(
                !reg.tui_visible,
                "Command '{}' is manual_only but tui_visible is true",
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

        // Verify the descriptor matches the metadata
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
    assert!(!reg.manual_only);
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
                // Network commands should be TUI visible unless manual_only
                if !reg.manual_only {
                    assert!(
                        reg.tui_visible,
                        "Non-manual SideEffectingNetwork command '{}' should be TUI visible",
                        reg.command_id
                    );
                }
            }
            CommandCategory::FrontendServer => {
                // Server commands should not be TUI visible
                assert!(
                    !reg.tui_visible,
                    "FrontendServer command '{}' should not be TUI visible",
                    reg.command_id
                );
            }
            CommandCategory::ConfigOutputHelper => {
                // Config commands should be manual-only (no programmatic exposure)
                // But not all are - doctor, config are CLI-only
            }
            _ => {}
        }
    }
}

#[test]
fn registry_metadata_alignment_with_all_operation_metadata() {
    // Every registered command with an operation_id should resolve to
    // an entry in ALL_OPERATION_METADATA (either directly or via alias).
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
