use ratatui::prelude::Stylize;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::cell::RefCell;
use crate::tc;

#[derive(Clone, Debug)]
pub struct DropdownInfo {
    pub area: Rect,
    pub items: Vec<(usize, String, bool)>,
    pub selected: usize,
    pub label: String,
    pub state: Option<ListState>,
}

impl DropdownInfo {
    pub fn render(&self, f: &mut Frame) {
        let fill: Vec<Line> = (0..self.area.height)
            .map(|_| Line::from(" ".repeat(self.area.width as usize)))
            .collect();
        let bg = Paragraph::new(fill)
            .style(Style::default().bg(tc!(surface)))
            .block(Block::default());
        f.render_widget(bg, self.area);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|(_, label, is_selected)| {
                let style = if *is_selected {
                    Style::default()
                        .fg(tc!(selected_text))
                        .bg(tc!(selected))
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(tc!(text)).bg(tc!(surface))
                };
                ListItem::new(label.as_str()).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(tc!(border_focused)))
                .bg(tc!(surface))
                .title(self.label.as_str()),
        );

        let mut state = self.state.unwrap_or_default();
        state.select(Some(self.selected));
        f.render_stateful_widget(list, self.area, &mut state);
    }
}

#[derive(Debug, Clone)]
pub struct SelectorItem {
    pub label: String,
    pub value: String,
}

