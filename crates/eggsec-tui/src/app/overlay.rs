//! Overlay input routing and per-overlay key rules (Phase 2 of
//! tui-architecture-usability-pass.md).
//!
//! `OverlayController` owns the input decoding rules for all overlay types.
//! It is a pure decoder: it asks `app.topmost_overlay()`, dispatches to the
//! matching overlay handler, and returns `Vec<UiAction>`. No mutations or I/O.
//!
//! When an overlay is topmost, any key reaching this layer that has no
//! specific binding still emits `UiAction::Noop` so the caller can early-return
//! and prevent leakage to global shortcuts or tab content.
//!
//! Precedence (exact, via `App::topmost_overlay`):
//! 1. PolicyConfirm
//! 2. ConfirmPopup
//! 3. CommandPalette
//! 4. QuickSwitch
//! 5. Search
//! 6. HttpOptions
//! 7. Help
//!
//! Ctrl-C is never swallowed here (returns empty vec to bubble).
//!
//! The controller owns the input rules; non-topmost overlays receive no input
//! because we only decode when `topmost_overlay().is_some()`.

use super::App;
use super::{CommandPaletteInput, QuickSwitchInput, UiAction};
use crate::OverlayType;
use crossterm::event::{KeyCode, KeyModifiers};

pub(crate) struct OverlayController;

impl OverlayController {
    pub(crate) fn new() -> Self {
        Self
    }

