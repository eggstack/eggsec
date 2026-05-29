use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::tc;
use crate::tui::app::{Notification, NotificationSeverity};

pub fn draw_notifications(f: &mut Frame, area: Rect, notifications: &std::collections::VecDeque<Notification>) {
    if notifications.is_empty() {
        return;
    }

    // Show up to 3 notifications
    let visible_notifs: Vec<&Notification> = notifications.iter().take(3).collect();
    
    let popup_width = 40;
    let popup_height = 3;
    let spacing = 1;

    for (i, notif) in visible_notifs.iter().enumerate() {
        let (icon, color) = match notif.severity {
            NotificationSeverity::Info => ("ℹ", tc!(status_idle)),
            NotificationSeverity::Success => ("✔", tc!(success)),
            NotificationSeverity::Warning => ("⚠", tc!(warning)),
            NotificationSeverity::Error => ("✖", tc!(error)),
        };

        let y_offset = (i as u16) * (popup_height + spacing);
        
        // Position in bottom right, above status bar
        let notif_area = Rect {
            x: area.width.saturating_sub(popup_width + 2),
            y: area.height.saturating_sub(y_offset + popup_height + 2),
            width: popup_width,
            height: popup_height,
        };

        f.render_widget(Clear, notif_area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(color))
            .title(format!(" {} ", icon));

        let paragraph = Paragraph::new(notif.message.as_str())
            .block(block)
            .style(Style::default().fg(tc!(text)));

        f.render_widget(paragraph, notif_area);
    }
}
