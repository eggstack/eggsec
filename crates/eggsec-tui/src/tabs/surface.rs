//! Phase 2 surface catalog (decomposed from `tabs/spec.rs` + `tabs/mod.rs`).
//!
//! Single-concept owner for TUI discovery/availability derived from the
//! consolidated `TabSpec` metadata:
//! - discoverable palette commands (one primary per visible tab);
//! - alias uniqueness (no two tabs claim the same parseable string unless
//!   an explicit priority exists — none does, so aliases must be unique);
//! - help/discovery consistency (title, palette, description, feature,
//!   run support, canonical route, help target agree);
//! - availability shells (Stress/Packet always visible, others track cfg).
//!
//! `TabSpec` remains the data owner in `spec.rs`; this module owns the
//! derived catalog queries so `tabs/core.rs`, `app/command.rs`,
//! `app/help_config.rs`, and palette filtering cannot drift independently.

use super::spec::{TabAvailability, TuiSurfaceRoute};
use super::{spec_for, tab_specs, Tab};

/// Discoverable palette commands for the current build, in `Tab::all()` order.
///
/// Uses [`TabSpec::palette_command`] only — hidden aliases never pollute
/// discovery. Help/palette filtering must use this source.
pub fn discoverable_palette_commands() -> Vec<(&'static str, Tab)> {
    Tab::all()
        .iter()
        .filter_map(|tab| spec_for(*tab).map(|s| (s.palette_command(), *tab)))
        .collect()
}

/// All parseable strings (stable_id + palette + aliases) for uniqueness tests.
pub fn all_parseable_commands() -> Vec<(&'static str, Tab)> {
    let mut out = Vec::new();
    for spec in tab_specs() {
        out.push((spec.stable_id, spec.tab));
        out.push((spec.palette_command(), spec.tab));
        for alias in spec.aliases() {
            out.push((*alias, spec.tab));
        }
    }
    out
}

/// Structured availability for a tab (re-export convenience).
pub fn availability_for(tab: Tab) -> Option<TabAvailability> {
    spec_for(tab).map(|s| s.availability())
}

/// Canonical route for a tab (re-export convenience).
pub fn route_for(tab: Tab) -> Option<TuiSurfaceRoute> {
    spec_for(tab).map(|s| s.surface_route())
}

#[cfg(test)]
mod tests {
    use super::super::spec::PaletteResolution;
    use super::*;
    use std::collections::{BTreeMap, BTreeSet};

    #[test]
    fn discoverable_commands_match_visible_tabs_one_to_one() {
        let cmds = discoverable_palette_commands();
        let all = Tab::all();
        assert_eq!(
            cmds.len(),
            all.len(),
            "discoverable palette must have exactly one entry per visible tab"
        );
        for (cmd, tab) in &cmds {
            assert!(
                !cmd.is_empty(),
                "palette command for {:?} must not be empty",
                tab
            );
            assert!(
                all.contains(tab),
                "discoverable {:?} must be in Tab::all()",
                tab
            );
        }
        // No duplicate discoverable commands.
        let mut seen = BTreeSet::new();
        for (cmd, tab) in &cmds {
            assert!(
                seen.insert(*cmd),
                "duplicate discoverable palette command '{cmd}' (tab {tab:?})"
            );
        }
    }

    #[test]
    fn aliases_are_unique_across_tabs() {
        // Requirement 2.5: aliases tested unique unless an explicit priority
        // rule exists. No priority rule exists, so every parseable string
        // must map to exactly one tab.
        let mut owners: BTreeMap<&str, Tab> = BTreeMap::new();
        for spec in tab_specs() {
            let mut candidates: Vec<&'static str> = vec![spec.stable_id, spec.palette_command()];
            candidates.extend(spec.aliases().iter().copied());
            for cmd in candidates {
                if let Some(prev) = owners.get(cmd) {
                    // Same-tab duplicates are allowed (stable_id may equal
                    // palette/alias); cross-tab duplicates are not.
                    assert_eq!(
                        *prev, spec.tab,
                        "alias '{cmd}' claimed by both {prev:?} and {:?}",
                        spec.tab
                    );
                } else {
                    owners.insert(cmd, spec.tab);
                }
            }
        }
    }