impl SelectorItem {
    pub fn new(label: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: value.into(),
        }
    }

    pub fn simple(value: impl Into<String>) -> Self {
        let v = value.into();
        Self {
            label: v.clone(),
            value: v,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Selector {
    pub label: String,
    pub items: Vec<SelectorItem>,
    pub selected: usize,
    pub expanded: bool,
    pub focused: bool,
    pub dropdown_state: RefCell<Option<ListState>>,
}

impl Selector {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            items: Vec::new(),
            selected: 0,
            expanded: false,
            focused: false,
            dropdown_state: RefCell::new(None),
        }
    }

    pub fn items(mut self, items: Vec<SelectorItem>) -> Self {
        self.items = items;
        self
    }

    pub fn simple_items(mut self, items: Vec<&str>) -> Self {
        self.items = items.into_iter().map(SelectorItem::simple).collect();
        self
    }

    pub fn select(&mut self, idx: usize) {
        if idx < self.items.len() {
            self.selected = idx;
        }
    }

    pub fn select_by_value(&mut self, value: &str) {
        if let Some(idx) = self.items.iter().position(|i| i.value == value) {
            self.selected = idx;
        }
    }

    pub fn selected_item(&self) -> Option<&SelectorItem> {
        self.items.get(self.selected)
    }

    pub fn selected_value(&self) -> Option<&str> {
        self.items.get(self.selected).map(|i| i.value.as_str())
    }

    pub fn selected_label(&self) -> Option<&str> {
        self.items.get(self.selected).map(|i| i.label.as_str())
    }

    pub fn toggle(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn expand(&mut self) {
        self.expanded = true;
    }

    pub fn collapse(&mut self) {
        self.expanded = false;
        *self.dropdown_state.borrow_mut() = None;
    }

    pub fn dropdown_info(&self, anchor_area: Rect) -> Option<DropdownInfo> {
        if !self.expanded {
            return None;
        }

        if self.dropdown_state.borrow().is_none() {
            *self.dropdown_state.borrow_mut() = Some(ListState::default());
        }

        let dropdown_area = Rect {
            x: anchor_area.x,
            y: anchor_area.y + anchor_area.height,
            width: anchor_area.width,
            height: (self.items.len() + 2).min(10) as u16,
        };

        let items: Vec<(usize, String, bool)> = self
            .items
            .iter()
            .enumerate()
            .map(|(i, item)| (i, item.label.clone(), i == self.selected))
            .collect();

        Some(DropdownInfo {
            area: dropdown_area,
            items,
            selected: self.selected,
            label: self.label.clone(),
            state: *self.dropdown_state.borrow(),
        })
    }

    pub fn next(&mut self) {
        if self.expanded && !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    pub fn prev(&mut self) {
        if self.expanded && !self.items.is_empty() {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn is_focused(&self) -> bool {
        self.focused
    }

    pub fn focus(&mut self) {
        self.focused = true;
        self.expanded = true;
    }

    pub fn blur(&mut self) {
        self.focused = false;
        self.collapse();
    }

    pub fn focus_last(&mut self) {
        self.focused = true;
        if !self.items.is_empty() {
            self.selected = self.items.len() - 1;
        }
    }

    pub fn handle_enter(&mut self) {
        self.expanded = !self.expanded;
    }

    pub fn handle_up(&mut self) {
        if self.expanded {
            self.prev();
        }
    }

    pub fn handle_down(&mut self) {
        if self.expanded {
            self.next();
        }
    }

    pub fn handle_left(&mut self) {}
    pub fn handle_right(&mut self) {}

    pub fn handle_char(&mut self, _c: char) {}
    pub fn handle_backspace(&mut self) {}

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let style = if self.focused {
            Style::default().fg(tc!(border_focused))
        } else {
            Style::default().fg(tc!(border))
        };

        let block = Block::default()
            .title(self.label.as_str())
            .borders(Borders::ALL)
            .border_style(style);

        let selected_text = self
            .items
            .get(self.selected)
            .map(|i| i.label.as_str())
            .unwrap_or("-");

        let text = if self.expanded {
            format!("{} ▲", selected_text)
        } else {
            format!("{} ▼", selected_text)
        };

        let paragraph = Paragraph::new(text).block(block);
        f.render_widget(paragraph, area);
    }
}

#[derive(Debug, Clone)]
pub struct Checkbox {
    pub label: String,
    pub checked: bool,
    pub focused: bool,
}

impl Checkbox {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            checked: false,
            focused: false,
        }
    }

    pub fn checked(mut self, checked: bool) -> Self {
        self.checked = checked;
        self
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.render_with_focus(self.focused, f, area);
    }

    pub fn render_with_focus(&self, focused: bool, f: &mut Frame, area: Rect) {
        let style = if focused {
            Style::default().fg(tc!(border_focused))
        } else {
            Style::default().fg(tc!(border))
        };

        let check = if self.checked { "[✓]" } else { "[ ]" };
        let text = format!("{} {}", check, self.label);

        let paragraph = Paragraph::new(text).style(style);
        f.render_widget(paragraph, area);
    }
}

#[derive(Debug, Clone)]
pub struct RadioGroup {
    pub label: String,
    pub options: Vec<String>,
    pub selected: Option<usize>,
    pub focused: bool,
}

impl RadioGroup {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            options: Vec::new(),
            selected: None,
            focused: false,
        }
    }

    pub fn options(mut self, options: Vec<&str>) -> Self {
        self.options = options.into_iter().map(String::from).collect();
        if self.selected.is_none() && !self.options.is_empty() {
            self.selected = Some(0);
        }
        self
    }

    pub fn select(&mut self, idx: usize) {
        if idx < self.options.len() {
            self.selected = Some(idx);
        }
    }

    pub fn selected_option(&self) -> Option<&str> {
        self.selected
            .and_then(|i| self.options.get(i).map(|s| s.as_str()))
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let style = if self.focused {
            Style::default().fg(tc!(border_focused))
        } else {
            Style::default().fg(tc!(border))
        };

        let label_width = self.label.len() + 2;
        let options_per_line = ((area.width as usize).saturating_sub(label_width)) / 12;

        if options_per_line >= self.options.len() || options_per_line == 0 {
            let spans: Vec<Span> = self
                .options
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let is_selected = Some(i) == self.selected;
                    let radio = if is_selected { "◉" } else { "○" };
                    Span::styled(format!(" {} {}", radio, opt), style)
                })
                .collect();

            let line = Line::from(
                std::iter::once(Span::styled(format!("{}: ", self.label), style))
                    .chain(spans)
                    .collect::<Vec<_>>(),
            );

            let paragraph = Paragraph::new(line);
            f.render_widget(paragraph, area);
        } else {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(format!("{}: ", self.label), style)));

            for (chunk_idx, chunk) in self.options.chunks(options_per_line).enumerate() {
                let base_idx = chunk_idx * options_per_line;
                let spans: Vec<Span> = chunk
                    .iter()
                    .enumerate()
                    .map(|(j, opt)| {
                        let is_selected = Some(base_idx + j) == self.selected;
                        let radio = if is_selected { "◉" } else { "○" };
                        Span::styled(format!(" {} {}", radio, opt), style)
                    })
                    .collect();
                lines.push(Line::from(spans));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, area);
        }
    }
}
