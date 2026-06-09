use super::draw;
use super::shell::{get_normal_status, get_tab_status};
use crate::app::{create_shared_history, App};
use crate::tabs::AppState;
use crate::theme::Theme;
use ratatui::{backend::TestBackend, Terminal};

fn buffer_to_text(buf: &ratatui::buffer::Buffer) -> String {
    let mut out = String::new();
    for y in 0..buf.area.height {
        for x in 0..buf.area.width {
            out.push_str(buf[(x, y)].symbol());
        }
        out.push('\n');
    }
    out
}

#[test]
fn quick_switch_renders_selected_tail_item_in_viewport() {
    let mut app = App::new_for_testing(create_shared_history());
    app.quick_switch.visible = true;
    app.quick_switch.query.clear();
    app.quick_switch.selected = app.get_quick_switch_results().len().saturating_sub(1);

    let backend = TestBackend::new(80, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();

    let text = buffer_to_text(terminal.backend().buffer());
    assert!(
        text.contains("Dashboard - View scan results dashboard"),
        "Expected selected tail quick-switch item to be visible in rendered popup"
    );
}

#[test]
fn get_tab_status_returns_theme_colors() {
    let theme = Theme::default();

    let (msg, color) = get_tab_status(&AppState::Idle, &theme);
    assert_eq!(msg, "Ready - Press Enter to start");
    assert_eq!(color, theme.colors.status_idle);

    let (msg, color) = get_tab_status(&AppState::Running, &theme);
    assert_eq!(msg, "Running - Ctrl+C to stop");
    assert_eq!(color, theme.colors.status_running);

    let (msg, color) = get_tab_status(&AppState::Completed, &theme);
    assert_eq!(msg, "Completed");
    assert_eq!(color, theme.colors.success);

    let (msg, color) = get_tab_status(&AppState::Error("oops".to_string()), &theme);
    assert_eq!(msg, "oops");
    assert_eq!(color, theme.colors.error);
}

#[test]
fn get_normal_status_returns_theme_colors() {
    let app = App::new_for_testing(create_shared_history());
    let theme = Theme::default();

    let (msg, color) = get_normal_status(&app, &theme);
    assert_eq!(color, theme.colors.status_idle);
    assert!(!msg.is_empty());
}

#[test]
fn render_with_overlays_does_not_panic() {
    let mut app = App::new_for_testing(create_shared_history());
    app.quick_switch.visible = true;
    app.quick_switch.query.clear();
    app.quick_switch.selected = 0;

    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();

    let text = buffer_to_text(terminal.backend().buffer());
    assert!(text.contains("Slapper"), "Shell should render title");
}
