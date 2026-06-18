use crate::components::selector::{Checkbox, DropdownInfo, RadioGroup, Selector};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Style,
    widgets::{Block, Borders, Paragraph},
    Frame,
};
use std::fmt::{Debug, Formatter};

#[derive(Clone, Debug)]
pub struct ValidationResult {
    pub valid: bool,
    pub message: String,
}

pub struct InputField {
    pub label: String,
    pub value: String,
    pub focused: bool,
    pub cursor_pos: usize,
    pub width: Option<usize>,
    pub autocomplete: Option<Vec<&'static str>>,
    pub validation: Option<ValidationResult>,
}

impl Clone for InputField {
    fn clone(&self) -> Self {
        InputField {
            label: self.label.clone(),
            value: self.value.clone(),
            focused: self.focused,
            cursor_pos: self.cursor_pos,
            width: self.width,
            autocomplete: self.autocomplete.clone(),
            validation: self.validation.clone(),
        }
    }
}

impl Debug for InputField {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("InputField")
            .field("label", &self.label)
            .field("value", &self.value)
            .field("focused", &self.focused)
            .field("cursor_pos", &self.cursor_pos)
            .field("width", &self.width)
            .finish()
    }
}

impl InputField {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            value: String::new(),
            focused: false,
            cursor_pos: 0,
            width: None,
            autocomplete: None,
            validation: None,
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        let v = value.into();
        self.cursor_pos = v.len(); // byte index, not char count
        self.value = v;
        self
    }

    pub fn with_width(mut self, width: usize) -> Self {
        self.width = Some(width);
        self
    }

    pub fn with_autocomplete(mut self, completions: Vec<&'static str>) -> Self {
        self.autocomplete = Some(completions);
        self
    }

    pub fn with_validation(mut self, result: ValidationResult) -> Self {
        self.validation = Some(result);
        self
    }

    pub fn get_autocomplete_suggestions(&self) -> Vec<String> {
        if let Some(ref completions) = self.autocomplete {
            if self.value.is_empty() {
                return completions.iter().map(|s| s.to_string()).collect();
            }
            let value_lower = self.value.to_lowercase();
            completions
                .iter()
                .filter(|s| s.to_lowercase().starts_with(&value_lower))
                .map(|s| s.to_string())
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn apply_autocomplete(&mut self, suggestion: &str) {
        self.value = suggestion.to_string();
        self.cursor_pos = self.value.len(); // byte index at end
    }

    pub fn insert(&mut self, c: char) {
        if self.focused {
            self.cursor_pos = self.cursor_pos.min(self.value.len());
            self.value.insert(self.cursor_pos, c);
            self.cursor_pos += c.len_utf8();
        }
    }

    pub fn paste(&mut self, text: &str) {
        if self.focused {
            self.cursor_pos = self.cursor_pos.min(self.value.len());
            for c in text.chars() {
                if c != '\n' && c != '\r' {
                    self.value.insert(self.cursor_pos, c);
                    self.cursor_pos += c.len_utf8();
                }
            }
        }
    }

    pub fn backspace(&mut self) {
        if self.focused && self.cursor_pos > 0 {
            if let Some(prev) = self.value[..self.cursor_pos].chars().next_back() {
                self.cursor_pos -= prev.len_utf8();
                self.value
                    .drain(self.cursor_pos..self.cursor_pos + prev.len_utf8());
            }
        }
    }

    pub fn delete(&mut self) {
        if self.focused && self.cursor_pos < self.value.len() {
            if let Some(next) = self.value[self.cursor_pos..].chars().next() {
                let end = self.cursor_pos + next.len_utf8();
                self.value.drain(self.cursor_pos..end);
            }
        }
    }

    pub fn move_left(&mut self) -> bool {
        if self.cursor_pos > 0 {
            if let Some(prev) = self.value[..self.cursor_pos].chars().next_back() {
                self.cursor_pos -= prev.len_utf8();
                return true;
            }
        }
        false
    }

    pub fn move_right(&mut self) -> bool {
        if self.cursor_pos < self.value.len() {
            if let Some(next) = self.value[self.cursor_pos..].chars().next() {
                self.cursor_pos += next.len_utf8();
                return true;
            }
        }
        false
    }

    pub fn move_home(&mut self) {
        self.cursor_pos = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor_pos = self.value.len(); // byte index at end
    }

    pub fn move_word_forward(&mut self) {
        if self.cursor_pos < self.value.len() {
            let mut found_non_word = false;
            let mut new_pos = self.cursor_pos;

            for (i, c) in self.value[self.cursor_pos..].char_indices() {
                if i == 0 {
                    continue;
                }
                if c.is_whitespace() || c == '/' || c == '.' || c == '-' || c == '_' || c == ':' {
                    found_non_word = true;
                } else if found_non_word {
                    new_pos = self.cursor_pos + i;
                    break;
                }
            }

            if new_pos == self.cursor_pos {
                self.move_end();
            } else {
                self.cursor_pos = new_pos;
            }
        }
    }

    pub fn move_word_backward(&mut self) {
        if self.cursor_pos > 0 {
            let mut found_word = false;
            let mut new_pos = 0;

            for (i, c) in self.value[..self.cursor_pos].char_indices().rev() {
                if !c.is_whitespace() && c != '/' && c != '.' && c != '-' && c != '_' && c != ':' {
                    found_word = true;
                } else if found_word {
                    new_pos = i + c.len_utf8();
                    break;
                }
            }

            self.cursor_pos = new_pos;
        }
    }

    /// Convert byte offset to character position
    fn byte_to_char_pos(&self) -> usize {
        self.value
            .char_indices()
            .take_while(|(i, _)| *i < self.cursor_pos)
            .count()
    }

    pub fn clear(&mut self) {
        self.value.clear();
        self.cursor_pos = 0;
    }

    pub fn is_at_left_edge(&self) -> bool {
        self.cursor_pos == 0
    }

    pub fn is_at_right_edge(&self) -> bool {
        self.cursor_pos >= self.value.len()
    }

    pub fn get_value(&self) -> String {
        self.value.clone()
    }

    pub fn validate_url(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "URL cannot be empty".to_string(),
            };
        }
        if !self.value.starts_with("http://") && !self.value.starts_with("https://") {
            return ValidationResult {
                valid: false,
                message: "URL must start with http:// or https://".to_string(),
            };
        }
        ValidationResult {
            valid: true,
            message: String::new(),
        }
    }

    pub fn validate_ip(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "IP address cannot be empty".to_string(),
            };
        }
        let parts: Vec<&str> = self.value.split('.').collect();
        if parts.len() != 4 {
            return ValidationResult {
                valid: false,
                message: "Invalid IP address format (expected x.x.x.x)".to_string(),
            };
        }
        for part in parts {
            match part.parse::<u8>() {
                Ok(_) => {}
                Err(_) => {
                    return ValidationResult {
                        valid: false,
                        message: format!("Invalid octet: {}", part),
                    };
                }
            }
        }
        ValidationResult {
            valid: true,
            message: String::new(),
        }
    }

    pub fn validate_port(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "Port cannot be empty".to_string(),
            };
        }
        match self.value.parse::<u16>() {
            Ok(port) => {
                if port == 0 {
                    return ValidationResult {
                        valid: false,
                        message: "Port must be between 1-65535".to_string(),
                    };
                }
                ValidationResult {
                    valid: true,
                    message: String::new(),
                }
            }
            Err(_) => ValidationResult {
                valid: false,
                message: "Invalid port number".to_string(),
            },
        }
    }

    pub fn validate_port_range(&self) -> ValidationResult {
        if self.value.is_empty() {
            return ValidationResult {
                valid: false,
                message: "Port range cannot be empty".to_string(),
            };
        }
        if self.value.contains('-') {
            let parts: Vec<&str> = self.value.split('-').collect();
            if parts.len() != 2 {
                return ValidationResult {
                    valid: false,
                    message: "Invalid port range format (expected: start-end)".to_string(),
                };
            }
            match (parts[0].parse::<u16>(), parts[1].parse::<u16>()) {
                (Ok(start), Ok(end)) => {
                    if start > end || start == 0 || end == 0 {
                        return ValidationResult {
                            valid: false,
                            message: "Invalid port range".to_string(),
                        };
                    }
                }
                _ => {
                    return ValidationResult {
                        valid: false,
                        message: "Invalid port numbers in range".to_string(),
                    };
                }
            }
        } else if self.value.contains(',') {
            for port_str in self.value.split(',') {
                match port_str.trim().parse::<u16>() {
                    Ok(0) => {
                        return ValidationResult {
                            valid: false,
                            message: format!("Invalid port: {}", port_str),
                        };
                    }
                    Err(_) => {
                        return ValidationResult {
                            valid: false,
                            message: format!("Invalid port: {}", port_str),
                        };
                    }
                    _ => {}
                }
            }
        }
        ValidationResult {
            valid: true,
            message: String::new(),
        }
    }

    pub fn validate_number(&self, min: u64, max: u64) -> ValidationResult {
        match self.value.parse::<u64>() {
            Ok(n) => {
                if n < min || n > max {
                    return ValidationResult {
                        valid: false,
                        message: format!("Value must be between {} and {}", min, max),
                    };
                }
                ValidationResult {
                    valid: true,
                    message: String::new(),
                }
            }
            Err(_) => ValidationResult {
                valid: false,
                message: "Invalid number".to_string(),
            },
        }
    }

    pub fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(f, area, insert_mode, &theme);
    }

    pub fn render_with_theme(&self, f: &mut Frame, area: Rect, insert_mode: bool, theme: &crate::theme::Theme) {
        let (border_style, title_style) = if self.focused {
            (
                Style::default()
                    .fg(theme.colors.focus_input)
                    .add_modifier(ratatui::style::Modifier::BOLD),
                Style::default()
                    .fg(theme.colors.focus_input)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            )
        } else if let Some(ref validation) = self.validation {
            if validation.valid {
                (
                    Style::default().fg(theme.colors.success),
                    Style::default().fg(theme.colors.text_dim),
                )
            } else {
                (
                    Style::default().fg(theme.colors.error),
                    Style::default()
                        .fg(theme.colors.error)
                        .add_modifier(ratatui::style::Modifier::BOLD),
                )
            }
        } else {
            (
                Style::default().fg(theme.colors.border),
                Style::default().fg(theme.colors.text_dim),
            )
        };

        let title = if self.focused {
            format!("▶ {}", self.label)
        } else {
            self.label.clone()
        };

        let block = Block::default()
            .title(ratatui::text::Span::styled(title, title_style))
            .borders(Borders::ALL)
            .border_style(border_style);

        let cursor_char_pos = self.byte_to_char_pos();
        let char_count = self.value.chars().count();

        let display_value = if let Some(w) = self.width {
            let available = w.saturating_sub(2);
            if char_count > available {
                let start = if cursor_char_pos <= available / 2 {
                    0
                } else if cursor_char_pos >= char_count - available / 2 {
                    char_count.saturating_sub(available)
                } else {
                    cursor_char_pos.saturating_sub(available / 2)
                };
                let end = (start + available).min(char_count);
                let visible: String = self.value.chars().skip(start).take(end - start).collect();
                let prefix = if start > 0 { "..." } else { "" };
                let suffix = if end < char_count { "..." } else { "" };
                format!("{}{}{}", prefix, visible, suffix)
            } else {
                self.value.clone()
            }
        } else {
            self.value.clone()
        };

        let text_style = if self.focused {
            Style::default()
                .add_modifier(ratatui::style::Modifier::BOLD)
                .fg(theme.colors.focus_input)
        } else {
            Style::default().fg(theme.colors.text)
        };

        let paragraph = Paragraph::new(display_value.as_str())
            .style(text_style)
            .block(block);
        f.render_widget(paragraph, area);

        if self.focused && insert_mode {
            let display_cursor = if let Some(w) = self.width {
                let available = w.saturating_sub(2);
                if char_count > available {
                    let start = if cursor_char_pos <= available / 2 {
                        0
                    } else if cursor_char_pos >= char_count - available / 2 {
                        char_count.saturating_sub(available)
                    } else {
                        cursor_char_pos.saturating_sub(available / 2)
                    };
                    let prefix_len = if start > 0 { 3 } else { 0 };
                    if cursor_char_pos >= start && cursor_char_pos < start + available {
                        (cursor_char_pos - start + prefix_len).min(u16::MAX as usize) as u16
                    } else {
                        available.min(u16::MAX as usize) as u16
                    }
                } else {
                    cursor_char_pos.min(u16::MAX as usize) as u16
                }
            } else {
                cursor_char_pos.min(u16::MAX as usize) as u16
            };

            let cursor_x = area.x.saturating_add(display_cursor).saturating_add(1);
            let cursor_y = area.y + 1;
            if cursor_x < area.x + area.width {
                f.set_cursor_position((cursor_x, cursor_y));
            }
        }
    }
}

