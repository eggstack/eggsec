#![allow(dead_code)]

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

#[derive(Debug, Clone, PartialEq)]
pub enum PopupKind {
    Info,
    Warning,
    Error,
    Confirm,
    Help,
}

impl PopupKind {
    pub fn default_color(&self) -> Color {
        match self {
            PopupKind::Info => Color::Cyan,
            PopupKind::Warning => Color::Yellow,
            PopupKind::Error => Color::Red,
            PopupKind::Confirm => Color::Magenta,
            PopupKind::Help => Color::Green,
        }
    }
}

pub struct Popup {
    pub title: String,
    pub content: Vec<String>,
    pub kind: PopupKind,
    pub width: u16,
    pub height: u16,
    pub active_button: usize,
    pub buttons: Vec<String>,
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
        }
    }

    pub fn content(mut self, content: Vec<String>) -> Self {
        self.height = (content.len() + 5).min(20).max(5) as u16;
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

    pub fn next_button(&mut self) {
        if !self.buttons.is_empty() {
            self.active_button = (self.active_button + 1) % self.buttons.len();
        }
    }

    pub fn prev_button(&mut self) {
        if !self.buttons.is_empty() {
            self.active_button = if self.active_button == 0 {
                self.buttons.len() - 1
            } else {
                self.active_button - 1
            };
        }
    }

    pub fn selected_button(&self) -> Option<&str> {
        self.buttons.get(self.active_button).map(|s| s.as_str())
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let popup_area = centered_rect(self.width, self.height, area);

        f.render_widget(Clear, popup_area);

        let color = self.kind.default_color();

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

        let paragraph = Paragraph::new(content_lines).wrap(Wrap { trim: true });
        f.render_widget(paragraph, chunks[0]);

        if !self.buttons.is_empty() {
            let button_area = chunks[1];
            let button_widths: Vec<u16> =
                self.buttons.iter().map(|b| (b.len() + 4) as u16).collect();
            let total_width: u16 = button_widths.iter().sum();
            let spacing = (button_area.width.saturating_sub(total_width))
                / (self.buttons.len().saturating_sub(1).max(1) as u16);

            let mut x_offset = button_area.x;
            for (i, (button, width)) in self.buttons.iter().zip(button_widths.iter()).enumerate() {
                let is_active = i == self.active_button;
                let style = if is_active {
                    Style::default()
                        .fg(Color::Black)
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

pub fn centered_rect(width: u16, height: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(r.height.saturating_sub(height) / 2),
            Constraint::Length(height),
            Constraint::Min(0),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(r.width.saturating_sub(width) / 2),
            Constraint::Length(width),
            Constraint::Min(0),
        ])
        .split(popup_layout[1])[1]
}

pub fn help_popup() -> Popup {
    Popup::new("Help - Keyboard Shortcuts", PopupKind::Help)
        .content(vec![
            "General:".to_string(),
            "  Tab/Shift+Tab    - Next/Previous tab".to_string(),
            "  h/l              - Previous/Next tab".to_string(),
            "  j/k              - Navigate down/up".to_string(),
            "  1-9              - Jump to tab by index".to_string(),
            "  Enter            - Edit field / Start task / Confirm".to_string(),
            "  Esc              - Blur input / Close popup".to_string(),
            "  Ctrl+C           - Stop running task".to_string(),
            "  q                - Quit (when idle)".to_string(),
            "  Space            - Show this help".to_string(),
            "  r                - Reset form to defaults".to_string(),
        ])
        .with_width(55)
        .with_height(18)
        .buttons(vec!["Close"])
}

pub fn help_popup_for_tab(tab: crate::tui::tabs::Tab) -> Popup {
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
        "  h/l              - Previous/Next tab".to_string(),
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
        match tab {
            crate::tui::tabs::Tab::Recon => "  Enter            - Start reconnaissance".to_string(),
            crate::tui::tabs::Tab::Load => "  Enter            - Start load test".to_string(),
            crate::tui::tabs::Tab::ScanPorts => "  Enter            - Start port scan".to_string(),
            crate::tui::tabs::Tab::ScanEndpoints => {
                "  Enter            - Start endpoint scan".to_string()
            }
            crate::tui::tabs::Tab::Fingerprint => {
                "  Enter            - Start service fingerprinting".to_string()
            }
            crate::tui::tabs::Tab::Fuzz => "  Enter            - Start fuzzing".to_string(),
            crate::tui::tabs::Tab::Waf => "  Enter            - Start WAF detection".to_string(),
            crate::tui::tabs::Tab::WafStress => {
                "  Enter            - Start WAF stress test".to_string()
            }
            crate::tui::tabs::Tab::Scan => "  Enter            - Start pipeline scan".to_string(),
            crate::tui::tabs::Tab::Resume => "  Enter            - Load session file".to_string(),
            crate::tui::tabs::Tab::Proxy => "  Enter            - Execute action".to_string(),
            crate::tui::tabs::Tab::Packet => "  Enter            - Run packet tool".to_string(),
            crate::tui::tabs::Tab::Settings => "  s               - Save settings".to_string(),
            crate::tui::tabs::Tab::History => "  Up/Down         - Navigate entries".to_string(),
            crate::tui::tabs::Tab::Dashboard => "  j/k             - Scroll dashboard".to_string(),
        },
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
