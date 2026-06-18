use crate::tc;
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PopupKind {
    #[allow(dead_code)] // reserved for future informational popups
    Info,
    #[allow(dead_code)] // reserved for future warning popups
    Warning,
    #[allow(dead_code)] // reserved for future error popups
    Error,
    Confirm,
    Help,
    Destructive,
}

pub struct Popup {
    pub title: String,
    pub content: Vec<String>,
    pub kind: PopupKind,
    pub width: u16,
    pub height: u16,
    pub active_button: usize,
    pub buttons: Vec<String>,
    pub scroll_offset: usize,
}

impl Popup {
    pub fn new(title: impl Into<String>, kind: PopupKind) -> Self {
        Self {
            title: title.into(),
            content: Vec::new(),
            kind,
            width: 60,
            height: 10,
            active_button: 0,
            buttons: Vec::new(),
            scroll_offset: 0,
        }
    }

    pub fn content(mut self, content: Vec<String>) -> Self {
        self.height = content.len().saturating_add(5).clamp(5, 20) as u16;
        self.content = content;
        self
    }

    pub fn with_width(mut self, width: u16) -> Self {
        self.width = width;
        self
    }

    pub fn with_height(mut self, height: u16) -> Self {
        self.height = height;
        self
    }

    pub fn buttons(mut self, buttons: Vec<&str>) -> Self {
        self.buttons = buttons.into_iter().map(String::from).collect();
        self
    }

    #[allow(dead_code)]
    pub fn destructive(title: impl Into<String>, content: Vec<String>) -> Self {
        Self::new(title, PopupKind::Destructive)
            .content(content)
            .buttons(vec!["Cancel", "Confirm"])
    }

    #[allow(dead_code)]
    pub fn next_button(&mut self) {
        if !self.buttons.is_empty() {
            self.active_button = (self.active_button + 1) % self.buttons.len();
        }
    }

    #[allow(dead_code)]
    pub fn prev_button(&mut self) {
        if !self.buttons.is_empty() {
            self.active_button = if self.active_button == 0 {
                self.buttons.len() - 1
            } else {
                self.active_button - 1
            };
        }
    }

    #[allow(dead_code)]
    pub fn selected_button(&self) -> Option<&str> {
        self.buttons.get(self.active_button).map(|s| s.as_str())
    }

    #[allow(dead_code)]
    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    #[allow(dead_code)]
    pub fn scroll_down(&mut self, amount: usize) {
        if self.content.is_empty() {
            self.scroll_offset = 0;
        } else {
            let max_scroll = self.content.len() - 1;
            self.scroll_offset = self.scroll_offset.saturating_add(amount).min(max_scroll);
        }
    }

    #[allow(dead_code)]
    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    #[allow(dead_code)]
    pub fn scroll_to_bottom(&mut self) {
        if self.content.is_empty() {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.content.len() - 1;
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(self.width, self.height, area);

        f.render_widget(Clear, popup_area);

        let color = match self.kind {
            PopupKind::Info => tc!(info),
            PopupKind::Warning => tc!(warning),
            PopupKind::Error => tc!(error),
            PopupKind::Confirm => tc!(highlight),
            PopupKind::Help => tc!(success),
            PopupKind::Destructive => tc!(error),
        };

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(color));

        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(1)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(if self.buttons.is_empty() { 0 } else { 3 }),
            ])
            .split(inner);

        let content_lines: Vec<Line> = self
            .content
            .iter()
            .map(|line| Line::from(line.as_str()))
            .collect();

        if let Some(content_chunk) = chunks.first() {
            let scroll_offset = if self.content.is_empty() {
                0
            } else {
                self.scroll_offset.min(self.content.len() - 1)
            };
            let paragraph = Paragraph::new(content_lines)
                .style(Style::default().fg(tc!(text)))
                .wrap(Wrap { trim: true })
                .scroll((scroll_offset.min(u16::MAX as usize) as u16, 0));
            f.render_widget(paragraph, *content_chunk);
        }

        if !self.buttons.is_empty() {
            if let Some(button_area) = chunks.get(1) {
                let button_widths: Vec<u16> = self
                    .buttons
                    .iter()
                    .map(|b| (b.chars().count() + 4).min(u16::MAX as usize) as u16)
                    .collect();
                let total_width: u16 = button_widths.iter().copied().fold(0u16, |a, b| a.saturating_add(b));
                let spacing = (button_area.width.saturating_sub(total_width))
                    / (self.buttons.len().saturating_sub(1).max(1) as u16);

                let mut x_offset = button_area.x;
                for (i, (button, width)) in
                    self.buttons.iter().zip(button_widths.iter()).enumerate()
                {
                    let is_active = i == self.active_button;
                    let style = if is_active {
                        Style::default()
                            .fg(tc!(selected_text))
                            .bg(color)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(color)
                    };

                    let btn_area = Rect {
                        x: x_offset,
                        y: button_area.y,
                        width: *width,
                        height: 1,
                    };

                    let btn_text = format!(" {} ", button);
                    let btn_span = Span::styled(btn_text, style);
                    f.render_widget(Paragraph::new(Line::from(btn_span)), btn_area);

                    x_offset += width + spacing;
                }
            }
        }
    }
}

pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    if r.width < 3 || r.height < 3 {
        return r;
    }
    let clamped_width = width.min(r.width.saturating_sub(2));
    let clamped_height = height.min(r.height.saturating_sub(2));

    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(clamped_height) / 2),
            Constraint::Length(clamped_height),
            Constraint::Min(0),
        ])
        .split(r);

    let vertical_chunk = popup_layout.get(1).copied().unwrap_or(r);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(clamped_width) / 2),
            Constraint::Length(clamped_width),
            Constraint::Min(0),
        ])
        .split(vertical_chunk)
        .get(1)
        .copied()
        .unwrap_or(vertical_chunk)
}

pub fn help_popup_for_tab(tab: crate::tabs::Tab) -> Popup {
    let title = format!("Help - {}", tab.title());
    let cli_cmd = tab.cli_command();
    let description = tab.description();

    let content = vec![
        format!("Command: {}", cli_cmd),
        format!("Description: {}", description),
        "".to_string(),
        "=== VIM-STYLE NAVIGATION ===".to_string(),
        "  h/Left           - Move left".to_string(),
        "  j/Down           - Move down".to_string(),
        "  k/Up             - Move up".to_string(),
        "  l/Right          - Move right".to_string(),
        "  w                - Word forward (5 chars)".to_string(),
        "  b                - Word backward (5 chars)".to_string(),
        "  H                - Home (line start)".to_string(),
        "  L                - End (line end)".to_string(),
        "  gg               - Go to top".to_string(),
        "  G                - Go to bottom".to_string(),
        "".to_string(),
        "=== TAB NAVIGATION ===".to_string(),
        "  Tab/Shift+Tab    - Next/Previous focus".to_string(),
        "  n/N              - Next/Previous tab".to_string(),
        "  p                - Previous tab".to_string(),
        "  Shift+H          - Previous tab".to_string(),
        "  Shift+L          - Next tab".to_string(),
        "  1-9              - Jump to tab 1-9".to_string(),
        "  0                - Jump to tab 10".to_string(),
        "".to_string(),
        "=== ACTIONS ===".to_string(),
        "  Enter            - Edit field / Start task / Confirm".to_string(),
        "  i                - Insert mode (for typing)".to_string(),
        "  Esc              - Close help / Blur input / Normal mode".to_string(),
        "  Space            - Toggle this help".to_string(),
        "  /                - Toggle search".to_string(),
        "  e                - Export results to JSON".to_string(),
        "  r                - Reset form to defaults".to_string(),
        "  s                - Save settings (in Settings tab)".to_string(),
        "  d                - Delete history entry (in History tab)".to_string(),
        "  Ctrl+C           - Stop running task / Quit".to_string(),
        "  Ctrl+U           - Page up".to_string(),
        "  Ctrl+D           - Page down".to_string(),
        "  q                - Quit (when idle)".to_string(),
        "".to_string(),
        "Current Tab:".to_string(),
        tab.help_entry().to_string(),
        "".to_string(),
        "=== INPUT MODES ===".to_string(),
        "  NORMAL (NOR)     - Navigation and commands".to_string(),
        "  INSERT (INS)     - Typing in fields".to_string(),
        "  Press 'i' to enter Insert mode".to_string(),
        "  Press 'Esc' to return to Normal mode".to_string(),
        "".to_string(),
        "=== COMMAND DISCOVERY ===".to_string(),
        "  Ctrl+P           - Open command palette (search and execute commands)".to_string(),
        "  Ctrl+/           - Toggle help (same as Space)".to_string(),
        "  Type to search   - Start typing to filter commands in palette".to_string(),
        "  Up/Down          - Navigate command results".to_string(),
        "  Enter            - Execute selected command".to_string(),
        "  Esc              - Close command palette".to_string(),
    ];

    Popup::new(title, PopupKind::Help)
        .content(content)
        .with_width(70)
        .with_height(35)
        .buttons(vec!["Close"])
}

