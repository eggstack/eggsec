use crate::app::App;
use crate::app::InputMode;
use crate::app::OverlayType;
use crate::tabs::Tab;

/// A single action hint displayed in the status bar.
pub struct ActionHint {
    pub key: &'static str,
    pub label: &'static str,
}

/// Get context-aware action hints based on current app state.
///
/// Priority order:
/// 1. Running task hints
/// 2. Overlay-specific hints
/// 3. Insert-mode (input focused) hints
/// 4. Tab-specific normal-mode hints
pub fn get_action_hints(app: &App) -> Vec<ActionHint> {
    if app.task_state.handle.is_some() {
        return task_hints(app);
    }

    match app.topmost_overlay() {
        Some(OverlayType::PolicyConfirm) => return policy_confirm_hints(),
        Some(OverlayType::ConfirmPopup) => return confirm_popup_hints(),
        Some(OverlayType::CommandPalette) => return command_palette_hints(),
        Some(OverlayType::QuickSwitch) => return quick_switch_hints(),
        Some(OverlayType::Search) => return search_hints(),
        Some(OverlayType::Help) => return help_hints(),
        Some(OverlayType::HttpOptions) => return http_options_hints(),
        None => {}
    }

    if app.mode == InputMode::Insert {
        return insert_mode_hints();
    }

    get_tab_hints(app)
}

fn task_hints(app: &App) -> Vec<ActionHint> {
    if app.is_paused() {
        vec![
            ActionHint { key: "C", label: "stop" },
            ActionHint { key: "Y", label: "resume" },
        ]
    } else {
        vec![
            ActionHint { key: "C", label: "stop" },
            ActionHint { key: "Z", label: "pause" },
        ]
    }
}

fn policy_confirm_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Enter", label: "confirm" },
        ActionHint { key: "Esc", label: "cancel" },
    ]
}

fn confirm_popup_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "y", label: "yes" },
        ActionHint { key: "n", label: "no" },
    ]
}

fn command_palette_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Enter", label: "run" },
        ActionHint { key: "↑↓", label: "select" },
        ActionHint { key: "Esc", label: "close" },
    ]
}

fn quick_switch_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Enter", label: "go" },
        ActionHint { key: "↑↓", label: "select" },
        ActionHint { key: "Esc", label: "close" },
    ]
}

fn search_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Enter", label: "search" },
        ActionHint { key: "Bksp", label: "edit" },
        ActionHint { key: "Esc", label: "close" },
    ]
}

fn help_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Esc", label: "close" },
        ActionHint { key: "h/l", label: "pane" },
        ActionHint { key: "j/k", label: "scroll" },
    ]
}

fn http_options_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "h", label: "close" },
    ]
}

fn insert_mode_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Esc", label: "normal" },
        ActionHint { key: "Tab", label: "next" },
        ActionHint { key: "Enter", label: "confirm" },
    ]
}

fn get_tab_hints(app: &App) -> Vec<ActionHint> {
    match app.current_tab {
        Tab::Settings => settings_hints(),
        Tab::History => history_hints(),
        Tab::Dashboard => dashboard_hints(),
        _ => default_normal_hints(app),
    }
}

fn settings_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "s", label: "save" },
        ActionHint { key: "r", label: "reset" },
        ActionHint { key: "Tab", label: "next" },
    ]
}

fn history_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "↑↓", label: "nav" },
        ActionHint { key: "d", label: "delete" },
        ActionHint { key: "r", label: "clear" },
    ]
}

fn dashboard_hints() -> Vec<ActionHint> {
    vec![
        ActionHint { key: "Enter", label: "open" },
        ActionHint { key: "n/p", label: "tabs" },
    ]
}

fn default_normal_hints(app: &App) -> Vec<ActionHint> {
    let has_target = app
        .current_tab_target()
        .map(|t| !t.is_empty())
        .unwrap_or(false);

    if has_target {
        vec![
            ActionHint { key: "Enter", label: "run" },
            ActionHint { key: "n/p", label: "tabs" },
            ActionHint { key: "/", label: "search" },
        ]
    } else {
        vec![
            ActionHint { key: "Enter", label: "focus" },
            ActionHint { key: "n/p", label: "tabs" },
            ActionHint { key: "/", label: "search" },
        ]
    }
}

