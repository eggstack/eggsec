use ratatui::{
    layout::Rect,
    style::{Color, Style},
    widgets::{Block, Borders, Gauge},
    Frame,
};

pub struct ProgressGauge {
    pub label: String,
    pub current: u64,
    pub total: u64,
    pub color: Color,
}

impl ProgressGauge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            current: 0,
            total: 100,
            color: Color::Blue,
        }
    }

    pub fn with_current(mut self, current: u64) -> Self {
        self.current = current;
        self
    }

    pub fn with_total(mut self, total: u64) -> Self {
        self.total = total;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.color = color;
        self
    }

    pub fn update(&mut self, current: u64) {
        self.current = current;
    }

    pub fn percent(&self) -> u16 {
        if self.total == 0 {
            return 0;
        }
        ((self.current as f64 / self.total as f64) * 100.0).min(100.0) as u16
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height < 3 {
            return;
        }
        let percent = self.percent();
        let label = format!(
            "{} - {}/{} ({}%)",
            self.label, self.current, self.total, percent
        );

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL).title("Progress"))
            .gauge_style(Style::default().fg(self.color))
            .percent(percent)
            .label(label);

        f.render_widget(gauge, area);
    }

    pub fn render_simple(&self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height < 3 {
            return;
        }
        let percent = self.percent();

        let gauge = Gauge::default()
            .block(Block::default().borders(Borders::ALL))
            .gauge_style(Style::default().fg(self.color))
            .percent(percent);

        f.render_widget(gauge, area);
    }
}

#[allow(dead_code)]
pub struct StatusBar {
    pub status: String,
    pub status_color: Color,
    pub help_text: String,
}

#[allow(dead_code)]
impl StatusBar {
    pub fn new() -> Self {
        Self {
            status: String::new(),
            status_color: Color::Gray,
            help_text: " [Tab] Next tab | [?] Help | [q] Quit ".to_string(),
        }
    }

    pub fn status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn color(mut self, color: Color) -> Self {
        self.status_color = color;
        self
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let chunks = ratatui::layout::Layout::default()
            .direction(ratatui::layout::Direction::Horizontal)
            .constraints([
                ratatui::layout::Constraint::Percentage(50),
                ratatui::layout::Constraint::Percentage(50),
            ])
            .split(area);

        let status = ratatui::widgets::Paragraph::new(self.status.as_str())
            .style(Style::default().fg(self.status_color));
        f.render_widget(status, chunks[0]);

        let help = ratatui::widgets::Paragraph::new(self.help_text.as_str())
            .style(Style::default().fg(Color::DarkGray));
        f.render_widget(help, chunks[1]);
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}