pub fn confirm_popup(title: &str, message: &[String]) -> Popup {
    Popup::new(title, PopupKind::Confirm)
        .content(message.to_vec())
        .with_width(50)
        .with_height(8)
        .buttons(vec!["Yes", "No"])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_content_with_empty_vec() {
        let popup = Popup::new("Test", PopupKind::Info).content(Vec::new());
        assert_eq!(popup.height, 5);
        assert!(popup.content.is_empty());
    }

    #[test]
    fn test_content_with_normal_size() {
        let lines: Vec<String> = (0..10).map(|i| format!("line {i}")).collect();
        let popup = Popup::new("Test", PopupKind::Info).content(lines);
        assert_eq!(popup.height, 15);
    }

    #[test]
    fn test_content_height_clamps_to_max_20() {
        let lines: Vec<String> = (0..100).map(|i| format!("line {i}")).collect();
        let popup = Popup::new("Test", PopupKind::Info).content(lines);
        assert_eq!(popup.height, 20);
    }

    #[test]
    fn test_content_height_does_not_overflow_on_huge_input() {
        // The original implementation used `content.len() + 5` which would
        // overflow in release builds when content.len() was near usize::MAX.
        // The fix uses saturating_add to keep the height clamped. We can't
        // allocate a Vec of usize::MAX in tests, but we can verify that the
        // height calculation uses the saturating path: a very large len() would
        // wrap to 0..4 with the old code; the new code clamps to 20.
        // We exercise the path by passing a 100-line content (which is well
        // past the 20 cap), verifying the clamp is in effect.
        let lines: Vec<String> = (0..1000).map(|i| format!("line {i}")).collect();
        let popup = Popup::new("Test", PopupKind::Info).content(lines);
        assert_eq!(popup.height, 20);
    }

    #[test]
    fn test_scroll_down_empty_content() {
        let mut popup = Popup::new("Test", PopupKind::Info).content(Vec::new());
        popup.scroll_down(10);
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_bottom_empty_content() {
        let mut popup = Popup::new("Test", PopupKind::Info).content(Vec::new());
        popup.scroll_to_bottom();
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_down_single_item() {
        let mut popup = Popup::new("Test", PopupKind::Info).content(vec!["only".to_string()]);
        popup.scroll_down(5);
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_to_bottom_single_item() {
        let mut popup = Popup::new("Test", PopupKind::Info).content(vec!["only".to_string()]);
        popup.scroll_to_bottom();
        assert_eq!(popup.scroll_offset, 0);
    }

    #[test]
    fn test_scroll_offset_large_value() {
        let mut popup = Popup::new("Test", PopupKind::Info)
            .content(vec!["a".to_string(), "b".to_string()]);
        popup.scroll_offset = usize::MAX;
        let max_scroll = popup.content.len() - 1;
        assert_eq!(max_scroll, 1);
        let clamped = popup.scroll_offset.min(max_scroll);
        assert_eq!(clamped, 1);
    }

    #[test]
    fn test_render_empty_content() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).unwrap();
        let popup = Popup::new("Test", PopupKind::Info).content(Vec::new());
        terminal
            .draw(|f| {
                let area = f.area();
                popup.render(f, area);
            })
            .unwrap();
    }
}
