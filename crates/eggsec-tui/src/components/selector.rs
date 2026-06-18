use ratatui::prelude::Stylize;
use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph},
    Frame,
};
use std::cell::RefCell;

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
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(f, &theme);
    }

    pub fn render_with_theme(&self, f: &mut Frame, theme: &crate::theme::Theme) {
        let fill: Vec<Line> = (0..self.area.height)
            .map(|_| Line::from(" ".repeat(self.area.width as usize)))
            .collect();
        let bg = Paragraph::new(fill)
            .style(Style::default().bg(theme.colors.surface))
            .block(Block::default());
        f.render_widget(bg, self.area);

        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|(_, label, is_selected)| {
                let style = if *is_selected {
                    Style::default()
                        .fg(theme.colors.selected_text)
                        .bg(theme.colors.selected)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(theme.colors.text).bg(theme.colors.surface)
                };
                ListItem::new(label.as_str()).style(style)
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .border_style(Style::default().fg(theme.colors.border_focused))
                .bg(theme.colors.surface)
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

    pub fn set_items(&mut self, items: Vec<SelectorItem>) {
        self.items = items;
        if !self.items.is_empty() && self.selected >= self.items.len() {
            self.selected = self.items.len() - 1;
        }
    }

    /// Append a single item to the list, deduplicating by value (used to
    /// keep a "missing" current theme visible after a refresh that omitted
    /// it). Safe to call multiple times - duplicates are skipped.
    pub fn set_items_with_extra(&mut self, item: SelectorItem) {
        if !self.items.iter().any(|i| i.value == item.value) {
            self.items.push(item);
        }
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

    pub fn dropdown_info(&self, anchor_area: Rect, viewport_height: u16) -> Option<DropdownInfo> {
        if !self.expanded {
            return None;
        }

        if self.dropdown_state.borrow().is_none() {
            *self.dropdown_state.borrow_mut() = Some(ListState::default());
        }

        let max_height = 10u16;
        let desired_height = (self.items.len().min(u16::MAX as usize - 2) + 2).min(max_height as usize) as u16;

        let below_y = anchor_area.y.saturating_add(anchor_area.height);
        let below_fits = below_y.saturating_add(desired_height) <= viewport_height;

        let (y, height) = if below_fits {
            (below_y, desired_height)
        } else {
            let available_below = viewport_height.saturating_sub(below_y);
            if available_below >= desired_height {
                (below_y, desired_height)
            } else if available_below >= 3 {
                (below_y, available_below)
            } else {
                let above_y = anchor_area.y.saturating_sub(desired_height);
                if above_y + desired_height <= viewport_height && anchor_area.y >= desired_height {
                    (above_y, desired_height)
                } else {
                    let h = anchor_area.y.min(desired_height);
                    (anchor_area.y.saturating_sub(h), h)
                }
            }
        };

        let dropdown_area = Rect {
            x: anchor_area.x,
            y,
            width: anchor_area.width,
            height: height.max(1),
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
    }

    pub fn focus_open(&mut self) {
        self.focused = true;
        self.open();
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
        if self.expanded {
            if self.confirm().is_none() {
                tracing::debug!("selector confirm returned None (no items or already closed)");
            }
        } else {
            self.open();
        }
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

    pub fn handle_left(&mut self) {
        if self.expanded && !self.items.is_empty() && self.selected > 0 {
            self.selected -= 1;
        }
    }
    pub fn handle_right(&mut self) {
        if self.expanded && !self.items.is_empty() && self.selected < self.items.len() - 1 {
            self.selected += 1;
        }
    }

    pub fn handle_char(&mut self, _c: char) {}
    pub fn handle_backspace(&mut self) {}

    pub fn is_open(&self) -> bool {
        self.expanded
    }

    pub fn open(&mut self) {
        self.expanded = true;
    }

    pub fn close(&mut self) {
        self.expanded = false;
        *self.dropdown_state.borrow_mut() = None;
    }

    pub fn confirm(&mut self) -> Option<&SelectorItem> {
        if self.expanded {
            self.expanded = false;
            *self.dropdown_state.borrow_mut() = None;
            self.items.get(self.selected)
        } else {
            None
        }
    }

    pub fn cancel(&mut self) {
        self.expanded = false;
        *self.dropdown_state.borrow_mut() = None;
    }

    pub fn move_next(&mut self) {
        if self.expanded && !self.items.is_empty() {
            self.selected = (self.selected + 1) % self.items.len();
        }
    }

    pub fn move_prev(&mut self) {
        if self.expanded && !self.items.is_empty() {
            self.selected = if self.selected == 0 {
                self.items.len() - 1
            } else {
                self.selected - 1
            };
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(f, area, &theme);
    }

    pub fn render_with_theme(&self, f: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        let border_style = if self.focused {
            Style::default().fg(theme.colors.border_focused)
        } else {
            Style::default().fg(theme.colors.border)
        };

        let block = Block::default()
            .title(self.label.as_str())
            .borders(Borders::ALL)
            .border_style(border_style);

        let selected_text = self
            .items
            .get(self.selected)
            .map(|i| i.label.as_str())
            .unwrap_or("-");

        let prefix = if self.focused { "▶" } else { "" };
        let arrow = if self.expanded { "▲" } else { "▼" };
        let text = if self.focused {
            format!("{} {} {}", prefix, selected_text, arrow)
        } else {
            format!("{} {}", selected_text, arrow)
        };

        let text_style = if self.focused {
            Style::default()
                .add_modifier(Modifier::BOLD)
                .fg(theme.colors.focus_input)
        } else {
            Style::default().fg(theme.colors.text)
        };

        let paragraph = Paragraph::new(text).style(text_style).block(block);
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

    pub fn reset(&mut self) {
        self.checked = false;
        self.focused = false;
    }

    pub fn toggle(&mut self) {
        self.checked = !self.checked;
    }

    pub fn render(&self, f: &mut Frame, area: Rect) {
        self.render_with_focus(self.focused, f, area);
    }

    pub fn render_with_focus(&self, focused: bool, f: &mut Frame, area: Rect) {
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(focused, f, area, &theme);
    }

    pub fn render_with_theme(&self, focused: bool, f: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        let check = if self.checked { "[✓]" } else { "[ ]" };
        let prefix = if focused { "▶ " } else { "  " };
        let text = format!("{}{}{}", prefix, check, self.label);

        let style = if focused {
            Style::default()
                .fg(theme.colors.focus_input)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(theme.colors.text)
        };

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
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(f, area, &theme);
    }

    pub fn render_with_theme(&self, f: &mut Frame, area: Rect, theme: &crate::theme::Theme) {
        let label_style = if self.focused {
            Style::default().fg(theme.colors.border_focused)
        } else {
            Style::default().fg(theme.colors.border)
        };

        let label_width = self.label.chars().count() + 2;
        let options_per_line = ((area.width as usize).saturating_sub(label_width)) / 12;

        let item_style = |i: usize| -> Style {
            let is_selected = Some(i) == self.selected;
            if is_selected {
                if self.focused {
                    Style::default().fg(theme.colors.accent)
                } else {
                    Style::default().fg(theme.colors.selected)
                }
            } else {
                if self.focused {
                    Style::default().fg(theme.colors.border_focused)
                } else {
                    Style::default().fg(theme.colors.border)
                }
            }
        };

        if options_per_line >= self.options.len() || options_per_line == 0 {
            let spans: Vec<Span> = self
                .options
                .iter()
                .enumerate()
                .map(|(i, opt)| {
                    let is_selected = Some(i) == self.selected;
                    let radio = if is_selected { "◉" } else { "○" };
                    Span::styled(format!(" {} {}", radio, opt), item_style(i))
                })
                .collect();

            let line = Line::from(
                std::iter::once(Span::styled(format!("{}: ", self.label), label_style))
                    .chain(spans)
                    .collect::<Vec<_>>(),
            );

            let paragraph = Paragraph::new(line);
            f.render_widget(paragraph, area);
        } else {
            let mut lines = Vec::new();
            lines.push(Line::from(Span::styled(
                format!("{}: ", self.label),
                label_style,
            )));

            for (chunk_idx, chunk) in self.options.chunks(options_per_line).enumerate() {
                let base_idx = chunk_idx * options_per_line;
                let spans: Vec<Span> = chunk
                    .iter()
                    .enumerate()
                    .map(|(j, opt)| {
                        let is_selected = Some(base_idx + j) == self.selected;
                        let radio = if is_selected { "◉" } else { "○" };
                        Span::styled(format!(" {} {}", radio, opt), item_style(base_idx + j))
                    })
                    .collect();
                lines.push(Line::from(spans));
            }

            let paragraph = Paragraph::new(lines);
            f.render_widget(paragraph, area);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_focus_does_not_change_selection() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        let initial_selected = selector.selected;
        selector.focus();
        assert_eq!(
            selector.selected, initial_selected,
            "Focus should not change selection"
        );
        assert!(!selector.expanded, "Focus should not expand the selector");
    }

    #[test]
    fn selector_focus_open_expands() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.focus_open();
        assert!(selector.expanded, "focus_open should expand the selector");
        assert!(selector.focused, "focus_open should also focus");
    }

    #[test]
    fn selector_toggle_changes_expanded_state() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        assert!(!selector.expanded, "Should start collapsed");
        selector.toggle();
        assert!(selector.expanded, "Toggle should expand");
        selector.toggle();
        assert!(!selector.expanded, "Toggle should collapse");
    }

    #[test]
    fn selector_enter_toggles_expanded() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        assert!(!selector.expanded);
        selector.handle_enter();
        assert!(selector.expanded, "Enter should open closed selector");
        selector.handle_enter();
        assert!(
            !selector.expanded,
            "Enter should confirm and close open selector"
        );
    }

    #[test]
    fn selector_up_moves_selection_when_expanded() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        selector.selected = 2;
        selector.handle_up();
        assert_eq!(selector.selected, 1, "Up should move selection up");
    }

    #[test]
    fn selector_down_moves_selection_when_expanded() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        selector.selected = 0;
        selector.handle_down();
        assert_eq!(selector.selected, 1, "Down should move selection down");
    }

    #[test]
    fn selector_up_does_nothing_when_collapsed() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.selected = 2;
        selector.handle_up();
        assert_eq!(
            selector.selected, 2,
            "Up should not change selection when collapsed"
        );
    }

    #[test]
    fn selector_down_does_nothing_when_collapsed() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.selected = 0;
        selector.handle_down();
        assert_eq!(
            selector.selected, 0,
            "Down should not change selection when collapsed"
        );
    }

    #[test]
    fn selector_collapse_resets_dropdown_state() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        assert!(selector.expanded);
        selector.collapse();
        assert!(!selector.expanded);
    }

    #[test]
    fn selector_next_wraps_around() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        selector.selected = 2;
        selector.next();
        assert_eq!(selector.selected, 0, "Next should wrap to first item");
    }

    #[test]
    fn selector_prev_wraps_around() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        selector.selected = 0;
        selector.prev();
        assert_eq!(selector.selected, 2, "Prev should wrap to last item");
    }

    #[test]
    fn selector_blur_closes_dropdown() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.focus_open();
        assert!(selector.expanded);
        selector.blur();
        assert!(!selector.expanded, "Blur should close dropdown");
        assert!(!selector.focused, "Blur should remove focus");
    }

    #[test]
    fn selector_select_by_value_works() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.select_by_value("B");
        assert_eq!(selector.selected, 1);
        assert_eq!(selector.selected_value(), Some("B"));
    }

    #[test]
    fn selector_select_out_of_range_is_ignored() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.select(99);
        assert_eq!(
            selector.selected, 0,
            "Out of range selection should be ignored"
        );
    }

    #[test]
    fn selector_selected_item_returns_correct_item() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.select(1);
        let item = selector.selected_item().unwrap();
        assert_eq!(item.label, "B");
        assert_eq!(item.value, "B");
    }

    #[test]
    fn selector_empty_items_handles_gracefully() {
        let mut selector = Selector::new("Test");
        selector.next();
        selector.prev();
        assert_eq!(selector.selected, 0);
    }

    #[test]
    fn selector_focus_last_selects_last_item() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.focus_last();
        assert_eq!(selector.selected, 2, "focus_last should select last item");
        assert!(selector.focused);
    }

    #[test]
    fn selector_is_focused_returns_correct_state() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        assert!(!selector.is_focused());
        selector.focus();
        assert!(selector.is_focused());
    }

    #[test]
    fn selector_is_open_returns_correct_state() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        assert!(!selector.is_open(), "Should start closed");
        selector.open();
        assert!(selector.is_open(), "After open() should be open");
        selector.close();
        assert!(!selector.is_open(), "After close() should be closed");
    }

    #[test]
    fn selector_confirm_returns_item_when_open() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.open();
        selector.selected = 1;
        let item = selector.confirm();
        assert!(item.is_some(), "confirm() should return item when open");
        assert_eq!(item.unwrap().value, "B");
        assert!(
            !selector.is_open(),
            "confirm() should close after returning item"
        );
    }

    #[test]
    fn selector_confirm_returns_none_when_closed() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        let item = selector.confirm();
        assert!(item.is_none(), "confirm() should return None when closed");
    }

    #[test]
    fn selector_cancel_closes_without_changing_selection() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.open();
        selector.selected = 2;
        selector.cancel();
        assert!(!selector.is_open(), "cancel() should close");
        assert_eq!(selector.selected, 2, "cancel() should not change selection");
    }

    #[test]
    fn selector_move_next_works() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.open();
        selector.selected = 0;
        selector.move_next();
        assert_eq!(selector.selected, 1);
    }

    #[test]
    fn selector_move_prev_works() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.open();
        selector.selected = 1;
        selector.move_prev();
        assert_eq!(selector.selected, 0);
    }

    #[test]
    fn selector_left_right_do_not_mutate_when_closed() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.selected = 1;
        selector.handle_left();
        assert_eq!(selector.selected, 1);
        selector.handle_right();
        assert_eq!(selector.selected, 1);
    }

    #[test]
    fn selector_move_next_does_nothing_when_closed() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.selected = 0;
        selector.move_next();
        assert_eq!(
            selector.selected, 0,
            "move_next should do nothing when closed"
        );
    }

    #[test]
    fn selector_move_prev_does_nothing_when_closed() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.selected = 1;
        selector.move_prev();
        assert_eq!(
            selector.selected, 1,
            "move_prev should do nothing when closed"
        );
    }

    #[test]
    fn dropdown_info_returns_none_when_collapsed() {
        let selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        let anchor = Rect::new(0, 0, 20, 3);
        assert!(selector.dropdown_info(anchor, 24).is_none());
    }

    #[test]
    fn dropdown_info_fits_below_anchor_when_space_available() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        let anchor = Rect::new(0, 0, 20, 3);
        let info = selector.dropdown_info(anchor, 24).unwrap();
        assert_eq!(info.area.y, 3);
        assert_eq!(info.area.height, 5);
    }

    #[test]
    fn dropdown_info_clamps_height_when_near_bottom() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        let anchor = Rect::new(0, 20, 20, 3);
        let info = selector.dropdown_info(anchor, 24).unwrap();
        assert!(info.area.y + info.area.height <= 24);
        assert!(info.area.height >= 1);
    }

    #[test]
    fn dropdown_info_flips_above_anchor_when_no_space_below() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        let anchor = Rect::new(0, 22, 20, 3);
        let info = selector.dropdown_info(anchor, 24).unwrap();
        assert!(info.area.y + info.area.height <= 24);
        assert!(info.area.y < anchor.y);
    }

    #[test]
    fn dropdown_info_never_exceeds_viewport() {
        let mut selector = Selector::new("Test").simple_items(vec![
            "A", "B", "C", "D", "E", "F", "G", "H", "I", "J",
        ]);
        selector.expand();
        let anchor = Rect::new(0, 0, 20, 3);
        let info = selector.dropdown_info(anchor, 12).unwrap();
        assert!(info.area.y + info.area.height <= 12);
    }

    #[test]
    fn dropdown_info_never_goes_above_y_zero() {
        let mut selector = Selector::new("Test").simple_items(vec!["A", "B", "C"]);
        selector.expand();
        let anchor = Rect::new(0, 0, 20, 3);
        let info = selector.dropdown_info(anchor, 10).unwrap();
        assert!(info.area.y + info.area.height <= 10);
    }

    #[test]
    fn selector_render_with_theme_does_not_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        use crate::theme::palette::{Theme, ThemeMode, ThemeColors};
        use ratatui::style::Color;

        let theme = Theme {
            mode: ThemeMode::Dark,
            name: "test-theme".to_string(),
            colors: ThemeColors {
                primary: Color::Red,
                secondary: Color::Blue,
                accent: Color::Cyan,
                background: Color::Black,
                foreground: Color::White,
                surface: Color::DarkGray,
                border: Color::Gray,
                border_focused: Color::Yellow,
                text: Color::White,
                text_dim: Color::DarkGray,
                text_bright: Color::White,
                success: Color::Green,
                warning: Color::Yellow,
                error: Color::Red,
                info: Color::Cyan,
                selected: Color::Blue,
                selected_text: Color::White,
                highlight: Color::Yellow,
                mode_normal: Color::Green,
                mode_insert: Color::Yellow,
                tab_active: Color::Cyan,
                tab_inactive: Color::Gray,
                status_running: Color::Green,
                status_idle: Color::Gray,
                status_error: Color::Red,
                focus_normal: Color::Green,
                focus_input: Color::Yellow,
                focus_results: Color::Cyan,
                safe: Color::Green,
                danger: Color::Red,
                muted: Color::DarkGray,
                active_task: Color::Green,
                paused_task: Color::Yellow,
                scope_match: Color::Green,
                scope_miss: Color::Red,
                policy_required: Color::Yellow,
                policy_denied: Color::Red,
            },
        };

        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        let selector = Selector::new("Test").simple_items(vec!["Alpha", "Beta", "Gamma"]);
        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 30, 3);
                selector.render_with_theme(f, area, &theme);
            })
            .unwrap();

        let mut expanded = Selector::new("Test").simple_items(vec!["X", "Y"]);
        expanded.focus_open();
        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 30, 3);
                expanded.render_with_theme(f, area, &theme);
            })
            .unwrap();
    }

    #[test]
    fn checkbox_render_with_theme_does_not_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        use crate::theme::palette::{Theme, ThemeMode, ThemeColors};
        use ratatui::style::Color;

        let theme = Theme {
            mode: ThemeMode::Dark,
            name: "test".to_string(),
            colors: ThemeColors {
                primary: Color::Red, secondary: Color::Blue, accent: Color::Cyan,
                background: Color::Black, foreground: Color::White, surface: Color::DarkGray,
                border: Color::Gray, border_focused: Color::Yellow, text: Color::White,
                text_dim: Color::DarkGray, text_bright: Color::White, success: Color::Green,
                warning: Color::Yellow, error: Color::Red, info: Color::Cyan,
                selected: Color::Blue, selected_text: Color::White, highlight: Color::Yellow,
                mode_normal: Color::Green, mode_insert: Color::Yellow,
                tab_active: Color::Cyan, tab_inactive: Color::Gray,
                status_running: Color::Green, status_idle: Color::Gray,
                status_error: Color::Red, focus_normal: Color::Green,
                focus_input: Color::Yellow, focus_results: Color::Cyan,
                safe: Color::Green, danger: Color::Red, muted: Color::DarkGray,
                active_task: Color::Green, paused_task: Color::Yellow,
                scope_match: Color::Green, scope_miss: Color::Red,
                policy_required: Color::Yellow, policy_denied: Color::Red,
            },
        };

        let mut terminal = Terminal::new(TestBackend::new(30, 3)).unwrap();
        let cb = Checkbox::new("Enable feature").checked(true);
        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 30, 1);
                cb.render_with_theme(true, f, area, &theme);
            })
            .unwrap();
        terminal
            .draw(|f| {
                let area = Rect::new(0, 0, 30, 1);
                cb.render_with_theme(false, f, area, &theme);
            })
            .unwrap();
    }
}
