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

    // ---------------------------------------------------------------------
    // Transition shims for any remaining direct call sites in KeyHandler.
    // These are intentionally not pub and are only for the old private fns
    // that now delegate. Phase 2 callers should prefer the main `decode`.
    // ---------------------------------------------------------------------

    pub(crate) fn decode_command_palette_for_shim(
        &self,
        key: &crossterm::event::KeyEvent,
    ) -> Vec<UiAction> {
        self.decode_command_palette(key)
    }

    pub(crate) fn decode_quick_switch_for_shim(
        &self,
        key: &crossterm::event::KeyEvent,
    ) -> Vec<UiAction> {
        self.decode_quick_switch(key)
    }

    pub(crate) fn decode_overlay_input_for_shim(
        &self,
        app: &App,
        key: &crossterm::event::KeyEvent,
    ) -> Vec<UiAction> {
        self.decode_overlay_input(app, key)
    }
}
