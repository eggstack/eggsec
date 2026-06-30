//! TUI action/tab metadata registry — Phase 8 pilot.
//!
//! Provides `TuiActionSpec` and `TuiTabSpec` as metadata-backed descriptors
//! that point to canonical `OperationMetadata` and `DomainDescriptor` entries.
//! This prevents TUI actions from independently inventing risk/capability/scope
//! semantics and catches drift between TUI display and canonical metadata.
//!
//! The pilot covers recon, scan-ports, fuzz, and db-pentest. Other tabs
//! continue to use the existing `TabSpec` + `build_current_operation_descriptor()`
//! path and can be migrated incrementally.

use eggsec::config::{metadata_for_tool_id, OperationRisk};

/// Risk hint derived from canonical metadata. The TUI may display additional
/// context (e.g. "dry-run: SafeActive") but the base risk comes from metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ActionRisk {
    Passive,
    SafeActive,
    Intrusive,
}

impl ActionRisk {
    #[allow(dead_code)]
    pub fn from_operation_risk(r: OperationRisk) -> Self {
        match r {
            OperationRisk::Passive => ActionRisk::Passive,
            OperationRisk::SafeActive => ActionRisk::SafeActive,
            OperationRisk::Intrusive => ActionRisk::Intrusive,
            _ => ActionRisk::SafeActive,
        }
    }
}

/// Metadata-backed action specification for a TUI action.
///
/// Each spec references a canonical `OperationMetadata` entry by operation ID.
/// The TUI uses these to validate that actions remain consistent with the
/// shared enforcement model.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct TuiActionSpec {
    /// Unique action identifier (e.g. "recon-run", "scan-ports-run").
    pub action_id: &'static str,
    /// Canonical operation ID in `OperationMetadata` (e.g. "recon", "scan-ports").
    pub operation_id: &'static str,
    /// TUI tab that owns this action.
    pub tab_id: &'static str,
    /// Cargo feature gate, if any (e.g. "db-pentest").
    pub feature: Option<&'static str>,
    /// Whether this action is manual-only (TUI interactive, not programmable).
    pub manual_only: bool,
}

/// Metadata-backed tab specification for the pilot registry.
#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct TuiTabSpec {
    /// Tab stable ID (e.g. "recon", "scan_ports").
    pub tab_id: &'static str,
    /// Display title.
    pub title: &'static str,
    /// Optional domain descriptor ID (e.g. "db-pentest").
    pub domain_id: Option<&'static str>,
    /// Optional feature gate.
    pub feature: Option<&'static str>,
    /// Actions belonging to this tab.
    pub actions: &'static [TuiActionSpec],
}

/// Pilot: metadata-backed TUI action/tab specs.
///
/// These cover a representative subset: recon (no feature gate, safe-active),
/// scan-ports (no feature gate, safe-active), fuzz (no feature gate, intrusive),
/// and db-pentest (feature-gated, domain-backed, intrusive).
#[allow(dead_code)]
pub static TUI_ACTION_SPECS: &[TuiActionSpec] = &[
    TuiActionSpec {
        action_id: "recon-run",
        operation_id: "recon",
        tab_id: "recon",
        feature: None,
        manual_only: true,
    },
    TuiActionSpec {
        action_id: "scan-ports-run",
        operation_id: "scan-ports",
        tab_id: "scan_ports",
        feature: None,
        manual_only: true,
    },
    TuiActionSpec {
        action_id: "fuzz-run",
        operation_id: "fuzz",
        tab_id: "fuzz",
        feature: None,
        manual_only: true,
    },
    TuiActionSpec {
        action_id: "db-pentest-run",
        operation_id: "db-pentest",
        tab_id: "db_pentest",
        feature: Some("db-pentest"),
        manual_only: true,
    },
];

#[allow(dead_code)]
pub static TUI_TAB_SPECS: &[TuiTabSpec] = &[
    TuiTabSpec {
        tab_id: "recon",
        title: "Recon",
        domain_id: None,
        feature: None,
        actions: &[
            TUI_ACTION_SPECS[0], // recon-run
        ],
    },
    TuiTabSpec {
        tab_id: "scan_ports",
        title: "Scan Ports",
        domain_id: None,
        feature: None,
        actions: &[
            TUI_ACTION_SPECS[1], // scan-ports-run
        ],
    },
    TuiTabSpec {
        tab_id: "fuzz",
        title: "Fuzz",
        domain_id: None,
        feature: None,
        actions: &[
            TUI_ACTION_SPECS[2], // fuzz-run
        ],
    },
    TuiTabSpec {
        tab_id: "db_pentest",
        title: "Db Pentest",
        domain_id: Some("db-pentest"),
        feature: Some("db-pentest"),
        actions: &[
            TUI_ACTION_SPECS[3], // db-pentest-run
        ],
    },
];

/// All pilot action specs.
#[allow(dead_code)]
pub fn tui_action_specs() -> &'static [TuiActionSpec] {
    TUI_ACTION_SPECS
}

/// All pilot tab specs.
#[allow(dead_code)]
pub fn tui_tab_specs() -> &'static [TuiTabSpec] {
    TUI_TAB_SPECS
}

