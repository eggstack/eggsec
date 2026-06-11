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
    assert!(text.contains("Eggsec"), "Shell should render title");
}

// Phase 5: advisory preflight indicators in status for target-bearing tabs (via TabSpec + enforcement).
// Simple contains checks (no snapshot requirement). Uses default test app (Recon, empty target).
#[test]
fn get_normal_status_for_target_tab_surfaces_mode_and_scope() {
    let app = App::new_for_testing(create_shared_history());
    let theme = Theme::default();

    let (msg, color) = get_normal_status(&app, &theme);
    // Default Recon tab declares an operation in its TabSpec -> preflight path taken.
    assert!(
        msg.contains("Mode:") || msg.contains("manual"),
        "expected enforcement mode indicator in status for target tab, got: {}",
        msg
    );
    assert!(
        msg.contains("Scope:") || msg.contains("default") || msg.contains("Scope"),
        "expected scope provenance indicator in status, got: {}",
        msg
    );
    // Empty-target safe case on permissive profile keeps idle color (preserves prior test contract).
    assert_eq!(color, theme.colors.status_idle);
}

#[test]
fn render_status_bar_contains_preflight_indicators() {
    let mut app = App::new_for_testing(create_shared_history());
    // Start on a target-bearing tab (Recon has operation + primary_target delegation).
    app.current_tab = crate::tabs::Tab::Recon;

    let backend = TestBackend::new(100, 24);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();

    let text = buffer_to_text(terminal.backend().buffer());
    // Status bar (bottom row) should include the Phase 5 indicators (mode/scope/risk).
    // These are advisory and computed live from spec + enforcement + loaded_scope.
    assert!(
        text.contains("Mode:") || text.contains("manual") || text.contains("Scope:"),
        "status bar should surface manual mode / scope provenance for target-bearing tab"
    );
}

// Phase 9 small-terminal tests: 60x20 usable (nav/runs), 40x12 too-small shows fallback, 120x40 unchanged.
// Use buffer text checks for "too small" or breadcrumb tab mode. No panic on render.
#[test]
fn render_at_60x20_is_usable_no_garble() {
    let mut app = App::new_for_testing(create_shared_history());
    let backend = TestBackend::new(60, 20);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();
    let text = buffer_to_text(terminal.backend().buffer());
    assert!(
        text.contains("Eggsec"),
        "60x20 should still render core shell"
    );
    // Not the too-small fallback
    assert!(!text.contains("Terminal too small"));
}

#[test]
fn render_at_40x12_shows_too_small_fallback() {
    let mut app = App::new_for_testing(create_shared_history());
    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();
    let text = buffer_to_text(terminal.backend().buffer());
    assert!(
        text.contains("Terminal too small") || text.contains("Resize to at least 60x20"),
        "very small terminal must render clear fallback, not garbled UI"
    );
}

#[test]
fn render_at_120x40_unchanged_from_large() {
    let mut app = App::new_for_testing(create_shared_history());
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();
    let text = buffer_to_text(terminal.backend().buffer());
    assert!(text.contains("Eggsec"), "large viewport renders normally");
    // No too-small path taken
    assert!(!text.contains("Terminal too small"));
}

#[test]
fn render_policy_confirm_on_small_terminal_still_readable() {
    let mut app = App::new_for_testing(create_shared_history());
    // Force a pending policy confirm (simulates RequireConfirmation path; message() produces readable lines).
    use crate::app::confirmation::PendingPolicyConfirmation;
    use eggsec::config::{
        IntendedUse, OperationDescriptor, OperationMode, OperationRisk, PolicyDecision,
    };
    let desc = OperationDescriptor {
        operation: "test-op".to_string(),
        mode: OperationMode::StandardAssessment,
        risk: OperationRisk::SafeActive,
        intended_uses: vec![IntendedUse::WebAssessment],
        target: Some("example.com".to_string()),
        required_features: vec![],
        required_policy_flags: vec![],
        requires_private_or_local_target: false,
        requires_explicit_scope: false,
        required_capabilities: vec![],
    };
    // Use allowed() constructor (no Default); the pending path still renders the popup content via message().
    let decision = PolicyDecision::allowed(
        "test-op",
        OperationMode::StandardAssessment,
        OperationRisk::SafeActive,
        vec![IntendedUse::WebAssessment],
    );
    app.overlay.pending_policy = Some(PendingPolicyConfirmation {
        descriptor: desc,
        decision,
        required_classes: vec![],
        reason_input: String::new(),
        captured_task_config: None,
    });

    // Very small viewport
    let backend = TestBackend::new(40, 12);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, &mut app)).unwrap();
    let text = buffer_to_text(terminal.backend().buffer());
    // Policy confirm path still taken (even in too-small guard); confirm_popup uses clamped centered_rect.
    // Message contains "Policy Confirmation" or operation/target lines.
    assert!(
        text.contains("Policy Confirmation")
            || text.contains("test-op")
            || text.contains("example.com"),
        "policy confirm must still render readably on small terminal (clamped)"
    );
}
