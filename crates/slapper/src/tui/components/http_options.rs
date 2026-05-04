use ratatui::{
    layout::Rect,
    style::Style,
    widgets::{Block, BorderType, Borders, Clear, Paragraph},
    Frame,
};
use crate::tc;
use crate::tui::app::GlobalHttpOptions;
use super::centered_rect;

pub fn draw_http_options_popup(f: &mut Frame, area: Rect, opts: &GlobalHttpOptions) {
    let popup_width = 50;
    let popup_height = 18;

    let popup_area = centered_rect(popup_width, popup_height, area);

    f.render_widget(Clear, popup_area);

    let block = Block::default()
        .title("Global HTTP Options (press h to close)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(tc!(primary)));

    let inner = block.inner(popup_area);
    f.render_widget(block, popup_area);

    let redacted = |v: Option<&str>| {
        if v.is_some() {
            "********".to_string()
        } else {
            "(not set)".to_string()
        }
    };
    let content = vec![
        format!(
            "  --insecure: {}",
            if opts.insecure { "true" } else { "false" }
        ),
        format!(
            "  --proxy: {}",
            opts.proxy.as_deref().unwrap_or("(not set)")
        ),
        format!("  --proxy-auth: {}", redacted(opts.proxy_auth.as_deref())),
        format!("  --auth: {}", redacted(opts.auth.as_deref())),
        format!("  --bearer: {}", redacted(opts.bearer.as_deref())),
        format!("  --cookie: {}", redacted(opts.cookie.as_deref())),
        format!("  --api-key: {}", redacted(opts.api_key.as_deref())),
        format!(
            "  --user-agent: {}",
            opts.user_agent.as_deref().unwrap_or("(not set)")
        ),
        format!(
            "  --stealth: {}",
            if opts.stealth { "true" } else { "false" }
        ),
        format!(
            "  --rate-limit: {}",
            opts.rate_limit
                .map(|r| r.to_string())
                .unwrap_or("(not set)".to_string())
        ),
        format!(
            "  --jitter: {}",
            opts.jitter.as_deref().unwrap_or("(not set)")
        ),
    ];

    let paragraph = Paragraph::new(content.join("\n")).style(Style::default().fg(tc!(text)));
    f.render_widget(paragraph, inner);
}