pub struct InputGroup {
    pub fields: Vec<InputField>,
    pub focused: Option<usize>,
}

impl InputGroup {
    pub fn new() -> Self {
        Self {
            fields: Vec::new(),
            focused: None,
        }
    }

    pub fn add(mut self, field: InputField) -> Self {
        self.fields.push(field);
        self
    }

    /// Return the current focused index if it is valid, or clear stale state and return None.
    fn valid_focused_index(&mut self) -> Option<usize> {
        match self.focused {
            Some(idx) if idx < self.fields.len() => Some(idx),
            Some(_) => {
                self.focused = None;
                None
            }
            None => None,
        }
    }

    /// Read-only version of valid_focused_index that does not mutate self.
    fn valid_focused_index_ref(&self) -> Option<usize> {
        self.focused.filter(|&idx| idx < self.fields.len())
    }

    pub fn focus_next(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].focused = false;
            let next = (idx + 1) % self.fields.len();
            self.fields[next].focused = true;
            self.focused = Some(next);
        } else if !self.fields.is_empty() {
            self.fields[0].focused = true;
            self.focused = Some(0);
        }
    }

    pub fn focus_prev(&mut self) {
        if self.fields.is_empty() {
            return;
        }
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].focused = false;
            let prev = if idx == 0 {
                self.fields.len() - 1
            } else {
                idx - 1
            };
            self.fields[prev].focused = true;
            self.focused = Some(prev);
        } else if !self.fields.is_empty() {
            let last = self.fields.len() - 1;
            self.fields[last].focused = true;
            self.focused = Some(last);
        }
    }

    pub fn focus(&mut self, idx: usize) {
        if idx < self.fields.len() {
            if let Some(current) = self.focused {
                if current < self.fields.len() {
                    self.fields[current].focused = false;
                }
            }
            self.fields[idx].focused = true;
            self.focused = Some(idx);
        }
    }

    pub fn blur(&mut self) {
        if let Some(idx) = self.focused {
            if idx < self.fields.len() {
                self.fields[idx].focused = false;
            }
        }
        self.focused = None;
    }

    pub fn insert(&mut self, c: char) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].insert(c);
        }
    }

    pub fn paste(&mut self, text: &str) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].paste(text);
        }
    }

    pub fn backspace(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].backspace();
        }
    }

    pub fn delete(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].delete();
        }
    }

    pub fn handle_autocomplete(&mut self) -> bool {
        if let Some(idx) = self.valid_focused_index() {
            let suggestions = self.fields[idx].get_autocomplete_suggestions();
            if let Some(first) = suggestions.first() {
                self.fields[idx].apply_autocomplete(first);
                return true;
            }
        }
        false
    }

    pub fn move_left(&mut self) -> bool {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_left()
        } else {
            false
        }
    }

    pub fn move_right(&mut self) -> bool {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_right()
        } else {
            false
        }
    }

    pub fn move_word_forward(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_word_forward();
        }
    }

    pub fn move_word_backward(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_word_backward();
        }
    }

    pub fn move_home(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_home();
        }
    }

    pub fn move_end(&mut self) {
        if let Some(idx) = self.valid_focused_index() {
            self.fields[idx].move_end();
        }
    }

    pub fn get_focused_value(&self) -> Option<String> {
        self.valid_focused_index_ref().map(|idx| self.fields[idx].get_value())
    }

    pub fn is_focused(&self) -> bool {
        self.focused.is_some()
    }

    pub fn is_at_left_edge(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].is_at_left_edge()
        } else {
            true
        }
    }

    pub fn is_at_right_edge(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].is_at_right_edge()
        } else {
            true
        }
    }

    pub fn can_move_left(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].cursor_pos > 0
        } else {
            false
        }
    }

    pub fn can_move_right(&self) -> bool {
        if let Some(idx) = self.valid_focused_index_ref() {
            self.fields[idx].cursor_pos < self.fields[idx].value.len()
        } else {
            false
        }
    }
}

