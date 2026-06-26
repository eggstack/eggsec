//! UI Action layer (Phase 1 of tui-architecture-usability-pass.md).
//!
//! `UiAction` models *operator intent* only. There are no side effects,
//! I/O, or mutations performed during construction or pattern-matching
//! on a `UiAction`. All effects are centralized in `App::apply_action`
//! (and `apply_actions`).
//!
//! The `KeyHandler` decode path is responsible for translating a raw
//! `crossterm::event::KeyEvent` (plus read-only view of `App` for
//! guards such as active task, topmost overlay, input mode, and the
//! 1-char `pending_key` lookahead state machine for "gg") into zero
//! or more `UiAction`s plus an updated transient decode state for
//! `pending_key`.
//!
//! `App::apply_action` is now the single mutation site for all global
//! UI actions that originate from key handling. Existing mutation
//! methods on `App` (toggle_*, handle_*, confirm_*, etc.) are preserved
//! for compatibility with tests, command-palette execution, and other
//! call sites; `apply_action` may delegate to them during the Phase 1
//! transition.

use crate::tabs::Tab;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandPaletteInput {
    Char(char),
    Backspace,
    Enter,
    Up,
    Down,
    Tab,
    BackTab,
    Esc,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum QuickSwitchInput {
    Char(char),
    Backspace,
    Enter,
    Up,
    Down,
    PageUp,
    PageDown,
    Home,
    End,
    Esc,
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UiAction {
    Noop,

    // Session / task lifecycle
    Quit,
    StopActiveTask { message: String },

    // Overlay / mode toggles (global)
    ToggleHelp,
    ToggleCommandPalette,
    ToggleQuickSwitch,
    CloseQuickSwitch,
    ToggleSearch { global: bool },
    ToggleTheme,
    TogglePause,
    Resume,

    // Focus / navigation (may be routed overlay-local or to current tab)
    FocusNext,
    FocusPrev,
    PageUp,
    PageDown,
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    MoveTop,
    MoveBottom,

    // Normal-mode gg sequence (first g)
    BeginGgSequence,
    MoveWordForward,
    MoveWordBackward,
    Home,
    End,

    // Commit / cancel
    Enter,
    Escape,
    EnterInsertMode,

    // Insert-mode editing (routed to current tab dispatcher)
    InputChar(char),
    Backspace,
    Delete,

    // Autocomplete (Ctrl-Space in insert mode)
    Autocomplete,

    // Clipboard (apply performs the actual Clipboard I/O + dispatch)
    Paste(String),
    Copy,
    RequestPaste,
    RequestCopy,

    // Tab navigation
    SelectTab(Tab),
    NextTab,
    PrevTab,

    // Bookmarks + side-effect notification (notification created in apply)
    ToggleBookmark(Tab),

    // Export
    CycleExportFormat,
    ExportResults,

    // Destructive / confirmation-gated actions (the *request* forms show popup)
    ResetCurrent,
    ReloadThemes,
    SaveSettings,
    DeleteHistoryEntry,

    // Pending-action confirm/cancel (ConfirmPopup)
    ConfirmPendingAction,
    CancelPendingAction,
    ConfirmButtonToggle,

    // Policy enforcement confirm/cancel (PolicyConfirm) + reason editing
    ConfirmPolicyAction,
    CancelPolicyAction,
    PolicyReasonChar(char),
    PolicyReasonBackspace,

    // Overlay-specific incremental input (command palette / quick switch)
    CommandPaletteInput(CommandPaletteInput),
    QuickSwitchInput(QuickSwitchInput),

    // Search overlay (when Search is the topmost overlay)
    SearchQueryChar(char),
    SearchQueryBackspace,
    SearchQueryClear,
    SearchPerform,

    // Help overlay scrolling (when Help is topmost)
    HelpScrollUp,
    HelpScrollDown,
    HelpScrollTop,
    HelpScrollBottom,
    HelpScrollPageUp,
    HelpScrollPageDown,

    // HTTP options close affordance
    HttpOptionsClose,
}
