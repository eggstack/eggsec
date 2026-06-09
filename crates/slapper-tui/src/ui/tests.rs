use super::draw;
use crate::app::{create_shared_history, App};
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
