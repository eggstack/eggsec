use crate::tc;
use ratatui::style::Style;
use ratatui::text::Span;
use ratatui::{
    layout::Rect,
    style::Color,
    text::Line,
    widgets::{Block, Borders, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState},
    Frame,
};

#[derive(Clone)]
pub struct ScrollableText {
    pub title: String,
    pub lines: Vec<Line<'static>>,
    pub scroll_offset: usize,
    pub horizontal_offset: usize,
    pub wrap: bool,
}

impl ScrollableText {
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            lines: Vec::new(),
            scroll_offset: 0,
            horizontal_offset: 0,
            wrap: true,
        }
    }

    pub fn with_lines(mut self, lines: Vec<Line<'static>>) -> Self {
        self.lines = lines;
        self
    }

    pub fn add_line(&mut self, line: Line<'static>) {
        self.lines.push(line);
    }

    pub fn add_text(&mut self, text: &str, style: Option<Style>) {
        let style = style.unwrap_or_default();
        self.lines
            .push(Line::from(Span::styled(text.to_string(), style)));
    }

    pub fn clear(&mut self) {
        self.lines.clear();
        self.scroll_offset = 0;
        self.horizontal_offset = 0;
    }

    pub fn scroll_up(&mut self, amount: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(amount);
    }

    pub fn scroll_down(&mut self, amount: usize) {
        if self.lines.is_empty() {
            self.scroll_offset = 0;
        } else {
            let max_scroll = self.lines.len().saturating_sub(1);
            self.scroll_offset = self.scroll_offset.saturating_add(amount).min(max_scroll);
        }
    }

    pub fn scroll_left(&mut self, amount: usize) {
        self.horizontal_offset = self.horizontal_offset.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: usize) {
        if self.lines.is_empty() {
            self.horizontal_offset = 0;
        } else {
            let max_offset = self.lines.iter().map(|l| l.width()).max().unwrap_or(0);
            self.horizontal_offset = self
                .horizontal_offset
                .saturating_add(amount)
                .min(max_offset);
        }
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        if self.lines.is_empty() {
            self.scroll_offset = 0;
        } else {
            self.scroll_offset = self.lines.len() - 1;
        }
    }

    pub fn scroll_to_end(&mut self) {
        self.scroll_to_bottom();
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.scroll_up(page_size);
    }

    pub fn page_down(&mut self, page_size: usize) {
        self.scroll_down(page_size);
    }

    pub fn is_at_left_edge(&self) -> bool {
        self.lines.is_empty() || self.horizontal_offset == 0
    }

    pub fn is_at_right_edge(&self) -> bool {
        if self.lines.is_empty() {
            self.horizontal_offset == 0
        } else {
            let max_offset = self.lines.iter().map(|l| l.width()).max().unwrap_or(0);
            self.horizontal_offset >= max_offset
        }
    }

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn get_content(&self) -> String {
        self.lines
            .iter()
            .map(|l| {
                l.spans
                    .iter()
                    .map(|s| s.content.to_string())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn render(&self, f: &mut Frame, area: Rect, border_color: Option<Color>) {
        if area.width < 3 || area.height < 3 {
            return;
        }

        let border_color = border_color.unwrap_or(tc!(border));

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let visible_height = area.height.saturating_sub(2) as usize;
        let scroll_offset = if self.lines.is_empty() {
            0
        } else {
            self.scroll_offset.min(self.lines.len() - 1)
        };

        let paragraph = Paragraph::new(self.lines.clone()).block(block).scroll((
            scroll_offset.min(u16::MAX as usize) as u16,
            self.horizontal_offset.min(u16::MAX as usize) as u16,
        ));
        f.render_widget(paragraph, area);

        if self.lines.len() > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("█")
                .thumb_style(Style::default().fg(tc!(accent)))
                .track_style(Style::default().fg(tc!(border)));

            let mut scrollbar_state = ScrollbarState::new(self.lines.len())
                .position(scroll_offset)
                .viewport_content_length(visible_height);

            let scrollbar_area = Rect {
                x: area.x + area.width - 1,
                y: area.y + 1,
                width: 1,
                height: area.height - 2,
            };

            f.render_stateful_widget(scrollbar, scrollbar_area, &mut scrollbar_state);
        }
    }
}