/// Format action hints into a compact string for the status bar.
/// E.g. "C:stop Z:pause" or "Enter:run n/p:tabs"
pub fn format_hints(hints: &[ActionHint]) -> String {
    hints
        .iter()
        .map(|h| format!("{}:{}", h.key, h.label))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{create_shared_history, App, PendingAction};
    use crate::tabs::Tab;
    use crate::tabs::AppState;

    fn create_test_app() -> App {
        App::new_for_testing(create_shared_history())
    }

    #[tokio::test]
    async fn task_running_hints() {
        let mut app = create_test_app();
        app.task_state.handle = Some(tokio::spawn(async {}));
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].key, "C");
        assert_eq!(hints[0].label, "stop");
        assert_eq!(hints[1].key, "Z");
        assert_eq!(hints[1].label, "pause");
    }

    #[tokio::test]
    async fn task_paused_hints() {
        let mut app = create_test_app();
        app.task_state.handle = Some(tokio::spawn(async {}));
        app.task_state.paused = true;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].key, "C");
        assert_eq!(hints[0].label, "stop");
        assert_eq!(hints[1].key, "Y");
        assert_eq!(hints[1].label, "resume");
    }

    #[test]
    fn policy_confirm_overlay_hints() {
        use eggsec::config::{
            OperationDescriptor, OperationMode, OperationRisk, PolicyDecision,
        };

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
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].key, "Enter");
        assert_eq!(hints[0].label, "confirm");
        assert_eq!(hints[1].key, "Esc");
        assert_eq!(hints[1].label, "cancel");
    }

    #[test]
    fn confirm_popup_overlay_hints() {
        let mut app = create_test_app();
        app.request_confirmation(PendingAction::ResetTab);
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].key, "y");
        assert_eq!(hints[0].label, "yes");
        assert_eq!(hints[1].key, "n");
        assert_eq!(hints[1].label, "no");
    }

    #[test]
    fn command_palette_overlay_hints() {
        let mut app = create_test_app();
        app.toggle_command_palette();
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "Enter");
        assert_eq!(hints[1].key, "↑↓");
        assert_eq!(hints[2].key, "Esc");
    }

    #[test]
    fn search_overlay_hints() {
        let mut app = create_test_app();
        app.overlay.show_search = true;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "Enter");
        assert_eq!(hints[0].label, "search");
        assert_eq!(hints[1].key, "Bksp");
        assert_eq!(hints[2].key, "Esc");
    }

    #[test]
    fn help_overlay_hints() {
        let mut app = create_test_app();
        app.overlay.show_help = true;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "Esc");
        assert_eq!(hints[0].label, "close");
        assert_eq!(hints[1].key, "h/l");
        assert_eq!(hints[1].label, "pane");
        assert_eq!(hints[2].key, "j/k");
        assert_eq!(hints[2].label, "scroll");
    }

    #[test]
    fn insert_mode_hints() {
        let mut app = create_test_app();
        app.mode = InputMode::Insert;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "Esc");
        assert_eq!(hints[0].label, "normal");
        assert_eq!(hints[1].key, "Tab");
        assert_eq!(hints[1].label, "next");
        assert_eq!(hints[2].key, "Enter");
        assert_eq!(hints[2].label, "confirm");
    }

    #[test]
    fn settings_tab_hints() {
        let mut app = create_test_app();
        app.current_tab = Tab::Settings;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "s");
        assert_eq!(hints[0].label, "save");
        assert_eq!(hints[1].key, "r");
        assert_eq!(hints[1].label, "reset");
        assert_eq!(hints[2].key, "Tab");
        assert_eq!(hints[2].label, "next");
    }

    #[test]
    fn history_tab_hints() {
        let mut app = create_test_app();
        app.current_tab = Tab::History;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "↑↓");
        assert_eq!(hints[0].label, "nav");
        assert_eq!(hints[1].key, "d");
        assert_eq!(hints[1].label, "delete");
        assert_eq!(hints[2].key, "r");
        assert_eq!(hints[2].label, "clear");
    }

    #[test]
    fn dashboard_tab_hints() {
        let mut app = create_test_app();
        app.current_tab = Tab::Dashboard;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 2);
        assert_eq!(hints[0].key, "Enter");
        assert_eq!(hints[0].label, "open");
        assert_eq!(hints[1].key, "n/p");
        assert_eq!(hints[1].label, "tabs");
    }

    #[test]
    fn recon_tab_default_hints_without_target() {
        let mut app = create_test_app();
        app.current_tab = Tab::Recon;
        let hints = get_action_hints(&app);
        assert_eq!(hints.len(), 3);
        assert_eq!(hints[0].key, "Enter");
        assert_eq!(hints[0].label, "focus");
    }

    #[test]
    fn format_hints_compact() {
        let hints = vec![
            ActionHint { key: "C", label: "stop" },
            ActionHint { key: "Z", label: "pause" },
        ];
        assert_eq!(format_hints(&hints), "C:stop Z:pause");
    }

    #[test]
    fn format_hints_empty() {
        let hints: Vec<ActionHint> = vec![];
        assert_eq!(format_hints(&hints), "");
    }

    #[test]
    fn overlay_overrides_tab_hints() {
        let mut app = create_test_app();
        app.current_tab = Tab::Settings;
        app.overlay.show_help = true;
        let hints = get_action_hints(&app);
        assert_eq!(hints[0].key, "Esc");
        assert_eq!(hints[0].label, "close");
    }

    #[tokio::test]
    async fn task_overrides_overlay_hints() {
        let mut app = create_test_app();
        app.task_state.handle = Some(tokio::spawn(async {}));
        app.overlay.show_help = true;
        let hints = get_action_hints(&app);
        assert_eq!(hints[0].key, "C");
        assert_eq!(hints[0].label, "stop");
    }
}