impl Default for InputGroup {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug)]
pub enum FieldVariant {
    Input(InputField),
    Checkbox(Checkbox),
    Selector(Selector),
    RadioGroup(RadioGroup),
}

pub struct FormBuilder {
    title: String,
    fields: Vec<FieldVariant>,
    row_height: u16,
}

impl FormBuilder {
    pub fn new(title: &str) -> Self {
        Self {
            title: title.to_string(),
            fields: Vec::new(),
            row_height: 3,
        }
    }

    pub fn add_input(mut self, field: InputField) -> Self {
        self.fields.push(FieldVariant::Input(field));
        self
    }

    pub fn add_checkbox(mut self, cb: Checkbox) -> Self {
        self.fields.push(FieldVariant::Checkbox(cb));
        self
    }

    pub fn add_selector(mut self, sel: Selector) -> Self {
        self.fields.push(FieldVariant::Selector(sel));
        self
    }

    pub fn add_radio(mut self, rg: RadioGroup) -> Self {
        self.fields.push(FieldVariant::RadioGroup(rg));
        self
    }

    pub fn row_height(mut self, height: u16) -> Self {
        self.row_height = height;
        self
    }

    fn calculate_constraints(&self) -> Vec<Constraint> {
        self.fields
            .iter()
            .map(|field| match field {
                FieldVariant::Input(_) => Constraint::Length(self.row_height),
                FieldVariant::Checkbox(_) => Constraint::Length(2),
                FieldVariant::Selector(_) => Constraint::Length(3),
                FieldVariant::RadioGroup(_) => Constraint::Length(2),
            })
            .collect()
    }