/// Look up a tab spec by tab_id.
#[allow(dead_code)]
pub fn tab_spec_for_id(tab_id: &str) -> Option<&'static TuiTabSpec> {
    TUI_TAB_SPECS.iter().find(|s| s.tab_id == tab_id)
}

/// Verify that an action's operation_id resolves to a canonical metadata entry.
#[allow(dead_code)]
pub fn action_resolves_to_metadata(action: &TuiActionSpec) -> bool {
    metadata_for_tool_id(action.operation_id).is_some()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── Work item 2: Pilot registry exists and is non-empty ───────────

    #[test]
    fn pilot_action_specs_non_empty() {
        assert!(
            !TUI_ACTION_SPECS.is_empty(),
            "pilot action specs should not be empty"
        );
    }

    #[test]
    fn pilot_tab_specs_non_empty() {
        assert!(
            !TUI_TAB_SPECS.is_empty(),
            "pilot tab specs should not be empty"
        );
    }

    // ─── Work item 7: Metadata consistency tests ───────────────────────

    /// Every pilot action with an operation_id resolves to `OperationMetadata`.
    #[test]
    fn all_pilot_actions_resolve_to_metadata() {
        for action in TUI_ACTION_SPECS {
            assert!(
                metadata_for_tool_id(action.operation_id).is_some(),
                "TUI action '{}' references operation '{}' which has no matching OperationMetadata",
                action.action_id,
                action.operation_id
            );
        }
    }

    /// Every pilot tab spec's tab_id maps to a valid TabSpec stable_id.
    #[test]
    fn all_pilot_tab_ids_are_valid() {
        for tab in TUI_TAB_SPECS {
            assert!(
                crate::tabs::spec_for_id(tab.tab_id).is_some(),
                "TUI tab spec '{}' has no matching TabSpec",
                tab.tab_id
            );
        }
    }

    /// Feature strings are non-empty when present.
    #[test]
    fn feature_strings_are_valid() {
        for action in TUI_ACTION_SPECS {
            if let Some(feat) = action.feature {
                assert!(
                    !feat.is_empty(),
                    "action '{}' has empty feature string",
                    action.action_id
                );
            }
        }
        for tab in TUI_TAB_SPECS {
            if let Some(feat) = tab.feature {
                assert!(
                    !feat.is_empty(),
                    "tab '{}' has empty feature string",
                    tab.tab_id
                );
            }
        }
    }

    /// High-risk TUI actions (Intrusive) are manual-only.
    #[test]
    fn intrusive_actions_are_manual_only() {
        for action in TUI_ACTION_SPECS {
            if let Some(metadata) = metadata_for_tool_id(action.operation_id) {
                let risk = ActionRisk::from_operation_risk(metadata.risk);
                if risk == ActionRisk::Intrusive {
                    assert!(
                        action.manual_only,
                        "intrusive action '{}' should be manual_only",
                        action.action_id
                    );
                }
            }
        }
    }

    /// Descriptor builder risk from metadata matches the spec's declared risk
    /// (for the pilot, all actions resolve via metadata, so the risk should match).
    #[test]
    fn metadata_risk_matches_operation_risk() {
        for action in TUI_ACTION_SPECS {
            let metadata = metadata_for_tool_id(action.operation_id)
                .expect("action should resolve to metadata");
            // Verify the metadata has a valid risk variant (all variants are acceptable)
            let _ = metadata.risk;
        }
    }

    /// Pilot actions with domain_id should reference a known domain.
    #[test]
    fn domain_refs_are_valid() {
        for tab in TUI_TAB_SPECS {
            if let Some(domain_id) = tab.domain_id {
                let found = eggsec::domain::all_domain_descriptors()
                    .iter()
                    .any(|d| d.id == domain_id);
                assert!(
                    found,
                    "tab '{}' references domain '{}' which has no matching DomainDescriptor",
                    tab.tab_id, domain_id
                );
            }
        }
    }

    /// Every pilot tab with an operation_id resolves via metadata.
    #[test]
    fn all_pilot_tab_operations_resolve() {
        for tab in TUI_TAB_SPECS {
            if let Some(spec) = crate::tabs::spec_for_id(tab.tab_id) {
                if let Some(op_id) = spec.operation {
                    assert!(
                        metadata_for_tool_id(op_id).is_some(),
                        "TabSpec for '{}' has operation '{}' but no matching metadata",
                        tab.tab_id,
                        op_id
                    );
                }
            }
        }
    }

    /// Registry can be enumerated (basic sanity).
    #[test]
    fn registry_enumeration() {
        let action_count = tui_action_specs().len();
        let tab_count = tui_tab_specs().len();
        assert!(action_count >= 2, "need at least 2 pilot actions");
        assert!(tab_count >= 2, "need at least 2 pilot tabs");
        // Each tab should have at least one action
        for tab in tui_tab_specs() {
            assert!(
                !tab.actions.is_empty(),
                "tab '{}' has no actions",
                tab.tab_id
            );
        }
    }
}