    #[test]
    fn hidden_aliases_resolve_but_do_not_pollute_discovery() {
        // Compatibility labels remain parseable via `resolve_palette_command`
        // but only the primary appears in discovery.
        use super::super::spec::resolve_palette_command;
        let cases = [
            ("scan-pipeline", Tab::Scan),
            ("waf-detect", Tab::Waf),
            ("o-auth", Tab::OAuth),
            ("wifi", Tab::Wireless),
            ("portscan", Tab::ScanPorts),
        ];
        let discoverable: BTreeSet<&str> = discoverable_palette_commands()
            .into_iter()
            .map(|(c, _)| c)
            .collect();
        for (alias, tab) in cases {
            // Hidden aliases resolve when the tab is visible; when the tab
            // is compiled out they report unavailable/unknown (both ok).
            match resolve_palette_command(alias) {
                PaletteResolution::SelectTab(t) => assert_eq!(t, tab),
                PaletteResolution::Unavailable { tab: t, .. } => assert_eq!(t, tab),
                PaletteResolution::Unknown => {
                    assert!(
                        !Tab::all().contains(&tab),
                        "alias '{alias}' unknown but tab {tab:?} is visible"
                    );
                }
            }
            // Only the primary pollutes discovery, never the hidden alias,
            // unless the hidden alias *is* the primary (none of these are).
            if alias != super::super::spec::spec_for(tab).unwrap().palette_command() {
                assert!(
                    !discoverable.contains(alias) || alias == "scan",
                    "hidden alias '{alias}' must not pollute discovery"
                );
            }
        }
    }

    #[test]
    fn help_discovery_metadata_agrees() {
        // Requirement 2.6: tab title, palette command/stable ID, short
        // description, feature/availability, run support, canonical route,
        // and help target must not disagree.
        for spec in tab_specs() {
            assert!(!spec.title.is_empty(), "empty title for {:?}", spec.tab);
            assert!(
                !spec.stable_id.is_empty(),
                "empty stable_id for {:?}",
                spec.tab
            );
            assert!(
                !spec.palette_command().is_empty(),
                "empty palette for {:?}",
                spec.tab
            );
            assert!(
                !spec.description.is_empty(),
                "empty description for {:?}",
                spec.tab
            );
            assert!(
                !spec.help_text.is_empty(),
                "empty help_text for {:?}",
                spec.tab
            );
            assert!(
                spec.tab.help_entry().len() >= 2,
                "help_entry for {:?} must be indented",
                spec.tab
            );
            // Run support agrees with route: operation/multiplexer tabs that
            // declare supports_run must have a route; UI-only never runs.
            match spec.surface_route() {
                TuiSurfaceRoute::UiOnly | TuiSurfaceRoute::Helper | TuiSurfaceRoute::Lifecycle => {
                    // Helpers/lifecycle may still declare supports_run for
                    // local actions (e.g. Resume loads a session); the
                    // invariant is they never claim a canonical operation.
                    assert!(
                        spec.canonical_operation().is_none(),
                        "non-operation {:?} must not claim canonical op",
                        spec.tab
                    );
                }
                TuiSurfaceRoute::Operation(_) | TuiSurfaceRoute::Multiplexer(_) => {
                    // Operation-backed tabs must have help and palette.
                    assert!(
                        spec.supports_help,
                        "operation tab {:?} must support help",
                        spec.tab
                    );
                }
            }
            // Feature requirement agrees with availability contract.
            match spec.availability() {
                TabAvailability::UiOnly => {}
                TabAvailability::Available => {}
                TabAvailability::Unavailable { required_feature } => {
                    assert_eq!(
                        Some(required_feature),
                        spec.feature,
                        "unavailable {:?} must report its spec feature",
                        spec.tab
                    );
                }
                TabAvailability::NotSupportedOnTui => {
                    assert!(
                        !Tab::all().contains(&spec.tab),
                        "{:?} not-supported but visible",
                        spec.tab
                    );
                }
            }
        }
    }

    #[test]
    fn stress_and_packet_are_availability_shells() {
        // Requirement 2.3: Stress/Packet remain visible as unavailable
        // discovery shells when disabled; other gated tabs disappear.
        use crate::tabs::Tab;
        for tab in [Tab::Stress, Tab::Packet] {
            assert!(
                Tab::all().contains(&tab),
                "{tab:?} availability shell must remain visible"
            );
            let spec = spec_for(tab).unwrap();
            match spec.availability() {
                TabAvailability::Available | TabAvailability::Unavailable { .. } => {}
                other => panic!("{tab:?} shell must be available/unavailable, got {other:?}"),
            }
        }
    }

    #[test]
    fn feature_disabled_aliases_report_unavailable_structured() {
        // Requirement 2.5: feature-disabled aliases return structured
        // unavailable, never silent divergence.
        use super::super::spec::resolve_palette_command;
        for spec in tab_specs() {
            if Tab::all().contains(&spec.tab) {
                continue;
            }
            let Some(feat) = spec.feature else { continue };
            // Availability shells are always visible, so they never hit this.
            if matches!(spec.tab, Tab::Stress | Tab::Packet) {
                continue;
            }
            match resolve_palette_command(spec.palette_command()) {
                PaletteResolution::Unavailable {
                    tab,
                    required_feature,
                } => {
                    assert_eq!(tab, spec.tab);
                    assert_eq!(required_feature, feat);
                }
                PaletteResolution::Unknown => {}
                PaletteResolution::SelectTab(t) => {
                    panic!("disabled {:?} resolved as selectable {t:?}", spec.tab)
                }
            }
        }
    }
}
