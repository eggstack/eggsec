use crate::tc;
use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Gauge, Paragraph},
    Frame,
};

const SPINNER_FRAMES: &[&str] = &["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub struct ProgressGauge {
    pub label: String,
    pub current: u64,
    pub total: u64,
    pub color: Color,
    pub spinner_frame: usize,
}

impl ProgressGauge {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            current: 0,
            total: 100,
            color: tc!(secondary),
            spinner_frame: 0,
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
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
    }

    pub fn tick_spinner(&mut self) {
        self.spinner_frame = (self.spinner_frame + 1) % SPINNER_FRAMES.len();
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Progress")
                    .border_style(Style::default().fg(tc!(border))),
            )
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
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(tc!(border))),
            )
            .gauge_style(Style::default().fg(self.color))
            .percent(percent);

        f.render_widget(gauge, area);
    }

    pub fn render_indeterminate(&self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height < 3 {
            return;
        }
        let spinner = SPINNER_FRAMES.get(self.spinner_frame % SPINNER_FRAMES.len()).unwrap_or(&"?");
        let label = format!("{} {} - running...", spinner, self.label);

        let gauge = Gauge::default()
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title("Progress")
                    .border_style(Style::default().fg(tc!(border))),
            )
            .gauge_style(Style::default().fg(self.color))
            .percent(0)
            .label(label);

        f.render_widget(gauge, area);
    }

    pub fn render_status_line(&self, f: &mut Frame, area: Rect) {
        if area.width < 5 || area.height < 1 {
            return;
        }
        let spinner = SPINNER_FRAMES.get(self.spinner_frame % SPINNER_FRAMES.len()).unwrap_or(&"?");
        let text = if self.total > 0 {
            format!(
                "{} {} - {}/{} ({}%)",
                spinner,
                self.label,
                self.current,
                self.total,
                self.percent()
            )
        } else {
            format!("{} {} - running...", spinner, self.label)
        };

        let paragraph = Paragraph::new(Line::from(Span::styled(
            text,
            Style::default().fg(self.color),
        )));
        f.render_widget(paragraph, area);
    }
}
