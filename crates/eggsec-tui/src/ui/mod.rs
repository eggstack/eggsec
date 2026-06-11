mod popups;
mod shell;

#[cfg(test)]
mod tests;

pub use popups::*;
pub use shell::*;

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    widgets::Paragraph,
    Frame,
};

use crate::components::{confirm_popup, help_popup_for_tab};
use crate::App;

pub const LAYOUT_MARGIN: u16 = 1;
pub const TAB_BAR_HEIGHT: u16 = 3;

pub fn draw(f: &mut Frame, app: &mut App) {
    let area = f.area();
    app.last_tab_area_width = area.width.saturating_sub(LAYOUT_MARGIN * 2);

    // Phase 9: very small terminal fallback. Render clear message, skip normal complex layout/popups.
    // Still allow policy confirm to render (clamped) for readability. Basic input (q/Esc/Ctrl-C) works via key path.
    // 80x24 good, 60x20 usable, <40x10 triggers this.
    if shell::is_terminal_too_small(area) {
        let theme = app.theme_manager.current().clone();
        let msg = "Terminal too small\nResize to at least 60x20 for full UI\n(q / Esc / Ctrl-C still work)";
        let p = Paragraph::new(msg)
            .style(Style::default().fg(theme.colors.text))
            .alignment(ratatui::layout::Alignment::Center);
        f.render_widget(p, area);

        // Still render policy confirm even on very small (preserve readability per plan).
        if let Some(ref pending) = app.overlay.pending_policy {
            let (title, message) = pending.message();
            let popup = confirm_popup(&title, &message);
            popup.render(f, area); // will be clamped inside centered_rect
        }
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(LAYOUT_MARGIN)
        .constraints([
            Constraint::Length(TAB_BAR_HEIGHT),
            Constraint::Length(1),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(f.area());

    let tab_area = chunks.first().copied().unwrap_or(area);
    let breadcrumb_area = chunks.get(1).copied().unwrap_or(area);
    let content_area = chunks.get(2).copied().unwrap_or(area);
    let status_area = chunks.get(3).copied().unwrap_or(area);

    let theme = app.theme_manager.current().clone();

    draw_tabs(f, app, &theme, tab_area);
    draw_breadcrumb(f, app, &theme, breadcrumb_area);
    draw_content(f, app, content_area);
    draw_status_bar(f, app, &theme, status_area);

    if app.overlay.show_help {
        let mut help = help_popup_for_tab(app.current_tab);
        help.scroll_offset = app.overlay.help_scroll_offset;
        help.render(f, f.area());

        let context_help = app.get_current_help();
        let context_chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(help.height + 2), Constraint::Min(0)])
            .split(f.area());

        let context_paragraph = Paragraph::new(context_help).style(
            Style::default()
                .fg(theme.colors.text_dim)
                .add_modifier(Modifier::ITALIC),
        );
        f.render_widget(
            context_paragraph,
            context_chunks.get(1).copied().unwrap_or(area),
        );
    }

    if let Some(ref mut palette) = app.command_palette {
        if palette.visible {
            draw_command_palette(f, app, &theme);
        }
    }

    if app.overlay.show_search {
        draw_search_popup(f, app, &theme);
    }

    if app.overlay.show_search && !app.search.query.is_empty() {
        if let Some(ref search) = app.search.global_search {
            if !search.is_empty() {
                crate::search::draw_search_results(f, app);
            }
        }
    }

    if app.overlay.show_http_options {
        draw_http_options_popup(f, app, &theme);
    }

    if app.quick_switch.visible {
        draw_quick_switch(f, app, &theme);
    }

    if let Some(action) = app.overlay.pending_action {
        let (title, message) = action.message();
        let popup = confirm_popup(&title, &message);
        popup.render(f, f.area());
    }

    // Policy confirmation (highest precedence) — rich confirm for RequireConfirmation + manual override.
    // Uses the same confirm_popup component; the message() already contains kebab classes, target, risk,
    // reasons/warnings, reason input line, and [Enter] Proceed / [Esc] Cancel hints (narrow semantics).
    if let Some(ref pending) = app.overlay.pending_policy {
        let (title, message) = pending.message();
        let popup = confirm_popup(&title, &message);
        popup.render(f, f.area());
    }
}