    pub fn collect_dropdowns(&self, area: Rect, viewport_height: u16) -> Vec<DropdownInfo> {
        let theme = crate::theme::legacy::current_theme();
        self.collect_dropdowns_with_theme(area, viewport_height, &theme)
    }

    pub fn collect_dropdowns_with_theme(&self, area: Rect, viewport_height: u16, theme: &crate::theme::Theme) -> Vec<DropdownInfo> {
        self.fields
            .iter()
            .enumerate()
            .filter_map(|(i, field)| {
                if let FieldVariant::Selector(sel) = field {
                    let constraints = self.calculate_constraints();
                    let block = Block::default()
                        .title(self.title.as_str())
                        .borders(Borders::ALL)
                        .border_style(Style::default().fg(theme.colors.border));
                    let inner = block.inner(area);
                    let mut anchor_y = inner.y;
                    for j in 0..i {
                        let h = match constraints.get(j) {
                            Some(Constraint::Length(h)) => *h,
                            _ => 3,
                        };
                        anchor_y = anchor_y.saturating_add(h);
                    }
                    let chunk_height = match constraints.get(i) {
                        Some(Constraint::Length(h)) => *h,
                        _ => 3,
                    };
                    let anchor = Rect {
                        x: inner.x,
                        y: anchor_y,
                        width: inner.width,
                        height: chunk_height,
                    };
                    sel.dropdown_info(anchor, viewport_height)
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn render(&self, f: &mut Frame, area: Rect, insert_mode: bool) {
        let theme = crate::theme::legacy::current_theme();
        self.render_with_theme(f, area, insert_mode, &theme);
    }

    pub fn render_with_theme(&self, f: &mut Frame, area: Rect, insert_mode: bool, theme: &crate::theme::Theme) {
        let block = Block::default()
            .title(self.title.as_str())
            .borders(Borders::ALL)
            .border_style(Style::default().fg(theme.colors.border));

        let inner = block.inner(area);
        f.render_widget(block, area);

        let constraints = self.calculate_constraints();
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(inner);

        for (i, field) in self.fields.iter().enumerate() {
            if let Some(chunk) = chunks.get(i) {
                match field {
                    FieldVariant::Input(input) => input.render_with_theme(f, *chunk, insert_mode, theme),
                    FieldVariant::Checkbox(cb) => cb.render_with_theme(cb.focused, f, *chunk, theme),
                    FieldVariant::Selector(sel) => sel.render_with_theme(f, *chunk, theme),
                    FieldVariant::RadioGroup(rg) => rg.render_with_theme(f, *chunk, theme),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_with_value_sets_byte_cursor() {
        // "éx" is 3 bytes: é=2 bytes, x=1 byte
        // chars().count() would be 2, but we want value.len() = 3
        let field = InputField::new("Test").with_value("éx");
        assert_eq!(field.cursor_pos, "éx".len()); // 3, not 2
        assert_ne!(field.cursor_pos, "éx".chars().count()); // Ensure it's not 2
    }

    #[test]
    fn test_insert_in_middle_of_multibyte() {
        let mut field = InputField::new("Test").with_value("éx");
        field.focused = true; // Need to focus for insert/backspace to work
                              // cursor is at end (byte 3)
                              // move left to be between é and x (byte 2)
        field.move_left();
        assert_eq!(field.cursor_pos, 2); // between é (bytes 0-1) and x (byte 2)

        // Insert 'a' at cursor position
        field.insert('a');
        assert_eq!(field.value, "éax");
        assert_eq!(field.cursor_pos, 3); // after 'a' (byte 3)
    }

    #[test]
    fn test_backspace_deletes_character_not_byte() {
        let mut field = InputField::new("Test").with_value("éx");
        field.focused = true; // Need to focus for insert/backspace to work
                              // cursor at end (byte 3)
        field.backspace();
        // Should delete 'x' (1 byte), not just one byte of 'é'
        assert_eq!(field.value, "é");
        assert_eq!(field.cursor_pos, 2); // byte position of 'é'

        // Now backspace again to delete 'é' (2 bytes)
        field.backspace();
        assert_eq!(field.value, "");
        assert_eq!(field.cursor_pos, 0);
    }

    #[test]
    fn test_move_end_then_insert() {
        let mut field = InputField::new("Test").with_value("éx");
        field.focused = true; // Need to focus for insert to work
        field.move_home();
        assert_eq!(field.cursor_pos, 0);

        field.move_end();
        assert_eq!(field.cursor_pos, "éx".len()); // 3

        field.insert('a');
        assert_eq!(field.value, "éxa");
        assert_eq!(field.cursor_pos, 4);
    }

    #[test]
    fn test_render_long_multibyte_no_panic() {
        use ratatui::{backend::TestBackend, Terminal};
        let mut terminal = Terminal::new(TestBackend::new(20, 3)).unwrap();

        let mut field = InputField::new("Test").with_value("ééééééééééé"); // many multibyte chars
        field.width = Some(20);
        field.focused = true;

        // This should not panic
        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 20, 3);
                field.render(f, area, true);
            })
            .unwrap();
    }

    #[test]
    fn test_byte_to_char_pos() {
        let field = InputField::new("Test").with_value("éx");
        // cursor at end (byte 3)
        let char_pos = field.byte_to_char_pos();
        assert_eq!(char_pos, 2); // 2 characters

        let mut field2 = InputField::new("Test").with_value("abc");
        field2.move_home();
        assert_eq!(field2.byte_to_char_pos(), 0);

        field2.move_end();
        assert_eq!(field2.byte_to_char_pos(), 3);
    }

    #[test]
    fn test_render_with_theme_does_not_panic() {
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

        let mut terminal = Terminal::new(TestBackend::new(30, 5)).unwrap();
        let mut field = InputField::new("Target").with_value("192.168.1.1");
        field.focused = true;
        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 30, 3);
                field.render_with_theme(f, area, true, &theme);
            })
            .unwrap();

        let unfocused = InputField::new("Port").with_value("80");
        terminal
            .draw(|f| {
                let area = ratatui::layout::Rect::new(0, 0, 30, 3);
                unfocused.render_with_theme(f, area, false, &theme);
            })
            .unwrap();
    }

    #[test]
    fn stale_focus_insert_does_not_panic() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"));
        group.focused = Some(99); // stale index
        group.insert('a'); // should not panic
        assert!(group.focused.is_none()); // cleared by valid_focused_index
    }

    #[test]
    fn stale_focus_blur_does_not_panic() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"));
        group.focused = Some(99);
        group.blur(); // should not panic
        assert!(group.focused.is_none());
    }

    #[test]
    fn stale_focus_move_left_does_not_panic() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"));
        group.focused = Some(99);
        assert!(!group.move_left());
        assert!(group.focused.is_none());
    }

    #[test]
    fn stale_focus_get_focused_value_returns_none() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"));
        group.focused = Some(99);
        assert!(group.get_focused_value().is_none());
        // Note: get_focused_value uses the read-only helper, so focused is not cleared.
        // Use insert/blur to clear stale focus if needed.
    }

    #[test]
    fn focus_next_recovers_from_stale_focus() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"))
            .add(InputField::new("Field 2"));
        group.focused = Some(99); // stale
        group.focus_next();
        assert_eq!(group.focused, Some(0)); // recovered to first field
        assert!(group.fields[0].focused);
    }

    #[test]
    fn focus_prev_recovers_from_stale_focus() {
        let mut group = InputGroup::new()
            .add(InputField::new("Field 1"))
            .add(InputField::new("Field 2"));
        group.focused = Some(99); // stale
        group.focus_prev();
        assert_eq!(group.focused, Some(1)); // recovered to last field
        assert!(group.fields[1].focused);
    }
}