    /// Primary routing entry point. Returns decoded actions for the current
    /// topmost overlay (if any). Returns empty vec for Ctrl-C (bubble) or
    /// when no overlay is active.
    ///
    /// If an overlay is active but the key has no binding, returns a single
    /// Noop (signals "handled at overlay layer" to prevent leak).
    pub(crate) fn decode(&self, app: &App, key: &crossterm::event::KeyEvent) -> Vec<UiAction> {
        if key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('c') {
            // Ctrl-C is always allowed to bubble out of overlays (historical).
            return vec![];
        }

        let top = match app.topmost_overlay() {
            Some(o) => o,
            None => return vec![],
        };

        let actions = match top {
            OverlayType::PolicyConfirm => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Enter) => vec![UiAction::ConfirmPolicyAction],
                (KeyModifiers::NONE, KeyCode::Esc) => vec![UiAction::CancelPolicyAction],
                (KeyModifiers::NONE, KeyCode::Char(c)) => {
                    vec![UiAction::PolicyReasonChar(c)]
                }
                (KeyModifiers::NONE, KeyCode::Backspace) => {
                    vec![UiAction::PolicyReasonBackspace]
                }
                (KeyModifiers::NONE, KeyCode::Delete) => {
                    vec![UiAction::PolicyReasonBackspace]
                }
                _ => vec![UiAction::Noop],
            },
            OverlayType::ConfirmPopup => match (key.modifiers, key.code) {
                (KeyModifiers::NONE, KeyCode::Enter) => vec![UiAction::ConfirmPendingAction],
                (KeyModifiers::NONE, KeyCode::Esc) => vec![UiAction::CancelPendingAction],
                (KeyModifiers::NONE, KeyCode::Char('y')) => vec![UiAction::ConfirmPendingAction],
                (KeyModifiers::NONE, KeyCode::Char('n')) => vec![UiAction::CancelPendingAction],
                _ => vec![UiAction::Noop],
            },
            OverlayType::CommandPalette => self.decode_command_palette(key),
            OverlayType::QuickSwitch => self.decode_quick_switch(key),
            OverlayType::Search => self.decode_overlay_input(app, key),
            OverlayType::HttpOptions => self.decode_overlay_input(app, key),
            OverlayType::Help => self.decode_overlay_input(app, key),
        };

        if actions.is_empty() {
            vec![UiAction::Noop]
        } else {
            actions
        }
    }

    fn decode_command_palette(&self, key: &crossterm::event::KeyEvent) -> Vec<UiAction> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Esc)]
            }
            (KeyModifiers::CONTROL, KeyCode::Char('p')) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Close)]
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Enter)]
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Up)]
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Down)]
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                vec![UiAction::CommandPaletteInput(
                    CommandPaletteInput::Backspace,
                )]
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Char(c))]
            }
            (KeyModifiers::NONE, KeyCode::Tab) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::Tab)]
            }
            (KeyModifiers::SHIFT, KeyCode::BackTab) => {
                vec![UiAction::CommandPaletteInput(CommandPaletteInput::BackTab)]
            }
            _ => vec![],
        }
    }

    fn decode_quick_switch(&self, key: &crossterm::event::KeyEvent) -> Vec<UiAction> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Esc) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Esc)]
            }
            (KeyModifiers::CONTROL, KeyCode::Char('x')) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Close)]
            }
            (KeyModifiers::NONE, KeyCode::Enter) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Enter)]
            }
            (KeyModifiers::NONE, KeyCode::Up) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Up)]
            }
            (KeyModifiers::NONE, KeyCode::Down) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Down)]
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) | (KeyModifiers::NONE, KeyCode::PageUp) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::PageUp)]
            }
            (KeyModifiers::CONTROL, KeyCode::Char('d'))
            | (KeyModifiers::NONE, KeyCode::PageDown) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::PageDown)]
            }
            (KeyModifiers::NONE, KeyCode::Home) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Home)]
            }
            (KeyModifiers::NONE, KeyCode::End) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::End)]
            }
            (KeyModifiers::NONE, KeyCode::Backspace) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Backspace)]
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) => {
                vec![UiAction::QuickSwitchInput(QuickSwitchInput::Char(c))]
            }
            _ => vec![],
        }
    }

    fn decode_overlay_input(&self, app: &App, key: &crossterm::event::KeyEvent) -> Vec<UiAction> {
        match (key.modifiers, key.code) {
            (KeyModifiers::NONE, KeyCode::Enter) if app.is_search_visible() => {
                vec![UiAction::SearchPerform]
            }
            (KeyModifiers::NONE, KeyCode::Esc) => vec![UiAction::Escape],
            (KeyModifiers::CONTROL, KeyCode::Char('f')) if app.is_search_visible() => {
                vec![UiAction::SearchPerform]
            }
            (KeyModifiers::NONE, KeyCode::Backspace) if app.is_search_visible() => {
                vec![UiAction::SearchQueryBackspace]
            }
            (KeyModifiers::CONTROL, KeyCode::Char('u')) if app.is_search_visible() => {
                vec![UiAction::SearchQueryClear]
            }
            (KeyModifiers::NONE, KeyCode::Char(c)) if app.is_search_visible() => {
                vec![UiAction::SearchQueryChar(c)]
            }
            (KeyModifiers::NONE, KeyCode::Char('h')) if app.is_http_options_visible() => {
                vec![UiAction::HttpOptionsClose]
            }
            // Help overlay scrolling
            (KeyModifiers::NONE, KeyCode::Up | KeyCode::Char('k')) if app.is_help_visible() => {
                vec![UiAction::HelpScrollUp]
            }
            (KeyModifiers::NONE, KeyCode::Down | KeyCode::Char('j')) if app.is_help_visible() => {
                vec![UiAction::HelpScrollDown]
            }
            (KeyModifiers::NONE, KeyCode::Char('g')) if app.is_help_visible() => {
                vec![UiAction::HelpScrollTop]
            }
            (KeyModifiers::NONE, KeyCode::Char('G')) if app.is_help_visible() => {
                vec![UiAction::HelpScrollBottom]
            }
            (KeyModifiers::NONE, KeyCode::PageUp) if app.is_help_visible() => {
                vec![UiAction::HelpScrollPageUp]
            }
            (KeyModifiers::NONE, KeyCode::PageDown) if app.is_help_visible() => {
                vec![UiAction::HelpScrollPageDown]
            }
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{create_test_app, PendingAction};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use eggsec::config::{OperationDescriptor, OperationMode, OperationRisk, PolicyDecision};

    #[test]
    fn test_ctrl_c_bubbles_through_overlays() {
        let app = create_test_app();
        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);

        // No overlay active — Ctrl+C bubbles (empty vec)
        let actions = OverlayController::new().decode(&app, &ctrl_c);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_ctrl_c_bubbles_through_help_overlay() {
        let mut app = create_test_app();
        app.overlay.show_help = true;
        assert!(app.topmost_overlay().is_some());

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let actions = OverlayController::new().decode(&app, &ctrl_c);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_ctrl_c_bubbles_through_policy_confirm() {
        let mut app = create_test_app();
        let desc = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = PolicyDecision::denied(
            "fuzz",
            OperationMode::StandardAssessment,
            OperationRisk::Intrusive,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk",
        );
        app.request_policy_confirmation(desc, decision, None);
        assert_eq!(
            app.topmost_overlay(),
            Some(OverlayType::PolicyConfirm)
        );

        let ctrl_c = KeyEvent::new(KeyCode::Char('c'), KeyModifiers::CONTROL);
        let actions = OverlayController::new().decode(&app, &ctrl_c);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_noop_when_no_overlay() {
        let app = create_test_app();
        assert!(app.topmost_overlay().is_none());

        // Any key with no overlay returns empty vec (no overlay to route to)
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert!(actions.is_empty());
    }

    #[test]
    fn test_unknown_key_returns_noop_when_overlay_active() {
        let mut app = create_test_app();
        app.overlay.show_help = true;

        // Ctrl+Z is unbound for Help overlay — should return Noop, not empty
        let key = KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::Noop]);
    }

    #[test]
    fn test_unknown_key_returns_noop_for_policy_confirm() {
        let mut app = create_test_app();
        let desc = OperationDescriptor {
            operation: "stress".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::StressTest,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://lab.example".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = PolicyDecision::denied(
            "stress",
            OperationMode::StandardAssessment,
            OperationRisk::StressTest,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk",
        );
        app.request_policy_confirmation(desc, decision, None);

        // Down arrow is unbound for PolicyConfirm — should return Noop
        let key = KeyEvent::new(KeyCode::Down, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::Noop]);
    }

    #[test]
    fn test_policy_confirm_enter_returns_confirm() {
        let mut app = create_test_app();
        let desc = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = PolicyDecision::denied(
            "fuzz",
            OperationMode::StandardAssessment,
            OperationRisk::Intrusive,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk",
        );
        app.request_policy_confirmation(desc, decision, None);

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::ConfirmPolicyAction]);
    }

    #[test]
    fn test_policy_confirm_esc_returns_cancel() {
        let mut app = create_test_app();
        let desc = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = PolicyDecision::denied(
            "fuzz",
            OperationMode::StandardAssessment,
            OperationRisk::Intrusive,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk",
        );
        app.request_policy_confirmation(desc, decision, None);

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::CancelPolicyAction]);
    }

    #[test]
    fn test_search_enter_performs_search() {
        let mut app = create_test_app();
        app.overlay.show_search = true;
        app.search.query = "needle".to_string();

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::SearchPerform]);
    }

    #[test]
    fn test_search_backspace_removes_char() {
        let mut app = create_test_app();
        app.overlay.show_search = true;
        app.search.query = "needle".to_string();

        let key = KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::SearchQueryBackspace]);
    }

    #[test]
    fn test_confirm_popup_enter_returns_confirm_pending() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);
        assert!(app.is_confirm_popup_visible());

        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::ConfirmPendingAction]);
    }

    #[test]
    fn test_confirm_popup_esc_returns_cancel_pending() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);

        let key = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::CancelPendingAction]);
    }

    #[test]
    fn test_confirm_popup_unknown_key_returns_noop() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);

        // Right arrow is unbound for ConfirmPopup
        let key = KeyEvent::new(KeyCode::Right, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::Noop]);
    }

    #[test]
    fn test_precedence_policy_confirm_over_help() {
        let mut app = create_test_app();
        // Both active — PolicyConfirm should win (higher precedence)
        app.overlay.show_help = true;
        let desc = OperationDescriptor {
            operation: "fuzz".to_string(),
            mode: OperationMode::StandardAssessment,
            risk: OperationRisk::Intrusive,
            intended_uses: vec![eggsec::config::IntendedUse::WebAssessment],
            target: Some("https://example.com".to_string()),
            required_features: vec![],
            required_policy_flags: vec![],
            requires_private_or_local_target: false,
            requires_explicit_scope: false,
            required_capabilities: vec![],
        };
        let decision = PolicyDecision::denied(
            "fuzz",
            OperationMode::StandardAssessment,
            OperationRisk::Intrusive,
            vec![eggsec::config::IntendedUse::WebAssessment],
            "high risk",
        );
        app.request_policy_confirmation(desc, decision, None);

        // Enter should route to PolicyConfirm, not Help
        let key = KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE);
        let actions = OverlayController::new().decode(&app, &key);
        assert_eq!(actions, vec![UiAction::ConfirmPolicyAction]);
    }
}
