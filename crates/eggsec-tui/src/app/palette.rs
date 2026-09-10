//! Phase 2.4 palette -> typed action bridge (decomposed from `app/command.rs`).
//!
//! Key handlers and palette parsing produce the same typed [`UiAction`] when
//! they mean the same thing. Rendering functions never decide operation
//! semantics or start engine work; execution effects (`run`, `cancel`,
//! `reload`) are requested as actions and applied in `apply_action`.
//!
//! Navigation/local actions (switch tab, open help, copy, export, theme,
//! search) are separated from execution effects (run operation, cancel,
//! attach runtime) so the app state machine stays testable without a
//! terminal.

use crate::tabs::{resolve_palette_command, PaletteResolution, Tab};

use super::action::UiAction;

/// Typed palette intent: tab navigation vs global action vs unavailable vs unknown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PaletteAction {
    SelectTab(Tab),
    Global(UiAction),
    Unavailable {
        tab: Tab,
        required_feature: &'static str,
    },
    Unknown,
}

/// Parse a palette string into typed intent through the consolidated surface
/// metadata. No manual match for tabs; global commands mirror the key-handler
/// decode targets so both paths converge.
pub fn parse_palette_action(command: &str) -> PaletteAction {
    match resolve_palette_command(command) {
        PaletteResolution::SelectTab(tab) => PaletteAction::SelectTab(tab),
        PaletteResolution::Unavailable {
            tab,
            required_feature,
        } => PaletteAction::Unavailable {
            tab,
            required_feature,
        },
        PaletteResolution::Unknown => {
            if let Some(action) = global_action_for(command) {
                PaletteAction::Global(action)
            } else {
                // `stop`/`pause`/`resume`/`jump-active` and other legacy
                // execution effects are handled by `execute_command` arms;
                // they are not pure navigation actions.
                PaletteAction::Unknown
            }
        }
    }
}

/// Global palette commands that map to the same [`UiAction`] the key handler
/// emits. Kept exhaustive over the palette entries in `help_config.rs`
/// (minus tab navigation, which resolves via surface metadata).
pub fn global_action_for(command: &str) -> Option<UiAction> {
    let action = match command {
        "quit" | "exit" => UiAction::Quit,
        "next-tab" | "next" => UiAction::NextTab,
        "prev-tab" | "previous" | "prev" => UiAction::PrevTab,
        "help" | "help-current" => UiAction::ToggleHelp,
        "palette" => UiAction::ToggleCommandPalette,
        "quick-switch" | "open-quick" => UiAction::ToggleQuickSwitch,
        "search" | "open-search" => UiAction::ToggleSearch { global: true },
        "global-search" => UiAction::ToggleSearch { global: false },
        "theme" => UiAction::ToggleTheme,
        "export" => UiAction::ExportResults,
        "cycle-export" => UiAction::CycleExportFormat,
        "run" | "run-current" => UiAction::Enter,
        "reset" => UiAction::ResetCurrent,
        "save" | "save-settings" => UiAction::SaveSettings,
        "clear-history" => UiAction::DeleteHistoryEntry,
        "page-up" => UiAction::PageUp,
        "page-down" => UiAction::PageDown,
        "toggle-posture" | "enforcement" => UiAction::ToggleEnforcementPosture,
        _ => return None,
    };
    Some(action)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::create_test_app;

    #[test]
    fn palette_and_surface_agree_for_every_visible_tab() {
        for tab in Tab::all() {
            let spec = crate::tabs::spec_for(*tab).unwrap();
            let action = parse_palette_action(spec.palette_command());
            assert_eq!(
                action,
                PaletteAction::SelectTab(*tab),
                "palette '{}' must select {:?}",
                spec.palette_command(),
                tab
            );
        }
    }

    #[test]
    fn key_and_palette_produce_same_typed_action() {
        // Requirement 2.4/2.9: key event -> TuiAction and palette string ->
        // same TuiAction when they mean the same thing (no terminal needed).
        //
        // The key handler emits these `UiAction`s for the corresponding keys;
        // the palette must emit the identical variant for the same intent.
        let cases = [
            ("next-tab", UiAction::NextTab),
            ("next", UiAction::NextTab),
            ("prev-tab", UiAction::PrevTab),
            ("prev", UiAction::PrevTab),
            ("help", UiAction::ToggleHelp),
            ("theme", UiAction::ToggleTheme),
            ("export", UiAction::ExportResults),
            ("cycle-export", UiAction::CycleExportFormat),
            ("run", UiAction::Enter),
            ("reset", UiAction::ResetCurrent),
        ];
        for (palette, expected) in cases {
            assert_eq!(
                global_action_for(palette),
                Some(expected.clone()),
                "palette '{palette}' must match key-handler action"
            );
            assert_eq!(
                parse_palette_action(palette),
                PaletteAction::Global(expected),
                "parse must wrap global '{palette}'"
            );
        }
        // Tab navigation: palette string and `SelectTab` action agree.
        for tab in Tab::all() {
            let spec = crate::tabs::spec_for(*tab).unwrap();
            assert_eq!(
                parse_palette_action(spec.palette_command()),
                PaletteAction::SelectTab(*tab)
            );
        }
    }

    #[test]
    fn action_state_transition_is_testable_without_terminal() {
        // Requirement 2.4: action -> state transition without rendering.
        let mut app = create_test_app();
        let start = app.current_tab;
        app.apply_action(UiAction::NextTab);
        assert_eq!(app.current_tab, start.next());
        app.apply_action(UiAction::PrevTab);
        assert_eq!(app.current_tab, start);
        app.apply_action(UiAction::SelectTab(Tab::Fuzz));
        assert_eq!(app.current_tab, Tab::Fuzz);
    }

    #[test]
    fn reload_scope_is_not_discoverable_but_explains_restart() {
        // Requirement 2.8: no false affordance in discovery; direct
        // invocation explains restart-required.
        let mut app = create_test_app();
        // Not in discoverable palette entries.
        let entries = app.help_manager.get_command_palette_entries();
        assert!(
            !entries.iter().any(|e| e.command == "reload-scope"),
            "reload-scope must not be discoverable"
        );
        // Direct invocation is informational, not a silent no-op.
        app.execute_command("reload-scope");
        let msg = app
            .overlay
            .notification
            .as_ref()
            .expect("reload-scope must notify")
            .message
            .clone();
        assert!(
            msg.contains("restart") || msg.contains("Live reload"),
            "unexpected reload-scope message: {msg}"
        );
    }

    #[test]
    fn copy_cli_returns_explicit_unsupported_for_ui_only() {
        // Requirement 2.7: UI-only state returns explicit unsupported (None),
        // never a misleading command.
        let mut app = create_test_app();
        for tab in [Tab::Settings, Tab::History, Tab::Dashboard, Tab::Report] {
            app.current_tab = tab;
            assert_eq!(app.cli_argv(), None, "UI-only {tab:?} must have no argv");
            assert_eq!(
                app.copy_cli_equivalent(),
                None,
                "UI-only {tab:?} must have no CLI equivalent"
            );
        }
    }
}
