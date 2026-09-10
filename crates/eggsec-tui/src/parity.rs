//! Phase 0.4 TUI operation and feature parity guards (test-owned) +
//! Phase 2 production surface convergence.
//!
//! Classifies every TUI tab/action as operation-backed, UI-only/helper,
//! lifecycle, unavailable, or compatibility alias. Resolves the audited
//! baseline discrepancies explicitly so raw string differences never go
//! undocumented.
//!
//! Phase 2: the four-action pilot (`TUI_ACTION_SPECS`) is removed. The
//! production owner is `TabSpec` (`tabs/spec.rs`) plus the typed surface
//! model (`TuiSurfaceRoute`, `TabAvailability`, `resolve_palette_command`).
//! This module now pins production classification instead of maintaining a
//! parallel test-owned matrix.

#[cfg(test)]
mod tests {
    use std::collections::{BTreeMap, BTreeSet};

    use eggsec::config::{is_feature_enabled_registry, metadata_for_tool_id};

    use crate::tabs::{spec_for, tab_specs, Tab};

    /// Classification for a TUI tab.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum TabClass {
        OperationBacked,
        UiOnly,
        Helper,
        Lifecycle,
        UnavailableShell,
        CompatibilityAlias,
    }

    /// UI-only tabs: stable IDs with no operation and no CLI command.
    fn ui_only_tabs() -> BTreeSet<&'static str> {
        BTreeSet::from(["settings", "history", "dashboard"])
    }

    /// Helper tabs: report-style, no operation-backed dispatch.
    fn helper_tabs() -> BTreeSet<&'static str> {
        BTreeSet::from(["report", "resume", "proxy"])
    }

    /// Lifecycle tabs: daemon/server session management.
    fn lifecycle_tabs() -> BTreeSet<&'static str> {
        BTreeSet::from(["cluster"])
    }

    /// Availability shells: always visible, execution gated by feature.
    /// (Stress/Packet remain in base `Tab::all()` even when disabled.)
    fn availability_shells() -> BTreeMap<&'static str, &'static str> {
        BTreeMap::from([
            ("stress", "stress-testing"),
            ("packet", "packet-inspection"),
        ])
    }

    /// Compatibility aliases: TUI operation string differs from canonical but
    /// resolves via `ALL_OPERATION_METADATA_ALIASES` with a tested conversion.
    /// After Phase 0 normalization this set is empty (waf/pipeline normalized);
    /// the test pins that no raw alias strings remain in `TabSpec::operation`.
    fn compatibility_aliases() -> BTreeMap<&'static str, &'static str> {
        BTreeMap::new()
    }

    fn classify_tab(stable_id: &str, operation: Option<&str>) -> TabClass {
        if ui_only_tabs().contains(stable_id) {
            return TabClass::UiOnly;
        }
        if helper_tabs().contains(stable_id) {
            return TabClass::Helper;
        }
        if lifecycle_tabs().contains(stable_id) {
            return TabClass::Lifecycle;
        }
        if availability_shells().contains_key(stable_id) {
            // Shells are operation-backed when enabled; classification here
            // denotes the visibility contract, operation check below pins it.
            return TabClass::UnavailableShell;
        }
        if compatibility_aliases().contains_key(stable_id) {
            return TabClass::CompatibilityAlias;
        }
        if operation.is_some() {
            return TabClass::OperationBacked;
        }
        TabClass::UiOnly
    }

    #[test]
    fn every_tab_is_classified() {
        for spec in tab_specs() {
            let class = classify_tab(spec.stable_id, spec.operation);
            // Every spec must fall into exactly one bucket (no unclassified).
            match class {
                TabClass::OperationBacked
                | TabClass::UiOnly
                | TabClass::Helper
                | TabClass::Lifecycle
                | TabClass::UnavailableShell
                | TabClass::CompatibilityAlias => {}
            }
        }
        // UI-only tabs have no operation.
        for id in ui_only_tabs() {
            let spec = tab_specs()
                .iter()
                .find(|s| s.stable_id == id)
                .unwrap_or_else(|| panic!("ui-only tab '{id}' must have a spec"));
            assert!(
                spec.operation.is_none(),
                "ui-only tab '{id}' must have no operation"
            );
        }
        // Helper/lifecycle tabs have no operation-backed dispatch.
        for id in helper_tabs().union(&lifecycle_tabs()).collect::<Vec<_>>() {
            let spec = tab_specs()
                .iter()
                .find(|s| s.stable_id == *id)
                .unwrap_or_else(|| panic!("tab '{id}' must have a spec"));
            assert!(
                spec.operation.is_none(),
                "helper/lifecycle tab '{id}' must have no operation"
            );
        }
    }

    #[test]
    fn waf_normalized_to_canonical() {
        // Audited discrepancy: TUI `waf` vs canonical `waf-detect`.
        // Resolution: normalize to canonical; `waf` remains a tested alias.
        let spec = spec_for(Tab::Waf).expect("waf spec must exist");
        assert_eq!(
            spec.operation,
            Some("waf-detect"),
            "WAF tab must use canonical `waf-detect`"
        );
        assert!(
            metadata_for_tool_id("waf").is_some(),
            "`waf` alias must still resolve (compatibility)"
        );
        assert_eq!(
            metadata_for_tool_id("waf").unwrap().id,
            "waf-detect",
            "`waf` alias must resolve to `waf-detect`, not a second identity"
        );
    }

    #[test]
    fn scan_pipeline_normalized_to_canonical() {
        // Audited discrepancy: TUI `scan-pipeline` vs canonical `pipeline`.
        let spec = spec_for(Tab::Scan).expect("scan spec must exist");
        assert_eq!(
            spec.operation,
            Some("pipeline"),
            "Scan tab must use canonical `pipeline`"
        );
        assert_eq!(
            metadata_for_tool_id("scan-pipeline").unwrap().id,
            "pipeline",
            "`scan-pipeline` alias must resolve to `pipeline`"
        );
    }

    #[test]
    fn stress_and_packet_feature_ownership_is_canonical() {
        // Audited discrepancy: stress/packet tab feature ownership.
        let stress = spec_for(Tab::Stress).expect("stress spec must exist");
        assert_eq!(stress.operation, Some("stress-test"));
        assert_eq!(stress.feature, Some("stress-testing"));
        let packet = spec_for(Tab::Packet).expect("packet spec must exist");
        assert_eq!(packet.operation, Some("packet"));
        assert_eq!(packet.feature, Some("packet-inspection"));
        // Canonical metadata agrees.
        assert_eq!(
            metadata_for_tool_id("stress-test")
                .unwrap()
                .primary_feature(),
            Some("stress-testing")
        );
        assert_eq!(
            metadata_for_tool_id("packet").unwrap().primary_feature(),
            Some("packet-inspection")
        );
    }

    #[test]
    fn proxy_intercept_naming_is_explicit() {
        // Audited discrepancy: proxy/intercept represent related but not
        // identical capabilities. Proxy tab is helper (pool management, no
        // operation); Intercept tab is operation-backed traffic interception.
        let proxy = spec_for(Tab::Proxy).expect("proxy spec must exist");
        assert!(
            proxy.operation.is_none(),
            "Proxy tab must remain helper-only (no operation)"
        );
        let intercept = spec_for(Tab::Intercept).expect("intercept spec must exist");
        assert_eq!(intercept.operation, Some("proxy-intercept"));
        assert_eq!(intercept.feature, Some("web-proxy"));
        assert_eq!(
            metadata_for_tool_id("proxy-intercept").unwrap().id,
            "proxy-intercept"
        );
        // The `proxy` alias resolves to proxy-intercept at the operation
        // layer, but the TUI keeps them distinct: pool helper vs intercept op.
        assert_eq!(
            metadata_for_tool_id("proxy").unwrap().id,
            "proxy-intercept",
            "`proxy` alias resolves to `proxy-intercept` canonically"
        );
    }

    #[test]
    fn runtime_only_families_resolve_to_canonical() {
        // Audited discrepancy: compliance/storage/integrations/workflow/vuln.
        // Resolution: operation-backed TUI tabs with canonical feature gates;
        // bridge failures (when targetless) are explicit, not silent.
        let cases = [
            ("compliance", "compliance", "compliance"),
            ("storage", "storage", "database"),
            ("integrations", "integrations", "external-integrations"),
            ("workflow", "workflow", "finding-workflow"),
            ("vuln", "vuln", "vuln-management"),
        ];
        for (stable_id, op, feat) in cases {
            let spec = tab_specs()
                .iter()
                .find(|s| s.stable_id == stable_id)
                .unwrap_or_else(|| panic!("tab '{stable_id}' must have a spec"));
            assert_eq!(spec.operation, Some(op), "tab '{stable_id}' operation");
            assert_eq!(spec.feature, Some(feat), "tab '{stable_id}' feature");
            let meta = metadata_for_tool_id(op)
                .unwrap_or_else(|| panic!("operation '{op}' must have metadata"));
            assert_eq!(meta.id, op);
            assert!(
                meta.required_features.contains(&feat),
                "canonical '{op}' must declare feature '{feat}'"
            );
        }
    }

    #[test]
    fn canonical_operations_without_tui_tabs_are_intentional() {
        // Canonical operations with no TUI tab are intentionally
        // CLI/programmatic-only. New operations must be classified here.
        let tui_ops: BTreeSet<&str> = tab_specs().iter().filter_map(|s| s.operation).collect();
        // Aliases that resolve to a TUI-covered canonical op count as covered.
        let covered_via_alias: BTreeSet<&str> = BTreeSet::from(["waf", "scan-pipeline"]);
        let intentionally_cli_only: BTreeSet<&str> = BTreeSet::from([
            "waf-bypass",      // via `waf --bypass` flag path
            "remote",          // via exec/remote lifecycle surface
            "search",          // NoTarget helper operation
            "mobile-static",   // CLI `mobile` multiplexer; no TUI tab
            "mobile-dynamic",  // CLI `mobile dynamic` subcommand; no TUI tab
            "evasion",         // standalone CLI defense-lab; no TUI tab
            "postex",          // standalone CLI defense-lab; no TUI tab
            "wireless-deauth", // TUI Wireless active_mode override covers it; no dedicated tab
        ]);
        for meta in eggsec::config::all_operation_metadata() {
            let direct = tui_ops.contains(meta.id);
            let via_alias =
                eggsec::config::ALL_OPERATION_METADATA_ALIASES
                    .iter()
                    .any(|(alias, canonical)| {
                        *canonical == meta.id
                            && (tui_ops.contains(alias) || covered_via_alias.contains(alias))
                    });
            if !direct && !via_alias {
                assert!(
                    intentionally_cli_only.contains(meta.id),
                    "canonical operation '{}' has no TUI tab and no intentional classification",
                    meta.id
                );
            }
        }
        // No stale entries: every intentional entry must still lack a tab.
        for op in &intentionally_cli_only {
            // wireless-deauth is covered via active_mode override, not a tab.
            assert!(
                !tui_ops.contains(op),
                "intentionally_cli_only entry '{op}' now has a TUI tab — remove it"
            );
        }
    }

    #[test]
    fn operation_backed_tabs_resolve_to_metadata_with_matching_feature() {
        for spec in tab_specs() {
            let Some(op) = spec.operation else { continue };
            // Availability shells are covered by the dedicated test; skip the
            // strict feature-equality here when the feature is disabled at
            // compile time (tab visible, execution unavailable).
            let meta = metadata_for_tool_id(op).unwrap_or_else(|| {
                panic!("tab '{}' operation '{op}' has no metadata", spec.stable_id)
            });
            assert_eq!(
                meta.id, op,
                "tab '{}' operation '{op}' must be canonical (not an alias)",
                spec.stable_id
            );
            // Feature ownership must match canonical, except availability
            // shells always declare it (visibility != availability).
            assert_eq!(
                spec.feature,
                meta.primary_feature(),
                "tab '{}' feature {:?} != canonical {:?} for operation '{}'",
                spec.stable_id,
                spec.feature,
                meta.primary_feature(),
                op
            );
        }
    }

    #[test]
    fn feature_disabled_vs_unavailable_visible_is_intentional() {
        // Compile-time absence (tab missing from Tab::all) vs runtime
        // unavailable shell (tab visible, feature disabled) is intentional.
        let all = Tab::all();
        // Availability shells are always visible by design.
        for (stable_id, feat) in availability_shells() {
            let spec = tab_specs()
                .iter()
                .find(|s| s.stable_id == stable_id)
                .expect("shell must have a spec");
            assert!(
                all.contains(&spec.tab),
                "availability shell '{stable_id}' must remain visible"
            );
            assert!(
                eggsec::config::is_known_feature_registry(feat),
                "shell '{stable_id}' gates on unknown feature '{feat}'"
            );
            let _ = is_feature_enabled_registry(feat);
        }
        // Visibility-gated tabs track the compiled feature set.
        let gated = [
            (Tab::Nse, "nse"),
            (Tab::Hunt, "advanced-hunting"),
            (Tab::Browser, "headless-browser"),
            (Tab::Compliance, "compliance"),
            (Tab::Storage, "database"),
            (Tab::Integrations, "external-integrations"),
            (Tab::Workflow, "finding-workflow"),
            (Tab::Vuln, "vuln-management"),
            (Tab::Wireless, "wireless"),
            (Tab::DbPentest, "db-pentest"),
            (Tab::Intercept, "web-proxy"),
            (Tab::C2, "c2"),
        ];
        for (tab, feat) in gated {
            assert!(
                eggsec::config::is_known_feature_registry(feat),
                "gated tab {tab:?} references unknown feature '{feat}'"
            );
            assert_eq!(
                all.contains(&tab),
                is_feature_enabled_registry(feat),
                "tab {tab:?} visibility must track feature '{feat}'"
            );
        }
    }

    #[test]
    fn production_surface_model_covers_every_tab_without_second_identity() {
        // Phase 2: pilot removed. Every operation-backed tab resolves through
        // the production typed model (`surface_route` + `canonical_operation`)
        // with no second canonical identity.
        use crate::tabs::{resolve_palette_command, PaletteResolution, TuiSurfaceRoute};
        for spec in tab_specs() {
            match spec.surface_route() {
                TuiSurfaceRoute::Operation(op) => {
                    assert_eq!(
                        spec.canonical_operation(),
                        Some(op),
                        "tab '{}' operation '{op}' must be canonical",
                        spec.stable_id
                    );
                    let meta = metadata_for_tool_id(op).unwrap_or_else(|| {
                        panic!("tab '{}' operation '{op}' has no metadata", spec.stable_id)
                    });
                    assert_eq!(meta.id, op);
                    // Palette primary must resolve back to the same tab when visible.
                    if crate::tabs::Tab::all().contains(&spec.tab) {
                        assert_eq!(
                            resolve_palette_command(spec.palette_command()),
                            PaletteResolution::SelectTab(spec.tab),
                            "palette '{}' must resolve to {:?}",
                            spec.palette_command(),
                            spec.tab
                        );
                    }
                }
                TuiSurfaceRoute::Multiplexer(family) => {
                    assert_eq!(
                        spec.stable_id, "wireless",
                        "only Wireless is a multiplexer (family '{family}')"
                    );
                    assert!(
                        metadata_for_tool_id("wireless").is_some(),
                        "multiplexer family must have canonical metadata"
                    );
                }
                TuiSurfaceRoute::Helper | TuiSurfaceRoute::Lifecycle | TuiSurfaceRoute::UiOnly => {
                    assert!(
                        spec.canonical_operation().is_none(),
                        "non-operation tab '{}' must have no canonical operation",
                        spec.stable_id
                    );
                }
            }
        }
    }
}
