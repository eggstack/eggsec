use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
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
        let max_scroll = self.lines.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + amount).min(max_scroll);
    }

    pub fn scroll_left(&mut self, amount: usize) {
        self.horizontal_offset = self.horizontal_offset.saturating_sub(amount);
    }

    pub fn scroll_right(&mut self, amount: usize) {
        self.horizontal_offset += amount;
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn scroll_to_bottom(&mut self) {
        let max_scroll = self.lines.len().saturating_sub(1);
        self.scroll_offset = max_scroll;
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

    pub fn len(&self) -> usize {
        self.lines.len()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        if area.width < 3 || area.height < 3 {
            return;
        }

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Gray));

        let visible_height = area.height.saturating_sub(2) as usize;
        let scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));
        let visible_lines: Vec<Line<'static>> = self
            .lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .cloned()
            .collect();

        let paragraph = Paragraph::new(visible_lines).block(block);
        f.render_widget(paragraph, area);

        if self.lines.len() > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");

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

    pub fn render_with_style(&self, f: &mut Frame, area: Rect, border_color: Color) {
        if area.width < 3 || area.height < 3 {
            return;
        }

        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(border_color));

        let visible_height = area.height.saturating_sub(2) as usize;
        let scroll_offset = self.scroll_offset.min(self.lines.len().saturating_sub(1));
        let visible_lines: Vec<Line<'static>> = self
            .lines
            .iter()
            .skip(scroll_offset)
            .take(visible_height)
            .cloned()
            .collect();

        let paragraph = Paragraph::new(visible_lines).block(block);
        f.render_widget(paragraph, area);

        if self.lines.len() > visible_height {
            let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓"))
                .track_symbol(Some("│"))
                .thumb_symbol("█");

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
